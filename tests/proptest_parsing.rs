use proptest::prelude::*;
use shopbot::{parse_item_line, parse_items};

// Property: parse_item_line should never panic for arbitrary input
proptest! {
    #[test]
    fn prop_parse_item_line_no_panic(s in "(?s).*") {
        let _ = parse_item_line(&s);
    }
}

fn joined_items_strategy() -> impl Strategy<Value = (Vec<String>, String)> {
    prop::collection::vec("[a-zA-Z0-9]+", 1..6).prop_flat_map(|items| {
        let len = items.len();
        prop::collection::vec(proptest::sample::select(vec![", ", "\n", " and "]), len - 1)
            .prop_map(move |seps| {
                let mut text = String::new();
                text.push_str(&items[0]);
                for (sep, item) in seps.into_iter().zip(items.iter().skip(1)) {
                    text.push_str(sep);
                    text.push_str(item);
                }
                (items.clone(), text)
            })
    })
}

proptest! {
    #[test]
    fn prop_parse_items_separators((expected, text) in joined_items_strategy()) {
        let parsed = parse_items(&text);
        prop_assert_eq!(parsed, expected);
    }
}
