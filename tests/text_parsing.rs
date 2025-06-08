use shopbot::parse_item_line;

#[test]
fn test_parse_item_line() {
    // Normal line
    assert_eq!(parse_item_line("Milk"), Some("Milk".to_string()));
    // Leading emoji
    assert_eq!(parse_item_line("☑️ Apples"), Some("Apples".to_string()));
    assert_eq!(parse_item_line("⬜Bread"), Some("Bread".to_string()));
    // Extra spaces
    assert_eq!(parse_item_line("  Carrots  "), Some("Carrots".to_string()));
    // Archived marker
    assert_eq!(parse_item_line("--- Archived List ---"), None);
    // Empty line when only an emoji and spaces
    assert_eq!(parse_item_line("☑️   "), None);
    // Bullet prefix
    assert_eq!(parse_item_line("• Milk"), Some("Milk".to_string()));
}
