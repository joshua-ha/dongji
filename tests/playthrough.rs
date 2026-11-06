//! 간단한 봇이 게임을 끝까지 플레이해 보는 테스트.
//! `cargo test -- --nocapture`로 난이도 통계를 볼 수 있다.

use dongji::{Ending, Game, Input, PromptKind, Turn};

#[derive(Clone, Copy)]
struct Bot {
    class: usize,
    absorb: bool,
    accept_offer: bool,
    vow: usize,
    smart: bool,
}

fn find(t: &Turn, prefixes: &[&str]) -> Option<usize> {
    for p in prefixes {
        if let Some(i) = t
            .prompt
            .options
            .iter()
            .position(|o| o.starts_with(p) && !o.contains("주력 부족"))
        {
            return Some(i);
        }
    }
    None
}

fn hp_ratio(game: &Game) -> f64 {
    let p = game.player().unwrap();
    p.hp as f64 / p.max_hp as f64
}

/// returns (ending, retries)
fn play(seed: u64, bot: Bot) -> (Ending, u32) {
    let mut g = Game::new(seed);
    let mut t = g.start();
    let mut retries = 0;
    for _ in 0..20_000 {
        for l in &t.logs {
            assert!(!l.text.contains("{name}"), "치환 안 된 이름: {}", l.text);
        }
        assert!(t.to_json().starts_with("{\"logs\":["));
        if let Some(e) = g.ending() {
            return (e, retries);
        }
        let input = if t.prompt.kind == PromptKind::Text {
            Input::Text(String::new())
        } else {
            let title = t.prompt.title.clone();
            let i = if t.prompt.options.first().map(|o| o.starts_with("새로운 기록")).unwrap_or(false) {
                0
            } else if title.starts_with("당신은 어느") {
                bot.class
            } else if title.starts_with("현장 조사") {
                t.prompt.options.iter().position(|o| !o.contains("조사 완료")).unwrap()
            } else if title.starts_with("전투 전") {
                bot.vow
            } else if title.contains("턴 —") {
                let heavy = t.logs.iter().any(|l| l.text.contains("강력한 일격을 준비"));
                if bot.smart && hp_ratio(&g) < 0.4 {
                    find(&t, &["역전 치유", "회복 부적", "저주 흡혈", "방어"]).unwrap()
                } else if heavy && bot.smart {
                    find(&t, &["방어"]).unwrap()
                } else {
                    find(&t, &["영역 전개", "술식", "기본 공격"]).unwrap()
                }
            } else if title.starts_with("무명이 손을") {
                if bot.accept_offer { 1 } else { 0 }
            } else if title.starts_with("잔해를") {
                if bot.absorb { 1 } else { 0 }
            } else if title.starts_with("어떤 힘") {
                2
            } else if title.starts_with("다음 장") {
                0
            } else if title.starts_with("다시 써") {
                retries += 1;
                0
            } else {
                panic!("모르는 프롬프트: {:?}", t.prompt);
            };
            Input::Choice(i)
        };
        t = g.send(input);
    }
    panic!("게임이 끝나지 않음 (seed {})", seed);
}

#[test]
fn sorcerer_reaches_dawn() {
    let (e, _) = play(7, Bot { class: 0, absorb: false, accept_offer: false, vow: 3, smart: true });
    assert_eq!(e, Ending::Dawn);
}

#[test]
fn curse_user_reaches_named() {
    let (e, _) = play(11, Bot { class: 1, absorb: false, accept_offer: false, vow: 3, smart: true });
    assert_eq!(e, Ending::Named);
}

#[test]
fn absorbing_everything_taints_ending() {
    let (e, _) = play(3, Bot { class: 1, absorb: true, accept_offer: false, vow: 3, smart: true });
    assert_eq!(e, Ending::Tainted);
}

#[test]
fn accepting_offer_is_dark_ending() {
    let (e, _) = play(5, Bot { class: 0, absorb: false, accept_offer: true, vow: 3, smart: true });
    assert_eq!(e, Ending::Dark);
}

#[test]
fn same_seed_same_log() {
    let run = |seed| {
        let mut g = Game::new(seed);
        let mut all = g.start().to_json();
        for inp in [Input::Choice(0), Input::Choice(0), Input::Text("하늘".into()), Input::Choice(0), Input::Choice(1)] {
            all.push_str(&g.send(inp).to_json());
        }
        all
    };
    assert_eq!(run(99), run(99));
}

#[test]
fn save_and_continue() {
    // 2장까지 진행한 뒤 저장 → 새 게임에 붙여서 이어하기
    let mut g = Game::new(1);
    g.start();
    g.choose(0); // 새로운 기록
    g.choose(1); // 주저사
    let t = g.input_text("하늘");
    assert!(t.checkpoint.is_some(), "1장 시작 시 체크포인트");
    assert!(t.to_json().contains("\"checkpoint\":\"dongji-save v1"));
    let save = g.save_data().unwrap();

    let mut g2 = Game::new(2);
    g2.attach_save(&save).unwrap();
    let t = g2.start();
    assert!(t.prompt.options[0].starts_with("이어하기 — 제1장"), "{:?}", t.prompt.options);
    let t = g2.choose(0);
    assert!(t.logs.iter().any(|l| l.text.contains("제1장")));
    assert_eq!(g2.player().unwrap().name, "하늘");

    // 손상된 세이브는 거부
    let broken = save.replace("atk=19", "atk=999");
    assert!(Game::new(3).attach_save(&broken).is_err());
    assert!(Game::new(3).attach_save("아무거나").is_err());
}

#[test]
fn save_mid_game_keeps_progress() {
    // 봇으로 3장까지 진행하고 체크포인트를 확인
    let mut g = Game::new(77);
    let mut t = g.start();
    let mut last_cp = None;
    for _ in 0..5000 {
        if let Some(c) = &t.checkpoint { last_cp = Some(c.clone()); }
        if g.stage_index() == 2 && t.prompt.title.starts_with("현장 조사") { break; }
        let i = if t.prompt.kind == PromptKind::Text { t = g.input_text(""); continue }
            else if t.prompt.title.contains("턴 —") { find(&t, &["영역 전개", "술식", "기본 공격"]).unwrap() }
            else if t.prompt.title.starts_with("현장 조사") { t.prompt.options.len() - 1 }
            else { 0 };
        t = g.choose(i);
    }
    let cp = last_cp.unwrap();
    assert!(cp.contains("stage=2"));
    let mut g2 = Game::new(5);
    g2.attach_save(&cp).unwrap();
    g2.start();
    g2.choose(0);
    assert_eq!(g2.stage_index(), 2);
    assert!(g2.player().unwrap().level >= 3);
}

#[test]
fn balance_report() {
    for (label, bot) in [
        ("주술사/정화/서약X", Bot { class: 0, absorb: false, accept_offer: false, vow: 3, smart: true }),
        ("주저사/정화/서약X", Bot { class: 1, absorb: false, accept_offer: false, vow: 3, smart: true }),
        ("주술사/흡수/단기결전", Bot { class: 0, absorb: true, accept_offer: false, vow: 0, smart: true }),
        ("주저사/흡수/무방비", Bot { class: 1, absorb: true, accept_offer: false, vow: 1, smart: true }),
        ("주술사/정화/절제", Bot { class: 0, absorb: false, accept_offer: false, vow: 2, smart: true }),
        ("[무지성] 주술사", Bot { class: 0, absorb: false, accept_offer: false, vow: 3, smart: false }),
        ("[무지성] 주저사", Bot { class: 1, absorb: false, accept_offer: false, vow: 3, smart: false }),
    ] {
        let mut total = 0;
        let n = 40;
        for s in 1..=n {
            total += play(s * 7919, bot).1;
        }
        println!("{label}: 평균 재도전 {:.2}회", total as f64 / n as f64);
    }
}
