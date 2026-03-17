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
