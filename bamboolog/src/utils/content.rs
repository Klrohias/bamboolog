use std::collections::BTreeMap;

/// Renders post Markdown after expanding the portable content directives.
pub fn render_markdown(source: &str) -> Result<String, markdown::message::Message> {
    let (source, replacements) = extract_directives(source);
    let mut rendered = markdown::to_html_with_options(&source, &markdown::Options::gfm())?;
    for (marker, html) in replacements {
        rendered = rendered.replace(&format!("<p>{marker}</p>"), &html);
    }
    Ok(rendered)
}

/// Expands a small, intentionally constrained set of content directives.
///
/// The Hugo-compatible aliases keep imported Diary content working. Unsupported
/// directives remain untouched so they render as ordinary Markdown text.
pub fn expand_directives(source: &str) -> String {
    transform_directives(source, |_, html| html.to_string()).0
}

fn extract_directives(source: &str) -> (String, Vec<(String, String)>) {
    let mut replacements = Vec::new();
    let (source, _) = transform_directives(source, |index, html| {
        let marker = format!("BAMBOO_DIRECTIVE_{index}");
        replacements.push((marker.clone(), html.to_string()));
        marker
    });
    (source, replacements)
}

fn transform_directives<F>(source: &str, mut transform: F) -> (String, usize)
where
    F: FnMut(usize, &str) -> String,
{
    let mut output = String::with_capacity(source.len());
    let mut directive_count = 0;
    let mut in_code_block = false;

    for line in source.split_inclusive('\n') {
        let (content, newline) = match line.strip_suffix('\n') {
            Some(content) => (content, "\n"),
            None => (line, ""),
        };
        let trimmed = content.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_block = !in_code_block;
            output.push_str(line);
            continue;
        }
        if in_code_block || !trimmed.starts_with("{{<") || !trimmed.ends_with(">}}") {
            output.push_str(line);
            continue;
        }

        let directive = &trimmed[3..trimmed.len() - 3];
        let tokens = tokenize(directive);
        let rendered = tokens
            .first()
            .and_then(|name| render_directive(name, &tokens[1..]));
        let Some(rendered) = rendered else {
            output.push_str(line);
            continue;
        };

        let leading = &content[..content.find(trimmed).unwrap_or(0)];
        let trailing_start = content.find(trimmed).unwrap_or(0) + trimmed.len();
        output.push_str(leading);
        output.push_str(&transform(directive_count, &rendered));
        output.push_str(&content[trailing_start..]);
        output.push_str(newline);
        directive_count += 1;
    }
    (output, directive_count)
}

fn render_directive(name: &str, args: &[String]) -> Option<String> {
    let values = directive_values(args);
    match name {
        "bilibili" => bilibili_embed(args.first()?.as_str(), args.get(1).map(String::as_str)),
        "embed" if values.get("provider").is_some_and(|value| *value == "bilibili") => {
            bilibili_embed(*values.get("id")?, values.get("page").copied())
        }
        "insertFigure" | "figure" => figure(
            *values.get("src").or_else(|| values.get("img"))?,
            values.get("caption").copied().unwrap_or_default(),
            values.get("align").copied().unwrap_or("center"),
        ),
        _ => None,
    }
}

fn bilibili_embed(bvid: &str, page: Option<&str>) -> Option<String> {
    if !is_bilibili_bvid(bvid) {
        return None;
    }
    let page = page.and_then(|value| value.parse::<u32>().ok()).filter(|page| *page > 0).unwrap_or(1);
    Some(format!(
        "<div class=\"content-embed content-embed-bilibili\"><iframe src=\"https://player.bilibili.com/player.html?bvid={bvid}&amp;page={page}&amp;autoplay=0\" title=\"Bilibili video\" loading=\"lazy\" scrolling=\"no\" allowfullscreen></iframe></div>"
    ))
}

fn figure(src: &str, caption: &str, align: &str) -> Option<String> {
    if !is_safe_media_url(src) {
        return None;
    }
    let align = match align {
        "left" | "right" | "center" => align,
        _ => "center",
    };
    let src = escape_html(src);
    let caption_text = escape_html(caption);
    let caption = if caption_text.is_empty() {
        String::new()
    } else {
        format!("<figcaption>{caption_text}</figcaption>")
    };
    Some(format!(
        "<figure class=\"content-figure content-figure-{align}\"><a href=\"{src}\"><img src=\"{src}\" alt=\"{caption_text}\"></a>{caption}</figure>"
    ))
}

fn directive_values(args: &[String]) -> BTreeMap<&str, &str> {
    args.iter()
        .filter_map(|argument| argument.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .collect()
}

fn tokenize(value: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    for character in value.trim().chars() {
        match (quote, character) {
            (Some(delimiter), character) if character == delimiter => quote = None,
            (None, '\'' | '\"') => quote = Some(character),
            (None, character) if character.is_whitespace() => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(character),
        }
    }
    if quote.is_some() {
        return Vec::new();
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    tokens
}

fn is_bilibili_bvid(value: &str) -> bool {
    value.starts_with("BV")
        && (8..=20).contains(&value.len())
        && value.bytes().all(|character| character.is_ascii_alphanumeric())
}

fn is_safe_media_url(value: &str) -> bool {
    value.starts_with('/') || value.starts_with("https://") || value.starts_with("http://")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::{expand_directives, render_markdown};

    #[test]
    fn expands_hugo_compatible_and_portable_bilibili_directives() {
        let rendered = expand_directives("{{< bilibili BV1xx411c7mD 2 >}}\n{{< embed provider=\"bilibili\" id=\"BV1xx411c7mD\" >}}");
        assert_eq!(rendered.matches("player.bilibili.com").count(), 2);
        assert!(rendered.contains("page=2"));
        assert!(rendered.contains("page=1"));
    }

    #[test]
    fn expands_figures_without_accepting_unsafe_urls() {
        let rendered = expand_directives("{{< insertFigure img=\"/attachments/image\" caption=\"A & B\" align=\"left\" >}}");
        assert!(rendered.contains("content-figure-left"));
        assert!(rendered.contains("A &amp; B"));
        assert_eq!(
            expand_directives("{{< figure src=\"javascript:alert(1)\" >}}"),
            "{{< figure src=\"javascript:alert(1)\" >}}"
        );
    }

    #[test]
    fn renders_expanded_directives_as_html() {
        let rendered = render_markdown("{{< bilibili BV1xx411c7mD >}}").unwrap();
        assert!(rendered.contains("<iframe"));
        assert!(render_markdown("<script>alert(1)</script>")
            .unwrap()
            .contains("&lt;script&gt;"));
    }

    #[test]
    fn leaves_directives_in_code_blocks_untouched() {
        let source = "```text\n{{< bilibili BV1xx411c7mD >}}\n```";
        assert_eq!(expand_directives(source), source);
    }
}
