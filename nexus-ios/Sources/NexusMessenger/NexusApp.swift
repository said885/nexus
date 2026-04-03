import SwiftUI

@main
struct NexusApp: App {
    @StateObject private var appState = AppStateManager()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .onAppear {
                    appState.initialize()
                }
        }
    }

    init() {
        // Screenshot prevention is handled in SceneDelegate / WindowScene delegate
        // via UIApplication.userInterfaceLayoutDirectionDidChange or
        // by setting UITextField.isSecureTextEntry on the window in production builds.
        // UIWindow-level screenshot blocking is set up in AppStateManager.initialize().
    }
}
