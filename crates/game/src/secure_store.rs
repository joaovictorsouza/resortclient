//! Storage for the login's refresh token: the iOS Keychain or the Android Keystore, so the token
//! never sits in plain text in `profile.json`. Desktop (development only) has no secure store: the
//! callers keep the token in the profile file there.

/// Name under which the refresh token of `profile_name` is stored.
pub fn refresh_token_key(profile_name: Option<&str>) -> String {
    match profile_name {
        Some(name) if !name.trim().is_empty() => format!("refresh_token_{}", name.trim()),
        _ => "refresh_token".to_string(),
    }
}

/// Whether this platform has a secure store; when false the token stays in the profile file.
pub const fn is_available() -> bool {
    cfg!(any(target_os = "ios", target_os = "android"))
}

/// Reads a secret; `None` when absent or unreadable.
pub fn load(key: &str) -> Option<String> {
    platform::load(key)
}

/// Stores a secret; `false` when the platform refused (the caller then keeps it in the profile).
pub fn save(key: &str, value: &str) -> bool {
    platform::save(key, value)
}

/// Forgets a secret.
pub fn delete(key: &str) {
    platform::delete(key)
}

#[cfg(all(target_os = "ios", feature = "winit-host", not(feature = "kmp-host")))]
mod platform {
    use security_framework::passwords::{
        delete_generic_password, get_generic_password, set_generic_password,
    };

    const SERVICE: &str = "br.eng.joaovictor.resort";

    pub fn load(key: &str) -> Option<String> {
        let bytes = get_generic_password(SERVICE, key).ok()?;
        String::from_utf8(bytes).ok()
    }

    pub fn save(key: &str, value: &str) -> bool {
        set_generic_password(SERVICE, key, value.as_bytes()).is_ok()
    }

    pub fn delete(key: &str) {
        let _ = delete_generic_password(SERVICE, key);
    }
}

#[cfg(all(
    target_os = "android",
    feature = "winit-host",
    not(feature = "kmp-host")
))]
mod platform {
    use jni::objects::{JClass, JObject, JString, JValue};
    use jni::{JNIEnv, JavaVM};

    const SECURE_STORE_CLASS: &str = "br.eng.joaovictor.resort.SecureStore";

    /// Runs `f` with a JNI env, the activity and the `SecureStore` class.
    fn with_store<T>(
        f: impl FnOnce(&mut JNIEnv, &JObject, &JClass) -> jni::errors::Result<T>,
    ) -> Option<T> {
        let app = bevy::android::ANDROID_APP.get()?;
        // SAFETY: the pointers come from the live AndroidApp, valid for the whole process lifetime.
        let vm = unsafe { JavaVM::from_raw(app.vm_as_ptr().cast()) }.ok()?;
        let mut env = vm.attach_current_thread().ok()?;
        // SAFETY: as above; the activity reference is owned by the AndroidApp, never freed here.
        let activity = unsafe { JObject::from_raw(app.activity_as_ptr().cast()) };
        let result = (|| {
            // Native threads only see system classes, so load ours through the activity's loader.
            let loader = env
                .call_method(
                    &activity,
                    "getClassLoader",
                    "()Ljava/lang/ClassLoader;",
                    &[],
                )?
                .l()?;
            let name = env.new_string(SECURE_STORE_CLASS)?;
            let class: JClass = env
                .call_method(
                    &loader,
                    "loadClass",
                    "(Ljava/lang/String;)Ljava/lang/Class;",
                    &[JValue::Object(&name)],
                )?
                .l()?
                .into();
            f(&mut env, &activity, &class)
        })();
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result.ok()
    }

    pub fn load(key: &str) -> Option<String> {
        with_store(|env, activity, class| {
            let key = env.new_string(key)?;
            let value = env
                .call_static_method(
                    class,
                    "get",
                    "(Landroid/app/Activity;Ljava/lang/String;)Ljava/lang/String;",
                    &[JValue::Object(activity), JValue::Object(&key)],
                )?
                .l()?;
            if value.is_null() {
                return Ok(None);
            }
            Ok(Some(env.get_string(&JString::from(value))?.into()))
        })
        .flatten()
    }

    pub fn save(key: &str, value: &str) -> bool {
        with_store(|env, activity, class| {
            let key = env.new_string(key)?;
            let value = env.new_string(value)?;
            env.call_static_method(
                class,
                "put",
                "(Landroid/app/Activity;Ljava/lang/String;Ljava/lang/String;)Z",
                &[
                    JValue::Object(activity),
                    JValue::Object(&key),
                    JValue::Object(&value),
                ],
            )?
            .z()
        })
        .unwrap_or(false)
    }

    pub fn delete(key: &str) {
        let _ = with_store(|env, activity, class| {
            let key = env.new_string(key)?;
            env.call_static_method(
                class,
                "delete",
                "(Landroid/app/Activity;Ljava/lang/String;)V",
                &[JValue::Object(activity), JValue::Object(&key)],
            )?;
            Ok(())
        });
    }
}

/// KMP host: the Kotlin shell owns the Keychain (iOS) and the Keystore (Android); the game asks
/// for it through blocking native services (see `bevy_kmp::request_service_blocking`).
#[cfg(all(any(target_os = "ios", target_os = "android"), feature = "kmp-host"))]
mod platform {
    use bevy_kmp_ffi::bevy_kmp::request_service_blocking;
    use std::time::Duration;

    const TIMEOUT: Duration = Duration::from_secs(5);

    pub fn load(key: &str) -> Option<String> {
        let r = request_service_blocking("secure_get", key.as_bytes().to_vec(), TIMEOUT).ok()?;
        if !r.ok {
            return None;
        }
        String::from_utf8(r.payload).ok()
    }

    pub fn save(key: &str, value: &str) -> bool {
        let payload = format!("{key}\n{value}").into_bytes();
        request_service_blocking("secure_put", payload, TIMEOUT).is_ok_and(|r| r.ok)
    }

    pub fn delete(key: &str) {
        let _ = request_service_blocking("secure_delete", key.as_bytes().to_vec(), TIMEOUT);
    }
}

#[cfg(not(any(
    all(target_os = "ios", any(feature = "winit-host", feature = "kmp-host")),
    all(
        target_os = "android",
        any(feature = "winit-host", feature = "kmp-host")
    )
)))]
mod platform {
    pub fn load(_key: &str) -> Option<String> {
        None
    }

    pub fn save(_key: &str, _value: &str) -> bool {
        false
    }

    pub fn delete(_key: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_depends_on_the_profile() {
        assert_eq!(refresh_token_key(None), "refresh_token");
        assert_eq!(refresh_token_key(Some("  ")), "refresh_token");
        assert_eq!(refresh_token_key(Some(" p1 ")), "refresh_token_p1");
    }

    #[test]
    fn desktop_has_no_secure_store() {
        if !is_available() {
            assert!(!save("k", "v"));
            assert_eq!(load("k"), None);
        }
    }
}
