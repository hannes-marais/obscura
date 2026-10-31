use std::env;
use std::path::Path;
use std::process::Command;

fn env_value(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_version(version: String) -> String {
    version.strip_prefix('v').unwrap_or(&version).to_string()
}

fn github_tag_version() -> Option<String> {
    if env_value("GITHUB_REF_TYPE").as_deref() == Some("tag") {
        return env_value("GITHUB_REF_NAME").map(normalize_version);
    }

    env_value("GITHUB_REF")
        .and_then(|value| value.strip_prefix("refs/tags/").map(str::to_owned))
        .map(normalize_version)
}

/// Local builds are not cut from a tag, so the bare workspace version cannot
/// tell two checkouts apart. Append the commit when git is there to answer;
/// tarball and minimal-image builds without git keep the bare version.
fn local_version() -> String {
    let package = env::var("CARGO_PKG_VERSION").expect("Cargo sets CARGO_PKG_VERSION");
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|sha| sha.trim().to_string())
        .filter(|sha| !sha.is_empty());
    match sha {
        Some(sha) => format!("{package}-dev+{sha}"),
        None => package,
    }
}

fn watch_git_revision() {
    let git_path = |name: &str| {
        Command::new("git").args(["rev-parse", "--git-path", name]).output().ok()
            .filter(|out| out.status.success())
            .and_then(|out| String::from_utf8(out.stdout).ok())
            .map(|path| path.trim().to_owned())
            .filter(|path| !path.is_empty())
    };
    let symbolic_ref = Command::new("git").args(["symbolic-ref", "-q", "HEAD"]).output().ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty());
    for name in [Some("HEAD"), symbolic_ref.as_deref(), Some("packed-refs")].into_iter().flatten() {
        if let Some(path) = git_path(name).filter(|path| Path::new(path).exists()) {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=OBSCURA_VERSION");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_TYPE");
    println!("cargo:rerun-if-env-changed=GITHUB_REF_NAME");
    println!("cargo:rerun-if-env-changed=GITHUB_REF");
    watch_git_revision();

    let version = env_value("OBSCURA_VERSION")
        .map(normalize_version)
        .or_else(github_tag_version)
        .unwrap_or_else(local_version);

    println!("cargo:rustc-env=OBSCURA_BUILD_VERSION={version}");
}
