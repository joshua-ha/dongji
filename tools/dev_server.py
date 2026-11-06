#!/usr/bin/env python3
"""개발용 서버: web/ 폴더를 서빙하고, /api 요청을 Rust 엔진(libdongji.so)에 ctypes로 전달한다.
iOS 앱의 Swift 브리지와 똑같은 C 함수(dongji_*)를 쓰므로, 여기서 돌아가면 iOS에서도 같은 결과가 나온다.

  cargo build --release && python3 tools/dev_server.py [--port 8000] [--seed 42]
브라우저에서 http://localhost:8000 (개발자 도구의 iPhone 모드 추천)
"""
import ctypes, json, os, sys, random, argparse
from http.server import ThreadingHTTPServer, SimpleHTTPRequestHandler

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
WEB = os.path.join(ROOT, "web")
SAVE = os.environ.get("DONGJI_SAVE", os.path.join(ROOT, "dev_save.txt"))
ext = "dylib" if sys.platform == "darwin" else "so"
lib = ctypes.CDLL(os.path.join(ROOT, "target", "release", f"libdongji.{ext}"))
lib.dongji_new.restype = ctypes.c_void_p
lib.dongji_new.argtypes = [ctypes.c_uint64]
lib.dongji_free.argtypes = [ctypes.c_void_p]
for fn in ("dongji_start", "dongji_choose", "dongji_input_text"):
    getattr(lib, fn).restype = ctypes.c_void_p
lib.dongji_start.argtypes = [ctypes.c_void_p]
lib.dongji_choose.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
lib.dongji_input_text.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.dongji_attach_save.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
lib.dongji_attach_save.restype = ctypes.c_int32
lib.dongji_string_free.argtypes = [ctypes.c_void_p]

STATE = {"game": None, "seed": None}


def take(ptr):
    s = ctypes.cast(ptr, ctypes.c_char_p).value.decode("utf-8")
    lib.dongji_string_free(ptr)
    return s


class H(SimpleHTTPRequestHandler):
    def __init__(self, *a, **k):
        super().__init__(*a, directory=WEB, **k)

    def log_message(self, *a):
        pass

    def end_headers(self):
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def do_POST(self):
        if not self.path.endswith("/api"):
            return self.send_error(404)
        msg = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
        cmd = msg.get("cmd")
        out = "{}"
        if cmd == "boot":
            if STATE["game"]:
                lib.dongji_free(STATE["game"])
            seed = STATE["seed"] or random.getrandbits(63)
            g = lib.dongji_new(seed)
            STATE["game"] = g
            if os.path.exists(SAVE):
                lib.dongji_attach_save(g, open(SAVE, encoding="utf-8").read().encode())
            out = take(lib.dongji_start(g))
        elif cmd == "choose":
            out = take(lib.dongji_choose(STATE["game"], int(msg["index"])))
        elif cmd == "text":
            out = take(lib.dongji_input_text(STATE["game"], msg.get("text", "").encode()))
        elif cmd == "save":
            with open(SAVE, "w", encoding="utf-8") as f:
                f.write(msg.get("data", ""))
        body = out.encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=8000)
    ap.add_argument("--seed", type=int)
    ap.add_argument("--fresh", action="store_true", help="세이브 파일 삭제 후 시작")
    a = ap.parse_args()
    STATE["seed"] = a.seed
    if a.fresh and os.path.exists(SAVE):
        os.remove(SAVE)
    print(f"http://localhost:{a.port}")
    ThreadingHTTPServer(("0.0.0.0", a.port), H).serve_forever()
