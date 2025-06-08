use std::process::Command;

fn main() {
    // Expose HEAD's tag if it's a release commit.
    let tag = Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    println!("cargo:rustc-env=RELEASE_VERSION={}", tag);

    // Always expose the latest release tag for dev builds.
    let latest = Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    println!("cargo:rustc-env=LATEST_TAG={}", latest);

    // Count commits since the latest tag so we can display how far ahead we are.
    let ahead = if latest.is_empty() {
        String::new()
    } else {
        Command::new("git")
            .args(["rev-list", "--count", &format!("{}..HEAD", latest)])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };

    println!("cargo:rustc-env=COMMITS_AHEAD={}", ahead);
}
