import GoogleSignIn
import Shared
import UIKit

/// Google sign-in for the shell: GoogleSignIn-iOS reads GIDClientID / GIDServerClientID from
/// Info.plist; the ID token it returns carries the server client id as audience (what Nakama needs).
final class GoogleProvider: NSObject, GoogleSignInProvider {
    func signIn(serverClientId: String, callback: @escaping (String?, String?) -> Void) {
        DispatchQueue.main.async {
            guard let presenter = Self.presentingViewController() else {
                callback(nil, "no view controller to present the Google sign-in")
                return
            }
            GIDSignIn.sharedInstance.signIn(withPresenting: presenter) { result, error in
                if let error = error as NSError? {
                    let cancelled = error.domain == kGIDSignInErrorDomain
                        && error.code == GIDSignInError.canceled.rawValue
                    callback(nil, cancelled ? "cancelled" : "\(error.domain) \(error.code)")
                } else if let token = result?.user.idToken?.tokenString {
                    callback(token, nil)
                } else {
                    callback(nil, "Google sign-in returned no id_token")
                }
            }
        }
    }

    private static func presentingViewController() -> UIViewController? {
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        let scene = scenes.first(where: { $0.activationState == .foregroundActive })
            ?? scenes.first(where: { $0.activationState == .foregroundInactive })
            ?? scenes.first
        let window = scene?.windows.first(where: \.isKeyWindow) ?? scene?.windows.first
        var top = window?.rootViewController
        while let presented = top?.presentedViewController { top = presented }
        return top
    }
}
