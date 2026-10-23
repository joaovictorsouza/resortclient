import GoogleSignIn
import UIKit

// Google sign-in bridge. It owns no UIApplicationDelegate/UISceneDelegate: whoever owns them (the
// bevy_ios_app_delegate crate, see ios_google.rs) forwards the redirect URL to `resort_google_handle_url`.

/// Called from Rust (`ios_google.rs`) to start the native Google sign-in. The client ids come from
/// Info.plist (GIDClientID / GIDServerClientID), which GoogleSignIn reads on its own.
@_cdecl("resort_google_sign_in_start")
public func resortGoogleSignInStart() {
    DispatchQueue.main.async {
        guard let presenter = presentingViewController() else {
            report(status: 2, value: "no view controller to present the Google sign-in")
            return
        }
        GIDSignIn.sharedInstance.signIn(withPresenting: presenter) { result, error in
            if let error = error as NSError? {
                let cancelled = error.domain == kGIDSignInErrorDomain
                    && error.code == GIDSignInError.canceled.rawValue
                report(status: cancelled ? 1 : 2, value: "\(error.domain) \(error.code): \(error.localizedDescription)")
            } else if let token = result?.user.idToken?.tokenString {
                report(status: 0, value: token)
            } else {
                report(status: 2, value: "Google sign-in returned no id_token")
            }
        }
    }
}

/// Forwards a redirect URL to GoogleSignIn. Call it from `application(_:open:options:)` and
/// `scene(_:openURLContexts:)`; returns true when the URL belonged to the Google flow.
@_cdecl("resort_google_handle_url")
public func resortGoogleHandleURL(_ url: UnsafePointer<CChar>?) -> Bool {
    guard let url, let parsed = URL(string: String(cString: url)) else { return false }
    return GIDSignIn.sharedInstance.handle(parsed)
}

/// Same hook for Objective-C/Swift delegates.
@objc(ResortGoogleSignIn)
public final class ResortGoogleSignIn: NSObject {
    @objc public static func handle(_ url: URL) -> Bool {
        GIDSignIn.sharedInstance.handle(url)
    }
}

private func report(status: Int32, value: String) {
    value.withCString { resort_google_sign_in_result(status, $0) }
}

/// Top view controller of the foreground-active window scene (above winit's window), falling back to
/// any connected scene.
private func presentingViewController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    let scene = scenes.first(where: { $0.activationState == .foregroundActive })
        ?? scenes.first(where: { $0.activationState == .foregroundInactive })
        ?? scenes.first
    let window = scene?.windows.first(where: \.isKeyWindow) ?? scene?.windows.first
    var top = window?.rootViewController
    while let presented = top?.presentedViewController {
        top = presented
    }
    return top
}
