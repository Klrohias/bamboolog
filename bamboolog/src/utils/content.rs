/// Renders post Markdown using the system's standard GFM configuration.
pub fn render_markdown(source: &str) -> Result<String, markdown::message::Message> {
    markdown::to_html_with_options(source, &markdown::Options::gfm())
}

#[cfg(test)]
mod tests {
    use super::render_markdown;

    #[test]
    fn renders_gfm_and_escapes_raw_html() {
        let rendered =
            render_markdown("# Heading\n\n~~removed~~\n\n<script>alert(1)</script>").unwrap();

        assert!(rendered.contains("<h1>Heading</h1>"));
        assert!(rendered.contains("<del>removed</del>"));
        assert!(rendered.contains("&lt;script&gt;"));
    }

    #[test]
    fn does_not_execute_theme_specific_shortcodes() {
        let source = "{{< bilibili BV1xx411c7mD >}}";
        let rendered = render_markdown(source).unwrap();

        assert!(rendered.contains("bilibili BV1xx411c7mD"));
        assert!(!rendered.contains("<iframe"));
    }
}
