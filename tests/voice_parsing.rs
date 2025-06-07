use shopbot::parse_voice_items;

#[test]
fn test_parse_voice_items() {
    let items = parse_voice_items("milk, eggs and bread");
    assert_eq!(items, vec!["milk", "eggs", "bread"]);

    let single = parse_voice_items("just apples");
    assert_eq!(single, vec!["just apples"]);
}
