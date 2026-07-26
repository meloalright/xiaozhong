//! xiaozhongsi.sh 一肩挑：
//!   ssh xiaozhongsi.sh  → 可走动的小钟寺，同时在里面的人彼此看得见
//!   curl xiaozhongsi.sh → 一句提示，指向 ssh
//! 钟声簿与头像都是本地文件，跑在 Fly 单实例上，挂 volume 持久化。

mod avatars;
mod counter;
mod sha256;
mod site;
mod space;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use avatars::Avatars;
use counter::Counter;
use space::{Action, Choosing, World};
use russh::keys::{HashAlg, PrivateKey};
use russh::server::{Auth, Handler, Msg, Session};
use russh::MethodKind;
use russh::{Channel, ChannelId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// 沒帶公鑰時的謝客話：認人靠公鑰指紋，無鑰不記名，教他鑄一把再來
const NO_KEY: &str = "\
寺曰：無鑰之人 · 認你不得 · 鐘聲不敢記名 🔔\r\n\
\r\n\
且先鑄一把金鑰：\r\n\
\r\n\
    ssh-keygen -t ed25519\r\n\
\r\n\
一路 Enter 即可。鑄畢再來 · ssh xiaozhongsi.sh 便能撞鐘。";

/// 带命令、或没有终端的连接：香得自己进廟上，不能隔空代拜
const NEED_TTY: &str = "\
寺曰：心誠須親至 · 隔空代撞不算數 🔔\r\n\
\r\n\
請在終端直接連入 · 不要附帶命令：\r\n\
\r\n\
    ssh xiaozhongsi.sh";

/// 离廟时清屏、恢复光标
const LEAVE: &str = "\x1b[2J\x1b[H\x1b[?25h鐘聲遠去 · 慢走 🔔\r\n";

/// 每次进寺最多待一分钟，时辰一到自动送客——免得有人长占席位
const SESSION_CAP: std::time::Duration = std::time::Duration::from_secs(60);
const TIME_UP: &str =
    "\x1b[2J\x1b[H\x1b[?25h一炷香的緣分已滿 · 鐘樓暫別 · 慢走 🔔\r\n";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ssh_ports: Vec<u16> = env_ports("SSH_PORT", "2222");
    let http_ports: Vec<u16> = env_ports("HTTP_PORT", "8080");
    let host_key_path =
        PathBuf::from(std::env::var("XIAOZHONGSI_HOST_KEY").unwrap_or_else(|_| "host_key".into()));
    let data_dir = PathBuf::from(std::env::var("XIAOZHONGSI_DATA_DIR").unwrap_or_else(|_| "data".into()));
    let salt = std::env::var("XIAOZHONGSI_SALT").unwrap_or_else(|_| "xiaozhongsi".into());

    let counter = Arc::new(Mutex::new(Counter::open(&data_dir, salt)?));
    let avatars = {
        let mut a = Avatars::open(&data_dir)?;
        a.compact_if_needed();
        Arc::new(Mutex::new(a))
    };

    let config = Arc::new(russh::server::Config {
        inactivity_timeout: Some(TIMEOUT),
        auth_rejection_time: std::time::Duration::from_secs(1),
        // ssh 客户端总是先发一次 none 探测可用认证方式，我们必然拒绝它。
        // 不置零的话每个连接都要白等 auth_rejection_time，实测每次约 1.04 秒。
        auth_rejection_time_initial: Some(std::time::Duration::ZERO),
        keys: vec![load_or_create_host_key(&host_key_path)?],
        ..Default::default()
    });

    let world = Arc::new(Mutex::new(World::default()));
    let (tick, _) = tokio::sync::broadcast::channel::<bool>(64);
    let next_id = Arc::new(std::sync::atomic::AtomicU64::new(1));

    let temple = Temple {
        counter: Arc::clone(&counter),
        avatars: Arc::clone(&avatars),
        fingerprint: None,
        has_pty: false,
        world: Arc::clone(&world),
        tick: tick.clone(),
        id: 0,
        choosing: None,
    };

    let mut tasks = Vec::new();

    // 同时在处理的 SSH 会话上限。满了就直接断开新连接：
    // 与其所有人一起排队到超时，不如少数被快速拒绝、其余正常撞钟。
    let max_sessions: usize = std::env::var("XIAOZHONGSI_MAX_SESSIONS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(128);
    let gate = Arc::new(tokio::sync::Semaphore::new(max_sessions));
    println!("[小鐘寺] ssh  并发上限 {max_sessions}");

    for port in ssh_ports {
        let socket = TcpListener::bind(("0.0.0.0", port)).await?;
        let config = Arc::clone(&config);
        let temple = temple.clone();
        let gate = Arc::clone(&gate);
        let next_id = Arc::clone(&next_id);
        let world = Arc::clone(&world);
        let tick = tick.clone();
        println!("[小鐘寺] ssh  门开在 {port}");
        tasks.push(tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = socket.accept().await else {
                    continue;
                };
                // try_acquire 不排队：满了立刻放弃这个连接
                let Ok(permit) = Arc::clone(&gate).try_acquire_owned() else {
                    drop(stream);
                    continue;
                };
                let config = Arc::clone(&config);
                let mut temple = temple.clone();
                temple.id = next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let id = temple.id;
                let world = Arc::clone(&world);
                let tick = tick.clone();
                tokio::spawn(async move {
                    let _permit = permit; // 会话结束才归还名额
                    match russh::server::run_stream(config, stream, temple).await {
                        Ok(session) => {
                            let _ = session.await;
                        }
                        Err(e) => eprintln!("[小鐘寺] 会话建立失败: {e}"),
                    }
                    // 断线也要把人从廟里移走，否则会留下幽灵
                    world.lock().unwrap_or_else(|e| e.into_inner()).leave(id);
                    let _ = tick.send(false);
                });
            }
        }));
    }

    for port in http_ports {
        let socket = TcpListener::bind(("0.0.0.0", port)).await?;
        println!("[小鐘寺] http 门开在 {port}");
        tasks.push(tokio::spawn(async move { serve_http(socket).await }));
    }

    for t in tasks {
        let _ = t.await;
    }
    Ok(())
}

fn env_ports(name: &str, default: &str) -> Vec<u16> {
    std::env::var(name)
        .unwrap_or_else(|_| default.into())
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect()
}

/// Fly 在 443 终结 TLS 后转发明文 HTTP 到这里，所以只做纯 HTTP。
async fn serve_http(socket: TcpListener) {
    loop {
        let Ok((mut stream, _)) = socket.accept().await else {
            continue;
        };
        tokio::spawn(async move {
            // 加超时：只连不发的客户端否则会把 task 和 socket 永久占住
            let mut buf = vec![0u8; 4096];
            let n = match tokio::time::timeout(TIMEOUT, stream.read(&mut buf)).await {
                Ok(Ok(n)) if n > 0 => n,
                _ => return,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let mut lines = req.lines();
            let (method, path) = match lines.next().and_then(parse_request_line) {
                Some(v) => v,
                None => return,
            };
            let ua = lines
                .find_map(|l| {
                    let (k, v) = l.split_once(':')?;
                    k.eq_ignore_ascii_case("user-agent").then(|| v.trim())
                })
                .unwrap_or("");

            let (status, ctype, body) = if method != "GET" && method != "HEAD" {
                (405, "text/plain; charset=utf-8", "小鐘寺只受 GET 之禮\n".to_string())
            } else {
                site::route(&path, ua)
            };

            let head = format!(
                "HTTP/1.1 {status} {}\r\ncontent-type: {ctype}\r\ncontent-length: {}\r\ncache-control: no-store\r\nconnection: close\r\n\r\n",
                reason(status),
                body.len()
            );
            let _ = stream.write_all(head.as_bytes()).await;
            if method != "HEAD" {
                let _ = stream.write_all(body.as_bytes()).await;
            }
            let _ = stream.shutdown().await;
        });
    }
}

fn parse_request_line(line: &str) -> Option<(String, String)> {
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?;
    let path = target.split('?').next().unwrap_or(target).to_string();
    Some((method, path))
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "OK",
    }
}

/// 主机密钥必须固定，换了所有拜过的人都会看到大红警告。
/// 优先读环境变量里的 OpenSSH 私钥全文，这样跑在无状态容器里也不用挂盘。
fn load_or_create_host_key(path: &PathBuf) -> Result<PrivateKey, Box<dyn std::error::Error>> {
    if let Ok(pem) = std::env::var("XIAOZHONGSI_HOST_KEY_PEM") {
        if !pem.trim().is_empty() {
            return Ok(PrivateKey::from_openssh(pem.replace("\\n", "\n").as_bytes())?);
        }
    }
    if path.exists() {
        return Ok(PrivateKey::read_openssh_file(path)?);
    }
    let key = PrivateKey::random(&mut rand::rng(), russh::keys::Algorithm::Ed25519)?;
    key.write_openssh_file(path, russh::keys::ssh_key::LineEnding::LF)?;
    println!("[小鐘寺] 已生成主机密钥 {}", path.display());
    Ok(key)
}

struct Temple {
    counter: Arc<Mutex<Counter>>,
    avatars: Arc<Mutex<Avatars>>,
    /// 本次连接用的公钥指纹，认证时记下
    fingerprint: Option<String>,
    /// 客户端有没有申请 PTY。没有就说明是脚本或 agent，不放进寺里。
    has_pty: bool,
    /// 廟里所有人共享一份，谁动了都要给所有人重画
    world: Arc<Mutex<World>>,
    /// 有人变动就往这里喊一声，各会话的推帧任务据此重绘
    tick: tokio::sync::broadcast::Sender<bool>,
    /// 本次会话在世界里的编号
    id: space::Id,
    /// 还没进世界时的选头像状态。会话私有，所以不参与 Clone。
    choosing: Option<Choosing>,
}

/// 克隆出来的是一份全新会话：共享的东西继续共享，身份与状态一律归零
impl Clone for Temple {
    fn clone(&self) -> Self {
        Self {
            counter: Arc::clone(&self.counter),
            avatars: Arc::clone(&self.avatars),
            fingerprint: None,
            has_pty: false,
            world: Arc::clone(&self.world),
            tick: self.tick.clone(),
            id: 0,
            choosing: None,
        }
    }
}

impl Handler for Temple {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        _channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    /// 来者皆是信众，不设门槛；公钥只用来认人，不做授权
    async fn auth_publickey(
        &mut self,
        _user: &str,
        key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<Auth, Self::Error> {
        self.fingerprint = Some(key.fingerprint(HashAlg::Sha256).to_string());
        Ok(Auth::Accept)
    }

    /// ssh 客户端总是先试 none。这里拒掉并要求 publickey；
    /// 沒公鑰的退到 keyboard-interactive 也放進來，但進來後只勸他去鑄鑰、不記鐘聲。
    async fn auth_none(&mut self, _user: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Reject {
            proceed_with_methods: Some(
                [MethodKind::PublicKey, MethodKind::KeyboardInteractive]
                    .as_slice()
                    .into(),
            ),
            partial_success: false,
        })
    }

    /// 沒有公鑰的香客走這條路，放行但不認人（fingerprint 保持 None）
    async fn auth_keyboard_interactive<'a>(
        &'a mut self,
        _user: &str,
        _submethods: &str,
        _response: Option<russh::server::Response<'a>>,
    ) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _term: &str,
        _cw: u32,
        _rh: u32,
        _pw: u32,
        _ph: u32,
        _modes: &[(russh::Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.has_pty = true;
        session.channel_success(channel)?;
        Ok(())
    }

    /// 有 PTY 就进寺；没有（脚本、agent）一律谢客
    async fn shell_request(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.channel_success(channel)?;
        if self.has_pty && self.fingerprint.is_some() {
            self.enter_space(channel, session)
        } else {
            self.turn_away(channel, session)
        }
    }

    /// `ssh xiaozhongsi.sh <命令>` 不再代撞——钟得自己进寺敲
    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.turn_away(channel, session)
    }

    /// 交互空间的按键都从这里进来
    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        for key in space::parse_keys(data) {
            // 还在选头像：选定后才落地进世界
            if let Some(c) = self.choosing.as_mut() {
                match c.handle(key) {
                    Some(picked) => {
                        if let Some(id) = self.pilgrim_id() {
                            self.avatars
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .set(&id, picked);
                        }
                        self.choosing = None;
                        self.join_world(picked);
                        self.push(channel, session, false)?;
                    }
                    None => {
                        let frame = c.render();
                        session.data(channel, frame.into_bytes())?;
                    }
                }
                continue;
            }

            let action = self
                .world
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .handle(self.id, key);
            match action {
                Action::Idle => {}
                Action::Redraw => self.push(channel, session, false)?,
                Action::Ring => {
                    let blessing = self.record_ring();
                    self.world
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .set_blessing(self.id, blessing);
                    self.push(channel, session, true)?; // true = 给全场发响铃
                }
                Action::Burn => {
                    let blessing = self.record_burn();
                    self.world
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .set_blessing(self.id, blessing);
                    self.push(channel, session, false)?;
                }
                Action::Leave => {
                    self.world
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .leave(self.id);
                    let _ = self.tick.send(false);
                    session.data(channel, LEAVE.as_bytes().to_vec())?;
                    session.exit_status_request(channel, 0)?;
                    session.eof(channel)?;
                    session.close(channel)?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }
}

impl Temple {
    /// 进廟：首次来的公钥先选头像，认识的直接落进廟里
    fn enter_space(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), russh::Error> {
        let known = self.pilgrim_id().and_then(|id| {
            self.avatars
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .get(&id)
        });

        // 别人走动时也要给我重画，所以开一个推帧任务盯着世界变化
        self.watch(channel, session);
        // 一分钟时辰到就自动送客
        self.arm_session_cap(channel, session);

        match known {
            Some(avatar) => {
                self.join_world(avatar);
                self.push(channel, session, false)
            }
            None => {
                let c = Choosing::new();
                session.data(channel, c.render().into_bytes())?;
                self.choosing = Some(c);
                Ok(())
            }
        }
    }

    fn join_world(&mut self, avatar: usize) {
        self.world
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .join(self.id, avatar);
        let _ = self.tick.send(false);
    }

    /// 自己动了：先画给自己，再喊一声让别人也重画。
    /// bell=true 时前置一个终端响铃字符（\x07），敲钟时用。
    fn push(&self, channel: ChannelId, session: &mut Session, bell: bool) -> Result<(), russh::Error> {
        let frame = self
            .world
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .render(self.id);
        let mut out = Vec::new();
        if bell {
            out.push(0x07);
        }
        out.extend_from_slice(frame.as_bytes());
        session.data(channel, out)?;
        let _ = self.tick.send(bell);
        Ok(())
    }

    /// 盯着世界变化，别人动了就把新画面推给我
    fn watch(&self, channel: ChannelId, session: &mut Session) {
        let handle = session.handle();
        let world = Arc::clone(&self.world);
        let mut rx = self.tick.subscribe();
        let id = self.id;
        tokio::spawn(async move {
            while let Ok(bell) = rx.recv().await {
                let frame = {
                    let w = world.lock().unwrap_or_else(|e| e.into_inner());
                    // 自己已经离寺就不必再推了
                    if !w.is_in(id) {
                        break;
                    }
                    w.render(id)
                };
                let mut out = Vec::new();
                if bell {
                    out.push(0x07); // 别人敲钟，我这边也响一声
                }
                out.extend_from_slice(frame.as_bytes());
                if handle.data(channel, out).await.is_err() {
                    break;
                }
            }
        });
    }

    /// armed 一分钟定时器：时辰到就把人移出世界、送一句话、关闭连接
    fn arm_session_cap(&self, channel: ChannelId, session: &mut Session) {
        let handle = session.handle();
        let world = Arc::clone(&self.world);
        let tick = self.tick.clone();
        let id = self.id;
        tokio::spawn(async move {
            tokio::time::sleep(SESSION_CAP).await;
            let was_in = {
                let mut w = world.lock().unwrap_or_else(|e| e.into_inner());
                let inside = w.is_in(id);
                if inside {
                    w.leave(id);
                }
                inside
            };
            if was_in {
                let _ = tick.send(false); // 让还在寺里的人看到我离开
            }
            let _ = handle.data(channel, TIME_UP.as_bytes().to_vec()).await;
            let _ = handle.close(channel).await;
        });
    }

    /// 本次会话的落盘键：公钥指纹加盐哈希，和鐘聲簿用的是同一个
    fn pilgrim_id(&self) -> Option<String> {
        let fp = self.fingerprint.as_ref()?;
        let id = self
            .counter
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .id_of(fp);
        Some(id)
    }

    /// 记一次撞钟，返回给本人的那句祈愿
    fn record_ring(&mut self) -> String {
        let Some(fp) = self.fingerprint.clone() else {
            return String::new();
        };
        let mut c = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        let s = c.ring(&fp);
        if s.visits > 1 {
            format!("你今天第 {} 次撞鐘 · 今天共 {} 位撞過鐘 🔔", s.visits, s.total)
        } else {
            format!("你是今天第 {} 位撞鐘 🔔", s.rank)
        }
    }

    /// 记一次烧香，返回给本人的那句祈愿
    fn record_burn(&mut self) -> String {
        let Some(fp) = self.fingerprint.clone() else {
            return String::new();
        };
        let mut c = self.counter.lock().unwrap_or_else(|e| e.into_inner());
        let s = c.burn(&fp);
        if s.visits > 1 {
            format!("你今天第 {} 炷香 · 今天共 {} 位燒過香 🔥", s.visits, s.total)
        } else {
            format!("你是今天第 {} 位燒香 🔥", s.rank)
        }
    }

    /// 非交互的连接一律谢客：香得自己进廟上，不代拜也不记账
    fn turn_away(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), russh::Error> {
        session.channel_success(channel)?;
        let line = match &self.fingerprint {
            Some(_) => NEED_TTY,
            None => NO_KEY,
        };
        session.data(channel, format!("{line}\r\n").into_bytes())?;
        session.exit_status_request(channel, 0)?;
        session.eof(channel)?;
        session.close(channel)?;
        Ok(())
    }
}
