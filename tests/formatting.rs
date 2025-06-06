use shopbot::{format_list, format_delete_list, Item};

fn sample_items() -> Vec<Item> {
    vec![
        Item { id: 1, text: "Apples".to_string(), done: false },
        Item { id: 2, text: "Milk".to_string(), done: true },
    ]
}

#[test]
fn test_format_list() {
    let items = sample_items();
    let (text, keyboard) = format_list(&items);

    assert_eq!(text, "🛒 Apples\n✅ Milk\n");

    let labels: Vec<&str> = keyboard
        .inline_keyboard
        .iter()
        .map(|row| row[0].text.as_str())
        .collect();
    assert_eq!(labels, vec!["Apples", "✅ Milk"]);
}

#[test]
fn test_format_delete_list() {
    let items = sample_items();
    let (text, keyboard) = format_delete_list(&items);

    assert_eq!(text, "Tap an item to delete it. Tap 'Done' when finished.");

    let labels: Vec<&str> = keyboard
        .inline_keyboard
        .iter()
        .map(|row| row[0].text.as_str())
        .collect();
    assert_eq!(labels, vec!["❌ Apples", "❌ Milk", "✅ Done Deleting"]);
}
