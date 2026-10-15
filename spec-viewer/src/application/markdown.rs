use pulldown_cmark::{Options, Parser, html};

/// Renders spec text to HTML.
///
/// Anyone can write a proposal, so the HTML is sanitized: raw HTML like `<script>` and
/// `javascript:` links are removed.
pub fn render_markdown(markdown: &str) -> String {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_FOOTNOTES;
    let mut unsafe_html = String::new();
    html::push_html(&mut unsafe_html, Parser::new_ext(markdown, options));
    ammonia::clean(&unsafe_html)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_markdown() {
        let html = render_markdown("# Title\n\nSome *text*.\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");

        assert!(html.contains("<h1>Title</h1>"), "{html}");
        assert!(html.contains("<em>text</em>"), "{html}");
        assert!(html.contains("<td>1</td>"), "{html}");
    }

    #[test]
    fn removes_scripts_and_javascript_links() {
        let html = render_markdown(
            "<script>alert(1)</script>\n\n[link](javascript:alert(1))\n\n<img src=x onerror=alert(1)>",
        );

        assert!(!html.contains("<script"), "{html}");
        assert!(!html.contains("javascript:"), "{html}");
        assert!(!html.contains("onerror"), "{html}");
    }
}
