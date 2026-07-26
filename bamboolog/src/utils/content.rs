use std::borrow::Cow;

/// Renders Markdown with raw HTML enabled, then removes unsafe markup.
pub fn render_markdown(source: &str) -> Result<String, markdown::message::Message> {
    let mut options = markdown::Options::gfm();
    options.compile.allow_dangerous_html = true;
    options.compile.gfm_tagfilter = false;

    let rendered = markdown::to_html_with_options(source, &options)?;
    Ok(ammonia::Builder::default()
        // Syntax highlighters and themes commonly select code by class name.
        .add_generic_attributes(["class", "id"])
        .add_generic_attribute_prefixes(["aria-", "data-"])
        .id_prefix(Some("user-content-"))
        .add_tags(["input"])
        .add_tag_attribute_values("input", "type", ["checkbox"])
        .add_tag_attribute_values("input", "checked", [""])
        .add_tag_attribute_values("input", "disabled", [""])
        .add_tag_attribute_values("details", "open", [""])
        // Keep fragment links aligned with the prefixed IDs above.
        .attribute_filter(|tag, attribute, value| {
            if tag == "a"
                && attribute == "href"
                && value.starts_with('#')
                && !value.starts_with("#user-content-")
            {
                Some(Cow::Owned(format!("#user-content-{}", &value[1..])))
            } else {
                Some(Cow::Borrowed(value))
            }
        })
        .clean(&rendered)
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn renders_gfm_and_sanitizes_raw_html() {
        let rendered =
            render_markdown("# Heading\n\n~~removed~~\n\n<script>alert(1)</script>").unwrap();

        assert!(rendered.contains("<h1>Heading</h1>"));
        assert!(rendered.contains("<del>removed</del>"));
        assert!(!rendered.contains("<script"));
    }

    #[test]
    fn does_not_execute_theme_specific_shortcodes() {
        let source = "{{< bilibili BV1xx411c7mD >}}";
        let rendered = render_markdown(source).unwrap();

        assert!(rendered.contains("bilibili BV1xx411c7mD"));
        assert!(!rendered.contains("<iframe"));
    }

    #[test]
    fn renders_safe_raw_html_and_preserves_classes() {
        let rendered =
            render_markdown("<div class=\"callout\">Note</div>\n\n```rust\nlet value = 1;\n```")
                .unwrap();

        assert!(rendered.contains("<div class=\"callout\">Note</div>"));
        assert!(rendered.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn strips_unsafe_html() {
        let rendered = render_markdown(
            "<script>alert(1)</script><img src=\"/safe.png\" onerror=\"alert(1)\"><a href=\"javascript:alert(1)\">bad</a>",
        )
        .unwrap();

        assert!(!rendered.contains("<script"));
        assert!(!rendered.contains("onerror"));
        assert!(!rendered.contains("javascript:"));
        assert!(rendered.contains("<img src=\"/safe.png\""));
    }

    #[test]
    fn preserves_gfm_footnotes_and_task_lists() {
        let rendered = render_markdown("- [x] Done\n\nA note[^1].\n\n[^1]: Footnote").unwrap();

        assert!(rendered.contains("<input type=\"checkbox\""));
        assert!(rendered.contains("disabled=\"\""));
        assert!(rendered.contains("checked=\"\""));
        assert!(rendered.contains("data-footnote-ref=\"\""));
        assert!(rendered.contains("aria-describedby=\"footnote-label\""));
        assert!(rendered.contains("id=\"user-content-fnref-1\""));
        assert!(rendered.contains("href=\"#user-content-fn-1\""));
    }

    #[test]
    fn prefixes_raw_html_anchor_ids_and_links() {
        let rendered =
            render_markdown("<a href=\"#details\">Jump</a><h2 id=\"details\">Details</h2>")
                .unwrap();

        assert!(rendered.contains("href=\"#user-content-details\""));
        assert!(rendered.contains("id=\"user-content-details\""));
    }
}
