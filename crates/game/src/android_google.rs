//! Android side of the Google sign-in bridge: asks the Java `GoogleSignIn` class (Credential Manager)
//! for an id_token over JNI and forwards its answer to the waiting [`GoogleSignInResult`] channel.

use std::sync::Mutex;
use std::sync::mpsc::Sender;

use jni::objects::{JClass, JObject, JString, JValue};
use jni::{JNIEnv, JavaVM, sys::jint};

use crate::google_signin::{GoogleSignInBridge, GoogleSignInError, GoogleSignInResult};

const GOOGLE_SIGN_IN_CLASS: &str = "br.eng.joaovictor.resort.GoogleSignIn";

const STATUS_TOKEN: jint = 0;
const STATUS_CANCELLED: jint = 1;

/// Channel of the sign-in currently in flight (the Java side answers asynchronously).
static PENDING: Mutex<Option<Sender<GoogleSignInResult>>> = Mutex::new(None);

pub struct AndroidGoogleSignIn;

impl GoogleSignInBridge for AndroidGoogleSignIn {
    fn request_id_token(&self, server_client_id: &str, reply: Sender<GoogleSignInResult>) {
        if let Ok(mut pending) = PENDING.lock() {
            *pending = Some(reply.clone());
        }
        if let Err(message) = start_java_sign_in(server_client_id) {
            deliver(Err(GoogleSignInError::Failed(message)));
        }
    }
}

fn deliver(result: GoogleSignInResult) {
    if let Ok(mut pending) = PENDING.lock()
        && let Some(reply) = pending.take()
    {
        let _ = reply.send(result);
    }
}

fn start_java_sign_in(server_client_id: &str) -> Result<(), String> {
    let app = bevy::android::ANDROID_APP
        .get()
        .ok_or("android app not ready")?;
    // SAFETY: the pointers come from the live AndroidApp, valid for the whole process lifetime.
    let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    // SAFETY: as above; the activity global reference is owned by the AndroidApp, never freed here.
    let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };

    let result = call_start(&mut env, &activity, server_client_id);
    if result.is_err() {
        let _ = env.exception_clear();
    }
    result.map_err(|e| e.to_string())
}

fn call_start(
    env: &mut JNIEnv,
    activity: &JObject,
    server_client_id: &str,
) -> jni::errors::Result<()> {
    // Native threads only see system classes, so load ours through the activity's class loader.
    let loader = env
        .call_method(activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])?
        .l()?;
    let name = env.new_string(GOOGLE_SIGN_IN_CLASS)?;
    let class: JClass = env
        .call_method(
            &loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&name)],
        )?
        .l()?
        .into();
    let id = env.new_string(server_client_id)?;
    env.call_static_method(
        &class,
        "start",
        "(Landroid/app/Activity;Ljava/lang/String;)V",
        &[JValue::Object(activity), JValue::Object(&id)],
    )?;
    Ok(())
}

/// Maps the Java callback (`status`, `value`) to the result delivered to the game.
fn map_result(status: jint, value: String) -> GoogleSignInResult {
    match status {
        STATUS_TOKEN => Ok(value),
        STATUS_CANCELLED => Err(GoogleSignInError::Cancelled),
        _ => Err(GoogleSignInError::Failed(value)),
    }
}

/// Called by `GoogleSignIn.nativeOnResult` on the Android main thread.
#[unsafe(no_mangle)]
pub extern "system" fn Java_br_eng_joaovictor_resort_GoogleSignIn_nativeOnResult(
    mut env: JNIEnv,
    _class: JClass,
    status: jint,
    value: JString,
) {
    let value = env.get_string(&value).map(String::from).unwrap_or_default();
    deliver(map_result(status, value));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_result() {
        assert_eq!(map_result(0, "tok".into()), Ok("tok".to_string()));
        assert_eq!(
            map_result(1, "cancelled".into()),
            Err(GoogleSignInError::Cancelled)
        );
        assert_eq!(
            map_result(2, "boom".into()),
            Err(GoogleSignInError::Failed("boom".into()))
        );
    }
}
