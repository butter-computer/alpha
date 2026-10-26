//! Markdown → standalone HTML export.

use pulldown_cmark::{Options, Parser, html};

pub fn markdown_options() -> Options {
    Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES | Options::ENABLE_TASKLISTS
}

/// Renders just the body fragment.
pub fn markdown_to_html_fragment(md: &str) -> String {
    let mut out = String::new();
    html::push_html(&mut out, Parser::new_ext(md, markdown_options()));
    out
}

/// Renders a complete, self-contained HTML document.
pub fn markdown_to_html(md: &str, title: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n<article>\n{}</article>\n</body>\n</html>\n",
        escape(title),
        CSS,
        markdown_to_html_fragment(md)
    )
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

const CSS: &str = "body{margin:0;background:#fbfaf7;color:#262626}\
article{max-width:680px;margin:0 auto;padding:64px 24px;font:18px/1.65 Newsreader,Georgia,serif}\
h1,h2,h3,h4,h5,h6{line-height:1.25;margin:1.6em 0 .5em}\
a{color:#8a4b2a}code{font:.88em ui-monospace,Menlo,monospace;background:#0000000d;padding:.1em .3em;border-radius:4px}\
pre{background:#0000000a;padding:14px 16px;border-radius:6px;overflow:auto}pre code{background:none;padding:0}\
blockquote{margin:1em 0;padding-left:1em;border-left:3px solid #00000026;color:#555}\
hr{border:0;border-top:1px solid #00000026;margin:2em 0}\
@media(prefers-color-scheme:dark){body{background:#1c1b1a;color:#e4e1dc}a{color:#d9a27e}\
code{background:#ffffff14}pre{background:#ffffff0d}blockquote{border-color:#ffffff33;color:#aaa}hr{border-color:#ffffff33}}";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_basic_blocks() {
        let h = markdown_to_html_fragment("# Hi\n\n**b** *i* `c`\n\n- a\n- b\n\n---\n");
        assert!(h.contains("<h1>Hi</h1>"));
        assert!(h.contains("<strong>b</strong>"));
        assert!(h.contains("<em>i</em>"));
        assert!(h.contains("<code>c</code>"));
        assert!(h.contains("<li>a</li>"));
        assert!(h.contains("<hr />"));
    }

    #[test]
    fn renders_links_and_code_blocks() {
        let h = markdown_to_html_fragment("[x](http://a.b)\n\n```rs\nfn f(){}\n```\n\n> q\n");
        assert!(h.contains("<a href=\"http://a.b\">x</a>"));
        assert!(h.contains("<pre><code class=\"language-rs\">"));
        assert!(h.contains("<blockquote>"));
    }

    #[test]
    fn full_document_escapes_title() {
        let h = markdown_to_html("text", "a <b> & c");
        assert!(h.starts_with("<!doctype html>"));
        assert!(h.contains("<title>a &lt;b&gt; &amp; c</title>"));
        assert!(h.contains("<p>text</p>"));
    }
}
