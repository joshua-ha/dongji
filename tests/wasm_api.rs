//! 브라우저용 dj_* 함수를 네이티브에서 그대로 호출해, JS가 하는 일(메모리 읽기/쓰기)을 흉내 낸다.

use dongji::wasm::*;

fn read(len: u32) -> String {
    let p = dj_out_ptr();
    let bytes = unsafe { std::slice::from_raw_parts(p, len as usize) };
    String::from_utf8(bytes.to_vec()).unwrap()
}

fn write(s: &str) {
    let b = s.as_bytes();
    let p = dj_input_buf(b.len() as u32);
    unsafe { std::ptr::copy_nonoverlapping(b.as_ptr(), p, b.len()) };
}

#[test]
fn wasm_api_flow_and_save() {
    dj_new(0x1234, 0x5678);
    let t = read(dj_start());
    assert!(t.starts_with("{\"logs\":[") && t.contains("\"scene\":\"title\""));
    read(dj_choose(0)); // 새로운 기록
    read(dj_choose(0)); // 주술사
    write("하늘");
    let t = read(dj_text());
    assert!(t.contains("하늘"), "이름 입력이 전달돼야 함");
    assert!(t.contains("\"checkpoint\":\"dongji-save v1"));
    let start = t.find("\"checkpoint\":\"").unwrap() + 14;
    let end = start + t[start..].find("\",\"state\"").unwrap();
    let save = t[start..end].replace("\\n", "\n");

    // 새 게임에 세이브 붙이기 → 타이틀에 이어하기
    dj_new(0, 99);
    write(&save);
    assert_eq!(dj_attach_save(), 1);
    let t = read(dj_start());
    assert!(t.contains("\"continue\""), "{}", t);
    write("깨진 데이터");
    assert_eq!(dj_attach_save(), 0);
}
