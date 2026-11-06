//! 터미널용 플레이어. 엔진이 내보내는 로그를 그대로 그린다.
//!
//!   cargo run --release              색상 출력
//!   cargo run --release -- --plain   색상 없이
//!   cargo run --release -- --json    iOS가 받을 JSON 그대로 보기
//!   cargo run --release -- --seed=42 같은 전개 재현
//!   cargo run --release -- --save=내기록.txt  세이브 파일 위치 (기본: dongji_save.txt)
//!
//! 각 장이 시작될 때 자동 저장되고, 다음 실행 때 타이틀에 '이어하기'가 나타난다.

use dongji::{Game, Input, LogKind, PromptKind, Turn};
use std::io::{self, BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};

struct Style {
    plain: bool,
}

impl Style {
    fn c(&self, code: &str, s: &str) -> String {
        if self.plain {
            s.to_string()
        } else {
            format!("\x1b[{}m{}\x1b[0m", code, s)
        }
    }
}

fn render(turn: &Turn, st: &Style) {
    for l in &turn.logs {
        let line = match l.kind {
            LogKind::Title => format!("\n{}", st.c("1;33", &format!("━━ {} ━━", l.text))),
            LogKind::Narration => l.text.clone(),
            LogKind::Dialogue => {
                let sp = l.speaker.clone().unwrap_or_default();
                format!("{} \"{}\"", st.c("1;36", &format!("{}:", sp)), l.text)
            }
            LogKind::System => st.c("90", &format!("· {}", l.text)),
            LogKind::Status => st.c("34", &format!("[{}]", l.text)),
            LogKind::Battle => format!("⚔ {}", l.text),
            LogKind::Danger => st.c("31", &format!("! {}", l.text)),
            LogKind::Reward => st.c("32", &format!("+ {}", l.text)),
        };
        println!("{}", line);
    }
    let p = &turn.prompt;
    println!();
    if !p.title.is_empty() {
        println!("{}", st.c("1", &p.title));
    }
    for (i, o) in p.options.iter().enumerate() {
        println!("  {}) {}", i + 1, o);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let st = Style { plain: args.iter().any(|a| a == "--plain") };
    let json = args.iter().any(|a| a == "--json");
    let seed = args
        .iter()
        .find_map(|a| a.strip_prefix("--seed=").and_then(|s| s.parse::<u64>().ok()))
        .unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1)
        });

    let save_path = args
        .iter()
        .find_map(|a| a.strip_prefix("--save="))
        .unwrap_or("dongji_save.txt")
        .to_string();

    let mut game = Game::new(seed);
    if let Ok(data) = std::fs::read_to_string(&save_path) {
        if let Err(e) = game.attach_save(&data) {
            eprintln!("세이브 파일을 읽지 못했습니다: {}", e);
        }
    }
    let mut turn = game.start();
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        if let Some(cp) = &turn.checkpoint {
            match std::fs::write(&save_path, cp) {
                Ok(_) => {
                    if !json {
                        println!("{}", st.c("90", "· (자동 저장됨)"));
                    }
                }
                Err(e) => eprintln!("저장 실패: {}", e),
            }
        }
        if json {
            println!("{}", turn.to_json());
        } else {
            render(&turn, &st);
        }
        let input = loop {
            print!("> ");
            let _ = io::stdout().flush();
            let line = match lines.next() {
                Some(Ok(l)) => l.trim().to_string(),
                _ => return,
            };
            if line == "q" || line == "quit" {
                return;
            }
            match turn.prompt.kind {
                PromptKind::Text => break Input::Text(line),
                PromptKind::Choice => match line.parse::<usize>() {
                    Ok(n) if n >= 1 && n <= turn.prompt.options.len() => break Input::Choice(n - 1),
                    _ => println!("1~{} 사이의 번호를 입력하세요. (종료: q)", turn.prompt.options.len()),
                },
            }
        };
        turn = game.send(input);
    }
}
