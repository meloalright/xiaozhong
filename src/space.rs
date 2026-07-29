//! 终端里的小钟寺：选头像、走动、撞钟、从下方或左右侧门离开。
//! 同时在寺里的人共享一个世界，彼此看得见。
//! 只在客户端申请了 PTY 时启用；无 PTY 的连接一律谢客。

use std::collections::HashMap;

/// 可选头像，首次进廟时挑一个，之后按公钥记住。
/// 全部是单码位 emoji：ZWJ 组合序列（如 👩‍🦰）在不同终端会拆成两个字形、
/// 宽度从 2 格变 4 格，会把地图撑歪。
pub const AVATARS: &[&str] = &["🧑", "👧", "🧝", "🧛", "👸", "👷"];

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
/// 有人正在撞钟时，钟那格泛起星光——全场看得见钟在响
const BELL_RINGING: &str = "✨";
/// 广场上的香炉。单码位 🔥，稳稳占 2 格，不像三竖线那样宽度不定。
const FIRE: &str = "🔥";
/// 有人正在烧香时，香炉那格迸出火光
const FIRE_BURNING: &str = "✨";
/// 广场外圈的花篱。两个纯 ASCII 字符恒定 2 列，稳过歧义宽度的花朵符号。
const FLOWER: &str = "*,";
/// 寺庙的猫。单码位 🐱，稳占 2 格。只在广场溜达，不算香客、不入任何簿子。
const CAT: &str = "🐱";
/// 猫的初始落脚点（广场里）
const CAT_START: (usize, usize) = (10, 6);
/// 庙外（广场 + 下院）从第几排起——猫在这片(含下院)溜达，只是不上庙堂
const PLAZA_TOP: usize = 7;

/// 下院正中那棵树。单码位 🌳，稳占 2 格，挡路；靠近按空格能看看它。
const TREE: &str = "🌳";
/// 树的位置（和地图里的 'T' 对应），从相邻一格可交互
const TREE_AT: (usize, usize) = (14, 4);
/// 看树时弹的一句
const TREE_LINE: &str = "這是寺裡的一棵老樹";
/// 安保大爷 / 志愿者的头像（都是单码位）
const GUARD: &str = "🙋";
const VOLUNTEER: &str = "💇";
/// NPC 的几个固定落点
const GUARD_TREE: (usize, usize) = (15, 4); // 平时在下院、树下
const GUARD_HALL: (usize, usize) = (3, 2); // 14–16 点巡到庙堂
const VOLUNTEER_AT: (usize, usize) = (9, 2); // 夜里的志愿者在广场
/// 深夜 0–2 点，钟右侧飘着一只幽灵，能搭话但只吐省略号
const GHOST: &str = "👻";
const GHOST_AT: (usize, usize) = (0, 5); // 钟 (0,4) 的右邻
/// 下院可落东西的行：出生点那排到树那排之间（含两端），购物车落点用
const COURTYARD_ROWS: std::ops::RangeInclusive<usize> = START_AT.0..=TREE_AT.0; // 9..=14
/// 每天在出生点到树之间随机出现的购物车。单码位 🛒，有碰撞、能被推着走。
const CART: &str = "🛒";
/// 购物车每天几点出现（和信同一节律）
const CART_HOUR: u32 = 6;
/// 空庭：地面留白，只有星光、钟和人
const FLOOR: &str = "  ";
/// 窗棂格子墙
const LATTICE: &str = "++";

pub const W: usize = 9;
/// 上 7 排庙堂，中 5 排广场，下 6 排下院，靠门一路相连
pub const H: usize = 18;

/// 平面图：
///   'B' 钟   'F' 香炉 🔥   'T' 树 🌳（下院正中，挡路）
///   '.' 可走的空地    '+' 窗棂格子墙    'f' 花朵篱笆（挡路）
/// 顶排敞开：从钟的两侧一直走到左右边缘，走出去即离寺。
/// 广场／下院外圈用花篱围住；广场底门通往下院（不再是出口），
/// 下院底门才是出寺的新出口。
const MAP: [[char; W]; H] = [
    ['.', '.', '.', '.', 'B', '.', '.', '.', '.'], // 0 钟 · 两侧走到边缘即离寺
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 1
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 2
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 3 大爷 14–16 点巡到 (3,2)
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 4
    ['+', '.', '.', '.', '.', '.', '.', '.', '+'], // 5
    ['+', '+', '+', '.', '.', '.', '+', '+', '+'], // 6 寺门 → 广场
    ['.', '.', '.', '.', 'F', '.', '.', '.', '.'], // 7 香炉 🔥 · 两端开口出入
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 8 香炉正下方可烧香
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 9 出生点在正中 · 志愿者夜里在 (9,2)
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 10
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 11 广场直接连着下院（无隔篱）
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 12 下院
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 13
    ['f', '.', '.', '.', 'T', '.', '.', '.', 'f'], // 14 树 🌳
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 15 大爷平时站树下 (15,4)
    ['f', '.', '.', '.', '.', '.', '.', '.', 'f'], // 16
    ['f', 'f', 'f', '.', '.', '.', 'f', 'f', 'f'], // 17 下院底门 = 出寺新出口
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
    /// 正在撸猫
    petting: bool,
    /// 正在和 NPC 搭话
    talking: bool,
    /// 和工作人员对话的进度：1=刚说完今日之信、还有本来的话要说；否则 0
    talk_stage: u8,
    /// 是否挪过步——刚进寺没动过时给一次走动提示，动过就不再提示
    moved: bool,
    /// 撞钟/烧香/撸猫/搭话后要显示给本人的那句话
    blessing: Option<String>,
}

/// 每天出现的购物车。落在随机空地，有碰撞，能被人推着走。
#[derive(Clone, PartialEq, Debug)]
struct Cart {
    at: (usize, usize),
}

/// 值守的 NPC：任意时刻最多一个（安保大爷或志愿者），按上海时间决定。
#[derive(Clone, PartialEq, Debug)]
struct Npc {
    at: (usize, usize),
    glyph: &'static str,
    line: &'static str,
}

/// 按上海时间「当天第几分钟」返回此刻值守的 NPC（没有就 None）。纯函数，好测。
fn npc_for(min_of_day: u32) -> Option<Npc> {
    let guard = |at, line| {
        Some(Npc {
            at,
            glyph: GUARD,
            line,
        })
    };
    // 对话文案只放内容，显示时会在前面拼上「头像：」体现是谁说的
    let volunteer = || {
        Some(Npc {
            at: VOLUNTEER_AT,
            glyph: VOLUNTEER,
            line: "這個點兒沒什麼人，我這志願者也落得自在。",
        })
    };
    match min_of_day / 60 {
        // 0–2 点：钟右侧的幽灵，搭话只吐省略号
        0..=1 => Some(Npc {
            at: GHOST_AT,
            glyph: GHOST,
            line: "......",
        }),
        6..=9 => guard(GUARD_TREE, "早啊，這麼早就來啦？"),
        10..=11 => guard(GUARD_TREE, "快到中午了哦。"),
        // 12–13：午休，无人
        14..=15 => guard(GUARD_HALL, "下午好，寺裡隨便轉轉，別客氣。"),
        16..=17 => guard(GUARD_HALL, "等下就快下班了。"),
        18..=20 => volunteer(),
        21 if min_of_day < 21 * 60 + 30 => volunteer(),
        _ => None,
    }
}

/// 廟里此刻的所有人。所有会话共享一份，谁动了都要重画。
pub struct World {
    pilgrims: HashMap<Id, Pilgrim>,
    /// 寺庙的猫，在广场自己溜达。不算香客，撸它也不计数。
    cat_at: (usize, usize),
    /// 此刻值守的 NPC，按上海时间刷新。
    npc: Option<Npc>,
    /// 上海时间「当天第几分钟」，用来在状态栏报时辰。随 update_npc 刷新。
    min_of_day: u32,
    /// 今日的信内容，由环境变量注入。工作人员对话时会先带一句。
    letter_text: Option<String>,
    /// 大钟寺此刻的天色图标，由天气任务写入；None 则回落到时辰 ☀️/🌙。
    weather_icon: Option<&'static str>,
    /// 此刻场上那辆购物车（没有就 None）。
    cart: Option<Cart>,
    /// 已在纪元第几天放过车，保证一天只放一辆。
    cart_day: Option<i64>,
    /// 调试：把猫钉住不再溜达（给 showcase 演示「推车撞猫」用）。
    cat_pinned: bool,
}

impl Default for World {
    fn default() -> Self {
        Self {
            pilgrims: HashMap::new(),
            cat_at: CAT_START,
            npc: None,
            min_of_day: 0,
            letter_text: None,
            weather_icon: None,
            cart: None,
            cart_day: None,
            cat_pinned: false,
        }
    }
}

pub enum Action {
    Idle,
    Redraw,
    /// 撞钟达成，记一次，并给全场发响铃
    Ring,
    /// 烧香达成，给本人一句祈愿，全场看到香煙
    Burn,
    /// 撸猫达成，给本人一句文案（不计数）
    Pet,
    /// 和 NPC 搭话，弹一句对话（不计数）
    Talk,
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
                petting: false,
                talking: false,
                talk_stage: 0,
                moved: false,
                blessing: None,
            },
        );
    }

    /// 天气任务写入此刻天色图标（None = 回落到时辰）。变了返回 true（好广播重画）。
    pub fn set_weather(&mut self, icon: Option<&'static str>) -> bool {
        if self.weather_icon != icon {
            self.weather_icon = icon;
            true
        } else {
            false
        }
    }

    /// 状态栏那行：天气图标（有就用，没有兜底 ⚡）+ 真实上海时间 HH:MM。
    fn status_mark(&self) -> String {
        match self.weather_icon {
            Some(icon) => format!("{}  {}", icon, hhmm(self.min_of_day)),
            None => sky_mark(self.min_of_day),
        }
    }

    /// 注入信的内容（来自环境变量）。空字符串则没有信。
    pub fn set_letter_text(&mut self, text: String) {
        self.letter_text = if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
    }

    /// 今日的信内容（非空才有）。工作人员对话时会先带一句。
    pub fn letter_text(&self) -> Option<&str> {
        self.letter_text
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    /// 这格能不能站人／放东西：地图上是空地 '.'，且没有人、猫、NPC、车。
    fn cell_free(&self, at: (usize, usize)) -> bool {
        MAP[at.0][at.1] == '.'
            && !self.occupied(at)
            && at != self.cat_at
            && !self.is_npc(at)
            && !self.is_cart(at)
    }

    /// 有符号坐标落回网格；越界返回 None（相当于撞上地图边界这堵墙）。
    fn on_grid(&self, (r, c): (isize, isize)) -> Option<(usize, usize)> {
        if r >= 0 && (r as usize) < H && c >= 0 && (c as usize) < W {
            Some((r as usize, c as usize))
        } else {
            None
        }
    }

    /// 出生点到树之间的空地里随机挑一格（购物车落点用；和猫人 NPC 都不叠）。
    fn random_courtyard_cell(&self) -> Option<(usize, usize)> {
        let mut cells: Vec<(usize, usize)> = Vec::new();
        for r in COURTYARD_ROWS {
            for c in 0..W {
                if self.cell_free((r, c)) {
                    cells.push((r, c));
                }
            }
        }
        if cells.is_empty() {
            None
        } else {
            Some(cells[rand::random_range(0..cells.len())])
        }
    }

    fn is_cart(&self, at: (usize, usize)) -> bool {
        self.cart.as_ref().is_some_and(|c| c.at == at)
    }

    /// 站在车的上下左右相邻一格，就够得着推
    fn cart_near(&self, at: (usize, usize)) -> bool {
        self.cart
            .as_ref()
            .is_some_and(|c| at.0.abs_diff(c.at.0) + at.1.abs_diff(c.at.1) == 1)
    }

    /// 每天 06:00 起在出生点到树之间随机放一辆购物车，当天只放一次。放下返回 true。
    pub fn tick_cart(&mut self, min_of_day: u32, day: i64) -> bool {
        if min_of_day >= CART_HOUR * 60 && self.cart_day != Some(day) {
            if let Some(at) = self.random_courtyard_cell() {
                self.cart = Some(Cart { at });
                self.cart_day = Some(day);
                return true;
            }
        }
        false
    }

    /// 调试：把车钉在指定格（当天不再另投）。给 showcase / 测试用。
    pub fn place_cart(&mut self, at: (usize, usize)) {
        self.cart = Some(Cart { at });
        self.cart_day = Some(i64::MAX);
    }

    /// 调试：把猫钉在指定格、不再溜达。给 showcase 演示「推车撞猫」用。
    pub fn pin_cat(&mut self, at: (usize, usize)) {
        self.cat_at = at;
        self.cat_pinned = true;
    }

    /// 朝车按方向键：把车往同方向推一格，人跟进车原来那格。
    /// 车前方是墙（含地图边界/树）就随机往垂直方向弹开；两侧都堵就推不动。
    fn push_cart(&mut self, id: Id, cart_at: (usize, usize), key: Key) -> Action {
        let (dr, dc): (isize, isize) = match key {
            Key::Up => (-1, 0),
            Key::Down => (1, 0),
            Key::Left => (0, -1),
            Key::Right => (0, 1),
            _ => return Action::Idle,
        };
        let (cr, cc) = (cart_at.0 as isize, cart_at.1 as isize);
        match self.on_grid((cr + dr, cc + dc)) {
            // 前方是车能占的空地：正常推，车往前、人进到车原来那格
            Some(cell) if self.cart_free(cell) => {
                self.cart = Some(Cart { at: cell });
                self.move_into(id, cart_at);
                Action::Redraw
            }
            // 前方有任何碰撞体（人/猫/火/墙/树/信/大殿行政墙）：沿碰撞法线的垂直方向弹开
            Some(_) => self.bounce_cart(id, cart_at, dr, dc),
            // 推出了地图边界＝出寺：车回到出生那几行随机位置，人进到车原来那格
            None => {
                self.cart = self.random_courtyard_cell().map(|at| Cart { at });
                self.move_into(id, cart_at);
                Action::Redraw
            }
        }
    }

    /// 车能占的格：不进大殿(row<PLAZA_TOP)，且是空地、没别的东西。
    fn cart_free(&self, at: (usize, usize)) -> bool {
        at.0 >= PLAZA_TOP && self.cell_free(at)
    }

    /// 车撞上碰撞体（墙/树/火/人/猫/信/大殿）：沿碰撞法线的垂直方向随机弹一格；
    /// 两侧都堵着就纹丝不动。
    fn bounce_cart(&mut self, id: Id, cart_at: (usize, usize), dr: isize, _dc: isize) -> Action {
        // 推的是竖向(dr!=0)就往左右弹，推的是横向就往上下弹
        let perp: [(isize, isize); 2] = if dr != 0 {
            [(0, -1), (0, 1)]
        } else {
            [(-1, 0), (1, 0)]
        };
        let (cr, cc) = (cart_at.0 as isize, cart_at.1 as isize);
        let opts: Vec<(usize, usize)> = perp
            .iter()
            .filter_map(|(pr, pc)| self.on_grid((cr + pr, cc + pc)))
            .filter(|&cell| self.cart_free(cell))
            .collect();
        if opts.is_empty() {
            return Action::Idle;
        }
        self.cart = Some(Cart {
            at: opts[rand::random_range(0..opts.len())],
        });
        self.move_into(id, cart_at); // 人进到车原来那格
        Action::Redraw
    }

    /// 把某人挪到 at，并记下他动过（不再显示走动提示）
    fn move_into(&mut self, id: Id, at: (usize, usize)) {
        if let Some(p) = self.pilgrims.get_mut(&id) {
            p.at = at;
            p.moved = true;
        }
    }

    /// 按上海时间刷新值守 NPC 与时辰；NPC 变了返回 true（好决定要不要广播重画）。
    pub fn update_npc(&mut self, min_of_day: u32) -> bool {
        self.min_of_day = min_of_day;
        let next = npc_for(min_of_day);
        if next != self.npc {
            self.npc = next;
            true
        } else {
            false
        }
    }

    fn is_npc(&self, at: (usize, usize)) -> bool {
        self.npc.as_ref().is_some_and(|n| n.at == at)
    }

    /// 落脚点：优先 START_AT，被占了就找离它最近的空地。
    /// 全都占满（几乎不会）才退回 START_AT 叠一格。
    fn spawn_cell(&self) -> (usize, usize) {
        if self.cell_free(START_AT) {
            return START_AT;
        }
        let mut best: Option<((usize, usize), usize)> = None;
        for (r, row) in MAP.iter().enumerate() {
            for (c, _tile) in row.iter().enumerate() {
                if !self.cell_free((r, c)) {
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

    fn anyone_ringing(&self) -> bool {
        self.pilgrims.values().any(|p| p.ringing)
    }

    fn anyone_burning(&self) -> bool {
        self.pilgrims.values().any(|p| p.burning)
    }

    fn anyone_petting(&self) -> bool {
        self.pilgrims.values().any(|p| p.petting)
    }

    pub fn leave(&mut self, id: Id) {
        self.pilgrims.remove(&id);
    }

    /// 在线连接玩家数（只数真正连进来的香客，NPC 和猫都不算）。
    pub fn online(&self) -> usize {
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

        if me.ringing || me.burning || me.petting {
            // 撞完/上完香/撸完猫按任意键起身，继续自由走动
            if let Some(p) = self.pilgrims.get_mut(&id) {
                p.ringing = false;
                p.burning = false;
                p.petting = false;
                p.blessing = None;
            }
            return Action::Redraw;
        }
        if me.talking {
            // 和工作人员对话：说完信(stage 1)后按空格接着说本来的话；其它键（或已说完）起身
            if key == Key::Space && me.talk_stage == 1 {
                if let Some(line) = self
                    .npc_near(me.at)
                    .map(|n| format!("{} 「{}」", n.glyph, n.line))
                {
                    if let Some(p) = self.pilgrims.get_mut(&id) {
                        p.talk_stage = 2;
                        p.blessing = Some(line);
                    }
                    return Action::Talk;
                }
            }
            if let Some(p) = self.pilgrims.get_mut(&id) {
                p.talking = false;
                p.talk_stage = 0;
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
            // 站在猫身边按空格：撸猫
            Key::Space if can_pet(me.at, self.cat_at) => {
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.petting = true;
                }
                Action::Pet
            }
            // 站在 NPC 身边按空格：搭话。工作人员（大爷/志愿者）每次先带一句今日之信，
            // 再按一下才说本来的话（按两下）；幽灵、没配信时就单句。
            Key::Space if self.npc_near(me.at).is_some() => {
                let n = self.npc_near(me.at).unwrap();
                let glyph = n.glyph;
                let npc_line = n.line;
                let is_staff = glyph != GHOST;
                let (blessing, stage) = match self.letter_text() {
                    Some(letter) if is_staff => (format!("{glyph} 「{letter}」"), 1u8),
                    _ => (format!("{glyph} 「{npc_line}」"), 0u8),
                };
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.talking = true;
                    p.talk_stage = stage;
                    p.blessing = Some(blessing);
                }
                Action::Talk
            }
            // 站在树旁按空格：看看这棵老树
            Key::Space if can_tree(me.at) => {
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.talking = true;
                    p.blessing = Some(TREE_LINE.to_string());
                }
                Action::Talk
            }
            // 站在车旁按空格：告知能推
            Key::Space if self.cart_near(me.at) => {
                if let Some(p) = self.pilgrims.get_mut(&id) {
                    p.talking = true;
                    p.blessing = Some("這車推得動".to_string());
                }
                Action::Talk
            }
            _ => self.step(id, me.at, key),
        }
    }

    /// 站在 NPC 相邻一格时，返回这个 NPC；否则 None
    fn npc_near(&self, at: (usize, usize)) -> Option<&Npc> {
        self.npc
            .as_ref()
            .filter(|n| at.0.abs_diff(n.at.0) + at.1.abs_diff(n.at.1) == 1)
    }

    /// 猫随机挪一格：只在广场、只走空地、不踩人。挪动了返回 true。
    pub fn wander_cat(&mut self) -> bool {
        // 调试钉住、或正被撸时都定住不走
        if self.cat_pinned || self.anyone_petting() {
            return false;
        }
        let (r, c) = self.cat_at;
        let mut moves: Vec<(usize, usize)> = Vec::new();
        for (nr, nc) in [
            (r.wrapping_sub(1), c),
            (r + 1, c),
            (r, c.wrapping_sub(1)),
            (r, c + 1),
        ] {
            // 在庙外（广场 + 下院）里走，只是不上庙堂
            if !(PLAZA_TOP..H).contains(&nr) || nc >= W {
                continue;
            }
            // 墙/树挡在 MAP 里，另外别踩到人、NPC、信、车上
            if !self.cell_free((nr, nc)) {
                continue;
            }
            moves.push((nr, nc));
        }
        if moves.is_empty() {
            return false;
        }
        self.cat_at = moves[rand::random_range(0..moves.len())];
        true
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
        // 目标格正好是购物车：不是撞停，而是朝同方向推它
        if self.is_cart(to) {
            return self.push_cart(id, to, key);
        }
        if MAP[to.0][to.1] != '.' {
            return Action::Idle; // 钟和窗棂挡路
        }
        if self.occupied(to) {
            return Action::Idle; // 那格有人：撞上，不穿过
        }
        if to == self.cat_at || self.is_npc(to) {
            return Action::Idle; // 猫 / NPC 挡在那儿：撞上，不穿过
        }
        self.move_into(id, to);
        Action::Redraw
    }

    /// 画出此刻的廟，视角是 id 这个人。相机只显示世界的一截，跟着人上下滚动。
    pub fn render(&self, id: Id) -> String {
        // 有人正在敲/烧时，钟和香炉换个字形，全场都看得见
        let ringing = self.anyone_ringing();
        let burning = self.anyone_burning();

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
                if (r, c) == self.cat_at {
                    line.push_str(CAT);
                    continue;
                }
                if let Some(n) = self.npc.as_ref().filter(|n| n.at == (r, c)) {
                    line.push_str(n.glyph);
                    continue;
                }
                if self.is_cart((r, c)) {
                    line.push_str(CART);
                    continue;
                }
                line.push_str(match tile {
                    'B' => {
                        if ringing {
                            BELL_RINGING
                        } else {
                            BELL
                        }
                    }
                    'F' => {
                        if burning {
                            FIRE_BURNING
                        } else {
                            FIRE
                        }
                    }
                    'T' => TREE,
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
        // 顶行：天气+时辰，后跟绿色 ● + 在线连接玩家数；空一行再接画面
        out.push_str(&format!(
            "  \x1b[2m{}\x1b[0m   \x1b[32m● {}\x1b[0m\r\n\r\n",
            self.status_mark(),
            self.online()
        ));
        for line in &world[top..(top + VIEW_H).min(total)] {
            out.push_str(line);
            out.push_str("\r\n");
        }

        out.push_str("\r\n");
        match self.pilgrims.get(&id) {
            Some(p) if p.ringing || p.burning || p.petting || p.talking => {
                if let Some(line) = &p.blessing {
                    out.push_str(&format!("  \x1b[33m{line}\x1b[0m\r\n"));
                }
            }
            Some(p) if can_ring(p.at) => out.push_str("  \x1b[2m鐘 · 按空格撞鐘\x1b[0m\r\n"),
            Some(p) if can_burn(p.at) => out.push_str("  \x1b[2m香爐 · 按空格燒香\x1b[0m\r\n"),
            Some(p) if can_pet(p.at, self.cat_at) => {
                out.push_str("  \x1b[2m🐱 貓 · 按空格撸貓\x1b[0m\r\n")
            }
            Some(p) if self.npc_near(p.at).is_some() => {
                let g = self.npc.as_ref().map_or("", |n| n.glyph);
                out.push_str(&format!("  \x1b[2m{g} · 按空格搭話\x1b[0m\r\n"));
            }
            Some(p) if can_tree(p.at) => out.push_str("  \x1b[2m🌳 老樹 · 按空格看看\x1b[0m\r\n"),
            Some(p) if self.cart_near(p.at) => {
                out.push_str("  \x1b[2m🛒 購物車 · 按空格看看\x1b[0m\r\n")
            }
            // 站在广场下缘，再往下一步就出寺
            Some(p) if p.at.0 == H - 1 => out.push_str("  \x1b[2m↓ 再往下一步 · 即出寺\x1b[0m\r\n"),
            // 站在广场左/右缘，再往外一步就出寺
            Some(p) if p.at.1 == 0 => out.push_str("  \x1b[2m← 再往左一步 · 即出寺\x1b[0m\r\n"),
            Some(p) if p.at.1 == W - 1 => out.push_str("  \x1b[2m→ 再往右一步 · 即出寺\x1b[0m\r\n"),
            // 走动提示只在刚进寺、还没挪过步时给一次；动过之后就留空行，不再唠叨
            Some(p) if !p.moved => out.push_str("  \x1b[2m← ↑ → 方向鍵走動\x1b[0m\r\n"),
            _ => out.push_str("\r\n"),
        }
        out
    }
}

/// 上海时间「当天第几分钟」格式化成 HH:MM
fn hhmm(min_of_day: u32) -> String {
    format!("{:02}:{:02}", min_of_day / 60, min_of_day % 60)
}

/// 拿不到天气时的兜底图标：⚡ —— 大概整个地球都被雷劈了，所以查询失败。
/// 单码位、默认彩色，且状态栏是自由文本行，怎么放都安全。
const FALLBACK_ICON: &str = "⚡";

/// 没有天气数据时的状态栏：图标一律 ⚡，时间照旧 HH:MM。
fn sky_mark(min_of_day: u32) -> String {
    format!("{}  {}", FALLBACK_ICON, hhmm(min_of_day))
}

/// 把 Open-Meteo 的原始字段映射成状态栏那行的天色图标。
/// 天气优先（雷雨/雪/雨/雾不分昼夜），其次按 is_day 定晴/多云/阴与昼夜。
/// 只放在状态栏那一行（自由文本，多码位 VS16 emoji 不参与网格对齐，安全）。
pub fn weather_icon(is_day: bool, code: u32, cloud: u32, rain: f64) -> &'static str {
    if (95..=99).contains(&code) {
        "⛈️" // 雷雨
    } else if (71..=77).contains(&code) || (85..=86).contains(&code) {
        "🌨️" // 雪
    } else if rain > 0.0 || (51..=67).contains(&code) || (80..=82).contains(&code) {
        "🌧️" // 雨（含毛毛雨、阵雨）
    } else if code == 45 || code == 48 {
        "🌫️" // 雾
    } else if is_day {
        if cloud >= 80 || code == 3 {
            "☁️" // 阴
        } else if code == 2 || cloud >= 40 {
            "⛅" // 多云
        } else {
            "☀️" // 晴
        }
    } else if cloud >= 80 || code == 3 {
        "☁️" // 阴夜
    } else {
        "🌙" // 晴夜
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

/// 站在猫的上下左右相邻一格，就能撸猫
fn can_pet(at: (usize, usize), cat: (usize, usize)) -> bool {
    at.0.abs_diff(cat.0) + at.1.abs_diff(cat.1) == 1
}

/// 站在树的上下左右相邻一格，就能看看老树
fn can_tree(at: (usize, usize)) -> bool {
    at.0.abs_diff(TREE_AT.0) + at.1.abs_diff(TREE_AT.1) == 1
}

/// 选头像界面里一次按键的结果
pub enum Choose {
    /// 无关键，画面不变
    Idle,
    /// 光标动了，重画选头像
    Redraw,
    /// 选定了这个下标，可以进世界
    Picked(usize),
    /// 离寺（q / Ctrl-C）
    Leave,
}

/// 首次进廟的选头像画面。这时还没进世界，所以状态是会话私有的。
pub struct Choosing {
    cursor: usize,
}

impl Choosing {
    pub fn new() -> Self {
        Self { cursor: 0 }
    }

    /// 选定 / 离寺 / 仅重绘选头像界面
    pub fn handle(&mut self, key: Key) -> Choose {
        match key {
            Key::Left => {
                self.cursor = (self.cursor + AVATARS.len() - 1) % AVATARS.len();
                Choose::Redraw
            }
            Key::Right => {
                self.cursor = (self.cursor + 1) % AVATARS.len();
                Choose::Redraw
            }
            Key::Enter | Key::Space => Choose::Picked(self.cursor),
            Key::Quit => Choose::Leave,
            _ => Choose::Idle,
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

/// 带缓冲的按键解析：SSH 可能把 `\x1b[A` 拆成多次 data，残段先攒着。
#[derive(Default)]
pub struct KeyParser {
    pending: Vec<u8>,
}

impl KeyParser {
    pub fn feed(&mut self, buf: &[u8]) -> Vec<Key> {
        if self.pending.is_empty() {
            return self.drain(buf);
        }
        self.pending.extend_from_slice(buf);
        let data = std::mem::take(&mut self.pending);
        self.drain(&data)
    }

    fn drain(&mut self, buf: &[u8]) -> Vec<Key> {
        let mut keys = Vec::new();
        let mut i = 0;
        while i < buf.len() {
            match buf[i] {
                0x1b => {
                    // CSI 至少三字节；不够就整段留到下次
                    if i + 1 >= buf.len() {
                        self.pending.extend_from_slice(&buf[i..]);
                        break;
                    }
                    if buf[i + 1] != b'[' {
                        keys.push(Key::Other);
                        i += 1;
                        continue;
                    }
                    if i + 2 >= buf.len() {
                        self.pending.extend_from_slice(&buf[i..]);
                        break;
                    }
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
                b'w' => {
                    keys.push(Key::Up);
                    i += 1;
                }
                b's' => {
                    keys.push(Key::Down);
                    i += 1;
                }
                b'a' => {
                    keys.push(Key::Left);
                    i += 1;
                }
                b'd' => {
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
}

/// 一次吃完整缓冲（仅测试）。有跨包方向键请用 [`KeyParser`]。
#[cfg(test)]
pub fn parse_keys(buf: &[u8]) -> Vec<Key> {
    KeyParser::default().feed(buf)
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
        // 外圈两侧仍是花篱，挡路：往侧边花篱走停住
        put(&mut w, me, (9, 1));
        assert!(matches!(w.handle(me, Key::Left), Action::Idle)); // 左边 (9,0) 是花
        assert_eq!(w.pilgrims[&me].at, (9, 1));
        // 广场与下院之间不再有隔篱：从广场直接往下走进下院
        put(&mut w, me, (10, 4));
        assert!(matches!(w.handle(me, Key::Down), Action::Redraw));
        assert_eq!(w.pilgrims[&me].at, (11, 4));
        // 下院里两侧还是花篱，挡路
        put(&mut w, me, (15, 1));
        assert!(matches!(w.handle(me, Key::Left), Action::Idle)); // 左边 (15,0) 是花
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
        assert!(seen.contains("● 2"), "顶行绿点显示 2 人在线");

        w.leave(2);
        assert!(!w.render(1).contains(AVATARS[5]), "走了就看不见了");
    }

    #[test]
    fn cat_is_rendered_but_not_a_person() {
        let mut w = World::default();
        w.join(1, 0);
        w.cat_at = (9, 5);
        put(&mut w, 1, (9, 4));
        let seen = w.render(1);
        assert!(seen.contains(CAT), "看得见猫");
        assert!(seen.contains("● 1"), "猫不算在线玩家");
    }

    #[test]
    fn person_and_cat_block_each_other() {
        let mut w = World::default();
        w.join(1, 0);
        w.cat_at = (9, 4);
        put(&mut w, 1, (9, 3));
        // 人撞猫：走不过去
        assert!(matches!(w.handle(1, Key::Right), Action::Idle));
        assert_eq!(w.pilgrims[&1].at, (9, 3));
        // 猫撞人：四邻全被占，猫挪不动
        w.join(2, 1);
        w.join(3, 2);
        w.join(4, 3);
        put(&mut w, 1, (8, 4));
        put(&mut w, 2, (10, 4));
        put(&mut w, 3, (9, 3));
        put(&mut w, 4, (9, 5));
        assert!(!w.wander_cat(), "四周都占满，猫不动");
        assert_eq!(w.cat_at, (9, 4));
    }

    #[test]
    fn pet_cat_when_adjacent() {
        let mut w = World::default();
        w.join(1, 0);
        w.cat_at = (9, 4);
        put(&mut w, 1, (9, 3));
        assert!(w.render(1).contains("按空格撸貓"), "挨着猫有提示");
        assert!(matches!(w.handle(1, Key::Space), Action::Pet));
        let seen = w.render(1);
        assert!(seen.contains(AVATARS[0]), "撸猫时头像不变");
        assert!(!seen.contains("按空格撸貓"), "撸猫姿态里不再提示");
        assert!(seen.contains(CAT), "猫仍是 🐱");
        assert!(!w.wander_cat(), "被撸时猫定住不走");
        assert!(matches!(w.handle(1, Key::Other), Action::Redraw)); // 起身
        assert!(w.render(1).contains("按空格撸貓"), "起身后又能撸");
        // 隔一格就撸不到了
        put(&mut w, 1, (9, 2));
        assert!(!w.render(1).contains("按空格撸貓"));
    }

    #[test]
    fn cat_wanders_within_plaza() {
        let mut w = World::default();
        for _ in 0..300 {
            w.wander_cat();
            let (r, c) = w.cat_at;
            assert!(
                (PLAZA_TOP..H).contains(&r) && c < W,
                "猫在庙外(广场+下院): {:?}",
                (r, c)
            );
            assert_eq!(MAP[r][c], '.', "猫只踩空地: {:?}", (r, c));
        }
    }

    #[test]
    fn npc_schedule_follows_shanghai_time() {
        let m = |h: u32, mi: u32| h * 60 + mi;
        // 早
        assert_eq!(npc_for(m(5, 59)), None, "06 点前无人");
        assert_eq!(npc_for(m(6, 0)).unwrap().at, GUARD_TREE);
        assert!(npc_for(m(6, 0)).unwrap().line.contains("早"));
        // 午饭寒暄
        assert!(npc_for(m(10, 0)).unwrap().line.contains("中午"));
        // 午休消失
        assert_eq!(npc_for(m(12, 0)), None, "午休无人");
        assert_eq!(npc_for(m(13, 59)), None);
        // 下午进庙堂
        let a = npc_for(m(14, 0)).unwrap();
        assert_eq!(a.at, GUARD_HALL, "14–16 点在庙堂");
        assert_eq!(a.glyph, GUARD);
        // 16–18 也在庙堂
        assert_eq!(npc_for(m(16, 0)).unwrap().at, GUARD_HALL);
        // 志愿者夜场
        let v = npc_for(m(18, 0)).unwrap();
        assert_eq!((v.at, v.glyph), (VOLUNTEER_AT, VOLUNTEER));
        assert!(v.line.contains("志願者"), "对话强调志愿者身份");
        assert!(npc_for(m(21, 29)).is_some(), "21:29 志愿者还在");
        assert_eq!(npc_for(m(21, 30)), None, "21:30 后无人");
        assert_eq!(npc_for(m(3, 0)), None, "后半夜无人");
    }

    #[test]
    fn npc_and_tree_block_and_talk() {
        let mut w = World::default();
        w.join(1, 0);
        w.update_npc(6 * 60); // 早上：大爷在树下 GUARD_TREE=(15,4)
                              // 站到大爷相邻一格
        put(&mut w, 1, (16, 4));
        assert!(w.render(1).contains("按空格搭話"), "挨着 NPC 有提示");
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("🙋 「早"), "对话头像后带空格和引号");
        assert!(matches!(w.handle(1, Key::Other), Action::Redraw)); // 起身
                                                                    // NPC 挡路：往大爷那格走停住
        assert!(matches!(w.handle(1, Key::Up), Action::Idle));
        assert_eq!(w.pilgrims[&1].at, (16, 4));
        // 树可交互：站树上方 (13,4) 相邻，按空格看树
        put(&mut w, 1, (13, 4));
        assert!(w.render(1).contains("按空格看看"), "挨着树有提示");
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("老樹"), "弹出看树的话");
        assert!(matches!(w.handle(1, Key::Other), Action::Redraw)); // 起身
                                                                    // 树也挡路：往树那格走停住
        assert!(matches!(w.handle(1, Key::Down), Action::Idle), "树挡路");
    }

    #[test]
    fn online_counts_only_connected_players() {
        let mut w = World::default();
        w.join(1, 0);
        w.update_npc(8 * 60); // 早上大爷在
                              // 在线数只算连进来的香客，NPC 不算
        assert_eq!(w.online(), 1, "大爷不算在线玩家");
        assert!(w.render(1).contains("● 1"));
        w.join(2, 3);
        assert_eq!(w.online(), 2);
        assert!(w.render(1).contains("● 2"));
    }

    #[test]
    fn ghost_haunts_bell_after_midnight() {
        // 0–2 点：钟右侧飘着幽灵，能搭话，只吐省略号
        let n = npc_for(60).expect("凌晨一点该有幽灵");
        assert_eq!(n.glyph, "👻");
        assert_eq!(n.at, (0, 5)); // 钟 (0,4) 右邻
        assert_eq!(n.line, "......");
        assert!(npc_for(2 * 60).is_none(), "两点后幽灵散去");

        // 站到幽灵下方能搭上话，弹出的正是省略号
        let mut w = World::default();
        w.update_npc(60);
        w.join(1, 0);
        put(&mut w, 1, (1, 5)); // 幽灵下方相邻一格
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("👻 「......」"));
        assert_eq!(w.online(), 1, "幽灵不算在线玩家");
    }

    #[test]
    fn staff_talk_says_letter_first_then_line() {
        let mut w = World::default();
        w.set_letter_text("今天是隔壁活動日".into());
        w.update_npc(6 * 60); // 早上：大爷在树下 (15,4)
        w.join(1, 0);
        put(&mut w, 1, (16, 4)); // 大爷相邻
                                 // 第一下：先说今日之信
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(
            w.render(1).contains("🙋 「今天是隔壁活動日」"),
            "先带一句信"
        );
        // 第二下（空格）：再说本来的话
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("🙋 「早"), "接着说本来的话");
        // 之后任意键起身
        assert!(matches!(w.handle(1, Key::Other), Action::Redraw));
        assert!(!w.render(1).contains("「今天是隔壁活動日」"));
    }

    #[test]
    fn no_letter_or_ghost_talk_is_single_line() {
        // 没配信：大爷直接说本来的话，一下就完
        let mut w = World::default();
        w.update_npc(6 * 60);
        w.join(1, 0);
        put(&mut w, 1, (16, 4));
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("🙋 「早"));
        assert!(
            matches!(w.handle(1, Key::Space), Action::Redraw),
            "单句，空格即起身"
        );

        // 幽灵：即使配了信也只吐省略号
        let mut w = World::default();
        w.set_letter_text("今天是隔壁活動日".into());
        w.update_npc(60); // 凌晨幽灵在钟右 (0,5)
        w.join(2, 0);
        put(&mut w, 2, (1, 5));
        assert!(matches!(w.handle(2, Key::Space), Action::Talk));
        let s = w.render(2);
        assert!(
            s.contains("👻 「......」") && !s.contains("活動日"),
            "幽灵单句"
        );
    }

    #[test]
    fn weather_icon_maps_conditions() {
        // 天气优先，不分昼夜
        assert_eq!(weather_icon(true, 95, 10, 0.0), "⛈️", "雷雨");
        assert_eq!(weather_icon(false, 71, 90, 0.0), "🌨️", "雪");
        assert_eq!(weather_icon(true, 0, 0, 0.3), "🌧️", "有雨量就是雨");
        assert_eq!(weather_icon(true, 51, 80, 0.0), "🌧️", "毛毛雨算雨");
        assert_eq!(weather_icon(true, 45, 100, 0.0), "🌫️", "雾");
        // 白天晴/多云/阴
        assert_eq!(weather_icon(true, 0, 10, 0.0), "☀️", "白天晴");
        assert_eq!(weather_icon(true, 2, 50, 0.0), "⛅", "白天多云");
        assert_eq!(weather_icon(true, 3, 90, 0.0), "☁️", "白天阴");
        // 夜里
        assert_eq!(weather_icon(false, 0, 10, 0.0), "🌙", "晴夜");
        assert_eq!(weather_icon(false, 3, 90, 0.0), "☁️", "阴夜");
    }

    #[test]
    fn weather_overrides_fallback_icon_but_keeps_time() {
        let mut w = World::default();
        w.join(1, 0);
        w.update_npc(12 * 60 + 30); // 12:30
                                    // 无天气：兜底 ⚡ + 时间
        assert!(w.render(1).contains("⚡  12:30"));
        // 天气任务写入雨：图标变 🌧️，时间保留
        assert!(w.set_weather(Some("🌧️")));
        let s = w.render(1);
        assert!(s.contains("🌧️  12:30"), "图标随天气、时间照旧");
        assert!(!s.contains("⚡  12:30"));
        // 天气丢失回落：又变回 ⚡
        assert!(w.set_weather(None));
        assert!(w.render(1).contains("⚡  12:30"));
    }

    #[test]
    fn daily_cart_appears_in_range() {
        let mut w = World::default();
        assert!(w.tick_cart(6 * 60, 100), "06:00 起放一辆车");
        let at = w.cart.as_ref().unwrap().at;
        assert!(COURTYARD_ROWS.contains(&at.0), "落在出生点到树之间那几行");
        assert_eq!(MAP[at.0][at.1], '.', "落在空地");
        assert!(!w.tick_cart(9 * 60, 100), "当天只放一辆");
        assert!(w.tick_cart(6 * 60, 101), "隔天再放一辆");
    }

    #[test]
    fn pushing_cart_moves_both() {
        let mut w = World::default();
        w.join(1, 0);
        w.place_cart((12, 4));
        put(&mut w, 1, (12, 3)); // 车左侧
        assert!(w.render(1).contains("🛒 購物車"), "挨着车有提示");
        assert!(matches!(w.handle(1, Key::Space), Action::Talk));
        assert!(w.render(1).contains("推得動"), "按空格告知能推");
        assert!(matches!(w.handle(1, Key::Other), Action::Redraw)); // 起身
                                                                    // 朝右推：车和人都往右一格
        assert!(matches!(w.handle(1, Key::Right), Action::Redraw));
        assert_eq!(w.cart.as_ref().unwrap().at, (12, 5), "车往右一格");
        assert_eq!(w.pilgrims[&1].at, (12, 4), "人跟进车原来那格");
    }

    #[test]
    fn cart_bounces_off_wall() {
        let mut w = World::default();
        w.join(1, 0);
        w.place_cart((12, 1)); // 紧挨左墙（col0 是花篱）
        put(&mut w, 1, (12, 2)); // 车右侧
                                 // 朝左推 → 车前方是墙 → 垂直弹到上或下
        assert!(matches!(w.handle(1, Key::Left), Action::Redraw));
        let c = w.cart.as_ref().unwrap().at;
        assert!(c == (11, 1) || c == (13, 1), "撞墙往垂直弹开，实际 {c:?}");
        assert_ne!(c, (12, 0), "没被推进墙里");
        assert_eq!(w.pilgrims[&1].at, (12, 1), "人进到车原来那格");
    }

    #[test]
    fn cart_bounces_off_a_person_too() {
        let mut w = World::default();
        w.join(1, 0);
        w.join(2, 3);
        w.place_cart((12, 4));
        put(&mut w, 1, (12, 3)); // 推的人在车左侧
        put(&mut w, 2, (12, 5)); // 另一个人堵在车右侧
                                 // 朝右推 → 车前方是人（碰撞体）→ 垂直弹开，不是撞停
        assert!(matches!(w.handle(1, Key::Right), Action::Redraw));
        let c = w.cart.as_ref().unwrap().at;
        assert!(c == (11, 4) || c == (13, 4), "撞人往垂直弹开，实际 {c:?}");
        assert_ne!(c, (12, 5), "没叠到人身上");
        assert_eq!(w.pilgrims[&1].at, (12, 4), "人进到车原来那格");
    }

    #[test]
    fn cart_cannot_enter_the_hall() {
        let mut w = World::default();
        w.join(1, 0);
        w.place_cart((7, 3)); // 广场顶排、正对寺门开口(6,3)
        put(&mut w, 1, (8, 3)); // 车下方
                                // 朝上推 → 前方(6,3)是大殿，行政墙挡住 → 垂直弹开，绝不进大殿
        assert!(matches!(w.handle(1, Key::Up), Action::Redraw));
        let c = w.cart.as_ref().unwrap().at;
        assert!(c.0 >= PLAZA_TOP, "车没进大殿，实际 {c:?}");
        assert_eq!(c, (7, 2), "往可走的一侧弹开（(7,4)是香炉）");
    }

    #[test]
    fn cart_pushed_outside_returns_to_courtyard() {
        let mut w = World::default();
        w.join(1, 0);
        w.place_cart((17, 4)); // 下院底门开口
        put(&mut w, 1, (16, 4)); // 车上方
                                 // 朝下推 → 越过底门出寺 → 车回到出生那几行随机位置
        assert!(matches!(w.handle(1, Key::Down), Action::Redraw));
        let c = w.cart.as_ref().unwrap().at;
        assert!(COURTYARD_ROWS.contains(&c.0), "回到出生那几行，实际 {c:?}");
        assert_ne!(c, (17, 4), "不在原地");
        assert_eq!(w.pilgrims[&1].at, (17, 4), "人进到车原来那格");
    }

    #[test]
    fn walk_hint_only_before_first_move() {
        let mut w = World::default();
        w.join(1, 0); // 生成在广场中央，闲着没动
        assert!(w.render(1).contains("方向鍵走動"), "刚进寺给一次走动提示");
        // 往左挪一步到中性空地
        assert!(matches!(w.handle(1, Key::Left), Action::Redraw));
        assert!(!w.render(1).contains("方向鍵走動"), "动过之后不再提示");
    }

    #[test]
    fn sky_mark_is_hhmm() {
        let m = |h: u32, mm: u32| h * 60 + mm;
        // 没有天气数据时兜底成 ⚡，后面是真实上海时间 HH:MM
        assert_eq!(sky_mark(m(12, 0)), "⚡  12:00");
        assert_eq!(sky_mark(m(5, 9)), "⚡  05:09");
        assert_eq!(sky_mark(m(23, 45)), "⚡  23:45");
        assert_eq!(sky_mark(m(0, 0)), "⚡  00:00");
        // 时间钉在视窗上方，绿点在线数跟在后面
        let mut w = World::default();
        w.join(1, 0);
        w.update_npc(m(14, 23));
        let screen = w.render(1);
        assert!(screen.contains("⚡  14:23"), "无天气时兜底 ⚡ + 时间");
        assert!(screen.contains("● 1"), "顶行绿点显示在线数");
    }

    #[test]
    fn hint_at_bell() {
        let (mut w, me) = one();
        put(&mut w, me, (0, 3));
        assert!(w.render(me).contains("按空格撞鐘"));
    }

    #[test]
    fn bell_and_censer_change_while_active() {
        let mut w = World::default();
        w.join(1, 0);
        w.join(2, 5);
        // 观众站钟附近，看得见钟
        put(&mut w, 2, (1, 1));
        assert!(w.render(2).contains("🔔"), "平时是钟");
        // 有人撞钟 → 全场看到钟泛起星光
        put(&mut w, 1, (0, 3));
        assert!(matches!(w.handle(1, Key::Space), Action::Ring));
        let seen = w.render(2);
        assert!(seen.contains("✨") && !seen.contains("🔔"), "撞钟时钟变 ✨");
        w.handle(1, Key::Other); // 起身
        assert!(w.render(2).contains("🔔"), "起身后复原");

        // 观众挪到香炉附近
        put(&mut w, 2, (9, 2));
        assert!(w.render(2).contains("🔥"), "平时是香炉");
        put(&mut w, 1, (8, 4));
        assert!(matches!(w.handle(1, Key::Space), Action::Burn));
        let seen = w.render(2);
        assert!(
            seen.contains("✨") && !seen.contains("🔥"),
            "烧香时香炉迸 ✨"
        );
    }

    #[test]
    fn choosing_then_confirm() {
        let mut c = Choosing::new();
        assert!(c.render().contains("先擇一副面容"));
        assert!(matches!(c.handle(Key::Right), Choose::Redraw));
        assert!(matches!(c.handle(Key::Right), Choose::Redraw));
        assert!(matches!(c.handle(Key::Enter), Choose::Picked(2)));
    }

    #[test]
    fn choosing_quit_leaves() {
        let mut c = Choosing::new();
        assert!(matches!(c.handle(Key::Quit), Choose::Leave));
        assert!(matches!(c.handle(Key::Other), Choose::Idle));
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

    #[test]
    fn fragmented_csi_arrows_reassemble() {
        let mut p = KeyParser::default();
        assert!(p.feed(b"\x1b").is_empty());
        assert!(p.feed(b"[").is_empty());
        assert_eq!(p.feed(b"A"), vec![Key::Up]);
        // 半截 CSI 后再跟完整按键：残段拼上，后续照常
        assert!(p.feed(b"\x1b[").is_empty());
        assert_eq!(p.feed(b"B w"), vec![Key::Down, Key::Space, Key::Up]);
    }
}
