use shopbot::{format_delete_list, format_list, format_plain_list, Item};

fn sample_items() -> Vec<Item> {
    vec![
        Item {
            id: 1,
            text: "Apples".to_string(),
            done: false,
        },
        Item {
            id: 2,
            text: "Milk".to_string(),
            done: true,
        },
    ]
}

fn all_done_items() -> Vec<Item> {
    vec![
        Item {
            id: 1,
            text: "Apples".to_string(),
            done: true,
        },
        Item {
            id: 2,
            text: "Milk".to_string(),
            done: true,
        },
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
    use std::collections::HashSet;

    let items = sample_items();
    let mut selected = HashSet::new();
    selected.insert(1);
    let (text, keyboard) = format_delete_list(&items, &selected);

    assert_eq!(text, "Select items to delete, then tap 'Done Deleting'.");

    let labels: Vec<&str> = keyboard
        .inline_keyboard
        .iter()
        .map(|row| row[0].text.as_str())
        .collect();
    assert_eq!(labels, vec!["☑️ Apples", "❌ Milk", "✅ Done Deleting"]);
}

#[test]
fn test_format_plain_list() {
    let items = sample_items();
    let text = format_plain_list(&items);
    assert_eq!(text, "• Apples\n• Milk\n");
}

#[test]
fn test_format_list_all_done() {
    let items = all_done_items();
    let (text, _keyboard) = format_list(&items);
    assert!(text.ends_with("✅ All items checked off.\n"));
}
