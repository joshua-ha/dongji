'use strict';
/* 동지야행 UI.
 * 엔진(Rust)이 주는 Turn = { logs, prompt{options, ids, enabled}, state, checkpoint }만 보고 화면을 그린다.
 * 로그는 순서대로 "연출"로 재생된다: 지문/대사 → 대화창, 전투 로그 → 피격/데미지 연출, 보상 → 토스트.
 */

const A = (n) => `assets/${n}`;
const $ = (s, el = document) => el.querySelector(s);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const esc = (s) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' }[c]));

// ───────────────────────── 엔진 브리지 ─────────────────────────
// 세 가지 실행 환경을 자동으로 고른다.
//  1) iOS 앱: WKWebView의 messageHandlers.dongji → Swift가 Rust FFI 호출 → Bridge._reply(id, turn)
//  2) 웹(GitHub Pages): engine/dongji.wasm 을 불러와 브라우저 안에서 Rust 엔진 실행, 세이브는 localStorage
//  3) 개발: tools/dev_server.py 의 /api (ctypes로 같은 Rust 라이브러리 호출)
const Bridge = (() => {
  const ios = window.webkit && window.webkit.messageHandlers && window.webkit.messageHandlers.dongji;
  const SAVE_KEY = 'dongji.save.v1';
  const enc = new TextEncoder();
  const dec = new TextDecoder();
  let mode = null, wasm = null, seq = 0;
  const waiters = {};

  async function init() {
    if (mode) return;
    if (ios) { mode = 'ios'; return; }
    try {
      const r = await fetch('engine/dongji.wasm');
      if (r.ok) {
        const { instance } = await WebAssembly.instantiate(await r.arrayBuffer(), {});
        wasm = instance.exports;
        mode = 'wasm';
        return;
      }
    } catch (e) { console.warn('wasm 엔진을 불러오지 못함, 개발 서버로 연결', e); }
    mode = 'server';
  }

  // ── WASM: 메모리 버퍼로 문자열 주고받기 ──
  function wasmIn(str) {
    const bytes = enc.encode(str);
    const ptr = wasm.dj_input_buf(bytes.length);
    new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes); // 호출 뒤에 buffer를 다시 얻어야 메모리 증가에 안전
  }
  function wasmOut(len) {
    const ptr = wasm.dj_out_ptr();
    return JSON.parse(dec.decode(new Uint8Array(wasm.memory.buffer, ptr, len)));
  }
  function loadSave() { try { return localStorage.getItem(SAVE_KEY); } catch (e) { return null; } }
  function callWasm(msg) {
    switch (msg.cmd) {
      case 'boot': {
        const r = crypto.getRandomValues(new Uint32Array(2));
        wasm.dj_new(r[0], r[1]);
        const saved = loadSave();
        if (saved) { wasmIn(saved); wasm.dj_attach_save(); }
        return wasmOut(wasm.dj_start());
      }
      case 'choose': return wasmOut(wasm.dj_choose(msg.index));
      case 'text': wasmIn(msg.text || ''); return wasmOut(wasm.dj_text());
      case 'save': try { localStorage.setItem(SAVE_KEY, msg.data); } catch (e) { /* 사파리 사생활 보호 모드 등 */ } return null;
    }
    return null;
  }

  async function call(msg) {
    await init();
    if (mode === 'ios') {
      if (msg.cmd === 'save') { ios.postMessage({ ...msg, id: 0 }); return null; }
      return new Promise((res) => { const id = ++seq; waiters[id] = res; ios.postMessage({ ...msg, id }); });
    }
    if (mode === 'wasm') return callWasm(msg);
    const r = await fetch('api', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(msg) });
    return msg.cmd === 'save' ? null : r.json();
  }

  return {
    _reply(id, turn) { const w = waiters[id]; delete waiters[id]; if (w) w(turn); },
    mode: () => mode,
    boot: () => call({ cmd: 'boot' }),
    choose: (i) => call({ cmd: 'choose', index: i }),
    text: (t) => call({ cmd: 'text', text: t }),
    save: (data) => { call({ cmd: 'save', data }); },
  };
})();
window.Bridge = Bridge;

// ───────────────────────── 데이터 ─────────────────────────
const ART = {
  '도윤재': 'npc_doyunjae', '매화 할멈': 'npc_maehwa', '한서림': 'npc_hanseorim', '고하람': 'enemy_goharam',
  '무명': 'enemy_mumyeong', '빈자리': 'enemy_binjari', '막차 차장': 'enemy_conductor', '잡음의 성녀': 'enemy_saint', '망향': 'enemy_manghyang',
};
const CHAPTERS = [
  ['제1장', '빈자리', '은방울 중학교'], ['제2장', '막차', '신당역 지하 4층'], ['제3장', '연(鳶)', '재개발 아파트 옥상'],
  ['제4장', '잡음', '남산 송신탑'], ['제5장', '망향(望鄕)', '한강 잠수교 아래'], ['최종장', '무명(無名)', '광화문 지하'],
];
const STAGE_ENEMY = ['enemy_binjari', 'enemy_conductor', 'enemy_goharam', 'enemy_saint', 'enemy_manghyang', 'enemy_mumyeong'];
const INTENT = {
  strike: ['it_strike', '공격하려 해요'],
  heavy: ['it_heavy', '강공격 준비! 방어하세요!'],
  drain: ['it_drain', '주력을 빼앗으려 해요'],
  barrier: ['it_barrier', '몸을 굳혀요 · 주력 모을 기회!'],
  hex: ['it_hex', '저주를 걸려 해요'],
  bound: ['it_bound', '실에 묶여 못 움직여요'],
};
const HINTS = [
  '적 머리 위 말풍선을 봐! 강공격이 오면 방어!',
  '주력이 100이 되면 영역 전개를 쓸 수 있어!',
  '현장 조사에서 약점을 찾으면 훨씬 쉬워져.',
  '흡수하면 강해지지만… 오염도가 오르면 결말이 바뀌어.',
  '서약을 끝까지 지키면 최대 주력이 늘어나!',
  '지면 회복 부적을 한 장 더 받고 다시 도전할 수 있어.',
];

// 화면 자체가 이미 보여주는 안내는 토스트로 띄우지 않는다
const QUIET_SYSTEM = ['저주를 다루는 자는', '【', '술식 「', '주저사는 흡수', '주령이 모습을', '서약 —', '쓰러진', '3턴 동안', '이미 조사한'];

// ───────────────────────── 상태 ─────────────────────────
const G = {
  state: null, prompt: null, history: [], busy: false,
  mounted: null, tab: 'home', spotResults: {}, lastSpot: null, endingTitle: '', endingSummary: '',
  skipVN: false, vnArt: null, pendingPlace: null, selClass: null,
};

function artFor(name) {
  const p = G.state && G.state.player;
  if (p && name === p.name) return 'hero_' + p.class;
  return ART[name];
}
function heroArt() { const p = G.state.player; return p ? 'hero_' + p.class : 'hero_sorcerer'; }

// ───────────────────────── 배경 ─────────────────────────
let bgFlip = 0, bgCur = '';
function setBg(name) {
  if (bgCur === name) return;
  bgCur = name;
  const layers = document.querySelectorAll('#bg .bg-layer');
  bgFlip ^= 1;
  layers[bgFlip].style.backgroundImage = `url(${A(name)})`;
  layers[bgFlip].classList.add('on');
  layers[bgFlip ^ 1].classList.remove('on');
}
function stageBg() { return `bg_stage${G.state.stage}.jpg`; }

// ───────────────────────── 토스트 / 이펙트 ─────────────────────────
function toast(text, kind = '') {
  const box = $('#toasts');
  const t = document.createElement('div');
  t.className = 'toast ' + kind;
  t.textContent = text;
  box.appendChild(t);
  while (box.children.length > 3) box.firstChild.remove();
  setTimeout(() => { t.classList.add('out'); setTimeout(() => t.remove(), 400); }, 2000);
}
function fx(html, ms) {
  const el = document.createElement('div');
  el.innerHTML = html;
  const node = el.firstElementChild;
  $('#fx').appendChild(node);
  if (ms) setTimeout(() => node.remove(), ms);
  return node;
}
function floatText(target, text, cls) {
  if (!target) return;
  const r = target.getBoundingClientRect();
  const x = r.left + r.width / 2 + (Math.random() * 40 - 20) - 30;
  const y = r.top + r.height * 0.35;
  const n = fx(`<div class="float ${cls}" style="left:${x}px;top:${y}px">${esc(text)}</div>`, 950);
  return n;
}
function anim(el, cls) {
  if (!el) return;
  el.classList.remove(cls);
  void el.offsetWidth;
  el.classList.add(cls);
  setTimeout(() => el.classList.remove(cls), 600);
}
function flash(color) { fx(`<div class="flash" style="background:${color}"></div>`, 400); }
function waitTapOr(ms, el) {
  return new Promise((res) => {
    let done = false;
    const fin = () => { if (!done) { done = true; res(); } };
    setTimeout(fin, ms);
    (el || document).addEventListener('pointerdown', fin, { once: true });
  });
}

// ───────────────────────── 대화창 (비주얼 노벨) ─────────────────────────
function vnLine(log) {
  if (G.skipVN) return Promise.resolve();
  return new Promise((resolve) => {
    const vn = $('#vn');
    vn.hidden = false;
    const art = log.kind === 'dialogue' ? artFor(log.speaker) : null;
    const side = art && art.startsWith('hero_') ? 'left' : 'right';
    const enter = art !== G.vnArt ? ' enter' : '';
    G.vnArt = art;
    vn.innerHTML = `<div class="vn-dim"></div>
      ${art ? `<img class="vn-art ${side}${enter}" src="${A(art + '.png')}">` : ''}
      <div class="vn-box ${log.kind}">${log.kind === 'dialogue' ? `<div class="vn-name">${esc(log.speaker)}</div>` : ''}<div class="vn-text"></div><div class="vn-next">▼</div></div>
      <button class="vn-skip">건너뛰기 ▶▶</button>`;
    const textEl = $('.vn-text', vn);
    const full = log.text;
    let i = 0, typing = true;
    const timer = setInterval(() => {
      i += 1;
      textEl.textContent = full.slice(0, i);
      if (i >= full.length) { clearInterval(timer); typing = false; }
    }, 26);
    const onTap = (e) => {
      if (e.target.closest('.vn-skip')) {
        clearInterval(timer); G.skipVN = true; cleanup(); resolve(); return;
      }
      if (typing) { clearInterval(timer); textEl.textContent = full; typing = false; return; }
      cleanup(); resolve();
    };
    const cleanup = () => vn.removeEventListener('pointerup', onTap);
    vn.addEventListener('pointerup', onTap);
  });
}
function vnHide() { const vn = $('#vn'); vn.hidden = true; vn.innerHTML = ''; G.vnArt = null; }

// ───────────────────────── 로그 재생 ─────────────────────────
const RE_EHIT = /→ .+?에게 (\d+) 피해\. \(적 HP (\d+)\/(\d+)\)/;
const RE_DOMAIN_HIT = /필중 효과 — .+?에게 (\d+) 피해\. \(적 HP (\d+)\/(\d+)\)/;
const RE_PHIT = /← .+? (\d+) 피해를 입었다\. \(HP (\d+)\/(\d+)\)/;
const RE_HEAL = /HP \+(\d+)/;

function inBattle() { return G.mounted === 'battle'; }

async function playLogs(logs, newState) {
  for (let k = 0; k < logs.length; k++) {
    const log = logs[k];
    const t = log.text;
    switch (log.kind) {
      case 'status':
        if (newState.scene === 'ending') G.endingSummary = t;
        break;
      case 'title':
        await playTitle(t, logs[k + 1], newState);
        break;
      case 'narration':
      case 'dialogue':
        if (newState.scene === 'title') break; // 타이틀 문구는 로고 화면에 이미 있다
        await vnLine(log);
        break;
      case 'system':
        if (t.startsWith('▶')) break; // 행동 예고는 말풍선으로 표시
        if (G.pendingPlace === true) { G.pendingPlace = t; break; }
        if (!$('#vn').hidden && !G.skipVN) { await vnLine({ kind: 'narration', text: t }); break; }
        if (!QUIET_SYSTEM.some((q) => t.includes(q))) toast(t);
        break;
      default: // battle / danger / reward
        if (G.lastSpot !== null && (log.kind === 'reward' || log.kind === 'danger')) recordSpot(t);
        if (inBattle()) { vnHide(); await battleLog(log); } else { toast(t, log.kind === 'battle' ? '' : log.kind); await sleep(120); }
    }
  }
  G.skipVN = false;
  vnHide();
}

async function playTitle(t, next, st) {
  if (/^(제\d장|최종장|서장)/.test(t)) {
    vnHide();
    const m = t.match(/^(\S+) (.+)$/);
    const isStage = !t.startsWith('서장');
    if (isStage) { G.spotResults = {}; G.mounted = null; G.tab = 'home'; $('#scene').innerHTML = ''; setBg(stageBg()); }
    let place = '';
    if (isStage && next && next.kind === 'system') { place = next.text; G.pendingPlace = true; }
    const b = fx(`<div class="banner"><div class="ribbon"><div class="ch">${esc(m ? m[1] : '')}</div><div class="tt">${esc(m ? m[2].replace(/^— /, '') : t)}</div><div class="pl">${esc(place)}</div></div></div>`);
    if (!isStage) setBg('bg_lobby.jpg');
    await waitTapOr(2200, b);
    b.remove();
    if (G.pendingPlace) G.pendingPlace = null;
    return;
  }
  if (t.startsWith('⚔')) {
    vnHide();
    mountBattle(st);
    const n = fx(`<div class="big-word">VS<small>${esc(t.replace('⚔ ', ''))}</small></div>`, 1300);
    await sleep(1300);
    return;
  }
  if (t.startsWith('영역 전개')) {
    const name = t.replace(/^영역 전개 — /, '');
    const cls = (G.state.player && G.state.player.class) || 'sorcerer';
    fx(`<div class="domain-fx ${cls}"><div class="d1">영 역 전 개</div><div class="d2">${esc(name)}</div></div>`, 1800);
    await sleep(1700);
    return;
  }
  if (t.includes('쓰러뜨렸다')) {
    const e = $('.enemy-art');
    if (e) e.classList.add('dead');
    fx(`<div class="big-word">승리!</div>`, 1500);
    await sleep(1500);
    G.mounted = null;
    $('#scene').innerHTML = '';
    return;
  }
  if (t.startsWith('엔딩')) { G.endingTitle = t.replace(/^엔딩 — /, ''); G.mounted = null; $('#scene').innerHTML = ''; setBg(endingBg()); return; }
  if (t.startsWith('— 기록은 여기서')) {
    G.mounted = null;
    fx(`<div class="big-word red">쓰러졌다…</div>`, 1500);
    await sleep(1400);
    return;
  }
  if (t.startsWith('—')) { toast(t.replace(/—/g, '').trim() + ' 클리어!', 'reward'); return; }
  // 그 외(타이틀 로고 등)는 화면에서 처리
}

function recordSpot(t) {
  const i = G.lastSpot;
  let r = null;
  if (t.includes('약점')) r = ['ic_star', '약점 발견!'];
  else if (t.includes('회복 부적')) r = ['ic_heal_charm', '회복 부적 +1'];
  else if (t.includes('주력 부적')) r = ['ic_ce_charm', '주력 부적 +1'];
  else if (t.includes('기습')) r = ['it_heavy', '기습! (주력 +30)'];
  else if (t.includes('결의')) r = ['ic_scroll', '단서 (주력 +20)'];
  if (r && !G.spotResults[i]) G.spotResults[i] = r;
  if (r) G.lastSpot = null;
}

async function battleLog(log) {
  const t = log.text;
  addFeed(log);
  let m;
  if ((m = t.match(RE_EHIT)) || (m = t.match(RE_DOMAIN_HIT))) {
    const crit = G._crit; G._crit = false;
    setEnemyHp(+m[2], +m[3]);
    anim($('.enemy-art'), 'hit');
    floatText($('.enemy-art'), m[1], crit ? 'crit' : 'dmg-e');
    await sleep(430);
  } else if ((m = t.match(RE_PHIT))) {
    setPlayerHp(+m[2], +m[3]);
    anim($('.player-art'), 'hit');
    anim($('.battle'), 'shake');
    flash('rgba(255,60,80,.35)');
    floatText($('.player-art'), m[1], 'dmg-p');
    await sleep(430);
  } else if (t.includes('공명 타격')) {
    G._crit = true;
    flash('rgba(255,240,120,.6)');
    fx(`<div class="big-word" style="top:20%;font-size:46px">공명 타격!</div>`, 700);
    await sleep(500);
  } else if (/의 공격!|강력한 일격|의 저주|빨아들인다/.test(t)) {
    anim($('.enemy-art'), 'lunge');
    await sleep(320);
  } else if (log.kind === 'reward' && (m = t.match(RE_HEAL))) {
    floatText($('.player-art'), '+' + m[1], 'heal');
    await sleep(380);
  } else if (t.includes('서약 파기')) {
    fx(`<div class="big-word red" style="font-size:42px">서약 파기!</div>`, 1000);
    await sleep(900);
  } else {
    await sleep(260);
  }
}

function addFeed(log) {
  const f = $('.feed');
  if (!f) return;
  const d = document.createElement('div');
  d.className = log.kind;
  d.textContent = log.text;
  f.appendChild(d);
  while (f.children.length > 5) f.firstChild.remove();
}

// ───────────────────────── 턴 처리 ─────────────────────────
async function handle(turn) {
  if (turn.checkpoint) Bridge.save(turn.checkpoint);
  G.history.push(...turn.logs);
  const prevScene = G.state && G.state.scene;
  if (prevScene && prevScene !== turn.state.scene && prevScene !== 'battle' && prevScene !== 'boss_offer') $('#scene').innerHTML = '';
  G.state = turn.state;
  G.prompt = turn.prompt;
  await playLogs(turn.logs, turn.state);
  render();
  G.busy = false;
}

async function act(id) {
  if (G.busy) return;
  const i = G.prompt.ids.indexOf(id);
  if (i < 0) return;
  if (!G.prompt.enabled[i]) { toast(id === 'domain' ? '주력이 100 필요해요!' : '주력이 부족해요!', 'danger'); return; }
  G.busy = true;
  $('#modal').hidden = true; // 팝업에서 고른 경우, 이어지는 연출이 가려지지 않도록
  if (id.startsWith('spot_')) G.lastSpot = +id.slice(5);
  try { await handle(await Bridge.choose(i)); } catch (e) { console.error(e); G.busy = false; }
}
async function sendText(t) {
  if (G.busy) return;
  G.busy = true;
  await handle(await Bridge.text(t));
}
function opt(id) { const i = G.prompt.ids.indexOf(id); return i < 0 ? null : { text: G.prompt.options[i], enabled: G.prompt.enabled[i] }; }

// ───────────────────────── 렌더 ─────────────────────────
function render() {
  const s = G.state.scene;
  const scene = $('#scene');
  $('#modal').hidden = true;
  if (s !== 'battle' && s !== 'boss_offer') { G.mounted = null; }
  switch (s) {
    case 'title': return renderTitle(scene);
    case 'class_select': return renderClass(scene);
    case 'name_input': return renderName(scene);
    case 'investigate': return renderInvestigate(scene);
    case 'vow': return renderVow(scene);
    case 'battle': return renderBattle();
    case 'boss_offer': return renderOffer();
    case 'disposal': return renderDisposal(scene);
    case 'level_up': return renderLevelUp(scene);
    case 'interlude': return renderLobby(scene);
    case 'game_over': return renderGameOver(scene);
    case 'ending': return renderEnding(scene);
  }
}

function bindActs(root) {
  root.querySelectorAll('[data-act]').forEach((el) => el.addEventListener('click', () => act(el.dataset.act)));
}

// 타이틀
function renderTitle(scene) {
  setBg('bg_title.jpg');
  const cont = opt('continue');
  scene.innerHTML = `<div class="title-wrap">
    <div class="logo"><div class="hanja">冬至夜行</div><div class="kor">동지야행</div><div class="tag">1년 중 가장 긴 밤, 서울 아래에서 이름이 사라진다.</div></div>
    <div class="title-heroes"><img src="${A('hero_sorcerer.png')}"><img src="${A('hero_curse_user.png')}"></div>
    <div class="title-btns">
      ${cont ? `<button class="btn blue" data-act="continue">이어하기<small>${esc(cont.text.replace(/^이어하기 — /, ''))}</small></button>` : ''}
      <button class="btn" data-act="start_new">새로운 기록 시작</button>
    </div>
    <div class="touch">冬至까지 엿새</div></div>`;
  bindActs(scene);
}

// 직업 선택
function renderClass(scene) {
  setBg('bg_lobby.jpg');
  const card = (id, cls, art, name, tag, st, sk) => `
    <div class="ccard panel ${cls}" data-pick="${id}"><div class="check">✓</div>
      <div class="art"><img src="${A(art)}"></div>
      <h3>${name}</h3><div class="tagline">${tag}</div>
      <div class="stats">
        <span>체력</span><div class="bar hp"><i style="width:${st[0]}%"></i></div>
        <span>공격</span><div class="bar ehp"><i style="width:${st[1]}%"></i></div>
        <span>방어</span><div class="bar ce"><i style="width:${st[2]}%"></i></div>
      </div>
      <div class="skills-list">${sk}</div>
    </div>`;
  scene.innerHTML = `<div class="h-title">당신은 어느 쪽인가?</div><div class="h-sub">저주를 다루는 두 가지 길</div>
    <div class="class-row">
      ${card('class_sorcerer', 'sorc', 'hero_sorcerer.png', '주술사', '사라진 스승을 찾아<br>규율 아래 싸운다', [90, 62, 80],
        '<b>청뢰</b> 푸른 번개 공격<br><b>역전 치유</b> 체력 회복<br><b>패시브</b> 매 턴 주력 회복')}
      ${card('class_curse_user', 'curse', 'hero_curse_user.png', '주저사', '빼앗긴 이름을 찾아<br>규율 밖에서 살아남는다', [78, 76, 50],
        '<b>흑사</b> 검은 실로 묶기<br><b>저주 흡혈</b> 공격+회복<br><b>패시브</b> 맞으면 주력 충전')}
    </div>
    <div style="height:14px"></div>
    <button class="btn wide go" style="max-width:520px;margin:0 auto;display:block;width:100%" disabled>길을 선택하세요</button>`;
  const go = $('.go', scene);
  scene.querySelectorAll('[data-pick]').forEach((c) => c.addEventListener('click', () => {
    scene.querySelectorAll('.ccard').forEach((x) => x.classList.remove('sel'));
    c.classList.add('sel');
    G.selClass = c.dataset.pick;
    go.disabled = false;
    go.textContent = (G.selClass === 'class_sorcerer' ? '주술사' : '주저사') + '로 시작!';
  }));
  go.addEventListener('click', () => G.selClass && act(G.selClass));
}

// 이름
function renderName(scene) {
  const isS = G.selClass !== 'class_curse_user';
  const def = (G.prompt.title.match(/'(.+)'/) || [])[1] || '';
  scene.innerHTML = `<div class="center-col"><div class="name-box panel">
    <img src="${A(isS ? 'hero_sorcerer.png' : 'hero_curse_user.png')}">
    <h3>이름을 알려줘!</h3>
    <input id="nm" maxlength="12" placeholder="${esc(def)}" autocomplete="off">
    <button class="btn wide" id="nmgo">기록 시작</button></div></div>`;
  const inp = $('#nm');
  const go = () => sendText(inp.value);
  $('#nmgo').addEventListener('click', go);
  inp.addEventListener('keydown', (e) => { if (e.key === 'Enter') go(); });
}

function chapterPill() {
  const s = G.state;
  return `<span class="pill">${esc(s.chapter)} 「${esc(s.stage_title)}」</span>`;
}
function topbar(extra = '') {
  return `<div class="topbar">${chapterPill()}<div class="grow"></div>${extra}<button class="icon-btn" data-ui="log"><img src="${A('ic_scroll.png')}"></button></div>`;
}
function bindUi(root) {
  root.querySelectorAll('[data-ui="log"]').forEach((b) => b.addEventListener('click', showLog));
}

// 현장 조사
function renderInvestigate(scene) {
  setBg(stageBg());
  const s = G.state;
  const used = 2 - s.picks_left;
  let cards = '';
  for (let i = 0; i < 3; i++) {
    const o = opt('spot_' + i);
    const label = o.text.replace(' (조사 완료)', '');
    const r = G.spotResults[i];
    cards += o.enabled
      ? `<div class="spot" data-act="spot_${i}"><div class="q">?</div><div class="lb">${esc(label)}</div></div>`
      : `<div class="spot done">${r ? `<img src="${A(r[0] + '.png')}">` : ''}<div class="lb">${esc(label)}</div><div class="res">${esc(r ? r[1] : '조사 완료')}</div></div>`;
  }
  scene.innerHTML = `${topbar()}
    <div class="h-title">현장 조사</div>
    <div class="picks">${'<i></i>'.repeat(s.picks_left)}${'<i class="used"></i>'.repeat(used)}</div>
    <div class="hint">두 곳을 골라 단서를 찾아요. 약점을 찾으면 전투가 쉬워져요!</div>
    <div class="spots">${cards}</div>
    <button class="btn blue wide" style="max-width:520px;margin:12px auto 0" data-act="end_investigate">조사 마치고 기척을 쫓는다 ▶</button>`;
  bindActs(scene); bindUi(scene);
}

// 서약
const VOW_ICON = { vow_blitz: 'ic_technique_sorcerer', vow_reckless: 'ic_attack', vow_restraint: 'ic_heal_charm', vow_none: 'ic_scroll' };
function renderVow(scene) {
  setBg(stageBg());
  let cards = '';
  G.prompt.ids.forEach((id, i) => {
    const t = G.prompt.options[i];
    const m = t.match(/^(.+?) — (.+?) \((.+) \/ (.+)\)$/);
    if (m) {
      cards += `<div class="vow panel" data-act="${id}"><img src="${A(VOW_ICON[id] + '.png')}"><h4>${esc(m[1])}</h4><div class="rule">${esc(m[2])}</div>
        <div class="good">▲ ${esc(m[3])}</div><div class="bad">▼ ${esc(m[4])}</div></div>`;
    } else {
      cards += `<div class="vow panel none" data-act="${id}"><img src="${A(VOW_ICON[id] + '.png')}"><h4>서약 안 함</h4><div class="rule">평소대로 싸운다</div><div class="good">부담 없이 편하게!</div></div>`;
    }
  });
  scene.innerHTML = `${topbar()}<div class="h-title">서약 카드</div>
    <div class="h-sub">스스로 약속을 걸면 더 강해져요. 지키면 보상, 어기면 벌칙!</div>
    <div class="vows">${cards}</div>`;
  bindActs(scene); bindUi(scene);
}

// 전투
function mountBattle(st) {
  G.state = st;
  setBg(stageBg());
  const s = st;
  const eArt = ART[s.enemy.name] || STAGE_ENEMY[s.stage];
  const scene = $('#scene');
  scene.innerHTML = `<div class="battle">
    <div class="topbar">${chapterPill()}<span class="pill turn"></span><span class="vowchip"></span><div class="grow"></div><button class="icon-btn" data-ui="log"><img src="${A('ic_scroll.png')}"></button></div>
    <div class="arena">
      <div class="e-plate"><div class="e-name"></div><div class="bar ehp"><i></i><span></span></div></div>
      <div class="intent"></div>
      <img class="enemy-art" src="${A(eArt + '.png')}">
      <img class="player-art" src="${A(heroArt() + '.png')}">
      <div class="chips"></div>
    </div>
    <div class="feed"></div>
    <div class="hud panel"><div class="av"><img src="${A('pt_' + heroArt() + '.png')}"></div><div class="info">
      <div class="nm"><b class="pn"></b><span class="tn"></span></div>
      <div class="bar hp"><i></i><span></span></div>
      <div class="bar ce"><i></i><span></span><div class="mark"></div></div></div></div>
    <div class="extra-row"></div>
    <div class="skillbar"></div>
  </div>`;
  G.mounted = 'battle';
  bindUi(scene);
  updateBattle(true);
}

function setEnemyHp(hp, max) {
  const b = $('.e-plate .bar');
  if (!b) return;
  $('i', b).style.width = Math.max(0, (hp / max) * 100) + '%';
  $('span', b).textContent = `${hp} / ${max}`;
}
function setPlayerHp(hp, max) {
  const b = $('.hud .bar.hp');
  if (!b) return;
  $('i', b).style.width = Math.max(0, (hp / max) * 100) + '%';
  $('span', b).textContent = `HP ${hp} / ${max}`;
}

function updateBattle(fromMount) {
  const s = G.state, p = s.player, e = s.enemy, b = s.battle;
  if (!e || !b) return;
  const root = $('.battle');
  root.classList.remove('domain-sorcerer', 'domain-curse_user');
  if (b.domain_turns > 0) root.classList.add('domain-' + p.class);
  $('.turn', root).textContent = `${b.turn}턴`;
  $('.vowchip', root).innerHTML = b.vow !== 'none' ? `<span class="pill" style="${b.vow_broken ? 'opacity:.5;text-decoration:line-through' : 'background:#6c2bd9'}">서약: ${{ blitz: '단기결전', reckless: '무방비', restraint: '절제' }[b.vow]}${b.vow === 'blitz' && !b.vow_broken ? ` (${Math.max(0, 6 - b.turn)}턴)` : ''}</span>` : '';
  $('.e-name', root).innerHTML = `${esc(e.name)}<small>${esc(e.grade)}</small>`;
  setEnemyHp(e.hp, e.max_hp);
  const it = INTENT[e.intent] || INTENT.strike;
  const intent = $('.intent', root);
  intent.className = 'intent' + (e.intent === 'heavy' ? ' heavy' : '');
  intent.innerHTML = `<img src="${A(it[0] + '.png')}"><span>${it[1]}</span>`;
  $('.pn', root).textContent = `${p.name}  Lv.${p.level}`;
  $('.tn', root).textContent = `오염도 ${p.taint}%`;
  setPlayerHp(p.hp, p.max_hp);
  const ce = $('.hud .bar.ce', root);
  $('i', ce).style.width = (p.ce / p.max_ce) * 100 + '%';
  $('span', ce).textContent = `주력 ${p.ce} / ${p.max_ce}`;
  $('.mark', ce).style.left = Math.min(100, (100 / p.max_ce) * 100) + '%';
  let chips = '';
  if (b.weakness) chips += '<div class="chip gold">★ 약점 간파</div>';
  if (b.domain_turns > 0) chips += `<div class="chip gold">영역 ${b.domain_turns}턴</div>`;
  if (b.hex_turns > 0) chips += `<div class="chip bad">저주: 공격력↓</div>`;
  if (e.barrier) chips += '<div class="chip">적 방어막</div>';
  $('.chips', root).innerHTML = chips;

  // 스킬 버튼
  const cls = p.class;
  const SK = {
    attack: ['ic_attack', '기본 공격', '+12', true],
    technique: ['ic_technique_' + cls, p.technique.replace(/\(.+\)/, ''), cls === 'sorcerer' ? '30' : '25'],
    guard: ['ic_guard', '방어', '+15', true],
    special: ['ic_special_' + cls, p.special, cls === 'sorcerer' ? '40' : '20'],
  };
  const lowHp = p.hp / p.max_hp < 0.4;
  let html = '';
  for (const id of ['attack', 'technique', 'guard', 'special']) {
    const o = opt(id);
    if (!o) continue;
    const [ic, label, cost, plus] = SK[id];
    let rec = '';
    if (id === 'guard' && e.intent === 'heavy') rec = ' recommend';
    if (id === 'special' && cls === 'sorcerer' && lowHp && o.enabled) rec = ' recommend';
    html += `<button class="sk${o.enabled ? '' : ' off'}${rec}" data-act="${id}"><span class="cost${plus ? ' plus' : ''}">${plus ? cost : '주력 ' + cost}</span><img src="${A(ic + '.png')}">${esc(label)}</button>`;
  }
  $('.skillbar', root).innerHTML = html;
  let extra = '';
  const d = opt('domain');
  if (d) extra += `<button class="btn domain${d.enabled ? '' : ' off'}" data-act="domain"><img src="${A('ic_domain.png')}">영역 전개 <small style="display:inline">(주력 100)</small></button>`;
  const hc = opt('heal_charm');
  if (hc) extra += `<button class="btn green" data-act="heal_charm"><img src="${A('ic_heal_charm.png')}">×${p.heal_charms}</button>`;
  const cc = opt('ce_charm');
  if (cc) extra += `<button class="btn blue" data-act="ce_charm"><img src="${A('ic_ce_charm.png')}">×${p.ce_charms}</button>`;
  $('.extra-row', root).innerHTML = extra;
  $('.extra-row', root).style.display = extra ? '' : 'none';
  root.querySelectorAll('.skillbar [data-act], .extra-row [data-act]').forEach((el) => el.addEventListener('click', () => act(el.dataset.act)));
  $('.player-art', root).classList.toggle('guard', false);
}

function renderBattle() {
  if (G.mounted !== 'battle' || !$('.battle')) mountBattle(G.state);
  else updateBattle();
}

// 보스의 제안
function renderOffer() {
  if (G.mounted !== 'battle' || !$('.battle')) mountBattle(G.state);
  const m = $('#modal');
  m.hidden = false;
  m.innerHTML = `<div class="offer panel"><img src="${A('enemy_mumyeong.png')}">
    <h3>무명이 손을 내밀고 있다</h3>
    <p>"손을 잡아라. 이름도, 상실도, 고통도 없는 세상이 온다."</p>
    <button class="btn" data-act="refuse">거절한다 — 끝까지 싸운다!</button>
    <button class="btn dark" data-act="accept">손을 잡는다…</button></div>`;
  bindActs(m);
}

// 잔해 처리 (승리 후)
function renderDisposal(scene) {
  setBg(stageBg());
  const p = opt('purify'), a = opt('absorb');
  const split = (t) => { const m = t.match(/^(.+?) \((.+)\)$/); return m ? [m[1], m[2]] : [t, '']; };
  const [pl, pd] = split(p.text), [al, ad] = split(a.text);
  scene.innerHTML = `${topbar()}<div class="center-col">
    <div class="big-word" style="position:static;animation:none;font-size:56px">승리!</div>
    <img class="victory-art" src="${A(STAGE_ENEMY[G.state.stage] + '.png')}">
    <div class="h-sub">쓰러진 적의 잔해가 아직 꿈틀거린다. 어떻게 할까?</div>
    <div class="choice-grid">
      <div class="choice panel purify" data-act="purify"><img src="${A('ic_star.png')}"><h4>${esc(pl)}</h4><p>${esc(pd)}</p></div>
      <div class="choice panel absorb" data-act="absorb"><img src="${A('ic_taint.png')}"><h4>${esc(al)}</h4><p>${esc(ad)}</p></div>
    </div></div>`;
  bindActs(scene); bindUi(scene);
}

// 레벨 업
function renderLevelUp(scene) {
  const p = G.state.player;
  const C = { grow_hp: ['ic_hp', '체력 단련', `최대 HP ${p.max_hp} → ${p.max_hp + 25}`], grow_ce: ['ic_ce', '주력 수련', `최대 주력 ${p.max_ce} → ${p.max_ce + 20}`],
    grow_atk: ['ic_attack', '술식 연마', `공격력 ${p.atk} → ${p.atk + 4}`], grow_def: ['ic_guard', '결계 연습', `방어력 ${p.def} → ${p.def + 3}`] };
  scene.innerHTML = `<div class="center-col">
    <div class="big-word" style="position:static;font-size:52px">LEVEL UP!<small>Lv.${p.level} 달성 · 키울 능력을 하나 골라요</small></div>
    <div class="choice-grid">${G.prompt.ids.map((id) => `<div class="choice panel" data-act="${id}"><img src="${A(C[id][0] + '.png')}"><h4>${C[id][1]}</h4><p>${C[id][2]}</p></div>`).join('')}</div></div>`;
  bindActs(scene);
}

// 로비 (장 사이) — 쿠키런식 홈 화면
function renderLobby(scene) {
  setBg('bg_lobby.jpg');
  const s = G.state, p = s.player;
  const next = Math.min(s.stage + 1, 5);
  const tabs = [['home', 'ic_home', '홈'], ['map', 'ic_map', '모험 지도'], ['char', 'ic_person', '캐릭터'], ['log', 'ic_scroll', '기록']];
  let body = '';
  if (G.tab === 'home') {
    body = `<div class="lobby-stage">
      <div class="bubble">${esc(HINTS[(s.stage + p.level) % HINTS.length])}</div>
      <img class="lobby-hero" src="${A(heroArt() + '.png')}">
      <div class="side-btns"><div><button class="icon-btn" data-tab="char"><img src="${A('ic_person.png')}"></button><label>상태</label></div>
        <div><button class="icon-btn" data-tab="map"><img src="${A('ic_map.png')}"></button><label>지도</label></div></div>
    </div>`;
  } else if (G.tab === 'map') {
    body = `<div class="tab-body"><div class="map">${CHAPTERS.map((c, i) => {
      const st = i <= s.stage ? 'clear' : i === next ? 'next' : 'lock';
      const icon = st === 'clear' ? `<img src="${A('ic_star.png')}">` : st === 'lock' ? `<img src="${A('ic_lock.png')}">` : i + 1;
      return `<div class="node panel ${st}" ${st === 'next' ? 'data-act="next_stage"' : ''}><div class="num">${icon}</div><div><h5>${c[0]} 「${c[1]}」</h5><p>${c[2]}${st === 'next' ? ' · 다음 모험!' : st === 'clear' ? ' · 클리어' : ''}</p></div></div>`;
    }).join('')}</div></div>`;
  } else if (G.tab === 'char') {
    body = `<div class="tab-body">${charSheet(p)}</div>`;
  } else {
    body = `<div class="tab-body panel">${logHtml()}</div>`;
  }
  scene.innerHTML = `<div class="lobby-top">
      <div class="prof"><div class="av"><img src="${A('pt_' + heroArt() + '.png')}"></div><div><b>${esc(p.name)}</b><small>${p.class === 'sorcerer' ? '주술사' : '주저사'} Lv.${p.level}</small></div></div>
      <div class="res"><span class="pill"><img src="${A('ic_heal_charm.png')}">${p.heal_charms}</span><span class="pill"><img src="${A('ic_ce_charm.png')}">${p.ce_charms}</span><span class="pill"><img src="${A('ic_taint.png')}">${p.taint}%</span></div>
    </div>
    ${body}
    <button class="btn go-btn" data-act="next_stage">모험 떠나기 ▶<small>${CHAPTERS[next][0]} 「${CHAPTERS[next][1]}」 · ${CHAPTERS[next][2]}</small></button>
    <div class="tabbar">${tabs.map(([id, ic, lb]) => `<button class="tab${G.tab === id ? ' on' : ''}" data-tab="${id}"><img src="${A(ic + '.png')}">${lb}</button>`).join('')}</div>`;
  bindActs(scene);
  scene.querySelectorAll('[data-tab]').forEach((t) => t.addEventListener('click', () => { G.tab = t.dataset.tab; renderLobby(scene); }));
  if (G.tab === 'log') { const tb = $('.tab-body', scene); tb.scrollTop = tb.scrollHeight; }
}

function charSheet(p) {
  return `<div class="char-sheet panel"><img class="full" src="${A(heroArt() + '.png')}">
    <table><tr><td>직업</td><td>${p.class === 'sorcerer' ? '주술사' : '주저사'}</td></tr><tr><td>레벨</td><td>${p.level}</td></tr>
      <tr><td>HP</td><td>${p.hp} / ${p.max_hp}</td></tr><tr><td>최대 주력</td><td>${p.max_ce}</td></tr>
      <tr><td>공격력</td><td>${p.atk}</td></tr><tr><td>방어력</td><td>${p.def}</td></tr>
      <tr><td>정화 / 흡수</td><td>${p.purified} / ${p.absorbed}</td></tr></table>
    <div class="full-row"><b>오염도 ${p.taint}%</b> ${p.taint >= 60 ? '— 위험! 결말이 바뀔 수 있어요' : '(60%를 넘으면 결말이 바뀌어요)'}
      <div class="taint-bar"><i style="width:${p.taint}%"></i><div class="warn"></div></div></div>
    <div class="full-row">술식 「${esc(p.technique)}」 · 특수 「${esc(p.special)}」 · 영역 「${esc(p.domain)}」</div></div>`;
}

// 게임 오버
function renderGameOver(scene) {
  scene.innerHTML = `<div class="center-col"><div class="panel ending-card">
    <img src="${A(heroArt() + '.png')}" style="filter:grayscale(.7);height:150px">
    <h2>앗, 쓰러졌다…</h2>
    <div class="sum">괜찮아요! 다시 도전하면 <b>회복 부적</b>을 한 장 더 줄게요.<br>적이 강공격을 예고하면 <b>방어</b>를 눌러 봐요.</div>
    <button class="btn" data-act="retry">이 장 다시 도전!</button>
    <button class="btn dark" data-act="restart">처음부터 새로 시작</button></div></div>`;
  bindActs(scene);
}

// 엔딩
function endingBg() {
  const e = G.state && G.state.ending;
  return e === 'dawn' || e === 'named' ? 'bg_title.jpg' : 'bg_stage5.jpg';
}
function renderEnding(scene) {
  setBg(endingBg());
  const e = G.state.ending;
  const art = e === 'dark' || e === 'tainted' ? 'enemy_mumyeong' : e === 'dawn' ? 'npc_hanseorim' : heroArt();
  scene.innerHTML = `<div class="center-col"><div class="panel ending-card">
    <div class="lbl">ENDING</div><h2>${esc(G.endingTitle)}</h2>
    <img src="${A(art + '.png')}">
    <div class="sum">${esc(G.endingSummary)}</div>
    <button class="btn" data-act="restart">새로운 기록 시작</button>
    <button class="btn dark" data-ui="log">이번 기록 다시 읽기</button></div></div>`;
  bindActs(scene); bindUi(scene);
}

// 기록(로그) 보기
function logHtml() {
  return `<div class="logview">${G.history.map((l) => {
    if (l.kind === 'dialogue') return `<div class="lg-dialogue"><b>${esc(l.speaker)}</b> "${esc(l.text)}"</div>`;
    return `<div class="lg-${l.kind}">${esc(l.text)}</div>`;
  }).join('')}</div>`;
}
function showLog() {
  const m = $('#modal');
  m.hidden = false;
  m.innerHTML = `<div class="panel modal-card"><div class="mh">기록 <button class="x-btn">✕</button></div><div class="mb">${logHtml()}</div></div>`;
  $('.x-btn', m).addEventListener('click', () => { m.hidden = true; if (G.state.scene === 'boss_offer') renderOffer(); });
  const mb = $('.mb', m); mb.scrollTop = mb.scrollHeight;
}

// ───────────────────────── 시작 ─────────────────────────
(async function boot() {
  // 이미지 미리 불러오기
  ['bg_title.jpg', 'bg_lobby.jpg'].forEach((n) => { const i = new Image(); i.src = A(n); });
  G.busy = true;
  await handle(await Bridge.boot());
})();
window.__G = G; // 테스트/디버그용
