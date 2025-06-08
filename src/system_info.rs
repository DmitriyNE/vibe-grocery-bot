use git_version::git_version;

// include -modified if the working tree has uncommitted changes
const COMMIT: &str = git_version!(args = ["--abbrev=10", "--always", "--dirty=-modified"]);

pub fn get_system_info() -> String {
    let profile = if cfg!(debug_assertions) {
        "Dev"
    } else {
        "Release"
    };

    let latest = option_env!("LATEST_TAG").unwrap_or("");
    let ahead = option_env!("COMMITS_AHEAD").unwrap_or("");
    let version = match option_env!("RELEASE_VERSION") {
        Some(tag) if !tag.is_empty() => format!("release {}", tag),
        _ if !latest.is_empty() && !ahead.is_empty() => {
            format!("development branch {} commits ahead of {}", ahead, latest)
        }
        _ if !latest.is_empty() => format!("development branch ahead of {}", latest),
        _ => "development".to_string(),
    };

    format!(
        "{} - {}\nCommit: {}\n{} build",
        env!("CARGO_PKG_NAME"),
        version,
        COMMIT,
        profile
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use git_version::git_version;

    #[test]
    fn test_get_system_info() {
        let expected = git_version!(args = ["--abbrev=10", "--always", "--dirty=-modified"]);
        let info = get_system_info();
        assert!(info.contains(expected));
        assert!(info.contains("Dev build") || info.contains("Release build"));
        assert!(info.contains("release") || info.contains("development"));
    }
}
