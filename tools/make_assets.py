#!/usr/bin/env python3
"""동지야행 에셋 생성기.

모든 캐릭터/배경/아이콘을 벡터(SVG)로 직접 그린 뒤
  1) rsvg-convert로 투명 배경 PNG로 렌더링
  2) 알파 채널 기준으로 여백을 잘라내고(trim)
  3) 대화용 초상화는 전신 그림에서 얼굴 부분만 다시 잘라낸다.
나중에 생성형 이미지로 교체할 때는 web/assets/ 안의 같은 파일명으로 덮어쓰면 된다.

사용: python3 tools/make_assets.py  (rsvg-convert, Pillow 필요)
"""
import os
import subprocess
import tempfile
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "web", "assets")
SRC = os.path.join(ROOT, "tools", "svg")
os.makedirs(OUT, exist_ok=True)
os.makedirs(SRC, exist_ok=True)

INK = "#2b2233"
SKIN = "#ffe3cf"
SW = 'stroke="%s" stroke-width="6" stroke-linejoin="round" stroke-linecap="round"' % INK


def svg(w, h, body, defs=""):
    return (
        f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">'
        f"<defs>{defs}</defs>{body}</svg>"
    )


def render(name, svg_text, width, trim=True, pad=6, fmt="png"):
    path_svg = os.path.join(SRC, name + ".svg")
    with open(path_svg, "w") as f:
        f.write(svg_text)
    tmp = tempfile.mktemp(suffix=".png")
    subprocess.run(["rsvg-convert", "-w", str(width), "-o", tmp, path_svg], check=True)
    im = Image.open(tmp).convert("RGBA")
    if trim:
        bbox = im.getchannel("A").getbbox()
        if bbox:
            l, t, r, b = bbox
            im = im.crop((max(0, l - pad), max(0, t - pad), min(im.width, r + pad), min(im.height, b + pad)))
    os.remove(tmp)
    if fmt == "jpg":
        out = os.path.join(OUT, name + ".jpg")
        im.convert("RGB").save(out, quality=82, optimize=True, progressive=True)
    else:
        out = os.path.join(OUT, name + ".png")
        im.save(out, optimize=True)
    return out


def crop_portrait(src_name, dst_name, frac=0.58):
    """전신 그림의 위쪽(머리) 부분만 잘라 초상화로 만든다."""
    im = Image.open(os.path.join(OUT, src_name + ".png"))
    h = int(im.height * frac)
    head = im.crop((0, 0, im.width, h))
    bbox = head.getchannel("A").getbbox()
    head = head.crop(bbox)
    head.save(os.path.join(OUT, dst_name + ".png"), optimize=True)


# ───────────────────────── 캐릭터 파츠 ─────────────────────────

def eyes(color, style="normal", y=205):
    lx, rx = 152, 248
    if style == "glow":
        return "".join(
            f'<ellipse cx="{x}" cy="{y}" rx="16" ry="12" fill="{color}" filter="url(#glow)"/>'
            f'<ellipse cx="{x}" cy="{y}" rx="8" ry="6" fill="#fff"/>' for x in (lx, rx))
    if style == "closed":
        return "".join(f'<path d="M{x-20} {y} Q{x} {y+16} {x+20} {y}" fill="none" {SW}/>' for x in (lx, rx))
    out = ""
    for x in (lx, rx):
        ry = 27 if style != "sharp" else 20
        out += f'<ellipse cx="{x}" cy="{y}" rx="21" ry="{ry}" fill="#fff" {SW}/>'
        out += f'<ellipse cx="{x}" cy="{y+3}" rx="15" ry="{ry-6}" fill="{color}"/>'
        out += f'<ellipse cx="{x}" cy="{y+6}" rx="7" ry="{ry-14}" fill="{INK}"/>'
        out += f'<circle cx="{x-6}" cy="{y-7}" r="6" fill="#fff"/><circle cx="{x+6}" cy="{y+10}" r="3" fill="#fff" opacity=".8"/>'
    if style == "tired":
        out += "".join(f'<path d="M{x-24} {y-8} L{x+24} {y-8}" {SW} fill="none"/>'
                       f'<path d="M{x-22} {y-8} L{x+22} {y-8} L{x+22} {y-28} L{x-22} {y-28} Z" fill="{SKIN}"/>' for x in (lx, rx))
    if style == "sharp":
        out += f'<path d="M{lx-26} {y-26} L{lx+22} {y-16}" {SW} fill="none"/><path d="M{rx+26} {y-26} L{rx-22} {y-16}" {SW} fill="none"/>'
    return out


def face_base(skin=SKIN, mouth="smile", blush=True):
    s = f'<circle cx="200" cy="185" r="118" fill="{skin}" {SW}/>'
    if blush:
        s += '<ellipse cx="128" cy="245" rx="18" ry="10" fill="#ff9aa8" opacity=".55"/><ellipse cx="272" cy="245" rx="18" ry="10" fill="#ff9aa8" opacity=".55"/>'
    if mouth == "smile":
        s += f'<path d="M188 250 Q200 262 212 250" fill="none" {SW}/>'
    elif mouth == "flat":
        s += f'<path d="M190 254 L210 254" fill="none" {SW}/>'
    elif mouth == "smirk":
        s += f'<path d="M184 252 Q204 262 220 244" fill="none" {SW}/>'
    elif mouth == "open":
        s += f'<path d="M186 248 Q200 272 214 248 Z" fill="#c2405a" {SW}/>'
    return s


def body(coat, trim="#fff", legs="#2b2a3a", wide=0):
    w = wide
    return (
        f'<ellipse cx="200" cy="505" rx="{110+w}" ry="16" fill="#000" opacity=".18"/>'
        f'<rect x="150" y="425" width="38" height="66" rx="14" fill="{legs}" {SW}/>'
        f'<rect x="212" y="425" width="38" height="66" rx="14" fill="{legs}" {SW}/>'
        f'<ellipse cx="166" cy="492" rx="27" ry="13" fill="{INK}"/><ellipse cx="234" cy="492" rx="27" ry="13" fill="{INK}"/>'
        f'<path d="M{122-w} 300 Q200 268 {278+w} 300 L{300+w} 445 Q200 466 {100-w} 445 Z" fill="{coat}" {SW}/>'
        f'<path d="M200 292 L200 455" stroke="{trim}" stroke-width="5" opacity=".7"/>'
    )


def arms(coat, skin=SKIN, pose="down"):
    if pose == "up":  # 오른손 들기
        return (
            f'<ellipse cx="112" cy="360" rx="26" ry="50" fill="{coat}" {SW} transform="rotate(14 112 360)"/>'
            f'<circle cx="104" cy="408" r="20" fill="{skin}" {SW}/>'
            f'<ellipse cx="300" cy="310" rx="26" ry="52" fill="{coat}" {SW} transform="rotate(-40 300 310)"/>'
            f'<circle cx="334" cy="270" r="21" fill="{skin}" {SW}/>'
        )
    return (
        f'<ellipse cx="112" cy="360" rx="26" ry="50" fill="{coat}" {SW} transform="rotate(14 112 360)"/>'
        f'<circle cx="104" cy="408" r="20" fill="{skin}" {SW}/>'
        f'<ellipse cx="288" cy="360" rx="26" ry="50" fill="{coat}" {SW} transform="rotate(-14 288 360)"/>'
        f'<circle cx="296" cy="408" r="20" fill="{skin}" {SW}/>'
    )


def hair_back_short(c):
    return f'<circle cx="200" cy="172" r="130" fill="{c}" {SW}/>'


def hair_back_long(c, length=420):
    return f'<path d="M78 170 Q70 60 200 44 Q330 60 322 170 L340 {length} Q200 {length+30} 60 {length} Z" fill="{c}" {SW}/>'


def bangs(c, spiky=True):
    if spiky:
        d = "M80 196 Q84 66 200 56 Q316 66 320 196 L300 150 L284 196 L258 128 L236 186 L206 118 L178 186 L150 128 L124 196 L104 150 Z"
    else:
        d = "M80 200 Q84 64 200 56 Q316 64 320 200 Q300 140 250 128 Q230 170 200 150 Q170 170 150 128 Q100 140 80 200 Z"
    return f'<path d="{d}" fill="{c}" {SW}/>'


DEFS_GLOW = (
    '<filter id="glow" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="6" result="b"/>'
    '<feMerge><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>'
    '<filter id="soft"><feGaussianBlur stdDeviation="10"/></filter>'
)


def character(parts, w=400, h=520):
    return svg(w, h, parts, DEFS_GLOW)


# ───────────────────────── 캐릭터 ─────────────────────────

def hero_sorcerer():
    coat = "#1f2a4d"
    p = hair_back_short("#22263b")
    p += body(coat, trim="#f2c94c")
    p += f'<path d="M150 296 L200 330 L250 296" fill="none" stroke="#f2c94c" stroke-width="6"/>'
    p += "".join(f'<circle cx="200" cy="{y}" r="6" fill="#f2c94c" {SW}/>' for y in (350, 385, 420))
    p += arms(coat, pose="up")
    # 손끝의 청뢰
    p += ('<g filter="url(#glow)"><path d="M338 262 L362 214 L350 214 L376 160 L352 206 L366 206 Z" fill="#7fd6ff" stroke="#e8fbff" stroke-width="4"/>'
          '<circle cx="360" cy="210" r="34" fill="#4fb6ff" opacity=".25"/></g>')
    p += face_base(mouth="smile")
    p += eyes("#3b7bff")
    p += bangs("#22263b")
    p += '<path d="M120 110 Q200 70 286 112" stroke="#4a5590" stroke-width="8" fill="none" opacity=".6"/>'
    # 스승의 부러진 부채(허리춤)
    p += f'<path d="M110 420 L70 380 Q60 405 76 430 Z" fill="#fff6e0" {SW}/><path d="M70 380 L92 420 M76 430 L95 418" {SW}/>'
    return character(p)


def hero_curse():
    coat = "#262329"
    p = f'<path d="M66 190 Q70 40 200 34 Q330 40 334 190 L320 300 L80 300 Z" fill="#3a3540" {SW}/>'  # 후드
    p += body(coat, trim="#c0392b", wide=6)
    p += arms(coat, pose="down")
    # 흑사: 손에서 뻗는 검은 실
    p += ('<g fill="none" stroke="#120f16" stroke-width="5" stroke-linecap="round">'
          '<path d="M296 410 Q340 440 360 400 T390 360"/><path d="M296 410 Q330 470 380 460"/><path d="M104 410 Q60 440 40 400 T12 370"/></g>'
          '<g fill="#c0392b"><circle cx="390" cy="360" r="5"/><circle cx="380" cy="460" r="5"/><circle cx="12" cy="370" r="5"/></g>')
    p += face_base(mouth="flat", blush=False)
    p += eyes("#e0413a", "sharp")
    p += bangs("#e9e6f0")
    # 반쪽 마스크
    p += f'<path d="M110 236 Q200 222 290 236 Q284 296 200 302 Q116 296 110 236 Z" fill="#1b181f" {SW}/>'
    p += '<path d="M150 262 L250 262" stroke="#c0392b" stroke-width="4"/>'
    p += f'<path d="M70 190 Q74 70 200 64" fill="none" {SW}/>'
    return character(p)


def npc_doyunjae():
    coat = "#4a4f5c"
    p = hair_back_short("#5b3a29")
    p += body(coat, trim="#ffffff")
    p += f'<path d="M180 296 L200 360 L220 296 Z" fill="#fff" {SW}/><path d="M196 316 L204 316 L208 370 L200 380 L192 370 Z" fill="#2e5a8a"/>'
    p += arms(coat)
    p += face_base(mouth="flat")
    p += eyes("#6b4b3a", "tired")
    p += bangs("#5b3a29", spiky=True)
    p += (f'<rect x="120" y="182" width="64" height="48" rx="12" fill="#fff" fill-opacity=".25" {SW}/>'
          f'<rect x="216" y="182" width="64" height="48" rx="12" fill="#fff" fill-opacity=".25" {SW}/>'
          f'<path d="M184 200 L216 200" {SW}/>')
    p += f'<path d="M150 272 Q160 278 170 272" stroke="#8a6a5a" stroke-width="3" fill="none"/>'  # 다크서클 느낌
    return character(p)


def npc_maehwa():
    coat = "#6b3d7a"
    p = f'<circle cx="200" cy="58" r="44" fill="#cfcfd6" {SW}/>'  # 쪽머리
    p += hair_back_short("#cfcfd6")
    p += body(coat, trim="#f6c1d6", wide=10)
    p += f'<path d="M140 300 Q200 340 260 300" fill="#f6c1d6" {SW}/>'
    p += arms(coat)
    p += face_base(mouth="smile")
    p += eyes("#5a4a3a", "closed")
    p += bangs("#cfcfd6", spiky=False)
    p += f'<path d="M150 150 Q160 140 170 150 M230 150 Q240 140 250 150" stroke="{INK}" stroke-width="3" fill="none"/>'
    # 매화 비녀
    p += f'<path d="M150 40 L270 70" stroke="#8a5a2b" stroke-width="8"/>'
    p += "".join(f'<circle cx="{270+dx}" cy="{70+dy}" r="11" fill="#ff7fa8" {SW}/>' for dx, dy in ((0, 0), (14, -10), (16, 10), (4, 16), (-8, -12)))
    p += '<circle cx="276" cy="70" r="6" fill="#ffe066"/>'
    return character(p)


def npc_hanseorim():
    coat = "#f4f1ea"
    p = hair_back_long("#1a1a22", 430)
    p += body(coat, trim="#9b2335")
    p += f'<path d="M122 300 L200 380 L278 300" fill="none" stroke="#9b2335" stroke-width="8"/>'
    p += arms(coat)
    # 사슬
    p += "".join(f'<ellipse cx="{x}" cy="{400 + (x % 3) * 4}" rx="14" ry="9" fill="none" stroke="#8d8d99" stroke-width="6"/>' for x in range(96, 320, 24))
    p += face_base(mouth="smirk")
    p += eyes("#9b2335")
    p += bangs("#1a1a22", spiky=False)
    # 부채
    p += (f'<g transform="rotate(-20 330 380)"><path d="M330 380 L280 300 Q330 270 380 300 Z" fill="#fff" {SW}/>'
          f'<path d="M330 380 L300 300 M330 380 L330 286 M330 380 L360 300" stroke="#9b2335" stroke-width="3"/></g>')
    return character(p)


def villain_goharam():
    coat = "#2f5d50"
    p = (f'<g transform="rotate(-12 200 150)"><path d="M200 -10 L330 120 L200 250 L70 120 Z" fill="#f2e3c6" {SW}/>'
         f'<path d="M200 -10 L200 250 M70 120 L330 120" stroke="#b33" stroke-width="5"/><circle cx="200" cy="120" r="30" fill="none" stroke="#b33" stroke-width="5"/></g>')
    p += hair_back_short("#3b2f2a")
    p += body(coat, trim="#d9b44a")
    p += arms(coat, pose="up")
    p += '<path d="M334 270 Q380 200 392 120" stroke="#e8e0d0" stroke-width="3" fill="none"/>'
    p += face_base(mouth="smirk", blush=False)
    p += eyes("#d9b44a", "sharp")
    p += f'<path d="M84 190 Q90 70 200 60 Q310 66 318 176 Q260 90 160 120 Q110 140 84 190 Z" fill="#3b2f2a" {SW}/>'
    return character(p, 420, 540)


def enemy_binjari():
    # 출석부 몸통의 아이, 얼굴은 하얀 빈칸
    p = '<ellipse cx="200" cy="505" rx="110" ry="16" fill="#000" opacity=".2"/>'
    p += f'<rect x="150" y="430" width="34" height="62" rx="12" fill="#2d2a40" {SW}/><rect x="216" y="430" width="34" height="62" rx="12" fill="#2d2a40" {SW}/>'
    p += f'<rect x="108" y="280" width="184" height="170" rx="14" fill="#4c6b4a" {SW}/>'
    p += f'<rect x="124" y="296" width="152" height="140" rx="6" fill="#f3efe2" {SW}/>'
    for i in range(6):
        y = 318 + i * 20
        p += f'<path d="M136 {y} L230 {y}" stroke="#9aa" stroke-width="3"/>'
        p += f'<path d="M140 {y-4} L{196 + (i*13)%30} {y+2}" stroke="#7a2b2b" stroke-width="4"/>'  # 긁힌 이름
    p += f'<rect x="240" y="410" width="28" height="18" fill="#fff" stroke="#c33" stroke-width="3"/>'  # 빈칸
    p += f'<ellipse cx="100" cy="350" rx="22" ry="48" fill="#4c6b4a" {SW} transform="rotate(20 100 350)"/><ellipse cx="300" cy="350" rx="22" ry="48" fill="#4c6b4a" {SW} transform="rotate(-20 300 350)"/>'
    p += f'<path d="M88 30 Q200 0 312 30 L330 200 Q200 230 70 200 Z" fill="#1d1b24" {SW}/>'  # 단발
    p += f'<rect x="110" y="70" width="180" height="190" rx="22" fill="#ffffff" {SW}/>'
    p += '<rect x="132" y="92" width="136" height="146" rx="8" fill="none" stroke="#d33" stroke-width="5" stroke-dasharray="14 10"/>'
    p += f'<path d="M100 60 Q200 30 300 60 L300 110 Q200 80 100 110 Z" fill="#1d1b24" {SW}/>'
    p += '<g opacity=".8" fill="#c22"><path d="M190 230 l6 20 l6 -20z"/></g>'
    return character(p)


def enemy_conductor():
    p = '<ellipse cx="210" cy="525" rx="140" ry="18" fill="#000" opacity=".2"/>'
    p += f'<path d="M110 250 Q210 220 310 250 L350 510 L70 510 Z" fill="#2c3242" {SW}/>'
    p += "".join(f'<circle cx="{x}" cy="{y}" r="7" fill="#e6c25a" {SW}/>' for x in (185, 235) for y in (300, 360, 420))
    p += f'<ellipse cx="80" cy="360" rx="26" ry="70" fill="#2c3242" {SW} transform="rotate(16 80 360)"/>'
    p += f'<ellipse cx="340" cy="340" rx="26" ry="70" fill="#2c3242" {SW} transform="rotate(-30 340 340)"/>'
    # 승차권 다발
    p += "".join(f'<rect x="{360+i*6}" y="{240-i*10}" width="60" height="34" rx="4" fill="#f4e7b5" {SW} transform="rotate({-20+i*8} 390 260)"/>' for i in range(3))
    p += f'<ellipse cx="210" cy="170" rx="100" ry="110" fill="#0b0b12" {SW}/>'
    p += eyes("#ffd84a", "glow", y=180).replace("152", "172").replace("248", "248")
    p += f'<path d="M100 120 Q210 40 320 120 L330 140 L90 140 Z" fill="#2c3242" {SW}/>'
    p += f'<rect x="80" y="130" width="260" height="26" rx="10" fill="#1c2030" {SW}/>'
    p += f'<rect x="180" y="74" width="60" height="34" rx="6" fill="#e6c25a" {SW}/>'
    return character(p, 440, 545)


def enemy_saint():
    p = '<ellipse cx="200" cy="535" rx="120" ry="16" fill="#000" opacity=".15"/>'
    p += f'<path d="M80 130 Q60 40 200 30 Q340 40 320 130 L360 520 L40 520 Z" fill="#cfd3dc" {SW}/>'  # 긴 머리
    p += f'<path d="M120 270 Q200 240 280 270 L330 520 L70 520 Z" fill="#fbfbff" {SW}/>'
    p += f'<ellipse cx="96" cy="370" rx="22" ry="80" fill="#fbfbff" {SW} transform="rotate(16 96 370)"/><ellipse cx="304" cy="370" rx="22" ry="80" fill="#fbfbff" {SW} transform="rotate(-16 304 370)"/>'
    p += f'<circle cx="200" cy="160" r="100" fill="#e9ecf2" {SW}/>'
    p += eyes("#7a7f8f", "closed", y=150)
    # 스피커 입
    p += f'<circle cx="200" cy="215" r="34" fill="#2b2b33" {SW}/><circle cx="200" cy="215" r="20" fill="#555" /><circle cx="200" cy="215" r="8" fill="#111"/>'
    for i in range(3):
        r = 60 + i * 26
        p += f'<path d="M{200+r*0.7} {215-r*0.7} A{r} {r} 0 0 1 {200+r*0.7} {215+r*0.7}" fill="none" stroke="#7fe0ff" stroke-width="5" opacity="{0.9-i*0.25}"/>'
        p += f'<path d="M{200-r*0.7} {215-r*0.7} A{r} {r} 0 0 0 {200-r*0.7} {215+r*0.7}" fill="none" stroke="#7fe0ff" stroke-width="5" opacity="{0.9-i*0.25}"/>'
    # 노이즈 줄
    p += "".join(f'<rect x="{60+(i*53)%260}" y="{300+i*28}" width="{40+(i*29)%60}" height="5" fill="#9aa4b8" opacity=".7"/>' for i in range(7))
    return character(p, 400, 550)


def enemy_manghyang():
    p = '<ellipse cx="260" cy="535" rx="220" ry="22" fill="#000" opacity=".2"/>'
    p += f'<path d="M40 520 Q20 300 120 200 Q160 60 260 40 Q380 60 410 200 Q500 300 480 520 Z" fill="#2d6c8c" {SW}/>'
    p += '<path d="M60 480 Q140 440 220 480 T380 480 T470 470" fill="none" stroke="#9fe3ff" stroke-width="7" opacity=".8"/>'
    p += '<path d="M90 400 Q170 360 250 400 T420 390" fill="none" stroke="#9fe3ff" stroke-width="6" opacity=".6"/>'
    # 잠긴 집들
    for x, y, s in ((150, 300, 1.0), (280, 330, 1.2), (360, 260, .8), (210, 420, .9)):
        p += (f'<g transform="translate({x} {y}) scale({s})"><rect x="-30" y="-10" width="60" height="44" fill="#d9cfb8" {SW}/>'
              f'<path d="M-42 -6 L0 -44 L42 -6 Z" fill="#6c4a3a" {SW}/><rect x="-8" y="10" width="16" height="24" fill="#ffd84a" opacity=".85"/></g>')
    # 눈들
    for x, y, r in ((200, 150, 22), (300, 130, 26), (250, 210, 16), (140, 230, 14), (370, 200, 16)):
        p += f'<ellipse cx="{x}" cy="{y}" rx="{r}" ry="{r*0.7}" fill="#eaffff" {SW}/><circle cx="{x}" cy="{y}" r="{r*0.35}" fill="{INK}"/>'
    return character(p, 520, 560)


def boss_mumyeong():
    p = '<ellipse cx="220" cy="560" rx="140" ry="16" fill="#5b2a86" opacity=".35" filter="url(#soft)"/>'
    p += '<circle cx="220" cy="230" r="200" fill="#5b2a86" opacity=".28" filter="url(#soft)"/>'
    # 가사(袈裟)
    p += f'<path d="M80 300 Q220 230 360 300 L420 520 Q220 560 20 520 Z" fill="#7a2230" {SW}/>'
    p += f'<path d="M110 300 L330 520" stroke="#c9a14a" stroke-width="14"/><path d="M110 300 L330 520" stroke="{INK}" stroke-width="3" fill="none"/>'
    # 꿰매진 이름들 (짧은 글자줄)
    import random
    rnd = random.Random(7)
    for _ in range(46):
        x, y = rnd.randint(60, 380), rnd.randint(320, 515)
        p += f'<rect x="{x}" y="{y}" width="{rnd.randint(14,30)}" height="6" rx="2" fill="#f3e6c8" opacity=".85"/>'
        p += f'<path d="M{x-3} {y+3} l-4 -4 M{x-3} {y+3} l-4 4" stroke="#f3e6c8" stroke-width="2"/>'
    # 가부좌 손 (합장)
    p += f'<path d="M200 330 L220 270 L240 330 Z" fill="#e9dccb" {SW}/>'
    # 얼굴 없는 머리
    p += f'<ellipse cx="220" cy="170" rx="96" ry="112" fill="#efe8dc" {SW}/>'
    p += '<ellipse cx="220" cy="180" rx="60" ry="72" fill="#d9cfc0" opacity=".5"/>'
    p += '<path d="M150 120 Q220 100 290 120" stroke="#cfc3b0" stroke-width="4" fill="none"/>'
    # 후광
    p += '<circle cx="220" cy="170" r="140" fill="none" stroke="#c9a14a" stroke-width="6" stroke-dasharray="4 14" opacity=".8"/>'
    return character(p, 440, 580)


# ───────────────────────── 배경 ─────────────────────────

def sky(top, bottom, w=1200, h=1600):
    return (f'<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{top}"/><stop offset="1" stop-color="{bottom}"/></linearGradient>',
            f'<rect width="{w}" height="{h}" fill="url(#sky)"/>')


def stars(n=70, seed=1, w=1200, h=900):
    import random
    r = random.Random(seed)
    return "".join(f'<circle cx="{r.randint(0,w)}" cy="{r.randint(0,h)}" r="{r.choice([1.5,2,2.5,3])}" fill="#fff" opacity="{r.uniform(.3,.9):.2f}"/>' for _ in range(n))


def snow(n=90, seed=3, w=1200, h=1600):
    import random
    r = random.Random(seed)
    return "".join(f'<circle cx="{r.randint(0,w)}" cy="{r.randint(0,h)}" r="{r.choice([3,4,5,6])}" fill="#fff" opacity="{r.uniform(.4,.9):.2f}"/>' for _ in range(n))


def skyline(y, color, seed=5, w=1200):
    import random
    r = random.Random(seed)
    x, d = 0, f"M0 1600 L0 {y}"
    win = ""
    while x < w:
        bw, bh = r.randint(60, 140), r.randint(80, 320)
        d += f" L{x} {y-bh} L{x+bw} {y-bh}"
        for wy in range(y - bh + 20, y - 10, 34):
            for wx in range(x + 12, x + bw - 14, 26):
                if r.random() < .35:
                    win += f'<rect x="{wx}" y="{wy}" width="12" height="16" fill="#ffd36b" opacity=".75"/>'
        x += bw
    d += f" L{w} {y} L{w} 1600 Z"
    return f'<path d="{d}" fill="{color}"/>' + win


def bg_title():
    g, s = sky("#0b0d2a", "#3a1f4f")
    b = s + stars(120, 2)
    b += '<circle cx="860" cy="330" r="190" fill="#fff4d6"/><circle cx="860" cy="330" r="260" fill="#fff4d6" opacity=".12"/>'
    b += skyline(1150, "#151530", 9)
    # 광화문 실루엣
    b += ('<g fill="#0a0a18"><rect x="300" y="1060" width="600" height="160"/>'
          '<path d="M240 1070 Q600 960 960 1070 L900 1040 Q600 950 300 1040 Z"/>'
          '<path d="M330 960 Q600 870 870 960 L830 940 Q600 860 370 940 Z"/><rect x="380" y="955" width="440" height="110"/></g>'
          '<g fill="#3a1f4f"><path d="M520 1220 Q600 1120 680 1220 Z"/></g>')
    b += '<rect y="1200" width="1200" height="400" fill="#0a0a18"/>' + snow(80, 4)
    return svg(1200, 1600, b, g)


def bg_lobby():
    g, s = sky("#141a3d", "#5b3570")
    b = s + stars(80, 6)
    b += '<circle cx="250" cy="300" r="120" fill="#fff4d6" opacity=".95"/>'
    b += skyline(1050, "#1d1d42", 11)
    # 옥상 바닥과 난간
    b += '<rect y="1150" width="1200" height="450" fill="#2a2440"/><rect y="1150" width="1200" height="14" fill="#463d66"/>'
    b += "".join(f'<rect x="{x}" y="1060" width="10" height="94" fill="#463d66"/>' for x in range(0, 1200, 80)) + '<rect y="1060" width="1200" height="10" fill="#463d66"/>'
    # 등불
    for x in (180, 1020):
        b += f'<line x1="{x}" y1="980" x2="{x}" y2="1150" stroke="#1a1530" stroke-width="10"/><ellipse cx="{x}" cy="960" rx="40" ry="52" fill="#ff7b54"/><ellipse cx="{x}" cy="960" rx="80" ry="90" fill="#ff7b54" opacity=".18"/>'
    return svg(1200, 1600, b, g)


def bg_school():
    g, s = sky("#1b2440", "#56658a")
    b = s + stars(40, 7)
    b += '<rect x="120" y="620" width="960" height="560" fill="#3a3f52"/><rect x="100" y="600" width="1000" height="40" fill="#2a2e3d"/>'
    for row in range(3):
        for col in range(7):
            lit = (row * 7 + col) == 9
            b += f'<rect x="{170+col*130}" y="{680+row*160}" width="90" height="100" fill="{"#ffe28a" if lit else "#1f2333"}" stroke="#55596b" stroke-width="6"/>'
    b += '<rect x="520" y="480" width="160" height="140" fill="#2a2e3d"/><circle cx="600" cy="550" r="44" fill="#e9e3d0" stroke="#222" stroke-width="6"/><path d="M600 550 L600 520 M600 550 L622 560" stroke="#222" stroke-width="6"/>'
    b += '<path d="M380 470 L820 470 L800 500 L400 500 Z" fill="#c33"/><rect x="420" y="440" width="70" height="40" fill="#e9e3d0"/>'  # 찢긴 현수막
    b += '<rect y="1180" width="1200" height="420" fill="#e9eef7"/><path d="M0 1180 Q600 1140 1200 1180 L1200 1240 L0 1240 Z" fill="#fff"/>'
    b += snow(110, 8)
    return svg(1200, 1600, b, g)


def bg_subway():
    g = '<linearGradient id="sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#0c1016"/><stop offset="1" stop-color="#1f2a2f"/></linearGradient>'
    b = '<rect width="1200" height="1600" fill="url(#sky)"/>'
    b += '<path d="M0 300 L1200 300 L1200 340 L0 340 Z" fill="#26323a"/>'
    for x in range(60, 1200, 280):
        b += f'<rect x="{x}" y="340" width="70" height="900" fill="#2c3a42"/><rect x="{x}" y="340" width="70" height="30" fill="#3d4f59"/>'
        b += f'<ellipse cx="{x+35}" cy="390" rx="60" ry="14" fill="#cfe9d6" opacity=".25"/>'
    b += '<rect x="420" y="420" width="360" height="90" rx="10" fill="#0f3d2e" stroke="#7aa" stroke-width="5"/><rect x="450" y="450" width="300" height="28" fill="#7fd6a0" opacity=".6"/>'
    b += '<rect y="1240" width="1200" height="40" fill="#e6c25a"/><rect y="1280" width="1200" height="320" fill="#1a1f24"/>'
    b += '<path d="M0 1420 L1200 1420" stroke="#5a6066" stroke-width="12"/><path d="M0 1520 L1200 1520" stroke="#5a6066" stroke-width="12"/>'
    b += '<circle cx="1100" cy="820" r="140" fill="#ffd84a" opacity=".12"/><circle cx="1100" cy="820" r="50" fill="#ffd84a" opacity=".5"/>'
    return svg(1200, 1600, b, g)


def bg_apartment():
    g, s = sky("#2a1838", "#c76b4d")
    b = s + '<circle cx="900" cy="760" r="160" fill="#ffb35c" opacity=".9"/>'
    for i, x in enumerate((60, 360, 700, 980)):
        h = 700 + (i % 2) * 150
        b += f'<rect x="{x}" y="{1500-h}" width="240" height="{h}" fill="#3a2b3f"/>'
        for wy in range(1500 - h + 40, 1480, 70):
            for wx in (x + 30, x + 100, x + 170):
                b += f'<rect x="{wx}" y="{wy}" width="40" height="34" fill="#1f1726"/>'
        b += f'<text x="{x+120}" y="{1500-h+30}" font-size="34" text-anchor="middle" fill="#7a6a80" font-family="sans-serif">30{i+1}</text>'
    import random
    r = random.Random(12)
    for _ in range(16):
        x, y = r.randint(40, 1160), r.randint(100, 600)
        b += f'<line x1="{x}" y1="{y+40}" x2="600" y2="1200" stroke="#e8dcc6" stroke-width="1.5" opacity=".6"/>'
        b += f'<g transform="translate({x} {y}) rotate({r.randint(-20,20)})"><path d="M0 -40 L32 0 L0 40 L-32 0 Z" fill="#f2e3c6" stroke="#2b2233" stroke-width="3"/><circle r="10" fill="none" stroke="#b33" stroke-width="3"/></g>'
    b += '<rect y="1440" width="1200" height="160" fill="#22182a"/>'
    return svg(1200, 1600, b, g)


def bg_tower():
    g, s = sky("#060a1e", "#24345e")
    b = s + stars(140, 13)
    b += '<path d="M0 1300 Q300 1100 600 1120 Q900 1100 1200 1260 L1200 1600 L0 1600 Z" fill="#101a30"/>'
    b += '<rect x="570" y="420" width="60" height="720" fill="#c9ccd6"/><rect x="530" y="600" width="140" height="80" rx="20" fill="#dfe3ec"/><rect x="590" y="200" width="20" height="230" fill="#e44"/>'
    for i in range(5):
        r = 80 + i * 70
        b += f'<circle cx="600" cy="260" r="{r}" fill="none" stroke="#7fe0ff" stroke-width="5" opacity="{0.6-i*0.1:.2f}" stroke-dasharray="20 14"/>'
    b += skyline(1450, "#0b1224", 21)
    return svg(1200, 1600, b, g)


def bg_river():
    g, s = sky("#091a2c", "#2c5b78")
    b = s + stars(50, 17)
    b += skyline(820, "#0f2236", 25)
    b += '<rect y="820" width="1200" height="780" fill="#123850"/>'
    b += '<path d="M0 900 L1200 900 L1200 930 L0 930 Z" fill="#2a3d4c"/>' + "".join(f'<rect x="{x}" y="930" width="34" height="120" fill="#22323f"/>' for x in range(80, 1200, 240))
    for i in range(6):
        b += f'<ellipse cx="600" cy="1260" rx="{460-i*70}" ry="{170-i*26}" fill="none" stroke="#7fd6ff" stroke-width="7" opacity="{0.25+i*0.1:.2f}"/>'
    b += '<path d="M0 1100 Q150 1070 300 1100 T600 1100 T900 1100 T1200 1100" fill="none" stroke="#7fd6ff" stroke-width="5" opacity=".4"/>'
    return svg(1200, 1600, b, g)


def bg_gate():
    g = ('<radialGradient id="sky" cx=".5" cy=".45" r=".7"><stop offset="0" stop-color="#5b2a86"/><stop offset=".55" stop-color="#1a0f2a"/><stop offset="1" stop-color="#07050c"/></radialGradient>')
    b = '<rect width="1200" height="1600" fill="url(#sky)"/>'
    b += '<rect x="300" y="300" width="600" height="900" rx="20" fill="#1a1222" stroke="#c9a14a" stroke-width="12"/>'
    b += '<rect x="330" y="330" width="260" height="840" fill="#120c18"/><rect x="610" y="330" width="260" height="840" fill="#120c18"/>'
    b += '<rect x="588" y="330" width="24" height="840" fill="#d7b8ff" opacity=".85"/><rect x="560" y="330" width="80" height="840" fill="#d7b8ff" opacity=".2"/>'
    import random
    r = random.Random(31)
    for _ in range(40):
        x, y = r.randint(520, 680), r.randint(340, 1150)
        b += f'<rect x="{x}" y="{y}" width="{r.randint(14,34)}" height="6" fill="#f3e6c8" opacity="{r.uniform(.3,.9):.2f}"/>'
    for cx in (250, 950):
        b += "".join(f'<ellipse cx="{cx + (k%2)*6}" cy="{360+k*48}" rx="14" ry="22" fill="none" stroke="#6e6e7a" stroke-width="7"/>' for k in range(17))
    b += '<rect y="1220" width="1200" height="380" fill="#0b0810"/><path d="M0 1220 L1200 1220" stroke="#c9a14a" stroke-width="6"/>'
    for x in range(40, 1200, 160):
        b += f'<rect x="{x}" y="1260" width="100" height="40" fill="#1a1222"/>'
    return svg(1200, 1600, b, g)


# ───────────────────────── 아이콘 ─────────────────────────

def icon(body, bg=None):
    base = ""
    if bg:
        base = f'<circle cx="64" cy="64" r="58" fill="{bg}" stroke="{INK}" stroke-width="5"/>'
    return svg(128, 128, base + body, DEFS_GLOW)


ISW = f'stroke="{INK}" stroke-width="5" stroke-linejoin="round" stroke-linecap="round"'

ICONS = {
    "ic_attack": icon(f'<rect x="34" y="44" width="58" height="46" rx="16" fill="#ffd2b0" {ISW}/>'
                      f'<path d="M48 44 L48 60 M62 44 L62 60 M76 44 L76 60" {ISW}/><rect x="42" y="86" width="42" height="18" rx="6" fill="#d9534f" {ISW}/>', "#ffb35c"),
    "ic_technique_sorcerer": icon(f'<path d="M70 18 L36 70 L60 70 L48 112 L94 52 L68 52 L82 18 Z" fill="#bff0ff" {ISW}/>', "#3b7bff"),
    "ic_technique_curse_user": icon(f'<path d="M30 90 Q50 30 70 64 T104 40" fill="none" stroke="#111" stroke-width="9" stroke-linecap="round"/>'
                                    f'<path d="M30 90 Q50 30 70 64 T104 40" fill="none" stroke="#e0413a" stroke-width="3" stroke-linecap="round"/><circle cx="104" cy="40" r="7" fill="#e0413a"/>', "#4a3f55"),
    "ic_guard": icon(f'<path d="M64 20 L100 34 Q100 86 64 108 Q28 86 28 34 Z" fill="#d6e4ff" {ISW}/><path d="M64 34 L64 94" stroke="#7a95c8" stroke-width="6"/>', "#5b7bd6"),
    "ic_special_sorcerer": icon(f'<path d="M52 24 H76 V52 H104 V76 H76 V104 H52 V76 H24 V52 H52 Z" fill="#c9ffd9" {ISW}/>', "#38b46a"),
    "ic_special_curse_user": icon(f'<path d="M64 22 Q96 64 92 82 Q88 106 64 106 Q40 106 36 82 Q32 64 64 22 Z" fill="#e0413a" {ISW}/><path d="M50 78 Q56 92 70 92" stroke="#fff" stroke-width="5" fill="none" opacity=".7"/>', "#5a2333"),
    "ic_domain": icon(f'<circle cx="64" cy="64" r="40" fill="none" stroke="#fff" stroke-width="5"/><circle cx="64" cy="64" r="28" fill="none" stroke="#ffd84a" stroke-width="4" stroke-dasharray="6 6"/>'
                      f'<path d="M30 64 Q64 30 98 64 Q64 98 30 64 Z" fill="#fff" {ISW}/><circle cx="64" cy="64" r="12" fill="#5b2a86"/><circle cx="60" cy="60" r="4" fill="#fff"/>', "#5b2a86"),
    "ic_heal_charm": icon(f'<rect x="40" y="16" width="48" height="96" rx="6" fill="#fff3d6" {ISW}/><path d="M52 34 H76 M64 34 V90 M50 56 Q64 66 78 56 M52 78 H76" stroke="#d33" stroke-width="5" fill="none"/>'),
    "ic_ce_charm": icon(f'<rect x="40" y="16" width="48" height="96" rx="6" fill="#e2f0ff" {ISW}/><path d="M52 34 H76 M64 34 V90 M50 56 Q64 66 78 56 M52 78 H76" stroke="#2f6fe0" stroke-width="5" fill="none"/>'),
    "ic_hp": icon(f'<path d="M64 108 Q16 72 22 44 Q30 18 64 38 Q98 18 106 44 Q112 72 64 108 Z" fill="#ff5a6e" {ISW}/><circle cx="46" cy="48" r="8" fill="#fff" opacity=".7"/>'),
    "ic_ce": icon(f'<path d="M64 112 Q24 104 30 70 Q34 48 52 32 Q50 52 62 58 Q60 30 80 14 Q78 40 96 56 Q108 72 100 92 Q92 110 64 112 Z" fill="#4fb6ff" {ISW}/><path d="M64 100 Q48 94 52 78 Q60 84 66 74 Q80 86 72 98 Z" fill="#dff4ff"/>'),
    "ic_taint": icon(f'<path d="M64 14 Q100 62 100 82 Q100 112 64 112 Q28 112 28 82 Q28 62 64 14 Z" fill="#8e44ad" {ISW}/><circle cx="52" cy="80" r="8" fill="#e8c9ff"/>'),
    "ic_lock": icon(f'<rect x="30" y="56" width="68" height="54" rx="10" fill="#c9c9d6" {ISW}/><path d="M44 56 V42 Q44 20 64 20 Q84 20 84 42 V56" fill="none" {ISW}/><circle cx="64" cy="82" r="8" fill="{INK}"/>'),
    "ic_star": icon(f'<path d="M64 14 L78 48 L114 50 L86 72 L96 108 L64 88 L32 108 L42 72 L14 50 L50 48 Z" fill="#ffd84a" {ISW}/>'),
    "ic_scroll": icon(f'<rect x="26" y="26" width="76" height="76" rx="8" fill="#f6e9c9" {ISW}/><path d="M40 48 H88 M40 64 H88 M40 80 H72" stroke="#8a6a3a" stroke-width="5"/>'),
    "ic_map": icon(f'<path d="M18 30 L48 18 L80 30 L110 18 L110 98 L80 110 L48 98 L18 110 Z" fill="#cfe8c4" {ISW}/><path d="M48 18 V98 M80 30 V110" {ISW}/><path d="M30 80 Q50 60 64 70 T98 46" stroke="#d33" stroke-width="5" fill="none" stroke-dasharray="6 6"/>'),
    "ic_home": icon(f'<path d="M18 62 L64 22 L110 62 L98 62 L98 106 L30 106 L30 62 Z" fill="#ffe1a8" {ISW}/><rect x="54" y="76" width="20" height="30" fill="#b06a3a" {ISW}/>'),
    "ic_person": icon(f'<circle cx="64" cy="44" r="24" fill="#ffd2b0" {ISW}/><path d="M22 110 Q22 74 64 74 Q106 74 106 110 Z" fill="#7a95c8" {ISW}/>'),
    "ic_gear": icon(f'<circle cx="64" cy="64" r="34" fill="#c9c9d6" stroke="{INK}" stroke-width="16" stroke-dasharray="14 9"/><circle cx="64" cy="64" r="30" fill="#c9c9d6" {ISW}/><circle cx="64" cy="64" r="12" fill="#fff" {ISW}/>'),
    # 적의 의도(행동 예고) 아이콘
    "it_strike": icon(f'<path d="M30 98 L86 30 L98 42 L42 110 Z" fill="#e6e6ee" {ISW}/><path d="M26 86 L52 112" {ISW}/>', "#ffffff"),
    "it_heavy": icon(f'<path d="M64 10 L76 46 L114 40 L86 66 L108 98 L70 86 L60 120 L50 86 L14 98 L38 66 L12 40 L52 46 Z" fill="#ffd84a" {ISW}/><text x="64" y="80" text-anchor="middle" font-size="44" font-weight="900" fill="#c22" font-family="sans-serif">!</text>', "#ff6b6b"),
    "it_drain": icon(f'<path d="M64 24 A40 40 0 1 1 30 84" fill="none" stroke="#4fb6ff" stroke-width="9" stroke-linecap="round"/><path d="M18 74 L30 92 L44 76" fill="none" stroke="#4fb6ff" stroke-width="9" stroke-linecap="round"/>', "#ffffff"),
    "it_barrier": icon(f'<path d="M64 18 L100 32 Q100 86 64 108 Q28 86 28 32 Z" fill="#bfc9d9" {ISW}/>', "#ffffff"),
    "it_hex": icon(f'<circle cx="64" cy="56" r="32" fill="#e8c9ff" {ISW}/><circle cx="52" cy="54" r="8" fill="{INK}"/><circle cx="76" cy="54" r="8" fill="{INK}"/><path d="M48 92 V104 M64 92 V108 M80 92 V104" {ISW}/>', "#ffffff"),
    "it_bound": icon(f'<path d="M20 40 Q64 80 108 40 M20 64 Q64 104 108 64 M20 88 Q64 128 108 88" fill="none" stroke="#111" stroke-width="8"/>', "#ffffff"),
}


def main():
    chars = {
        "hero_sorcerer": hero_sorcerer(),
        "hero_curse_user": hero_curse(),
        "npc_doyunjae": npc_doyunjae(),
        "npc_maehwa": npc_maehwa(),
        "npc_hanseorim": npc_hanseorim(),
        "enemy_goharam": villain_goharam(),
        "enemy_binjari": enemy_binjari(),
        "enemy_conductor": enemy_conductor(),
        "enemy_saint": enemy_saint(),
        "enemy_manghyang": enemy_manghyang(),
        "enemy_mumyeong": boss_mumyeong(),
    }
    for name, s in chars.items():
        render(name, s, 600)
    # 대화용 초상화: 전신에서 머리 부분만 잘라낸다
    for src, dst in (("hero_sorcerer", "pt_hero_sorcerer"), ("hero_curse_user", "pt_hero_curse_user"),
                     ("npc_doyunjae", "pt_doyunjae"), ("npc_maehwa", "pt_maehwa"), ("npc_hanseorim", "pt_hanseorim"),
                     ("enemy_goharam", "pt_goharam"), ("enemy_mumyeong", "pt_mumyeong"), ("enemy_binjari", "pt_binjari"),
                     ("enemy_conductor", "pt_conductor"), ("enemy_saint", "pt_saint"), ("enemy_manghyang", "pt_manghyang")):
        crop_portrait(src, dst)
    bgs = {
        "bg_title": bg_title(), "bg_lobby": bg_lobby(), "bg_stage0": bg_school(), "bg_stage1": bg_subway(),
        "bg_stage2": bg_apartment(), "bg_stage3": bg_tower(), "bg_stage4": bg_river(), "bg_stage5": bg_gate(),
    }
    for name, s in bgs.items():
        render(name, s, 1080, trim=False, fmt="jpg")
    for name, s in ICONS.items():
        render(name, s, 192)
    total = sum(os.path.getsize(os.path.join(OUT, f)) for f in os.listdir(OUT))
    print(f"{len(os.listdir(OUT))} files, {total/1024:.0f} KB → {OUT}")


if __name__ == "__main__":
    main()
