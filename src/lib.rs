use ammonia::Builder;
use pulldown_cmark::{Event, Options, Parser, html};
use std::collections::{HashMap, HashSet};

pub const MAX_MARKDOWN_LEN: usize = 1200;

#[derive(Debug, thiserror::Error)]
pub enum MdError {
    #[error("поле `description` слишком длинное: максимум {max}, получено {actual}")]
    TooLong { max: usize, actual: usize },
}

pub fn validate_markdown(input: &str) -> Result<(), MdError> {
    if input.len() > MAX_MARKDOWN_LEN {
        return Err(MdError::TooLong { max: MAX_MARKDOWN_LEN, actual: input.len() });
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
