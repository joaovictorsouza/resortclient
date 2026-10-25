# JNI: the Rust library looks these classes and methods up by name.
-keep class br.eng.joaovictor.bevykmp.NativeBridge { *; }
-keepclasseswithmembernames class * { native <methods>; }
# UniFFI + JNA: the generated bindings are called reflectively (Gobley also adds its own rules).
-keep class br.eng.joaovictor.resort.ffi.** { *; }
-keep class com.sun.jna.** { *; }
-dontwarn java.awt.*
# Credential Manager / Google ID
-keep class com.google.android.libraries.identity.googleid.** { *; }
