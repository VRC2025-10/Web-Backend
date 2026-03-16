pub trait MarkdownRenderer: Send + Sync {
    /// Render markdown to sanitized HTML.
    /// Returns the sanitized HTML string.
    fn render(&self, markdown: &str) -> String;
}
