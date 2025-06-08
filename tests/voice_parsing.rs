use shopbot::parse_items;

#[test]
fn test_parse_items() {
    let items = parse_items("milk, eggs and bread");
    assert_eq!(items, vec!["milk", "eggs", "bread"]);

    let single = parse_items("just apples");
    assert_eq!(single, vec!["just apples"]);

    let numbers = parse_items("one milk, 2 eggs");
    assert_eq!(numbers, vec!["one milk", "2 eggs"]);
}
