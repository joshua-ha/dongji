import SwiftUI

@main
struct DongjiApp: App {
    var body: some Scene {
        WindowGroup {
            GameWebView()
                .ignoresSafeArea()
                .statusBarHidden(true)
                .background(Color.black)
        }
    }
}
