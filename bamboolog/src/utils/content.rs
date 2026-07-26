/// Renders Markdown with raw HTML enabled, then removes unsafe markup.
pub fn render_markdown(source: &str) -> Result<String, markdown::message::Message> {
    let mut options = markdown::Options::gfm();
    options.compile.allow_dangerous_html = true;
    options.compile.gfm_tagfilter = false;

    let rendered = markdown::to_html_with_options(source, &options)?;
    Ok(ammonia::Builder::default()
        // Syntax highlighters and themes commonly select code by class name.
        .add_generic_attributes(["class"])
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
}
