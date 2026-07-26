//! 终端里的小钟寺：选头像、走动、撞钟、从下方或左右侧门离开。
//! 同时在寺里的人共享一个世界，彼此看得见。
//! 只在客户端申请了 PTY 时启用；无 PTY 的连接一律谢客。

use std::collections::HashMap;

/// 可选头像，首次进廟时挑一个，之后按公钥记住。
/// 全部是单码位 emoji：ZWJ 组合序列（如 👩‍🦰）在不同终端会拆成两个字形、
/// 宽度从 2 格变 4 格，会把地图撑歪。
pub const AVATARS: &[&str] = &["🧑", "👧", "🧝", "🧛", "👻", "🤖"];

/// 选头像界面每行摆几个。摆太多会超出终端宽度。
const PER_ROW: usize = 10;

/// 按下标取头像。删过头像的话，旧记录里的下标可能已经越界，回落到第一个。
fn avatar_of(i: usize) -> &'static str {
    AVATARS.get(i).copied().unwrap_or(AVATARS[0])
}

// 場景全用 ASCII，每個圖塊正好 2 個字元，才能和 2 格寬的 emoji 對齊成方格。
/// 星空只是画在廟上方的装饰。它和地图叠成一整摞“世界行”，
/// 一起被相机视窗上下裁剪——挡路仍由地图第一行担着（r == 0 不能再往上）。
const SKY: [&str; 4] = [
    "  ✧         ✦",
    "     *    .",
    ".        ★       ✧",
    "   ✦          *",
];

/// 相机视窗的高度（行数）。比整摞世界矮，角色走动时视窗上下跟随滚动。
const VIEW_H: usize = 9;
/// 一口钟，悬在寺前正中。emoji 占 2 格，和地图对齐。
const BELL: &str = "🔔";
/// 广场上的香炉。单码位 🔥，稳稳占 2 格，不像三竖线那样宽度不定。
const FIRE: &str = "🔥";
/// 广场外圈的花篱。两个纯 ASCII 字符恒定 2 列，稳过歧义宽度的花朵符号。
const FLOWER: &str = "*,";
/// 空庭：地面留白，只有星光、钟和人
const FLOOR: &str = "  ";
/// 窗棂格子墙
const LATTICE: &str = "++";

pub const W: usize = 9;
/// 上 7 排是庙堂，下 5 排是广场，中间靠寺门相连
pub const H: usize = 12;

/// 平面图：
///   'B' 钟（顶排，从左右或正下方敲）   'F' 香炉 🔥（广场顶排正中，从正下方烧香）
///   '.' 可走的空地    '+' 窗棂格子墙    'f' 花朵篱笆（挡路）
/// 顶排敞开：从钟的两侧一直走到左右边缘，走出去即离寺。
/// 广场外圈用花篱围住，香炉那行（第 7 排）两端留口，下缘中间留门。
/// 寺门（第 6 排缺口）只是通往广场的过道，不再离寺。
const MAP: [[char; W]; H] = [
    ['.', '.', '.', '.', 'B', '.', '.', '.', '.'], // 0 钟 · 两侧走到边缘即离寺
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 1
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 2
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 3
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 4
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 5
    ['+', '+', '+', '.', '.', '.', '+', '+', '+'], // 6 寺门 → 广场
    ['.', '.', '.', '.', 'F', '.', '.', '.', '.'], // 7 香炉 🔥 · 两端开口出入
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 8 香炉正下方可烧香
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 9 出生点在正中
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 10
    ['f', 'f', 'f', '.', '.', '.', 'f', 'f', 'f'], // 11 花篱下缘 · 中间开口出寺（和寺门一样大）
];

/// 钟的位置，从它左右两格敲
const BELL_AT: (usize, usize) = (0, 4);
/// 香炉的位置，从它正下方一格烧香
const CENSER_AT: (usize, usize) = (7, 4);
/// 进寺时的落脚点：香炉正下方两格，广场中央偏上
const START_AT: (usize, usize) = (9, 4);

pub type Id = u64;

#[derive(Clone)]
struct Pilgrim {
    avatar: usize,
    at: (usize, usize),
    /// 正在撞钟
    ringing: bool,
    /// 正在烧香
    burning: bool,
    /// 撞钟/烧香后要显示给本人的那句话
    blessing: Option<String>,
}

/// 廟里此刻的所有人。所有会话共享一份，谁动了都要重画。
#[derive(Default)]
pub struct World {
    pilgrims: HashMap<Id, Pilgrim>,
}

pub enum Action {
    Idle,
    Redraw,
    /// 撞钟达成，记一次，并给全场发响铃
    Ring,
    /// 烧香达成，给本人一句祈愿，全场看到香煙
    Burn,
    /// 离寺，结束会话
    Leave,
}

impl World {
    pub fn join(&mut self, id: Id, avatar: usize) {
        let at = self.spawn_cell();
        self.pilgrims.insert(
            id,
            Pilgrim {
                avatar,
                at,
                ringing: false,
                burning: false,
                blessing: None,
            },
        );
    }

    /// 落脚点：优先 START_AT，被占了就找离它最近的空地。
    /// 全都占满（几乎不会）才退回 START_AT 叠一格。
    fn spawn_cell(&self) -> (usize, usize) {
        if !self.occupied(START_AT) {
            return START_AT;
        }
        let mut best: Option<((usize, usize), usize)> = None;
        for (r, row) in MAP.iter().enumerate() {
            for (c, &tile) in row.iter().enumerate() {
                if tile != '.' || self.occupied((r, c)) {
                    continue;
                }
                let d = r.abs_diff(START_AT.0) + c.abs_diff(START_AT.1);
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some(((r, c), d));
                }
            }
        }
        best.map_or(START_AT, |(cell, _)| cell)
    }

    fn occupied(&self, at: (usize, usize)) -> bool {
        self.pilgrims.values().any(|p| p.at == at)
    }

    pub fn leave(&mut self, id: Id) {
        self.pilgrims.remove(&id);
    }

    pub fn present(&self) -> usize {
        self.pilgrims.len()
    }

    pub fn is_in(&self, id: Id) -> bool {
        self.pilgrims.contains_key(&id)
    }

    pub fn set_blessing(&mut self, id: Id, line: String) {
        if let Some(p) = self.pilgrims.get_mut(&id) {
            p.blessing = Some(line);
        }
    }

    pub fn handle(&mut self, id: Id, key: Key) -> Action {
        let Some(me) = self.pilgrims.get(&id).cloned() else {
            return Action::Idle;
        };

        if me.ringing || me.burning {
            // 撞完/上完香按任意键起身，继续自由走动
            if let Some(p) = self.pilgrims.get_mut(&id) {
                p.ringing = false;
                p.burning = false;
                p.blessing = None;
            }
            return Action::Redraw;
        }

        match key {
            Key::Quit => Action::Leave,
            // 站在钟左右或正下方按空格：撞钟
            Key::Space if can_ring(me.at) => {
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.ringing = true;
                }
                Action::Ring
            }
            // 站在香炉正下方按空格：烧香
            Key::Space if can_burn(me.at) => {
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.burning = true;
                }
                Action::Burn
            }
            _ => self.step(id, me.at, key),
        }
    }

    fn step(&mut self, id: Id, from: (usize, usize), key: Key) -> Action {
        let (r, c) = from;
        let to = match key {
            Key::Up if r > 0 => (r - 1, c),
            // 广场下缘再往下一步就出寺（庙堂里够不到最后一排，只会当过道）
            Key::Down => {
                if r + 1 >= H {
                    return Action::Leave;
                }
                (r + 1, c)
            }
            // 走出广场左/右边缘就离寺。庙堂两侧都是墙，够不到 c==0 / c==W-1，
            // 所以这两条离开只会发生在广场里。
            Key::Left => {
                if c == 0 {
                    return Action::Leave;
                }
                (r, c - 1)
            }
            Key::Right => {
                if c + 1 >= W {
                    return Action::Leave;
                }
                (r, c + 1)
            }
            _ => return Action::Idle,
        };
        if MAP[to.0][to.1] != '.' {
            return Action::Idle; // 钟和窗棂挡路
        }
        if self.occupied(to) {
            return Action::Idle; // 那格有人：撞上，不穿过
        }
        if let Some(p) = self.pilgrims.get_mut(&id) {
            p.at = to;
        }
        Action::Redraw
    }

    /// 画出此刻的廟，视角是 id 这个人。相机只显示世界的一截，跟着人上下滚动。
    pub fn render(&self, id: Id) -> String {
        // 星空在上、地图在下，拼成一整摞“世界行”
        let mut world: Vec<String> = SKY.iter().map(|s| (*s).to_string()).collect();
        for (r, row) in MAP.iter().enumerate() {
            let mut line = String::new();
            for (c, tile) in row.iter().enumerate() {
                // 同格有多人时本人优先显示，免得自己被别人盖住
                let here = self
                    .pilgrims
                    .iter()
                    .filter(|(_, p)| p.at == (r, c))
                    .max_by_key(|(pid, _)| u8::from(**pid == id));
                if let Some((_, p)) = here {
                    line.push_str(avatar_of(p.avatar));
                    continue;
                }
                line.push_str(match tile {
                    'B' => BELL,
                    'F' => FIRE,
                    'f' => FLOWER,
                    '+' => LATTICE,
                    // 空地画成留白
                    _ => FLOOR,
                });
            }
            world.push(line);
        }

        // 相机：竖直方向跟着我，尽量把我摆在视窗正中，到顶/到底就贴边不越界
        let total = world.len();
        let focus = self
            .pilgrims
            .get(&id)
            .map_or(SKY.len() + START_AT.0, |p| SKY.len() + p.at.0);
        let top = if total <= VIEW_H {
            0
        } else {
            focus.saturating_sub(VIEW_H / 2).min(total - VIEW_H)
        };

        let mut out = String::from("\x1b[2J\x1b[H\x1b[?25l\r\n");
        for line in &world[top..(top + VIEW_H).min(total)] {
            out.push_str(line);
            out.push_str("\r\n");
        }

        out.push_str("\r\n");
        match self.pilgrims.get(&id) {
            Some(p) if p.ringing || p.burning => {
                if let Some(line) = &p.blessing {
                    out.push_str(&format!("  \x1b[33m{line}\x1b[0m\r\n"));
                }
            }
            Some(p) if can_ring(p.at) => out.push_str("  \x1b[2m🔔 鐘在側 · 按空格撞鐘\x1b[0m\r\n"),
            Some(p) if can_burn(p.at) => {
                out.push_str("  \x1b[2m🔥 香爐在前 · 按空格燒香\x1b[0m\r\n")
            }
            // 站在广场下缘，再往下一步就出寺
            Some(p) if p.at.0 == H - 1 => out.push_str("  \x1b[2m↓ 再往下一步 · 即出寺\x1b[0m\r\n"),
            // 站在广场左/右缘，再往外一步就出寺
            Some(p) if p.at.1 == 0 => out.push_str("  \x1b[2m← 再往左一步 · 即出寺\x1b[0m\r\n"),
            Some(p) if p.at.1 == W - 1 => out.push_str("  \x1b[2m→ 再往右一步 · 即出寺\x1b[0m\r\n"),
            _ => out.push_str("  \x1b[2m方向鍵走動\x1b[0m\r\n"),
        }
        out.push_str(&format!(
            "  \x1b[2m寺中此刻 {} 人\x1b[0m\r\n",
            self.present()
        ));
        out
    }
}

/// 站在钟的左边、右边或正下方那格，就能敲钟
fn can_ring(at: (usize, usize)) -> bool {
    at == (BELL_AT.0, BELL_AT.1 - 1)
        || at == (BELL_AT.0, BELL_AT.1 + 1)
        || at == (BELL_AT.0 + 1, BELL_AT.1)
}

/// 站在香炉正下方那格，就能烧香
fn can_burn(at: (usize, usize)) -> bool {
    at == (CENSER_AT.0 + 1, CENSER_AT.1)
}

/// 首次进廟的选头像画面。这时还没进世界，所以状态是会话私有的。
pub struct Choosing {
    cursor: usize,
}

impl Choosing {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// 返回 Some(下标) 表示选定
    pub fn handle(&mut self, key: Key) -> Option<usize> {
        match key {
            Key::Left => {
                self.cursor = (self.cursor + AVATARS.len() - 1) % AVATARS.len();
                None
            }
            Key::Right => {
                self.cursor = (self.cursor + 1) % AVATARS.len();
                None
            }
            Key::Enter | Key::Space => Some(self.cursor),
            _ => None,
        }
    }

    pub fn render(&self) -> String {
        let mut out = String::from("\x1b[2J\x1b[H\x1b[?25l");
        out.push_str("\r\n  小鐘寺前 · 先擇一副面容 🙏\r\n\r\n  ");
        for (n, a) in AVATARS.iter().enumerate() {
            if n > 0 && n % PER_ROW == 0 {
                out.push_str("\r\n  ");
            }
            if n == self.cursor {
                out.push_str(&format!("\x1b[43;30m {a} \x1b[0m"));
            } else {
                out.push_str(&format!(" {a} "));
            }
        }
        out.push_str("\r\n\r\n  \x1b[2m← → 挑選 · Enter 確定\x1b[0m\r\n");
        out
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Space,
    Enter,
    Quit,
    Other,
}

/// 解析终端按键。方向键是 ESC [ A~D 三字节序列。
pub fn parse_keys(buf: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut i = 0;
    while i < buf.len() {
        match buf[i] {
            0x1b if i + 2 < buf.len() && buf[i + 1] == b'[' => {
                keys.push(match buf[i + 2] {
                    b'A' => Key::Up,
                    b'B' => Key::Down,
                    b'C' => Key::Right,
                    b'D' => Key::Left,
                    _ => Key::Other,
                });
                i += 3;
            }
            b' ' => {
                keys.push(Key::Space);
                i += 1;
            }
            b'\r' | b'\n' => {
                keys.push(Key::Enter);
                i += 1;
            }
            0x03 | 0x04 | b'q' => {
                keys.push(Key::Quit);
                i += 1;
            }
            b'w' | b'k' => {
                keys.push(Key::Up);
                i += 1;
            }
            b's' | b'j' => {
                keys.push(Key::Down);
                i += 1;
            }
            b'a' | b'h' => {
                keys.push(Key::Left);
                i += 1;
            }
            b'd' | b'l' => {
                keys.push(Key::Right);
                i += 1;
            }
            _ => {
                keys.push(Key::Other);
                i += 1;
            }
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(w: &mut World, id: Id, at: (usize, usize)) {
        w.pilgrims.get_mut(&id).unwrap().at = at;
    }

    fn one() -> (World, Id) {
        let mut w = World::default();
        w.join(1, 0);
        (w, 1)
    }

    #[test]
    fn edges_and_shrine_block_movement() {
        let (mut w, me) = one();
        put(&mut w, me, (1, 1)); // 左边是点墙
        assert!(matches!(w.handle(me, Key::Left), Action::Idle));
        assert_eq!(w.pilgrims[&me].at, (1, 1));
        put(&mut w, me, (1, W - 2)); // 右边是点墙
        assert!(matches!(w.handle(me, Key::Right), Action::Idle));
        put(&mut w, me, (1, 4)); // 上面就是钟，挡路
        assert!(matches!(w.handle(me, Key::Up), Action::Idle));
        put(&mut w, me, (1, 2)); // 顶排敞开，能走上去
        assert!(matches!(w.handle(me, Key::Up), Action::Redraw));
        assert_eq!(w.pilgrims[&me].at, (0, 2));
    }

    #[test]
    fn walk_off_bell_side_leaves() {
        let (mut w, me) = one();
        put(&mut w, me, (0, 0)); // 钟排最左
        assert!(matches!(w.handle(me, Key::Left), Action::Leave));
        put(&mut w, me, (0, W - 1)); // 钟排最右
        assert!(matches!(w.handle(me, Key::Right), Action::Leave));
    }

    #[test]
    fn pilgrims_collide_not_overlap() {
        let mut w = World::default();
        w.join(1, 0);
        w.join(2, 0); // 第二人不该和第一人叠在同一格
        assert_ne!(w.pilgrims[&1].at, w.pilgrims[&2].at);
        // 把两人摆成相邻，往对方那格走应被挡住
        put(&mut w, 1, (3, 4));
        put(&mut w, 2, (3, 5));
        assert!(matches!(w.handle(1, Key::Right), Action::Idle));
        assert_eq!(w.pilgrims[&1].at, (3, 4));
    }

    #[test]
    fn garden_gates_leave_flowers_block() {
        let (mut w, me) = one();
        // 香炉那行（第 7 排）两端开口，走出去即离寺
        put(&mut w, me, (7, 0));
        assert!(matches!(w.handle(me, Key::Left), Action::Leave));
        put(&mut w, me, (7, W - 1));
        assert!(matches!(w.handle(me, Key::Right), Action::Leave));
        // 花篱下缘中间三格是出口，往下即离寺
        put(&mut w, me, (H - 1, 4));
        assert!(matches!(w.handle(me, Key::Down), Action::Leave));
        // 广场外圈是花篱，挡路：往花篱走停住
        put(&mut w, me, (9, 1));
        assert!(matches!(w.handle(me, Key::Left), Action::Idle)); // 左边 (9,0) 是花
        assert_eq!(w.pilgrims[&me].at, (9, 1));
        put(&mut w, me, (10, 1));
        assert!(matches!(w.handle(me, Key::Down), Action::Idle)); // 下边 (11,1) 是花
    }

    #[test]
    fn doorway_is_passage_not_exit() {
        let (mut w, me) = one();
        // 寺门缺口往下不再离寺，而是走进广场
        put(&mut w, me, (6, 3));
        assert!(matches!(w.handle(me, Key::Down), Action::Redraw));
        assert_eq!(w.pilgrims[&me].at, (7, 3));
        // 门正中下方是香炉，挡路
        put(&mut w, me, (6, 4));
        assert!(matches!(w.handle(me, Key::Down), Action::Idle));
    }

    #[test]
    fn burn_at_censer() {
        let (mut w, me) = one();
        put(&mut w, me, (8, 4)); // 香炉正下方
        assert!(w.render(me).contains("按空格燒香"), "站定有提示");
        assert!(matches!(w.handle(me, Key::Space), Action::Burn));
        assert!(
            !w.render(me).contains("按空格燒香"),
            "烧香姿态里不再提示可烧"
        );
        assert!(matches!(w.handle(me, Key::Other), Action::Redraw)); // 起身
        assert!(w.render(me).contains("按空格燒香"), "起身后又能烧");
        // 香炉本身挡路
        put(&mut w, me, (8, 4));
        assert!(matches!(w.handle(me, Key::Up), Action::Idle));
    }

    #[test]
    fn ring_from_bell_side() {
        let (mut w, me) = one();
        put(&mut w, me, (0, 3)); // 钟的左边
        assert!(matches!(w.handle(me, Key::Space), Action::Ring));
        assert!(
            !w.render(me).contains("按空格撞鐘"),
            "撞钟姿态里不再提示可撞"
        );
        assert!(matches!(w.handle(me, Key::Other), Action::Redraw));
        assert!(w.render(me).contains(AVATARS[0]));
        put(&mut w, me, (0, 5)); // 钟的右边
        assert!(matches!(w.handle(me, Key::Space), Action::Ring));
        w.handle(me, Key::Other); // 起身
        put(&mut w, me, (1, 4)); // 钟的正下方
        assert!(matches!(w.handle(me, Key::Space), Action::Ring));
    }

    #[test]
    fn gate_warns_before_leaving() {
        let (mut w, me) = one();
        put(&mut w, me, (7, 2));
        assert!(!w.render(me).contains("即出寺"), "还没到门口时不提示");
        put(&mut w, me, (7, 0));
        assert!(w.render(me).contains("← 再往左一步 · 即出寺"));
    }

    #[test]
    fn space_elsewhere_does_nothing() {
        let (mut w, me) = one();
        put(&mut w, me, (3, 2));
        assert!(matches!(w.handle(me, Key::Space), Action::Idle));
    }

    #[test]
    fn pilgrims_see_each_other() {
        let mut w = World::default();
        w.join(1, 0);
        w.join(2, 5);
        // 两人都在广场、同一个相机视窗内才互相看得见
        put(&mut w, 1, (9, 4));
        put(&mut w, 2, (9, 2));

        let seen = w.render(1);
        assert!(seen.contains(AVATARS[0]), "看得见自己");
        assert!(seen.contains(AVATARS[5]), "看得见别人");
        assert!(seen.contains("寺中此刻 2 人"));

        w.leave(2);
        assert!(!w.render(1).contains(AVATARS[5]), "走了就看不见了");
    }

    #[test]
    fn hint_at_bell() {
        let (mut w, me) = one();
        put(&mut w, me, (0, 3));
        assert!(w.render(me).contains("按空格撞鐘"));
    }

    #[test]
    fn choosing_then_confirm() {
        let mut c = Choosing::new();
        assert!(c.render().contains("先擇一副面容"));
        assert_eq!(c.handle(Key::Right), None);
        assert_eq!(c.handle(Key::Right), None);
        assert_eq!(c.handle(Key::Enter), Some(2));
    }

    #[test]
    fn stale_avatar_index_falls_back() {
        // 删过头像后，旧记录里的下标可能越界，不该 panic
        let mut w = World::default();
        w.join(1, AVATARS.len() + 99);
        assert!(w.render(1).contains(AVATARS[0]));
    }

    #[test]
    fn arrow_sequences_parse() {
        assert_eq!(parse_keys(b"\x1b[A"), vec![Key::Up]);
        assert_eq!(parse_keys(b"\x1b[B\x1b[C"), vec![Key::Down, Key::Right]);
        assert_eq!(parse_keys(b" "), vec![Key::Space]);
        assert_eq!(parse_keys(b"q"), vec![Key::Quit]);
    }
}
