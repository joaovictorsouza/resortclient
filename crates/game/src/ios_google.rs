//! iOS side of the Google sign-in bridge. The Swift code in `ios/Sources/GoogleSignInBridge.swift`
//! runs the GoogleSignIn flow (`resort_google_sign_in_start`) and reports back through
//! [`resort_google_sign_in_result`].

use bevy::prelude::On;
use std::ffi::{CStr, c_char, c_int};
use std::sync::Mutex;
use std::sync::mpsc::Sender;

use crate::google_signin::{GoogleSignInBridge, GoogleSignInError, GoogleSignInResult};

unsafe extern "C" {
    /// Implemented in Swift (`@_cdecl`); presents the Google sign-in sheet.
    fn resort_google_sign_in_start();
    /// Implemented in Swift; forwards a redirect URL to GoogleSignIn, true when it was theirs.
    fn resort_google_handle_url(url: *const c_char) -> bool;
}

/// Forwards a URL received by the app/scene delegate (`ios_scene`) to the Google sign-in.
/// Hands redirect URLs received by the scene/app delegate to GoogleSignIn.
pub(crate) fn forward_open_url(call: On<bevy_ios_app_delegate::AppDelegateCall>) {
    if let bevy_ios_app_delegate::AppDelegateCall::OpenURL(url) = call.event() {
        handle_url(url);
    }
}

pub(crate) fn handle_url(url: &str) -> bool {
    let Ok(url) = std::ffi::CString::new(url) else {
        return false;
    };
    // SAFETY: `url` is a valid NUL-terminated string that outlives the call.
    unsafe { resort_google_handle_url(url.as_ptr()) }
}

const STATUS_TOKEN: c_int = 0;
const STATUS_CANCELLED: c_int = 1;
/// Sent by the weak stub of builds without GoogleSignIn: the game falls back to the device login.
const STATUS_UNAVAILABLE: c_int = 3;

/// Channel of the sign-in currently in flight (the Swift side answers asynchronously).
static PENDING: Mutex<Option<Sender<GoogleSignInResult>>> = Mutex::new(None);

pub struct IosGoogleSignIn;

impl GoogleSignInBridge for IosGoogleSignIn {
    fn request_id_token(&self, _server_client_id: &str, reply: Sender<GoogleSignInResult>) {
        // The server client id is configured in Info.plist (GIDServerClientID).
        if let Ok(mut pending) = PENDING.lock() {
            *pending = Some(reply);
        }
        // SAFETY: the symbol is defined by the app target's Swift code and takes no arguments.
        unsafe { resort_google_sign_in_start() };
    }
}

fn map_result(status: c_int, value: String) -> GoogleSignInResult {
    match status {
        STATUS_TOKEN => Ok(value),
        STATUS_CANCELLED => Err(GoogleSignInError::Cancelled),
        STATUS_UNAVAILABLE => Err(GoogleSignInError::Unavailable),
        _ => Err(GoogleSignInError::Failed(value)),
    }
}

/// Reports the outcome of the Swift sign-in; `value` is the id_token or an error message.
///
/// # Safety
/// `value` must be null or point to a NUL-terminated string valid for the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn resort_google_sign_in_result(status: c_int, value: *const c_char) {
    let value = if value.is_null() {
        String::new()
    } else {
        // SAFETY: guaranteed by the caller, see above.
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned()
    };
    if let Ok(mut pending) = PENDING.lock()
        && let Some(reply) = pending.take()
    {
        let _ = reply.send(map_result(status, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_result() {
        assert_eq!(map_result(0, "tok".into()), Ok("tok".to_string()));
        assert_eq!(map_result(1, "x".into()), Err(GoogleSignInError::Cancelled));
        assert_eq!(
            map_result(2, "boom".into()),
            Err(GoogleSignInError::Failed("boom".into()))
        );
        assert_eq!(
            map_result(3, String::new()),
            Err(GoogleSignInError::Unavailable)
        );
    }
}
