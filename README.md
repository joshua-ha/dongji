# 冬至夜行 · 동지야행

주술 액션 텍스트 RPG. Rust로 만든 **UI 없는 게임 엔진**이며, 엔진이 내보내는 **로그가 곧 화면**이다.
터미널에서 바로 플레이할 수 있고, iOS에서는 같은 엔진을 정적 라이브러리(.a)로 링크해 SwiftUI로 그리면 된다.

> 동지(1년 중 가장 긴 밤)까지 남은 엿새. 서울에서 사람들의 '이름'이 지워지고 있다.
> 이름을 먹고 자라는 존재 **무명(無名)** 이 광화문 아래의 문을 열려 한다.

## 바로 해보기

```bash
cargo run --release              # 색상 출력
cargo run --release -- --plain   # 색상 없이
cargo run --release -- --json    # iOS가 받을 JSON 그대로 보기
cargo run --release -- --seed=42 # 같은 전개 재현
cargo run --release -- --save=내기록.txt  # 세이브 파일 위치 (기본 dongji_save.txt)
cargo test --release -- --nocapture   # 봇 플레이 테스트 + 난이도 통계
```

## 게임 구성

**직업 선택**
| | 주술사 | 주저사 |
|---|---|---|
| 동기 | 10년 전 사라진 스승 한서림을 찾는다 | 빼앗긴 자기 이름을 되찾는다 |
| 패시브 | 매 턴 주력 +10 | 원한: 받은 피해의 절반을 주력으로 |
| 술식 | 「청뢰」 강한 단일 피해 | 「흑사」 피해 + 35% 확률로 적 행동 봉쇄 |
| 특수 | 역전 치유 (HP 35% 회복) | 저주 흡혈 (피해 + 흡혈, 오염도 +2) |
| 영역 | 「백야정원」 | 「무간시장」 |

**장 구성 (6장)**: 폐교 → 지하 폐역 → 재개발 아파트(빌런 고하람) → 남산 송신탑 → 한강 → 광화문 지하(최종 보스 무명).
도입부 대사는 직업에 따라 달라지고, 최종장에서 두 이야기가 하나로 합쳐진다.

**한 장의 흐름**
1. **현장 조사**: 세 곳 중 두 곳을 고른다. 약점 파악 / 부적 / 기습(피해 대신 주력 보너스) / 단서(주력 보너스)
2. **서약**: 전투 전에 스스로 제약을 건다 — 단기결전 / 무방비 / 절제 / 안 함. 끝까지 지키면 최대 주력이 오르고, 어기면 대가를 치른다.
3. **전투**: 적의 다음 행동이 **미리 공개**된다(강공격 예고 → 방어, 몸 굳히기 → 그 틈에 주력 모으기). 주력이 100 이상이면 영역 전개(3턴 동안 필중·강화). 낮은 확률로 공명 타격(치명타)이 터진다.
4. **잔해 처리**: 정화(오염도 -5) 또는 흡수(공격력·주력이 오르지만 오염도 상승)
5. **성장**: 레벨 업 후 능력치 하나를 고른다.

**이 게임만의 시스템 — 오염도**
흡수할수록 강해지지만, 오염도가 60%를 넘으면 엔딩이 바뀐다. 최종전 중반에 무명이 손을 내미는데, 이 선택도 결말을 가른다.

**엔딩 4종**: 백야의 새벽(주술사) · 이름을 부르는 사람(주저사) · 잔예의 왕관(오염도 60%+) · 이름 없는 왕(무명의 손을 잡음)

## 세이브 / 로드

- **자동 저장**: 각 장이 시작되는 순간이 체크포인트다. 그때 `Turn.checkpoint`(JSON의 `"checkpoint"`)에 세이브 문자열이 실려 나온다. 호스트는 이 문자열을 그대로 저장해 두면 된다. 터미널판은 파일에 쓰고, iOS는 UserDefaults나 파일에 넣으면 된다.
- **불러오기**: 다음 실행 때 `start()`보다 먼저 `attach_save(data)`(FFI: `dongji_attach_save`)를 부르면 타이틀에 **"이어하기 — 제N장 · 이름 Lv"** 선택지가 생긴다. 이어하기도 로그와 선택지로만 처리되므로 UI 쪽에 따로 만들 화면이 없다.
- 세이브는 사람이 읽을 수 있는 텍스트이고, 체크섬이 있어서 손상되거나 고쳐진 파일은 거부한다.

```
dongji-save v1
stage=2
name=하늘
class=curse_user
level=3
...
check=1234567890
```

## 난이도 (초등학생 기준)

`src/game.rs` 맨 위의 상수만 바꾸면 전체 난이도가 조절된다.

| 상수 | 현재 | 의미 |
|---|---|---|
| `ENEMY_HP_SCALE` | 0.85 | 적 체력 배율 |
| `ENEMY_ATK_SCALE` | 0.85 | 적 공격력 배율 |
| `HEAVY_MULT` | 1.8 | 예고되는 강공격의 배율 |
| `START_CE` | 40 | 전투 시작 주력 |

그 밖에 초등학생을 위한 배려:
- 시작 부적이 한 장씩 늘었다 (주술사 회복 2장, 주저사 회복 1장 + 주력 1장).
- 지면 그 장을 다시 도전할 수 있고, 다시 도전할 때마다 회복 부적을 1장 준다.
- 봇 테스트 결과: 방어나 회복 없이 공격만 해도 평균 재도전 0~0.4회. 적의 예고를 보고 방어하면 거의 지지 않는다.

## 아키텍처

```
src/
  model.rs   UI와의 계약: LogEntry, Prompt, Turn(+JSON 직렬화)
  game.rs    상태 머신 / 전투 / 성장 / 엔딩
  data.rs    스토리·스테이지·적 데이터 (텍스트만 고쳐도 시나리오 수정 가능)
  rng.rs     결정적 난수 (seed 재현)
  ffi.rs     iOS용 C ABI
  wasm.rs    웹(WebAssembly)용 내보내기 함수
  main.rs    터미널 플레이어
include/dongji.h
web/                   게임 화면 (index.html, style.css, app.js, assets/)
ios/                   SwiftUI + WKWebView 래퍼 (XcodeGen project.yml)
tools/make_assets.py   에셋 생성 (SVG → 투명 PNG → 트림)
tools/dev_server.py    브라우저 개발 서버
build_ios.sh           iOS 빌드 스크립트
.github/workflows/pages.yml  웹 자동 배포 (WASM 빌드 → GitHub Pages)
tests/playthrough.rs   봇이 끝까지 플레이: 4개 엔딩, 재현성, 세이브/로드, 난이도 통계
```

외부 의존성은 없다(serde도 쓰지 않음). 덕분에 iOS 빌드가 단순하고 바이너리가 작다.

### 로그 포맷 (UI가 그릴 단위)

```json
{
  "logs": [
    {"kind":"title","speaker":null,"text":"제1장 「빈자리」"},
    {"kind":"dialogue","speaker":"도윤재","text":"장막 내릴게."},
    {"kind":"status","speaker":null,"text":"서윤 Lv.1 · HP 124/140 · 주력 60/120 · 오염도 0%"},
    {"kind":"danger","speaker":null,"text":"▶ 빈자리: 강력한 일격을 준비하고 있다! (방어 권장)"}
  ],
  "prompt": {"type":"choice","title":"3턴 — 행동을 선택하세요","options":["기본 공격 (주력 +12)", "..."]},
  "checkpoint": null
}
```

`kind`: `title` `narration` `dialogue` `system` `status` `battle` `danger` `reward`
`prompt.type`이 `text`이면 텍스트 입력창을, `choice`이면 선택지 버튼을 보여주면 된다.

## 화면 (UI)

쿠키런 / 세븐나이츠 키우기 구조를 참고한 모바일 게임 화면이다. `web/` 폴더의 HTML/CSS/JS로 만들었고, iOS 앱 안의 WKWebView에서 돈다.

| 화면 | 구성 |
|---|---|
| 타이틀 | 로고, 두 주인공, 이어하기 / 새로 시작 |
| 직업 선택 | 캐릭터 카드 2장 (능력치 바, 스킬 요약) → 이름 입력 |
| 스토리 | 대화창: 화자 전신 일러스트 + 이름표 + 타이핑 효과, 건너뛰기 |
| 장 시작 | 장 제목 배너 연출 |
| 현장 조사 | 뒤집는 단서 카드 3장 (결과 아이콘 표시) |
| 서약 | 서약 카드 4장 (보상 ▲ / 벌칙 ▼) |
| 전투 | 적 일러스트 + HP바 + **행동 예고 말풍선**, 주인공, 전투 로그 피드, HUD(HP/주력, 영역 기준선), 스킬 버튼 4개 + 영역 전개 + 부적 |
| 연출 | 피격 흔들림, 데미지 숫자, 공명 타격, 영역 전개 컷인, VS / 승리 / 서약 파기 배너 |
| 승리 후 | 정화 / 흡수 선택 카드 → LEVEL UP 카드 4장 |
| 로비 (장 사이) | 프로필, 부적·오염도 자원, 주인공 + 힌트 말풍선, 모험 떠나기 버튼, 하단 탭(홈 / 모험 지도 / 캐릭터 / 기록) |
| 기타 | 보스의 제안 팝업, 게임 오버(다시 도전), 엔딩 카드, 전체 기록(로그) 보기 |

iPhone 세로, iPad 세로·가로를 지원한다. iPad 가로 화면에서는 전투 화면이 좌우로 나뉜다.

**아이들을 위한 배려**: 강공격이 예고되면 방어 버튼이 반짝이고, 체력이 낮으면 치유 버튼이 반짝인다. 로비 말풍선이 공략 힌트를 준다.

### 에셋

`tools/make_assets.py`가 모든 그림을 벡터로 그린 뒤, 투명 PNG로 렌더링하고 여백을 잘라낸다(trim). 대화용 초상화(`pt_*`)는 전신 그림에서 머리 부분만 다시 잘라낸 것이다.

- 캐릭터 11종 (`hero_*`, `npc_*`, `enemy_*`) + 초상화 11종 (`pt_*`)
- 배경 8종 (`bg_title`, `bg_lobby`, `bg_stage0~5`)
- 아이콘 25종 (`ic_*`: 스킬/자원/탭, `it_*`: 적 행동 예고)

생성형 이미지로 바꿀 때는 `web/assets/`에 **같은 파일 이름**으로 덮어쓰면 된다. 캐릭터는 배경이 투명한 PNG, 배경은 세로형(약 3:4) JPG를 권장한다.

### 구조

```
[ web/ (HTML·CSS·JS) ]  ←JSON→  [ Swift: EngineBridge ]  ←C FFI→  [ Rust 엔진 (libdongji.a) ]
```

- JS → Swift: `webkit.messageHandlers.dongji.postMessage({cmd, id, ...})` (`boot` / `choose` / `text` / `save`)
- Swift → JS: `Bridge._reply(id, <Turn JSON>)`
- Turn JSON의 `state`(장면, HP, 적의 의도 등)와 `prompt.ids`(버튼 종류)로 화면을 그린다. 로그는 순서대로 연출로 재생된다.
- 자동 저장은 Swift가 `UserDefaults`에 한다.

## 실행 방법

**브라우저에서 바로 (개발용)**: iOS 앱과 똑같은 C 함수를 Python ctypes로 불러서 돌린다.
```bash
cargo build --release
python3 tools/dev_server.py --port 8000      # http://localhost:8000
python3 tools/dev_server.py --fresh          # 세이브 지우고 시작
```
같은 Wi-Fi에 있는 아이폰에서 `http://<Mac의 IP>:8000`으로 접속해도 플레이할 수 있다.

**웹으로 공개하기 (GitHub Pages)**: Rust 엔진을 WebAssembly로 빌드해 브라우저에서 바로 돈다. 빌드와 배포는 GitHub Actions(`.github/workflows/pages.yml`)가 자동으로 한다.
```bash
git init && git add . && git commit -m "동지야행"
git branch -M main
git remote add origin https://github.com/<아이디>/dongji.git
git push -u origin main
```
1. GitHub 저장소 → **Settings → Pages → Build and deployment → Source**를 **GitHub Actions**로 바꾼다.
2. **Actions** 탭에서 "Deploy to GitHub Pages"가 끝나면(1~3분) `https://<아이디>.github.io/dongji/`에서 플레이할 수 있다.
3. 아이폰/아이패드 사파리에서 열고 공유 버튼 → **홈 화면에 추가**를 누르면 아이콘이 생기고 전체 화면으로 실행된다.
4. 이후 `git push`할 때마다 자동으로 다시 배포된다. 세이브는 기기 브라우저의 `localStorage`에 저장된다.

화면 코드는 실행 환경을 스스로 고른다: iOS 앱이면 Swift 브리지, `engine/dongji.wasm`이 있으면 WebAssembly, 둘 다 없으면 개발 서버(`/api`).

**iPhone / iPad 앱 (Mac + Xcode 필요)**:
```bash
brew install xcodegen      # 처음 한 번
./build_ios.sh             # Rust → XCFramework → ios/Dongji.xcodeproj 생성
open ios/Dongji.xcodeproj  # Signing & Capabilities에서 팀 선택 → 기기 선택 후 실행
```

## 다음에 해볼 만한 것
- 세이브 슬롯 여러 개
- 장마다 직업 전용 분기 선택지 추가
- 효과음 / 배경음, 햅틱 진동
- 생성형 일러스트로 에셋 교체
