use crate::ai::stt::parse_items;

pub fn parse_items_with_fallback(
    text: &str,
    gpt_result: anyhow::Result<Vec<String>>,
    context: &str,
) -> Vec<String> {
    match gpt_result {
        Ok(list) => {
            tracing::debug!(context, count = list.len(), "Parsed items via GPT");
            list
        }
        Err(err) => {
            tracing::warn!(
                error = %err,
                context,
                "Falling back to local item parsing"
            );
            parse_items(text)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_items_with_fallback;

    #[test]
    fn parse_items_with_fallback_uses_gpt_success() {
        let result = parse_items_with_fallback("ignored", Ok(vec!["Item".to_string()]), "test");
        assert_eq!(result, vec!["Item".to_string()]);
    }

    #[test]
    fn parse_items_with_fallback_uses_local_parser_on_error() {
        let result = parse_items_with_fallback("a, b and c", Err(anyhow::anyhow!("nope")), "test");
        assert_eq!(
            result,
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }
}
