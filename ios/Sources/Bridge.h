// Rust entry points (crates/game/src/ios_google.rs) used by the Swift Google sign-in bridge.
#ifndef Bridge_h
#define Bridge_h

// status: 0 = id_token in `value`, 1 = cancelled, 2 = failed (`value` is the message).
void resort_google_sign_in_result(int status, const char *value);

#endif
