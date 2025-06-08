use shopbot::normalize_for_match;

#[test]
fn test_normalize_for_match() {
    assert_eq!(normalize_for_match("1 hammer"), "hammer");
    assert_eq!(normalize_for_match("   42  nails"), "nails");
    assert_eq!(normalize_for_match("Hammer"), "hammer");
}
