use ammonia::Builder;
use pulldown_cmark::{Event, Options, Parser, html};
use std::collections::{HashMap, HashSet};

/// Лимит markdown-текста в символах Unicode (не байтах).
pub const MAX_MARKDOWN_CHARS: usize = 4000;

#[derive(Debug, thiserror::Error)]
pub enum MdError {
    #[error("markdown слишком длинный: максимум {max} символов, получено {actual}")]
    TooLong { max: usize, actual: usize },
}

pub fn validate_markdown(input: &str) -> Result<(), MdError> {
    let actual = input.chars().count();
    if actual > MAX_MARKDOWN_CHARS {
        return Err(MdError::TooLong { max: MAX_MARKDOWN_CHARS, actual });
    }
    Ok(())
}

pub fn markdown_to_safe_html(input: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    // Перехватываем события парсера и вставляем спойлеры на лету
    let parser = Parser::new_ext(input, opts).flat_map(|event| {
        if let Event::Text(ref text) = event {
            let s = text.as_ref();
            if s.contains("||") {
                return expand_spoilers(s);
            }
        }
        vec![event]
    });

    let mut raw_html = String::with_capacity(input.len() * 2);
    html::push_html(&mut raw_html, parser);

    sanitize(&raw_html)
}

fn expand_spoilers(text: &str) -> Vec<Event<'static>> {
    let mut events = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("||") {
        if start > 0 {
            events.push(Event::Text(remaining[..start].to_string().into()));
        }
        let after = &remaining[start + 2..];
        if let Some(end) = after.find("||") {
            let content = &after[..end];
            let escaped = content
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            events.push(Event::Html(format!(r#"<span class="spoiler">{escaped}</span>"#).into()));
            remaining = &after[end + 2..];
        } else {
            // незакрытый || — оставляем как есть
            events.push(Event::Text(remaining[start..].to_string().into()));
            return events;
        }
    }

    if !remaining.is_empty() {
        events.push(Event::Text(remaining.to_string().into()));
    }
    events
}

fn sanitize(html: &str) -> String {
    let mut allowed_classes: HashMap<&str, HashSet<&str>> = HashMap::new();
    allowed_classes.insert("span", HashSet::from(["spoiler"]));

    Builder::default().add_tags(&["span"]).allowed_classes(allowed_classes).clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_is_in_chars_not_bytes() {
        // кириллица: 2 байта на символ, ровно лимит проходит
        assert!(validate_markdown(&"я".repeat(MAX_MARKDOWN_CHARS)).is_ok());
        match validate_markdown(&"я".repeat(MAX_MARKDOWN_CHARS + 1)) {
            Err(MdError::TooLong { max, actual }) => {
                assert_eq!((max, actual), (MAX_MARKDOWN_CHARS, MAX_MARKDOWN_CHARS + 1));
            },
            other => panic!("expected TooLong, got {other:?}"),
        }
    }
}
