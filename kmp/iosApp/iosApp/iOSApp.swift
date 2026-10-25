import SwiftUI
import Shared
import GoogleSignIn

struct ComposeView: UIViewControllerRepresentable {
    func makeUIViewController(context: Context) -> UIViewController {
        IosGoogleRegistry.shared.provider = GoogleProvider()
        return MainViewControllerKt.MainViewController()
    }
    func updateUIViewController(_ uiViewController: UIViewController, context: Context) {}
}

@main
struct iOSApp: App {
    var body: some Scene { WindowGroup {
            ComposeView().ignoresSafeArea(.all).statusBarHidden(true)
                .onOpenURL { url in GIDSignIn.sharedInstance.handle(url) }
        } }
}
