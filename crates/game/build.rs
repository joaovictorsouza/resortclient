//! Exposes the short git commit of the build as `GAME_GIT_COMMIT` (falls back to nothing when git
//! or the repository is not available, e.g. a source tarball).
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GAME_GIT_COMMIT");
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/refs");
    if std::env::var_os("GAME_GIT_COMMIT").is_some() {
        return; // provided by the caller (CI, Xcode phase)
    }
    let commit = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(commit) = commit {
        println!("cargo:rustc-env=GAME_GIT_COMMIT={commit}");
    }
}
