//! iOS(Swift)용 C ABI.
//!
//! 모든 함수는 UTF-8 JSON 문자열(`Turn::to_json`)을 돌려준다.
//! 돌려받은 문자열은 반드시 `dongji_string_free`로 해제할 것.
//! 헤더: include/dongji.h

use crate::game::Game;
use crate::model::Input;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

fn to_c(s: String) -> *mut c_char {
    // JSON 안에 NUL이 들어갈 일은 없지만 방어적으로 제거
    CString::new(s.replace('\0', "")).unwrap_or_default().into_raw()
}

/// 새 게임을 만든다. seed가 같으면 같은 전개가 재현된다.
#[no_mangle]
pub extern "C" fn dongji_new(seed: u64) -> *mut Game {
    Box::into_raw(Box::new(Game::new(seed)))
}

/// # Safety
/// `game`은 `dongji_new`가 돌려준 포인터여야 하고, 한 번만 해제해야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_free(game: *mut Game) {
    if !game.is_null() {
        drop(Box::from_raw(game));
    }
}

/// 타이틀 화면 로그 + 첫 프롬프트.
/// # Safety
/// `game`은 유효한 포인터여야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_start(game: *mut Game) -> *mut c_char {
    match game.as_mut() {
        Some(g) => to_c(g.start().to_json()),
        None => std::ptr::null_mut(),
    }
}

/// 선택지 고르기 (0부터 시작).
/// # Safety
/// `game`은 유효한 포인터여야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_choose(game: *mut Game, index: u32) -> *mut c_char {
    match game.as_mut() {
        Some(g) => to_c(g.send(Input::Choice(index as usize)).to_json()),
        None => std::ptr::null_mut(),
    }
}

/// 텍스트 입력 (이름 입력 등).
/// # Safety
/// `game`은 유효한 포인터, `text`는 NUL 종료 UTF-8 문자열이어야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_input_text(game: *mut Game, text: *const c_char) -> *mut c_char {
    let g = match game.as_mut() {
        Some(g) => g,
        None => return std::ptr::null_mut(),
    };
    let s = if text.is_null() {
        String::new()
    } else {
        CStr::from_ptr(text).to_string_lossy().into_owned()
    };
    to_c(g.send(Input::Text(s)).to_json())
}

/// 저장해 둔 세이브 데이터를 넣는다. `dongji_start` 전에 부르면 타이틀에 '이어하기'가 생긴다.
/// 성공 1, 실패(손상/형식 오류) 0.
/// # Safety
/// `game`은 유효한 포인터, `data`는 NUL 종료 UTF-8 문자열이어야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_attach_save(game: *mut Game, data: *const c_char) -> i32 {
    let (Some(g), false) = (game.as_mut(), data.is_null()) else { return 0 };
    let s = CStr::from_ptr(data).to_string_lossy();
    g.attach_save(&s).is_ok() as i32
}

/// 가장 최근 체크포인트. 없으면 NULL. 사용 후 `dongji_string_free`.
/// (각 Turn JSON의 "checkpoint" 필드로도 같은 값이 전달된다.)
/// # Safety
/// `game`은 유효한 포인터여야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_save_data(game: *const Game) -> *mut c_char {
    match game.as_ref().and_then(|g| g.save_data()) {
        Some(s) => to_c(s),
        None => std::ptr::null_mut(),
    }
}

/// 엔진이 돌려준 문자열 해제.
/// # Safety
/// `s`는 이 라이브러리가 돌려준 포인터여야 한다.
#[no_mangle]
pub unsafe extern "C" fn dongji_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}
