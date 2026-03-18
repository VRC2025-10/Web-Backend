use pulldown_cmark::{Options, Parser, html};

use crate::domain::ports::services::markdown_renderer::MarkdownRenderer;

pub struct PulldownCmarkRenderer;

impl PulldownCmarkRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PulldownCmarkRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer for PulldownCmarkRenderer {
    fn render(&self, markdown: &str) -> String {
        // Parse markdown with common extensions
        let options = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
        let parser = Parser::new_ext(markdown, options);

        // Render to HTML
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        // Sanitize with ammonia — allowlist-based to prevent XSS
        ammonia::Builder::new()
            .tags(ALLOWED_TAGS.iter().copied().collect())
            .add_tag_attributes("a", &["href"])
            .add_tag_attributes("img", &["src", "alt"])
            .url_schemes(["https"].iter().copied().collect())
            .link_rel(Some("noopener noreferrer"))
            .clean(&html_output)
            .to_string()
    }
}

const ALLOWED_TAGS: &[&str] = &[
    "p",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "strong",
    "em",
    "a",
    "ul",
    "ol",
    "li",
    "code",
    "pre",
    "blockquote",
    "br",
    "img",
    "del",
    "table",
    "thead",
    "tbody",
    "tr",
    "th",
    "td",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn render(input: &str) -> String {
        PulldownCmarkRenderer::new().render(input)
    }

    #[test]
    fn test_render_basic_markdown() {
        let html = render("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_render_strips_script_tags() {
        let html = render("<script>alert('xss')</script>");
        assert!(!html.contains("<script"));
        assert!(!html.contains("alert"));
    }

    #[test]
    fn test_render_strips_event_handlers() {
        let html = render("<img src=x onerror=alert(1)>");
        assert!(!html.contains("onerror"));
    }

    #[test]
    fn test_render_allows_safe_links() {
        let html = render("[link](https://example.com)");
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("rel=\"noopener noreferrer\""));
    }

    #[test]
    fn test_render_strips_javascript_links() {
        let html = render("[click](javascript:alert(1))");
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn test_render_empty_input() {
        let html = render("");
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn test_render_strikethrough() {
        let html = render("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_render_code_block() {
        let html = render("```\nfn main() {}\n```");
        assert!(html.contains("<code>"));
        assert!(html.contains("<pre>"));
    }

    #[test]
    fn test_render_strips_http_links() {
        // Only https is allowed per spec
        let html = render("[link](http://example.com)");
        assert!(!html.contains("href=\"http://example.com\""));
    }

    #[test]
    fn test_render_headings() {
        let html = render("# Heading 1\n## Heading 2");
        assert!(html.contains("<h1>Heading 1</h1>"));
        assert!(html.contains("<h2>Heading 2</h2>"));
    }

    #[test]
    fn test_render_lists() {
        let html = render("- item 1\n- item 2");
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>item 1</li>"));
        assert!(html.contains("<li>item 2</li>"));
    }

    #[test]
    fn test_render_img_tag_stripped_without_https() {
        // Raw HTML img with non-https src should be stripped
        let html = render("<img src=\"http://evil.com/img.png\">");
        assert!(!html.contains("evil.com"));
    }
}
