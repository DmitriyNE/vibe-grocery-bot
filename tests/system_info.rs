use git_version::git_version;
use shopbot::get_system_info;

#[test]
fn test_system_info_contains_commit_and_profile() {
    let expected = git_version!(args = ["--abbrev=10", "--always", "--dirty=-modified"]);
    let info = get_system_info();
    assert!(info.contains(expected));
    assert!(info.contains("Dev build") || info.contains("Release build"));
    assert!(info.contains("release") || info.contains("commits ahead of"));
}
