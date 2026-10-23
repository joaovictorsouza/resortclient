#import <Foundation/Foundation.h>

// Weak fallbacks for the Google sign-in bridge. The Xcode project replaces them with the Swift
// implementation (GoogleSignInBridge.swift); builds without GoogleSignIn (scripts/build-ios-device.sh)
// keep these, so the Rust staticlib always links and the login falls back to the device login.

extern void resort_google_sign_in_result(int status, const char *value);

__attribute__((weak)) void resort_google_sign_in_start(void) {
    resort_google_sign_in_result(3, "Google sign-in is not available in this build"); // 3 = unavailable (see ios_google.rs)
}

__attribute__((weak)) bool resort_google_handle_url(const char *url) {
    return false;
}
