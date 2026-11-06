//! 게임 상태 머신.
//!
//! 사용법:
//! ```ignore
//! let mut g = Game::new(seed);
//! let t = g.start();                 // 첫 로그 + 프롬프트
//! let t = g.send(Input::Choice(0));  // 선택지 인덱스 (0부터)
//! let t = g.send(Input::Text("이름".into()));
//! ```
//! 모든 출력은 `Turn.logs`(로그)와 `Turn.prompt`(다음 입력)로만 나온다.

use crate::data::*;
use crate::model::*;
use crate::model::json_escape;
use crate::rng::Rng;

// ───────────────────────── 기본 타입 ─────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Sorcerer,
    CurseUser,
}

impl Class {
    pub fn label(self) -> &'static str {
        match self {
            Class::Sorcerer => "주술사",
            Class::CurseUser => "주저사",
        }
    }
    pub fn technique(self) -> &'static str {
        match self {
            Class::Sorcerer => "청뢰(靑雷)",
            Class::CurseUser => "흑사(黑絲)",
        }
    }
    fn technique_cost(self) -> i32 {
        match self {
            Class::Sorcerer => 30,
            Class::CurseUser => 25,
        }
    }
    fn technique_mult(self) -> f64 {
        match self {
            Class::Sorcerer => 2.5,
            Class::CurseUser => 2.0,
        }
    }
    pub fn special(self) -> &'static str {
        match self {
            Class::Sorcerer => "역전 치유",
            Class::CurseUser => "저주 흡혈",
        }
    }
    fn special_cost(self) -> i32 {
        match self {
            Class::Sorcerer => 40,
            Class::CurseUser => 20,
        }
    }
    pub fn domain(self) -> &'static str {
        match self {
            Class::Sorcerer => "백야정원(白夜庭園)",
            Class::CurseUser => "무간시장(無間市場)",
        }
    }
    fn default_name(self) -> &'static str {
        match self {
            Class::Sorcerer => "서윤",
            Class::CurseUser => "무연",
        }
    }
    /// 흡수 시 오르는 오염도
    fn absorb_taint(self) -> i32 {
        match self {
            Class::Sorcerer => 25,
            Class::CurseUser => 15,
        }
    }
}

const DOMAIN_COST: i32 = 100;
const START_CE: i32 = 40;

// ── 난이도 (초등학생 기준으로 낮춤). 숫자만 바꾸면 전체 난이도가 조절된다.
/// 적 체력 배율
const ENEMY_HP_SCALE: f64 = 0.85;
/// 적 공격력 배율
const ENEMY_ATK_SCALE: f64 = 0.85;
/// 강공격 배율 (예고되는 큰 공격)
const HEAVY_MULT: f64 = 1.8;

const SAVE_HEADER: &str = "dongji-save v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vow {
    None,
    /// 단기결전: 5턴 안에 승리
    Blitz,
    /// 무방비: 방어 금지
    Reckless,
    /// 절제: 회복 금지
    Restraint,
}

impl Vow {
    pub fn label(self) -> &'static str {
        match self {
            Vow::None => "없음",
            Vow::Blitz => "단기결전",
            Vow::Reckless => "무방비",
            Vow::Restraint => "절제",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Player {
    pub name: String,
    pub class: Class,
    pub level: u32,
    pub max_hp: i32,
    pub hp: i32,
    pub max_ce: i32,
    pub ce: i32,
    pub atk: i32,
    pub def: i32,
    pub heal_charms: u32,
    pub ce_charms: u32,
    /// 오염도 0~100. 흡수할수록 오르고, 결말을 바꾼다.
    pub taint: i32,
    pub purified: u32,
    pub absorbed: u32,
}

impl Player {
    fn new(name: String, class: Class) -> Self {
        match class {
            Class::Sorcerer => Player {
                name, class, level: 1, max_hp: 140, hp: 140, max_ce: 120, ce: 0,
                atk: 17, def: 8, heal_charms: 2, ce_charms: 0, taint: 0, purified: 0, absorbed: 0,
            },
            Class::CurseUser => Player {
                name, class, level: 1, max_hp: 125, hp: 125, max_ce: 120, ce: 0,
                atk: 19, def: 5, heal_charms: 1, ce_charms: 1, taint: 10, purified: 0, absorbed: 0,
            },
        }
    }
}

#[derive(Clone, Debug)]
struct Enemy {
    name: &'static str,
    grade: &'static str,
    max_hp: i32,
    hp: i32,
    atk: f64,
    def: i32,
    pattern: &'static [EMove],
    step: usize,
}

impl Enemy {
    fn next_move(&self) -> EMove {
        self.pattern[self.step % self.pattern.len()]
    }
}

#[derive(Clone, Debug)]
struct Battle {
    enemy: Enemy,
    turn: u32,
    vow: Vow,
    vow_broken: bool,
    domain_turns: u8,
    guarding: bool,
    weakness: bool,
    hex_turns: u8,
    enemy_barrier: bool,
    enemy_bound: bool,
    offer_done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Title,
    ClassSelect,
    NameInput,
    Investigate,
    Vow,
    Battle,
    BossOffer,
    Disposal,
    LevelUp,
    Interlude,
    GameOver,
    Ending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BAct {
    Attack,
    Technique,
    Guard,
    Special,
    Domain,
    HealCharm,
    CeCharm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stat {
    Hp,
    Ce,
    Atk,
    Def,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Act {
    StartNew,
    PickClass(Class),
    Spot(usize),
    EndInvestigate,
    PickVow(Vow),
    Fight(BAct),
    Refuse,
    Accept,
    Purify,
    Absorb,
    Grow(Stat),
    NextStage,
    ShowStatus,
    Retry,
    Restart,
    Continue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// 무명의 손을 잡음
    Dark,
    /// 승리했지만 오염도 60% 이상
    Tainted,
    /// 주술사 진엔딩
    Dawn,
    /// 주저사 진엔딩
    Named,
}

// ───────────────────────── 한국어 조사 ─────────────────────────

fn batchim(s: &str) -> bool {
    s.chars()
        .last()
        .map(|c| {
            let u = c as u32;
            (0xAC00..=0xD7A3).contains(&u) && (u - 0xAC00) % 28 != 0
        })
        .unwrap_or(false)
}
fn i_ga(s: &str) -> String {
    format!("{}{}", s, if batchim(s) { "이" } else { "가" })
}
fn eun_neun(s: &str) -> String {
    format!("{}{}", s, if batchim(s) { "은" } else { "는" })
}
fn eul_reul(s: &str) -> String {
    format!("{}{}", s, if batchim(s) { "을" } else { "를" })
}

// ───────────────────────── 세이브 ─────────────────────────
// 사람이 읽을 수 있는 key=value 텍스트. 각 장이 시작되는 순간이 체크포인트다.

fn encode_save(stage: usize, p: &Player) -> String {
    let class = match p.class {
        Class::Sorcerer => "sorcerer",
        Class::CurseUser => "curse_user",
    };
    let body = format!(
        "stage={}\nname={}\nclass={}\nlevel={}\nmax_hp={}\nhp={}\nmax_ce={}\natk={}\ndef={}\nheal_charms={}\nce_charms={}\ntaint={}\npurified={}\nabsorbed={}",
        stage, p.name, class, p.level, p.max_hp, p.hp, p.max_ce, p.atk, p.def,
        p.heal_charms, p.ce_charms, p.taint, p.purified, p.absorbed
    );
    format!("{}\n{}\ncheck={}\n", SAVE_HEADER, body, checksum(&body))
}

fn checksum(s: &str) -> u32 {
    // FNV-1a — 실수로 깨진 파일을 걸러내는 용도
    let mut h: u32 = 0x811c_9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

fn parse_save(data: &str) -> Result<(usize, Player), String> {
    let data = data.replace("\r\n", "\n");
    let mut it = data.lines();
    if it.next() != Some(SAVE_HEADER) {
        return Err("세이브 형식이 아니다".into());
    }
    let mut body_lines = Vec::new();
    let mut check: Option<u32> = None;
    let mut map = std::collections::HashMap::new();
    for line in it {
        if line.is_empty() {
            continue;
        }
        let (k, v) = line.split_once('=').ok_or("잘못된 줄")?;
        if k == "check" {
            check = v.parse().ok();
        } else {
            body_lines.push(line);
            map.insert(k.to_string(), v.to_string());
        }
    }
    if check != Some(checksum(&body_lines.join("\n"))) {
        return Err("세이브가 손상되었다".into());
    }
    let num = |k: &str| -> Result<i32, String> {
        map.get(k).and_then(|v| v.parse::<i32>().ok()).ok_or(format!("{} 값이 없다", k))
    };
    let stage = num("stage")? as usize;
    if stage >= STAGES.len() {
        return Err("잘못된 장 번호".into());
    }
    let class = match map.get("class").map(|s| s.as_str()) {
        Some("sorcerer") => Class::Sorcerer,
        Some("curse_user") => Class::CurseUser,
        _ => return Err("잘못된 직업".into()),
    };
    let name = map.get("name").cloned().unwrap_or_default();
    if name.is_empty() {
        return Err("이름이 없다".into());
    }
    let max_hp = num("max_hp")?.max(1);
    let p = Player {
        name,
        class,
        level: num("level")?.max(1) as u32,
        max_hp,
        hp: num("hp")?.clamp(1, max_hp),
        max_ce: num("max_ce")?.max(1),
        ce: 0,
        atk: num("atk")?,
        def: num("def")?,
        heal_charms: num("heal_charms")?.max(0) as u32,
        ce_charms: num("ce_charms")?.max(0) as u32,
        taint: num("taint")?.clamp(0, 100),
        purified: num("purified")?.max(0) as u32,
        absorbed: num("absorbed")?.max(0) as u32,
    };
    Ok((stage, p))
}

// ───────────────────────── 게임 ─────────────────────────

pub struct Game {
    rng: Rng,
    phase: Phase,
    player: Option<Player>,
    stage: usize,
    snapshot: Option<Player>,
    weakness: bool,
    bonus_ce: i32,
    visited: [bool; 3],
    picks_left: u8,
    battle: Option<Battle>,
    pending_class: Option<Class>,
    ending: Option<Ending>,
    /// 가장 최근 체크포인트(세이브 데이터). 타이틀의 '이어하기'에 쓰인다.
    save_slot: Option<String>,
    /// 이번 Turn에 새로 만들어진 체크포인트 (호스트가 저장하면 됨)
    new_checkpoint: Option<String>,
    acts: Vec<Act>,
    out: Vec<LogEntry>,
}

impl Game {
    pub fn new(seed: u64) -> Self {
        Game {
            rng: Rng::new(seed),
            phase: Phase::Title,
            player: None,
            stage: 0,
            snapshot: None,
            weakness: false,
            bonus_ce: 0,
            visited: [false; 3],
            picks_left: 2,
            battle: None,
            pending_class: None,
            ending: None,
            save_slot: None,
            new_checkpoint: None,
            acts: Vec::new(),
            out: Vec::new(),
        }
    }

    // ── 공개 API ──

    pub fn start(&mut self) -> Turn {
        self.title_screen();
        self.finish()
    }

    pub fn send(&mut self, input: Input) -> Turn {
        match (self.phase, input) {
            (Phase::NameInput, Input::Text(t)) => self.set_name(&t),
            (Phase::NameInput, Input::Choice(_)) => self.sys("이름을 입력해야 한다."),
            (_, Input::Choice(i)) => match self.acts.get(i).copied() {
                Some(a) => self.act(a),
                None => self.sys("선택할 수 없는 번호다."),
            },
            (_, Input::Text(_)) => self.sys("지금은 선택지 중에서 골라야 한다."),
        }
        self.finish()
    }

    pub fn choose(&mut self, index: usize) -> Turn {
        self.send(Input::Choice(index))
    }

    pub fn input_text(&mut self, text: &str) -> Turn {
        self.send(Input::Text(text.to_string()))
    }

    pub fn player(&self) -> Option<&Player> {
        self.player.as_ref()
    }

    pub fn stage_index(&self) -> usize {
        self.stage
    }

    /// 호스트가 보관하던 세이브 데이터를 넣는다. `start()` 전에 호출하면
    /// 타이틀에 '이어하기'가 나타난다. 데이터가 깨져 있으면 Err.
    pub fn attach_save(&mut self, data: &str) -> Result<(), String> {
        parse_save(data)?;
        self.save_slot = Some(data.to_string());
        Ok(())
    }

    /// 가장 최근 체크포인트 (각 장이 시작될 때 자동으로 만들어진다).
    pub fn save_data(&self) -> Option<String> {
        self.save_slot.clone()
    }

    pub fn ending(&self) -> Option<Ending> {
        self.ending
    }

    // ── 로그 헬퍼 ──

    fn push(&mut self, kind: LogKind, speaker: Option<String>, text: impl Into<String>) {
        self.out.push(LogEntry { kind, speaker, text: text.into() });
    }
    fn title(&mut self, t: impl Into<String>) {
        self.push(LogKind::Title, None, t)
    }
    fn narr(&mut self, t: impl Into<String>) {
        self.push(LogKind::Narration, None, t)
    }
    fn sys(&mut self, t: impl Into<String>) {
        self.push(LogKind::System, None, t)
    }
    fn status(&mut self, t: impl Into<String>) {
        self.push(LogKind::Status, None, t)
    }
    fn blog(&mut self, t: impl Into<String>) {
        self.push(LogKind::Battle, None, t)
    }
    fn danger(&mut self, t: impl Into<String>) {
        self.push(LogKind::Danger, None, t)
    }
    fn reward(&mut self, t: impl Into<String>) {
        self.push(LogKind::Reward, None, t)
    }

    fn subst(&self, s: &str) -> String {
        match &self.player {
            Some(p) => s.replace("{name}", &p.name),
            None => s.to_string(),
        }
    }

    fn lines(&mut self, ls: &[Line]) {
        for (sp, tx) in ls {
            let text = self.subst(tx);
            if sp.is_empty() {
                self.narr(text);
            } else {
                let speaker = self.subst(sp);
                self.push(LogKind::Dialogue, Some(speaker), text);
            }
        }
    }

    fn pl(&self) -> &Player {
        self.player.as_ref().expect("player")
    }
    fn plm(&mut self) -> &mut Player {
        self.player.as_mut().expect("player")
    }
    fn bt(&self) -> &Battle {
        self.battle.as_ref().expect("battle")
    }
    fn btm(&mut self) -> &mut Battle {
        self.battle.as_mut().expect("battle")
    }

    fn finish(&mut self) -> Turn {
        let prompt = self.build_prompt();
        let state = self.state_json();
        Turn { logs: std::mem::take(&mut self.out), prompt, checkpoint: self.new_checkpoint.take(), state }
    }

    // ── 흐름 ──

    fn title_screen(&mut self) {
        self.phase = Phase::Title;
        self.title("冬至夜行 · 동지야행");
        self.narr("1년 중 가장 긴 밤, 서울 아래에서 이름이 사라진다.");
        self.narr("이것은 그 밤까지의 엿새를 적은 기록이다.");
    }

    fn act(&mut self, a: Act) {
        match a {
            Act::StartNew => {
                self.phase = Phase::ClassSelect;
                self.sys("저주를 다루는 자는 두 부류로 나뉜다. 규율 아래 싸우는 자와, 규율 밖에서 살아남는 자.");
            }
            Act::PickClass(c) => {
                self.pending_class = Some(c);
                match c {
                    Class::Sorcerer => {
                        self.sys("【주술사】 주술계 소속. 매 턴 주력이 자연 회복되고, 치유 술식을 쓴다.");
                        self.sys(format!("술식 「{}」 · 특수 「{}」 · 영역 「{}」", c.technique(), c.special(), c.domain()));
                    }
                    Class::CurseUser => {
                        self.sys("【주저사】 무소속 저주 사용자. 받은 피해를 주력으로 바꾸고(원한), 적의 생기를 빨아들인다.");
                        self.sys(format!("술식 「{}」 · 특수 「{}」 · 영역 「{}」", c.technique(), c.special(), c.domain()));
                        self.sys("주저사는 흡수에 익숙해 오염이 덜 쌓이지만, 이미 오염도 10%에서 시작한다.");
                    }
                }
                self.phase = Phase::NameInput;
            }
            Act::Spot(i) => self.investigate(i),
            Act::EndInvestigate => self.go_vow(),
            Act::PickVow(v) => self.begin_battle(v),
            Act::Fight(b) => self.fight(b),
            Act::Accept => {
                self.narr("당신은 천천히 손을 내밀었다.");
                self.finish_game(Ending::Dark);
            }
            Act::Refuse => {
                let refuse = match self.pl().class {
                    Class::Sorcerer => REFUSE_SORCERER,
                    Class::CurseUser => REFUSE_CURSE,
                };
                self.lines(refuse);
                self.btm().enemy.atk *= 1.15;
                self.danger("무명의 주력이 폭주한다! (적 공격력 상승)");
                self.phase = Phase::Battle;
                self.enemy_phase();
            }
            Act::Purify => {
                let st = &STAGES[self.stage];
                self.lines(st.purify_text);
                self.plm().purified += 1;
                self.add_taint(-5);
                self.reward("업이 맑아졌다. (오염도 -5)");
                self.level_up_start();
            }
            Act::Absorb => {
                let st = &STAGES[self.stage];
                self.lines(st.absorb_text);
                let t = self.pl().class.absorb_taint();
                {
                    let p = self.plm();
                    p.absorbed += 1;
                    p.atk += 3;
                    p.max_ce += 15;
                }
                self.reward("공격력 +3, 최대 주력 +15");
                self.add_taint(t);
                self.level_up_start();
            }
            Act::Grow(s) => {
                let msg = {
                    let p = self.plm();
                    match s {
                        Stat::Hp => { p.max_hp += 25; p.hp += 25; "최대 HP +25" }
                        Stat::Ce => { p.max_ce += 20; "최대 주력 +20" }
                        Stat::Atk => { p.atk += 4; "공격력 +4" }
                        Stat::Def => { p.def += 3; "방어력 +3" }
                    }
                };
                self.reward(msg);
                let heal = {
                    let p = self.plm();
                    let before = p.hp;
                    p.hp = (p.hp + p.max_hp / 2).min(p.max_hp);
                    p.ce = 0;
                    p.hp - before
                };
                let ch = STAGES[self.stage].chapter;
                self.title(format!("— {} 끝 —", ch));
                self.sys(format!("잠시 숨을 고른다. (HP +{})", heal));
                self.phase = Phase::Interlude;
            }
            Act::NextStage => {
                self.stage += 1;
                self.start_stage();
            }
            Act::ShowStatus => self.show_status(),
            Act::Retry => {
                self.player = self.snapshot.clone();
                self.sys("기록을 되감는다…");
                self.start_stage();
                // 다시 도전할 때마다 응원 선물 (체크포인트에는 포함되지 않음)
                self.plm().heal_charms += 1;
                self.reward("다시 도전하는 당신에게 회복 부적 1장이 주어졌다.");
            }
            Act::Restart => {
                let seed = self.rng.next_u64();
                let slot = self.save_slot.take();
                *self = Game::new(seed);
                self.save_slot = slot;
                self.title_screen();
            }
            Act::Continue => {
                let data = self.save_slot.clone().unwrap_or_default();
                match parse_save(&data) {
                    Ok((stage, player)) => {
                        let n = player.name.clone();
                        self.player = Some(player);
                        self.stage = stage;
                        self.sys(format!("기록을 펼친다. {}의 이야기가 이어진다.", n));
                        self.start_stage();
                    }
                    Err(e) => {
                        self.save_slot = None;
                        self.danger(format!("기록을 읽을 수 없다. ({})", e));
                    }
                }
            }
        }
    }

    fn set_name(&mut self, raw: &str) {
        let class = self.pending_class.unwrap_or(Class::Sorcerer);
        let trimmed: String = raw.trim().chars().filter(|c| !c.is_control()).take(12).collect();
        let name = if trimmed.is_empty() { class.default_name().to_string() } else { trimmed };
        self.player = Some(Player::new(name.clone(), class));
        self.sys(format!("{} {} — 기록을 시작합니다.", class.label(), name));
        let pro = match class {
            Class::Sorcerer => PROLOGUE_SORCERER,
            Class::CurseUser => PROLOGUE_CURSE,
        };
        self.title("서장 — 이름 지움");
        self.lines(pro);
        self.stage = 0;
        self.start_stage();
    }

    fn start_stage(&mut self) {
        self.snapshot = self.player.clone();
        let data = encode_save(self.stage, self.pl());
        self.save_slot = Some(data.clone());
        self.new_checkpoint = Some(data);
        self.weakness = false;
        self.bonus_ce = 0;
        self.visited = [false; 3];
        self.picks_left = 2;
        self.battle = None;
        let st = &STAGES[self.stage];
        self.title(format!("{} 「{}」", st.chapter, st.title));
        self.sys(st.place);
        self.lines(st.intro);
        let extra = match self.pl().class {
            Class::Sorcerer => st.intro_sorcerer,
            Class::CurseUser => st.intro_curse,
        };
        self.lines(extra);
        self.sys("주령이 모습을 드러내기 전에, 현장을 조사할 수 있다. (두 곳까지)");
        self.phase = Phase::Investigate;
    }

    fn investigate(&mut self, i: usize) {
        if i >= 3 || self.visited[i] {
            self.sys("이미 조사한 곳이다.");
            return;
        }
        self.visited[i] = true;
        self.picks_left -= 1;
        let spot = &STAGES[self.stage].spots[i];
        self.lines(spot.text);
        match spot.result {
            SpotResult::Weakness => {
                self.weakness = true;
                self.reward("약점 파악! 이번 전투에서 적의 방어를 무시하고, 피해 +20%.");
            }
            SpotResult::HealCharm => {
                self.plm().heal_charms += 1;
                self.reward("회복 부적 1장을 얻었다.");
            }
            SpotResult::CeCharm => {
                self.plm().ce_charms += 1;
                self.reward("주력 부적 1장을 얻었다.");
            }
            SpotResult::Ambush => {
                let dmg = {
                    let p = self.plm();
                    let d = (p.max_hp * 12 / 100).min(p.hp - 1).max(0);
                    p.hp -= d;
                    d
                };
                self.danger(format!("기습! HP -{}", dmg));
                self.bonus_ce += 30;
                self.reward("위기 속에서 감각이 날카로워졌다. 전투 시작 시 주력 +30.");
            }
            SpotResult::Lore => {
                self.bonus_ce += 20;
                self.reward("결의가 굳어졌다. 전투 시작 시 주력 +20.");
            }
        }
        if self.picks_left == 0 {
            self.sys("더 조사할 시간이 없다. 기척이 다가온다.");
            self.go_vow();
        }
    }

    fn go_vow(&mut self) {
        self.phase = Phase::Vow;
        self.sys("서약 — 스스로에게 제약을 걸면 그만큼 강해진다. 지켜내면 보상이 있지만, 어기면 대가를 치른다.");
    }

    fn begin_battle(&mut self, vow: Vow) {
        let spec = &STAGES[self.stage].enemy;
        let enemy = Enemy {
            name: spec.name,
            grade: spec.grade,
            max_hp: (spec.hp as f64 * ENEMY_HP_SCALE) as i32,
            hp: (spec.hp as f64 * ENEMY_HP_SCALE) as i32,
            atk: spec.atk as f64 * ENEMY_ATK_SCALE,
            def: spec.def,
            pattern: spec.pattern,
            step: 0,
        };
        self.battle = Some(Battle {
            enemy,
            turn: 1,
            vow,
            vow_broken: false,
            domain_turns: 0,
            guarding: false,
            weakness: self.weakness,
            hex_turns: 0,
            enemy_barrier: false,
            enemy_bound: false,
            offer_done: false,
        });
        let bonus = self.bonus_ce;
        {
            let p = self.plm();
            p.ce = (START_CE + bonus).min(p.max_ce);
        }
        if vow != Vow::None {
            self.reward(format!("서약 「{}」 — 몸 안의 주력이 팽팽하게 조여든다.", vow.label()));
        }
        self.lines(STAGES[self.stage].entrance);
        let (n, g) = (spec.name, spec.grade);
        self.title(format!("⚔ {} [{}]", n, g));
        if self.weakness {
            self.reward("간파한 약점을 노린다. (적 방어 무시, 피해 +20%)");
        }
        self.phase = Phase::Battle;
    }

    // ── 전투 ──

    fn gain_ce(&mut self, amount: i32) {
        let mul = {
            let b = self.bt();
            if b.vow == Vow::Restraint && !b.vow_broken { 2 } else { 1 }
        };
        let p = self.plm();
        p.ce = (p.ce + amount * mul).min(p.max_ce);
    }

    fn heal_player(&mut self, amount: i32) -> i32 {
        let p = self.plm();
        let before = p.hp;
        p.hp = (p.hp + amount).min(p.max_hp);
        p.hp - before
    }

    fn add_taint(&mut self, delta: i32) {
        let (before, after) = {
            let p = self.plm();
            let b = p.taint;
            p.taint = (p.taint + delta).clamp(0, 100);
            (b, p.taint)
        };
        if delta > 0 {
            self.danger(format!("오염도 +{} (현재 {}%)", after - before, after));
            if before < 30 && after >= 30 {
                self.danger("손끝이 차갑다. 몸 안 어딘가에서 누군가 속삭인다.");
            }
            if before < 60 && after >= 60 {
                self.danger("그림자가 당신보다 한 박자 늦게 움직인다. (오염도 60% 이상 — 이대로라면 결말이 바뀐다)");
            }
            if before < 90 && after >= 90 {
                self.danger("거울 속의 당신이, 먼저 웃었다.");
            }
        }
    }

    fn fight(&mut self, a: BAct) {
        let class = self.pl().class;
        let ce = self.pl().ce;
        let cost = match a {
            BAct::Technique => class.technique_cost(),
            BAct::Special => class.special_cost(),
            BAct::Domain => DOMAIN_COST,
            _ => 0,
        };
        if ce < cost {
            self.sys(format!("주력이 부족하다. (필요 {}, 보유 {})", cost, ce));
            return;
        }
        if a == BAct::Domain && self.bt().domain_turns > 0 {
            self.sys("이미 영역이 펼쳐져 있다.");
            return;
        }
        if a == BAct::HealCharm && self.pl().heal_charms == 0 {
            self.sys("회복 부적이 없다.");
            return;
        }
        if a == BAct::CeCharm && self.pl().ce_charms == 0 {
            self.sys("주력 부적이 없다.");
            return;
        }

        let (vow, vow_active) = {
            let b = self.bt();
            (b.vow, !b.vow_broken)
        };

        // 무방비 서약: 방어하면 파기
        if a == BAct::Guard && vow == Vow::Reckless && vow_active {
            let loss = {
                let p = self.plm();
                let l = (p.max_hp / 5).min(p.hp - 1).max(0);
                p.hp -= l;
                l
            };
            self.btm().vow_broken = true;
            self.danger(format!("서약 파기! '무방비'의 서약을 어겼다. 대가로 HP -{}", loss));
        }
        // 절제 서약: 회복 계열을 쓰면 행동 후 파기
        let restraint_break = vow == Vow::Restraint
            && vow_active
            && matches!(a, BAct::Special | BAct::HealCharm | BAct::CeCharm);

        self.plm().ce -= cost;
        let pname = self.pl().name.clone();
        let ename = self.bt().enemy.name;

        match a {
            BAct::Attack => {
                self.blog(format!("{} 주먹에 주력을 실어 내지른다.", eun_neun(&pname)));
                self.player_hit(1.0, false);
                self.gain_ce(12);
            }
            BAct::Technique => {
                match class {
                    Class::Sorcerer => self.blog(format!(
                        "술식 「청뢰」 — 푸른 번개가 손끝에서 갈라져 {} 꿰뚫는다.",
                        eul_reul(ename)
                    )),
                    Class::CurseUser => self.blog(format!(
                        "술식 「흑사」 — 그림자에서 검은 실이 솟구쳐 {} 옭아맨다.",
                        eul_reul(ename)
                    )),
                }
                self.player_hit(class.technique_mult(), true);
                if class == Class::CurseUser && self.bt().enemy.hp > 0 && self.rng.chance(0.35) {
                    self.btm().enemy_bound = true;
                    self.blog("검은 실이 팽팽하게 조여든다! 적의 다음 행동을 봉쇄했다.");
                }
            }
            BAct::Guard => {
                self.btm().guarding = true;
                self.blog("자세를 낮추고 주력을 온몸에 두른다.");
                self.gain_ce(15);
            }
            BAct::Special => match class {
                Class::Sorcerer => {
                    let amt = self.pl().max_hp * 35 / 100;
                    let h = self.heal_player(amt);
                    self.reward(format!("역전 치유 — 음(陰)의 주력을 뒤집어 상처를 되감는다. HP +{}", h));
                }
                Class::CurseUser => {
                    self.blog("저주 흡혈 — 손톱 끝으로 상대의 생기를 빨아들인다.");
                    let d = self.player_hit(1.3, false);
                    let h = self.heal_player(d * 6 / 10);
                    self.reward(format!("HP +{}", h));
                    self.add_taint(2);
                }
            },
            BAct::Domain => {
                let dname = class.domain();
                self.title(format!("영역 전개 — 「{}」", dname));
                match class {
                    Class::Sorcerer => self.narr("밤이 걷히고, 끝없이 하얀 정원이 펼쳐진다. 이 안에서 해는 지지 않는다. 숨을 곳도 없다."),
                    Class::CurseUser => self.narr("사방에 좌판이 들어선 끝없는 저잣거리. 이곳에서는 모든 것에 값이 매겨지고, 빚은 반드시 받아낸다."),
                }
                self.btm().domain_turns = 3;
                self.sys("3턴 동안 모든 공격이 필중하고 위력이 오른다. 적은 매 턴 추가 피해를 입는다.");
                self.player_hit(1.8, true);
            }
            BAct::HealCharm => {
                self.plm().heal_charms -= 1;
                let h = self.heal_player(50);
                self.reward(format!("회복 부적을 태웠다. HP +{}", h));
            }
            BAct::CeCharm => {
                self.plm().ce_charms -= 1;
                let p = self.plm();
                p.ce = (p.ce + 50).min(p.max_ce);
                self.reward("주력 부적을 태웠다. 주력 +50");
            }
        }

        if restraint_break {
            self.plm().ce = 0;
            self.btm().vow_broken = true;
            self.danger("서약 파기! '절제'의 서약을 어겼다. 모아둔 주력이 흩어진다.");
        }

        if self.check_enemy_dead() {
            return;
        }

        // 영역 필중 추가타
        if self.bt().domain_turns > 0 {
            let d = {
                let atk = self.pl().atk as f64;
                (atk * 0.6 * self.rng.range_f(0.9, 1.1)).max(1.0) as i32
            };
            let e = &mut self.btm().enemy;
            e.hp = (e.hp - d).max(0);
            let (hp, mx) = (e.hp, e.max_hp);
            let dn = class.domain();
            self.blog(format!("「{}」의 필중 효과 — {}에게 {} 피해. (적 HP {}/{})", dn, ename, d, hp, mx));
            if self.check_enemy_dead() {
                return;
            }
        }

        // 최종 보스 제안 이벤트
        if self.stage == STAGES.len() - 1 {
            let b = self.bt();
            if !b.offer_done && b.enemy.hp * 2 <= b.enemy.max_hp {
                self.btm().offer_done = true;
                self.lines(FINAL_OFFER);
                if class == Class::CurseUser {
                    self.lines(FINAL_OFFER_CURSE);
                }
                self.phase = Phase::BossOffer;
                return;
            }
        }

        self.enemy_phase();
    }

    fn player_hit(&mut self, mult: f64, tech: bool) -> i32 {
        let atk = self.pl().atk as f64;
        let (mut m, def, barrier, crit_p) = {
            let b = self.bt();
            let active = !b.vow_broken;
            let mut m = mult;
            let mut def = b.enemy.def as f64;
            if b.weakness {
                m *= 1.2;
                def = 0.0;
            }
            match b.vow {
                Vow::Blitz if active => m *= 1.4,
                Vow::Blitz => m *= 0.8,
                Vow::Reckless if active && tech => m *= 1.6,
                _ => {}
            }
            if b.domain_turns > 0 {
                m *= 1.5;
            }
            if b.hex_turns > 0 {
                m *= 0.75;
            }
            let mut cp = 0.08;
            if b.vow != Vow::None && active {
                cp += 0.10;
            }
            if b.domain_turns > 0 {
                cp += 0.20;
            }
            (m, def, b.enemy_barrier, cp)
        };
        if barrier {
            m *= 0.5;
        }
        let raw = (atk * m - def * 0.5).max(1.0) * self.rng.range_f(0.9, 1.1);
        let crit = self.rng.chance(crit_p);
        let dmg = (if crit { raw * 1.8 } else { raw }).max(1.0) as i32;

        if barrier {
            self.btm().enemy_barrier = false;
            self.blog("단단히 굳은 몸에 막혀 위력이 반감됐다.");
        }
        if crit {
            self.reward("공명 타격! 주먹과 주력이 한 점에서 겹쳐 울렸다. (피해 1.8배, 주력 +20)");
            self.gain_ce(20);
        }
        let (name, hp, mx) = {
            let e = &mut self.btm().enemy;
            e.hp = (e.hp - dmg).max(0);
            (e.name, e.hp, e.max_hp)
        };
        self.blog(format!("→ {}에게 {} 피해. (적 HP {}/{})", name, dmg, hp, mx));
        dmg
    }

    fn check_enemy_dead(&mut self) -> bool {
        if self.bt().enemy.hp <= 0 {
            self.victory();
            true
        } else {
            false
        }
    }

    fn enemy_phase(&mut self) {
        self.enemy_turn();
        if self.pl().hp <= 0 {
            self.game_over();
            return;
        }
        self.end_turn();
    }

    fn enemy_turn(&mut self) {
        let ename = self.bt().enemy.name;
        if self.bt().enemy_bound {
            let b = self.btm();
            b.enemy_bound = false;
            b.enemy.step += 1;
            self.blog(format!("{} 검은 실에 묶여 꼼짝도 하지 못한다!", eun_neun(ename)));
            return;
        }
        let mv = self.bt().enemy.next_move();
        self.btm().enemy.step += 1;
        match mv {
            EMove::Strike => {
                self.blog(format!("{}의 공격!", ename));
                self.enemy_hit(1.0);
            }
            EMove::Heavy => {
                self.danger(format!("{}의 강력한 일격!", ename));
                self.enemy_hit(HEAVY_MULT);
            }
            EMove::Drain => {
                self.blog(format!("{} 주력을 빨아들인다!", i_ga(ename)));
                self.enemy_hit(0.7);
                let stolen = {
                    let p = self.plm();
                    let s = p.ce.min(25);
                    p.ce -= s;
                    s
                };
                if stolen > 0 {
                    self.danger(format!("주력 -{}", stolen));
                }
            }
            EMove::Barrier => {
                self.btm().enemy_barrier = true;
                self.blog(format!("{} 몸을 단단히 굳혔다. 다음에 받는 피해가 절반이 된다.", i_ga(ename)));
            }
            EMove::Hex => {
                self.blog(format!("{}의 저주!", ename));
                self.enemy_hit(0.5);
                self.btm().hex_turns = 3;
                self.danger("저주에 걸렸다 — 2턴 동안 공격력 감소.");
            }
        }
    }

    fn enemy_hit(&mut self, mult: f64) {
        let (atk, guarding, domain) = {
            let b = self.bt();
            (b.enemy.atk, b.guarding, b.domain_turns > 0)
        };
        let pdef = self.pl().def as f64;
        let mut raw = (atk * mult - pdef * 0.5).max(1.0) * self.rng.range_f(0.9, 1.1);
        if guarding {
            raw *= 0.4;
        }
        if domain {
            raw *= 0.7;
        }
        let dmg = raw.max(1.0) as i32;
        let (name, hp, mx, class) = {
            let p = self.plm();
            p.hp = (p.hp - dmg).max(0);
            (p.name.clone(), p.hp, p.max_hp, p.class)
        };
        if guarding {
            self.blog("방어 자세로 충격을 흘려냈다.");
        }
        self.danger(format!("← {} {} 피해를 입었다. (HP {}/{})", i_ga(&name), dmg, hp, mx));
        if class == Class::CurseUser && hp > 0 {
            let g = (dmg / 2).max(1);
            self.gain_ce(g);
            self.reward(format!("원한이 주력으로 바뀐다. (주력 +{})", g));
        }
    }

    fn end_turn(&mut self) {
        let class = self.pl().class;
        let (domain_end, hex_end, blitz_fail) = {
            let b = self.btm();
            b.guarding = false;
            b.turn += 1;
            let mut de = false;
            if b.domain_turns > 0 {
                b.domain_turns -= 1;
                de = b.domain_turns == 0;
            }
            let mut he = false;
            if b.hex_turns > 0 {
                b.hex_turns -= 1;
                he = b.hex_turns == 0;
            }
            let bf = b.vow == Vow::Blitz && !b.vow_broken && b.turn > 5;
            if bf {
                b.vow_broken = true;
            }
            (de, he, bf)
        };
        if domain_end {
            self.sys("영역이 무너졌다.");
        }
        if hex_end {
            self.sys("저주가 풀렸다.");
        }
        if blitz_fail {
            self.plm().ce = 0;
            self.danger("서약 파기! 5턴 안에 끝내지 못했다. 주력이 소멸하고, 이후 피해가 20% 줄어든다.");
        }
        if class == Class::Sorcerer {
            self.gain_ce(10);
        }
    }

    fn victory(&mut self) {
        let b = self.battle.take().expect("battle");
        self.title(format!("{} 쓰러뜨렸다!", eul_reul(b.enemy.name)));
        if b.vow != Vow::None && !b.vow_broken {
            self.plm().max_ce += 10;
            self.reward(format!("'{}' 서약을 끝까지 지켜냈다. 최대 주력 +10.", b.vow.label()));
        }
        let st = &STAGES[self.stage];
        self.lines(st.victory);
        if self.stage == STAGES.len() - 1 {
            let p = self.pl();
            let ending = if p.taint >= 60 {
                Ending::Tainted
            } else if p.class == Class::Sorcerer {
                Ending::Dawn
            } else {
                Ending::Named
            };
            self.finish_game(ending);
        } else {
            self.phase = Phase::Disposal;
            self.sys(format!("쓰러진 {}의 잔해가 아직 꿈틀거린다. 어떻게 할까?", b.enemy.name));
        }
    }

    fn level_up_start(&mut self) {
        let lv = {
            let p = self.plm();
            p.level += 1;
            p.level
        };
        self.reward(format!("레벨 업! Lv.{}", lv));
        self.phase = Phase::LevelUp;
    }

    fn game_over(&mut self) {
        self.battle = None;
        let n = self.pl().name.clone();
        self.danger(format!("{}의 시야가 검게 물든다…", n));
        self.title("— 기록은 여기서 끊겨 있다 —");
        self.phase = Phase::GameOver;
    }

    fn finish_game(&mut self, ending: Ending) {
        self.battle = None;
        self.ending = Some(ending);
        let (title, ls) = match ending {
            Ending::Dark => ("이름 없는 왕", ENDING_DARK),
            Ending::Tainted => ("잔예(殘穢)의 왕관", ENDING_TAINTED),
            Ending::Dawn => ("백야의 새벽", ENDING_DAWN),
            Ending::Named => ("이름을 부르는 사람", ENDING_NAMED),
        };
        self.title(format!("엔딩 — {}", title));
        self.lines(ls);
        let p = self.pl().clone();
        self.status(format!(
            "{} · {} Lv.{} · 정화 {} · 흡수 {} · 최종 오염도 {}%",
            p.name, p.class.label(), p.level, p.purified, p.absorbed, p.taint
        ));
        self.title("冬至夜行 · 끝");
        self.phase = Phase::Ending;
    }

    fn show_status(&mut self) {
        let p = self.pl().clone();
        self.status(format!("{} · {} Lv.{}", p.name, p.class.label(), p.level));
        self.status(format!("HP {}/{} · 최대 주력 {} · 공격력 {} · 방어력 {}", p.hp, p.max_hp, p.max_ce, p.atk, p.def));
        self.status(format!("회복 부적 {} · 주력 부적 {}", p.heal_charms, p.ce_charms));
        self.status(format!("오염도 {}% · 정화 {} · 흡수 {}", p.taint, p.purified, p.absorbed));
        self.status(format!(
            "술식 「{}」 · 특수 「{}」 · 영역 「{}」",
            p.class.technique(), p.class.special(), p.class.domain()
        ));
    }

    fn battle_hud(&mut self) {
        let p = self.pl().clone();
        let b = self.bt().clone();
        self.status(format!(
            "{} Lv.{} · HP {}/{} · 주력 {}/{} · 오염도 {}%",
            p.name, p.level, p.hp, p.max_hp, p.ce, p.max_ce, p.taint
        ));
        self.status(format!("{} [{}] · HP {}/{}", b.enemy.name, b.enemy.grade, b.enemy.hp, b.enemy.max_hp));
        if b.vow != Vow::None {
            let state = if b.vow_broken {
                "파기됨".to_string()
            } else if b.vow == Vow::Blitz {
                format!("유지 중 · 남은 턴 {}", 6 - b.turn as i32)
            } else {
                "유지 중".to_string()
            };
            self.status(format!("서약 「{}」 {}", b.vow.label(), state));
        }
        if b.domain_turns > 0 {
            self.status(format!("영역 「{}」 전개 중 (남은 {}턴)", p.class.domain(), b.domain_turns));
        }
        if b.hex_turns > 0 {
            self.status(format!("저주: 공격력 감소 (남은 {}턴)", b.hex_turns.saturating_sub(1).max(1)));
        }
        if b.enemy_barrier {
            self.status(format!("{} 몸을 굳히고 있다 (다음 피해 절반)", i_ga(b.enemy.name)));
        }
        let e = b.enemy.name;
        if b.enemy_bound {
            self.sys(format!("▶ {}: 검은 실에 묶여 있다. (다음 행동 불가)", e));
            return;
        }
        match b.enemy.next_move() {
            EMove::Strike => self.sys(format!("▶ {}: 공격하려 한다.", e)),
            EMove::Heavy => self.danger(format!("▶ {}: 강력한 일격을 준비하고 있다! (방어 권장)", e)),
            EMove::Drain => self.sys(format!("▶ {}: 주력을 빨아들이려 한다.", e)),
            EMove::Barrier => self.sys(format!("▶ {}: 몸을 굳히려 한다. (주력을 모을 기회)", e)),
            EMove::Hex => self.sys(format!("▶ {}: 저주를 걸려 한다.", e)),
        }
    }

    // ── 프롬프트 ──

    fn build_prompt(&mut self) -> Prompt {
        let mut opts: Vec<(Act, String)> = Vec::new();
        let title: String;
        let mut kind = PromptKind::Choice;

        match self.phase {
            Phase::Title => {
                title = "".into();
                if let Some(desc) = self.save_slot.as_deref().and_then(|d| parse_save(d).ok()).map(|(st, p)| {
                    format!("이어하기 — {} 「{}」 · {} {} Lv.{}", STAGES[st].chapter, STAGES[st].title, p.class.label(), p.name, p.level)
                }) {
                    opts.push((Act::Continue, desc));
                }
                opts.push((Act::StartNew, "새로운 기록을 시작한다".into()));
            }
            Phase::ClassSelect => {
                title = "당신은 어느 쪽인가?".into();
                opts.push((
                    Act::PickClass(Class::Sorcerer),
                    "주술사 — 주술계 소속. 안정적이고 치유가 가능하다. 사라진 스승을 찾는다.".into(),
                ));
                opts.push((
                    Act::PickClass(Class::CurseUser),
                    "주저사 — 무소속 저주 사용자. 맞을수록 강해진다. 잃어버린 자기 이름을 찾는다.".into(),
                ));
            }
            Phase::NameInput => {
                kind = PromptKind::Text;
                let d = self.pending_class.unwrap_or(Class::Sorcerer).default_name();
                title = format!("이름을 입력하세요 (비워두면 '{}')", d);
            }
            Phase::Investigate => {
                title = format!("현장 조사 — 남은 조사 횟수 {}회", self.picks_left);
                for i in 0..3 {
                    let label = STAGES[self.stage].spots[i].label;
                    let s = if self.visited[i] { format!("{} (조사 완료)", label) } else { label.to_string() };
                    opts.push((Act::Spot(i), s));
                }
                opts.push((Act::EndInvestigate, "조사를 마치고 기척을 쫓는다".into()));
            }
            Phase::Vow => {
                title = "전투 전, 스스로에게 서약을 걸 수 있다".into();
                opts.push((Act::PickVow(Vow::Blitz), "단기결전 — 5턴 안에 이긴다 (피해 +40% / 실패 시 주력 소멸, 이후 피해 -20%)".into()));
                opts.push((Act::PickVow(Vow::Reckless), "무방비 — 방어하지 않는다 (술식·영역 피해 +60% / 방어 시 최대 HP의 20% 손실)".into()));
                opts.push((Act::PickVow(Vow::Restraint), "절제 — 회복하지 않는다 (주력 획득 2배 / 회복·흡혈·부적 사용 시 주력 소멸)".into()));
                opts.push((Act::PickVow(Vow::None), "서약하지 않는다".into()));
            }
            Phase::Battle => {
                self.battle_hud();
                let p = self.pl().clone();
                let b = self.bt().clone();
                let c = p.class;
                title = format!("{}턴 — 행동을 선택하세요", b.turn);
                let lack = |cost: i32| if p.ce < cost { " — 주력 부족" } else { "" };
                opts.push((Act::Fight(BAct::Attack), "기본 공격 (주력 +12)".into()));
                opts.push((
                    Act::Fight(BAct::Technique),
                    format!("술식 「{}」 (주력 {}){}", c.technique(), c.technique_cost(), lack(c.technique_cost())),
                ));
                opts.push((Act::Fight(BAct::Guard), "방어 (받는 피해 -60%, 주력 +15)".into()));
                let special_desc = match c {
                    Class::Sorcerer => "HP 35% 회복",
                    Class::CurseUser => "피해를 주고 일부 흡혈",
                };
                opts.push((
                    Act::Fight(BAct::Special),
                    format!("{} — {} (주력 {}){}", c.special(), special_desc, c.special_cost(), lack(c.special_cost())),
                ));
                if b.domain_turns == 0 {
                    opts.push((
                        Act::Fight(BAct::Domain),
                        format!("영역 전개 「{}」 (주력 {}){}", c.domain(), DOMAIN_COST, lack(DOMAIN_COST)),
                    ));
                }
                if p.heal_charms > 0 {
                    opts.push((Act::Fight(BAct::HealCharm), format!("회복 부적 사용 — HP +50 ({}장)", p.heal_charms)));
                }
                if p.ce_charms > 0 {
                    opts.push((Act::Fight(BAct::CeCharm), format!("주력 부적 사용 — 주력 +50 ({}장)", p.ce_charms)));
                }
            }
            Phase::BossOffer => {
                title = "무명이 손을 내밀고 있다".into();
                opts.push((Act::Refuse, "거절한다 — 끝까지 싸운다".into()));
                opts.push((Act::Accept, "손을 잡는다".into()));
            }
            Phase::Disposal => {
                let st = &STAGES[self.stage];
                let t = self.pl().class.absorb_taint();
                title = "잔해를 어떻게 처리할까?".into();
                opts.push((Act::Purify, format!("{} (오염도 -5)", st.purify_label)));
                opts.push((Act::Absorb, format!("{} (공격력 +3, 최대 주력 +15, 오염도 +{})", st.absorb_label, t)));
            }
            Phase::LevelUp => {
                title = "어떤 힘을 기를까?".into();
                opts.push((Act::Grow(Stat::Hp), "체력 단련 — 최대 HP +25".into()));
                opts.push((Act::Grow(Stat::Ce), "주력 수련 — 최대 주력 +20".into()));
                opts.push((Act::Grow(Stat::Atk), "술식 연마 — 공격력 +4".into()));
                opts.push((Act::Grow(Stat::Def), "결계 연습 — 방어력 +3".into()));
            }
            Phase::Interlude => {
                title = "다음 장으로 넘어갈까?".into();
                opts.push((Act::NextStage, "다음 장으로".into()));
                opts.push((Act::ShowStatus, "상태 확인".into()));
            }
            Phase::GameOver => {
                title = "다시 써 내려갈까?".into();
                opts.push((Act::Retry, "이 장을 처음부터 다시 도전한다".into()));
                opts.push((Act::Restart, "처음부터 새로 시작한다".into()));
            }
            Phase::Ending => {
                title = "".into();
                opts.push((Act::Restart, "새로운 기록을 시작한다".into()));
            }
        }

        self.acts = opts.iter().map(|(a, _)| *a).collect();
        let ids = self.acts.iter().map(|a| act_id(*a)).collect();
        let enabled = self.acts.iter().map(|a| self.act_enabled(*a)).collect();
        Prompt { kind, title, options: opts.into_iter().map(|(_, s)| s).collect(), ids, enabled }
    }

    fn act_enabled(&self, a: Act) -> bool {
        match a {
            Act::Spot(i) => !self.visited[i],
            Act::Fight(b) => {
                let p = self.pl();
                let c = p.class;
                match b {
                    BAct::Technique => p.ce >= c.technique_cost(),
                    BAct::Special => p.ce >= c.special_cost(),
                    BAct::Domain => p.ce >= DOMAIN_COST,
                    BAct::HealCharm => p.heal_charms > 0,
                    BAct::CeCharm => p.ce_charms > 0,
                    _ => true,
                }
            }
            _ => true,
        }
    }

    /// UI용 상태 스냅샷 (JSON 객체)
    fn state_json(&self) -> String {
        let scene = match self.phase {
            Phase::Title => "title",
            Phase::ClassSelect => "class_select",
            Phase::NameInput => "name_input",
            Phase::Investigate => "investigate",
            Phase::Vow => "vow",
            Phase::Battle => "battle",
            Phase::BossOffer => "boss_offer",
            Phase::Disposal => "disposal",
            Phase::LevelUp => "level_up",
            Phase::Interlude => "interlude",
            Phase::GameOver => "game_over",
            Phase::Ending => "ending",
        };
        let st = &STAGES[self.stage];
        let player = match &self.player {
            None => "null".to_string(),
            Some(p) => format!(
                "{{\"name\":\"{}\",\"class\":\"{}\",\"level\":{},\"hp\":{},\"max_hp\":{},\"ce\":{},\"max_ce\":{},\"atk\":{},\"def\":{},\"heal_charms\":{},\"ce_charms\":{},\"taint\":{},\"purified\":{},\"absorbed\":{},\"technique\":\"{}\",\"special\":\"{}\",\"domain\":\"{}\"}}",
                json_escape(&p.name),
                match p.class { Class::Sorcerer => "sorcerer", Class::CurseUser => "curse_user" },
                p.level, p.hp, p.max_hp, p.ce, p.max_ce, p.atk, p.def, p.heal_charms, p.ce_charms,
                p.taint, p.purified, p.absorbed, p.class.technique(), p.class.special(), p.class.domain()
            ),
        };
        let (enemy, battle) = match &self.battle {
            None => ("null".to_string(), "null".to_string()),
            Some(b) => {
                let intent = if b.enemy_bound {
                    "bound"
                } else {
                    match b.enemy.next_move() {
                        EMove::Strike => "strike",
                        EMove::Heavy => "heavy",
                        EMove::Drain => "drain",
                        EMove::Barrier => "barrier",
                        EMove::Hex => "hex",
                    }
                };
                (
                    format!(
                        "{{\"name\":\"{}\",\"grade\":\"{}\",\"hp\":{},\"max_hp\":{},\"intent\":\"{}\",\"barrier\":{}}}",
                        b.enemy.name, b.enemy.grade, b.enemy.hp, b.enemy.max_hp, intent, b.enemy_barrier
                    ),
                    format!(
                        "{{\"turn\":{},\"vow\":\"{}\",\"vow_broken\":{},\"domain_turns\":{},\"hex_turns\":{},\"weakness\":{}}}",
                        b.turn,
                        match b.vow { Vow::None => "none", Vow::Blitz => "blitz", Vow::Reckless => "reckless", Vow::Restraint => "restraint" },
                        b.vow_broken, b.domain_turns, b.hex_turns.saturating_sub(1), b.weakness
                    ),
                )
            }
        };
        let ending = match self.ending {
            None => "null",
            Some(Ending::Dark) => "\"dark\"",
            Some(Ending::Tainted) => "\"tainted\"",
            Some(Ending::Dawn) => "\"dawn\"",
            Some(Ending::Named) => "\"named\"",
        };
        format!(
            "{{\"scene\":\"{}\",\"stage\":{},\"stages_total\":{},\"chapter\":\"{}\",\"stage_title\":\"{}\",\"place\":\"{}\",\"picks_left\":{},\"player\":{},\"enemy\":{},\"battle\":{},\"ending\":{}}}",
            scene, self.stage, STAGES.len(), st.chapter, st.title, st.place, self.picks_left, player, enemy, battle, ending
        )
    }
}

fn act_id(a: Act) -> String {
    match a {
        Act::StartNew => "start_new".into(),
        Act::Continue => "continue".into(),
        Act::PickClass(Class::Sorcerer) => "class_sorcerer".into(),
        Act::PickClass(Class::CurseUser) => "class_curse_user".into(),
        Act::Spot(i) => format!("spot_{}", i),
        Act::EndInvestigate => "end_investigate".into(),
        Act::PickVow(v) => format!("vow_{}", match v { Vow::None => "none", Vow::Blitz => "blitz", Vow::Reckless => "reckless", Vow::Restraint => "restraint" }),
        Act::Fight(b) => match b {
            BAct::Attack => "attack",
            BAct::Technique => "technique",
            BAct::Guard => "guard",
            BAct::Special => "special",
            BAct::Domain => "domain",
            BAct::HealCharm => "heal_charm",
            BAct::CeCharm => "ce_charm",
        }
        .into(),
        Act::Refuse => "refuse".into(),
        Act::Accept => "accept".into(),
        Act::Purify => "purify".into(),
        Act::Absorb => "absorb".into(),
        Act::Grow(s) => format!("grow_{}", match s { Stat::Hp => "hp", Stat::Ce => "ce", Stat::Atk => "atk", Stat::Def => "def" }),
        Act::NextStage => "next_stage".into(),
        Act::ShowStatus => "show_status".into(),
        Act::Retry => "retry".into(),
        Act::Restart => "restart".into(),
    }
}
