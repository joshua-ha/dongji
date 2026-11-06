//! 冬至夜行 (동지야행) — 주술 액션 텍스트 RPG 엔진.
//!
//! 엔진은 UI를 모른다. 입력 하나를 받으면 `Turn { logs, prompt }`를 돌려줄 뿐이다.
//! 로그 자체가 화면이 되도록 설계했다: CLI는 그대로 출력하고, iOS는 JSON으로 받아 그린다.

pub mod ffi;
pub mod game;
pub mod model;
pub mod rng;
pub mod wasm;
mod data;

pub use game::{Class, Ending, Game, Player, Vow};
pub use model::{Input, LogEntry, LogKind, Prompt, PromptKind, Turn};
