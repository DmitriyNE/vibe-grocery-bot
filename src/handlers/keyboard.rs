use teloxide::types::InlineKeyboardButton;

pub fn build_item_buttons<T, F, G>(
    items: &[T],
    label: F,
    callback: G,
) -> Vec<Vec<InlineKeyboardButton>>
where
    F: Fn(&T) -> String,
    G: Fn(&T) -> String,
{
    items
        .iter()
        .map(|item| vec![InlineKeyboardButton::callback(label(item), callback(item))])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::build_item_buttons;
    use teloxide::types::InlineKeyboardButtonKind;

    #[test]
    fn build_item_buttons_creates_rows_with_callback_data() {
        let items = vec![1, 2];
        let buttons = build_item_buttons(
            &items,
            |item| format!("Item {item}"),
            |item| format!("cb_{item}"),
        );

        assert_eq!(buttons.len(), 2);
        assert_eq!(buttons[0][0].text, "Item 1");
        match &buttons[0][0].kind {
            InlineKeyboardButtonKind::CallbackData(data) => {
                assert_eq!(data, "cb_1");
            }
            _ => panic!("expected callback data"),
        }
    }
}
