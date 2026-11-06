import Foundation
import WebKit

/// JS ⇄ Rust 브리지.
/// JS: webkit.messageHandlers.dongji.postMessage({cmd, id, ...})
/// Swift: Rust FFI(dongji_*) 호출 → evaluateJavaScript("Bridge._reply(id, <Turn JSON>)")
final class EngineBridge: NSObject, WKScriptMessageHandler {
    weak var webView: WKWebView?
    private var game: OpaquePointer?
    private let saveKey = "dongji.save.v1"

    deinit {
        if let g = game { dongji_free(g) }
    }

    func userContentController(_ userContentController: WKUserContentController,
                               didReceive message: WKScriptMessage) {
        guard let body = message.body as? [String: Any],
              let cmd = body["cmd"] as? String else { return }
        let id = (body["id"] as? NSNumber)?.intValue ?? 0
        var out: UnsafeMutablePointer<CChar>? = nil

        switch cmd {
        case "boot":
            if let g = game { dongji_free(g) }
            game = dongji_new(UInt64.random(in: 1...UInt64.max))
            // 저장된 체크포인트가 있으면 타이틀에 '이어하기'가 생긴다
            if let saved = UserDefaults.standard.string(forKey: saveKey) {
                _ = saved.withCString { dongji_attach_save(game, $0) }
            }
            out = dongji_start(game)
        case "choose":
            let index = (body["index"] as? NSNumber)?.uint32Value ?? 0
            out = dongji_choose(game, index)
        case "text":
            let text = body["text"] as? String ?? ""
            out = text.withCString { dongji_input_text(game, $0) }
        case "save":
            if let data = body["data"] as? String {
                UserDefaults.standard.set(data, forKey: saveKey)
            }
            return
        default:
            return
        }

        guard let ptr = out else { return }
        let json = String(cString: ptr)
        dongji_string_free(ptr)
        // Turn JSON은 그대로 JS 객체 리터럴로 쓸 수 있다
        webView?.evaluateJavaScript("Bridge._reply(\(id), \(json));", completionHandler: nil)
    }
}
