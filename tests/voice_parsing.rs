use shopbot::parse_items;

#[test]
fn test_parse_items() {
    let items = parse_items("milk, eggs and bread");
    assert_eq!(items, vec!["milk", "eggs", "bread"]);

    let single = parse_items("just apples");
    assert_eq!(single, vec!["just apples"]);
}
