//! 브라우저(WebAssembly)용 내보내기 함수.
//!
//! wasm-bindgen 같은 외부 도구 없이, 정수와 메모리 버퍼만 주고받는다.
//!   출력: dj_start / dj_choose / dj_text 가 Turn JSON(UTF-8)의 길이를 돌려주고,
//!         JS는 dj_out_ptr()부터 그 길이만큼 메모리를 읽는다.
//!   입력: JS가 dj_input_buf(len)이 돌려준 주소에 UTF-8 바이트를 쓰고 나서
//!         dj_text / dj_attach_save 를 호출한다.
//! 브라우저는 한 번에 한 게임만 돌리므로 전역 상태 하나로 충분하다.
//! (네이티브 빌드에도 함께 컴파일되며, tests/wasm_api.rs 에서 같은 함수를 검증한다.)

use crate::game::Game;
use crate::model::Input;
use std::cell::RefCell;

thread_local! {
    static GAME: RefCell<Option<Game>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static INP: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn put_out(s: String) -> u32 {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        *o = s.into_bytes();
        o.len() as u32
    })
}

fn take_input() -> String {
    INP.with(|i| String::from_utf8_lossy(&i.borrow()).into_owned())
}

fn with_game<F: FnOnce(&mut Game) -> String>(f: F) -> u32 {
    let s = GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => f(game),
        None => "{}".to_string(),
    });
    put_out(s)
}

/// 입력 버퍼를 len 바이트로 준비하고 그 주소를 돌려준다.
#[no_mangle]
pub extern "C" fn dj_input_buf(len: u32) -> *mut u8 {
    INP.with(|i| {
        let mut i = i.borrow_mut();
        i.clear();
        i.resize(len as usize, 0);
        i.as_mut_ptr()
    })
}

/// 마지막 출력(JSON)의 시작 주소.
#[no_mangle]
pub extern "C" fn dj_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// 새 게임. seed는 64비트라 32비트 두 개로 나눠 받는다.
#[no_mangle]
pub extern "C" fn dj_new(seed_hi: u32, seed_lo: u32) {
    let seed = ((seed_hi as u64) << 32) | seed_lo as u64;
    GAME.with(|g| *g.borrow_mut() = Some(Game::new(seed)));
}

/// 입력 버퍼에 넣어 둔 세이브 데이터를 붙인다. 성공 1 / 실패 0.
#[no_mangle]
pub extern "C" fn dj_attach_save() -> i32 {
    let data = take_input();
    GAME.with(|g| match g.borrow_mut().as_mut() {
        Some(game) => game.attach_save(&data).is_ok() as i32,
        None => 0,
    })
}

#[no_mangle]
pub extern "C" fn dj_start() -> u32 {
    with_game(|g| g.start().to_json())
}

#[no_mangle]
pub extern "C" fn dj_choose(index: u32) -> u32 {
    with_game(|g| g.send(Input::Choice(index as usize)).to_json())
}

/// 입력 버퍼의 문자열을 텍스트 입력으로 보낸다.
#[no_mangle]
pub extern "C" fn dj_text() -> u32 {
    let text = take_input();
    with_game(|g| g.send(Input::Text(text)).to_json())
}
