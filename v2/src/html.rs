//! Rust GFM converter using Marked 14's output conventions for Copy HTML.
//! Preview uses beta's existing Markdown.tsx. See VALIDATION for parser edge gaps.
use crate::{MAX_HTML_BYTES, MAX_INPUT_BYTES};
use pulldown_cmark::{Alignment, CodeBlockKind, Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::Regex;
use std::sync::OnceLock;

fn escape(text: &str, encode: bool) -> String {
    static ENTITY: OnceLock<Regex> = OnceLock::new();
    let entity = ENTITY
        .get_or_init(|| Regex::new(r"^&(?:#[0-9]+|#[xX][0-9a-fA-F]+|[A-Za-z0-9]+);").unwrap());
    let mut out = String::new();
    let mut iter = text.char_indices().peekable();
    while let Some((offset, c)) = iter.next() {
        match c {
            '&' if !encode && entity.is_match(&text[offset..]) => {
                let end = text[offset..].find(';').unwrap() + offset;
                out.push_str(&text[offset..=end]);
                while iter.peek().is_some_and(|(i, _)| *i <= end) {
                    iter.next();
                }
            }
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}
fn attribute(text: &str) -> String {
    text.replace('"', "&quot;")
}
fn destination(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b";/?:@&=+$,-_.!~*'()#%".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
fn image_alt(raw: &str) -> &str {
    let Some(rest) = raw.strip_prefix("![") else {
        return "";
    };
    let mut level = 0;
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '[' => level += 1,
            ']' if level == 0 => return &rest[..i],
            ']' => level -= 1,
            _ => {}
        }
    }
    ""
}
fn autolink(text: &str, raw_block: bool) -> String {
    static URL: OnceLock<Regex> = OnceLock::new();
    let url = URL.get_or_init(|| Regex::new(r"(?i)(?:https?://|ftp://|www\.)[^\s<>]+|[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9_-]+(?:\.[a-z0-9_-]+)+").unwrap());
    let mut out = String::new();
    let mut last = 0;
    for found in url.find_iter(text) {
        let mut target = found
            .as_str()
            .trim_end_matches(['.', ',', ':', ';', '!', '?', '"', '\'']);
        while target.ends_with(')') && target.matches(')').count() > target.matches('(').count() {
            target = &target[..target.len() - 1];
        }
        if target.is_empty() {
            continue;
        }
        out.push_str(&if raw_block {
            text[last..found.start()].to_string()
        } else {
            escape(&text[last..found.start()], false)
        });
        let prefix = if target.to_ascii_lowercase().starts_with("www.") {
            "http://"
        } else if target.contains('@') && !target.contains("://") {
            "mailto:"
        } else {
            ""
        };
        out.push_str(&format!(
            "<a href=\"{}{}\">{}</a>",
            prefix,
            attribute(&destination(target)),
            escape(target, false)
        ));
        last = found.start() + target.len();
    }
    out.push_str(&if raw_block {
        text[last..].to_string()
    } else {
        escape(&text[last..], false)
    });
    out
}

pub fn render(input: &str) -> Result<String, String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err("Markdown exceeds the 1 MiB input limit; nothing was truncated".into());
    }
    let source = input.replace("\r\n", "\n").replace('\r', "\n");
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut out = String::new();
    let mut depth = 0usize;
    let mut code = false;
    let mut code_start = 0;
    let mut list_depth = 0;
    let mut quote_depth = 0;
    let mut link_depth = 0;
    let mut raw_link = false;
    let mut raw_block = false;
    let mut image_depth = 0;
    let mut alignments = Vec::new();
    let mut head = false;
    let mut body = false;
    let mut column = 0usize;
    let mut paragraph_last = 0;
    let mut paragraph = false;
    let mut events = Parser::new_ext(&source, options)
        .into_offset_iter()
        .peekable();
    while let Some((event, range)) = events.next() {
        if matches!(&event, Event::Start(_)) {
            depth += 1;
            if depth > 128 {
                return Err("Markdown nesting exceeds 128 levels; nothing was truncated".into());
            }
        }
        if matches!(&event, Event::End(_)) {
            depth = depth.saturating_sub(1);
        }
        if image_depth > 0 {
            match event {
                Event::Start(_) => image_depth += 1,
                Event::End(_) => image_depth -= 1,
                _ => {}
            }
            paragraph_last = range.end;
            continue;
        }
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    out.push_str("<p>");
                    paragraph = true;
                    paragraph_last = range.start;
                    if list_depth == 0 && quote_depth == 0 {
                        let start = source[..range.start].rfind('\n').map_or(0, |i| i + 1);
                        let prefix = &source[start..range.start];
                        if prefix.bytes().all(|b| b == b' ') && prefix.len() < 4 {
                            out.push_str(prefix);
                        }
                    }
                }
                Tag::Heading { level, .. } => out.push_str(&format!("<{level}>")),
                Tag::BlockQuote(_) => {
                    quote_depth += 1;
                    out.push_str("<blockquote>\n");
                }
                Tag::CodeBlock(kind) => {
                    code = true;
                    out.push_str("<pre><code");
                    if let CodeBlockKind::Fenced(info) = kind {
                        if let Some(lang) = info.split_whitespace().next() {
                            out.push_str(&format!(" class=\"language-{}\"", escape(lang, true)));
                        }
                    }
                    out.push('>');
                    code_start = out.len();
                }
                Tag::HtmlBlock => {}
                Tag::List(start) => {
                    list_depth += 1;
                    out.push_str(&match start {
                        Some(1) => "<ol>\n".into(),
                        Some(n) => format!("<ol start=\"{n}\">\n"),
                        None => "<ul>\n".into(),
                    });
                }
                Tag::Item => out.push_str("<li>"),
                Tag::Emphasis => out.push_str("<em>"),
                Tag::Strong => out.push_str("<strong>"),
                Tag::Strikethrough => out.push_str("<del>"),
                Tag::Link {
                    dest_url,
                    title,
                    link_type,
                    ..
                } => {
                    link_depth += 1;
                    let prefix = if link_type == LinkType::Email {
                        "mailto:"
                    } else {
                        ""
                    };
                    let target = if matches!(link_type, LinkType::Autolink | LinkType::Email) {
                        escape(&dest_url, false)
                    } else {
                        dest_url.to_string()
                    };
                    out.push_str(&format!(
                        "<a href=\"{prefix}{}\"",
                        attribute(&destination(&target))
                    ));
                    if !title.is_empty() {
                        out.push_str(&format!(" title=\"{}\"", escape(&title, false)));
                    }
                    out.push('>');
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    let raw = &source[range.clone()];
                    let alt = image_alt(raw);
                    out.push_str(&format!(
                        "<img src=\"{}\" alt=\"{}\"",
                        attribute(&destination(&dest_url)),
                        escape(&alt.replace("\\[", "[").replace("\\]", "]"), false)
                    ));
                    if !title.is_empty() {
                        out.push_str(&format!(" title=\"{}\"", escape(&title, false)));
                    }
                    out.push('>');
                    image_depth = 1;
                }
                Tag::Table(a) => {
                    alignments = a;
                    body = false;
                    out.push_str("<table>\n");
                }
                Tag::TableHead => {
                    head = true;
                    column = 0;
                    out.push_str("<thead>\n<tr>\n");
                }
                Tag::TableRow => {
                    if !body {
                        out.push_str("<tbody>");
                        body = true;
                    }
                    column = 0;
                    out.push_str("<tr>\n");
                }
                Tag::TableCell => {
                    out.push_str(if head { "<th" } else { "<td" });
                    let align = match alignments.get(column) {
                        Some(Alignment::Left) => Some("left"),
                        Some(Alignment::Center) => Some("center"),
                        Some(Alignment::Right) => Some("right"),
                        _ => None,
                    };
                    if let Some(align) = align {
                        out.push_str(&format!(" align=\"{align}\""));
                    }
                    out.push('>');
                    column += 1;
                }
                _ => return Err("Markdown construct has no compatible Copy HTML renderer".into()),
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    let suffix = &source[paragraph_last.min(range.end)..range.end];
                    let suffix = suffix.strip_suffix('\n').unwrap_or(suffix);
                    if suffix.chars().all(char::is_whitespace) {
                        out.push_str(suffix);
                    }
                    out.push_str("</p>\n");
                    paragraph = false;
                }
                TagEnd::Heading(level) => out.push_str(&format!("</{level}>\n")),
                TagEnd::BlockQuote(_) => {
                    quote_depth -= 1;
                    out.push_str("</blockquote>\n");
                }
                TagEnd::CodeBlock => {
                    if out.len() == code_start || !out.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str("</code></pre>\n");
                    code = false;
                }
                TagEnd::HtmlBlock => {
                    let tail = &source[range.end..];
                    let n = tail.bytes().take_while(|b| *b == b'\n').count();
                    out.push_str(&tail[..n]);
                }
                TagEnd::List(ordered) => {
                    list_depth -= 1;
                    out.push_str(if ordered { "</ol>\n" } else { "</ul>\n" });
                }
                TagEnd::Item => out.push_str("</li>\n"),
                TagEnd::Emphasis => {
                    out.push_str("</em>");
                    if paragraph {
                        paragraph_last = range.end;
                    }
                }
                TagEnd::Strong => {
                    out.push_str("</strong>");
                    if paragraph {
                        paragraph_last = range.end;
                    }
                }
                TagEnd::Strikethrough => {
                    out.push_str("</del>");
                    if paragraph {
                        paragraph_last = range.end;
                    }
                }
                TagEnd::Link => {
                    link_depth -= 1;
                    out.push_str("</a>");
                    if paragraph {
                        paragraph_last = range.end;
                    }
                }
                TagEnd::Table => {
                    if body {
                        out.push_str("</tbody>");
                    }
                    out.push_str("</table>\n");
                }
                TagEnd::TableHead => {
                    out.push_str("</tr>\n</thead>\n");
                    head = false;
                }
                TagEnd::TableRow => out.push_str("</tr>\n"),
                TagEnd::TableCell => out.push_str(if head { "</th>\n" } else { "</td>\n" }),
                _ => return Err("Markdown construct has no compatible Copy HTML renderer".into()),
            },
            Event::Text(text) => {
                let raw = &source[range.clone()];
                let mut merged = if !code
                    && raw.starts_with('&')
                    && range.start > 0
                    && source[..range.start]
                        .bytes()
                        .rev()
                        .take_while(|b| *b == b'\\')
                        .count()
                        % 2
                        == 1
                {
                    format!("&amp;{}", text.strip_prefix('&').unwrap_or(&text))
                } else if !code && raw.starts_with('&') && raw.ends_with(';') {
                    raw.to_string()
                } else {
                    text.to_string()
                };
                let mut end = range.end;
                while matches!(events.peek(), Some((Event::Text(_), _))) {
                    let (Event::Text(next), r) = events.next().unwrap() else {
                        unreachable!()
                    };
                    let raw = &source[r.clone()];
                    merged.push_str(if !code && raw.starts_with('&') && raw.ends_with(';') {
                        raw
                    } else {
                        &next
                    });
                    end = r.end;
                }
                if code {
                    out.push_str(&escape(&merged, true));
                } else {
                    out.push_str(&if link_depth > 0 || raw_link {
                        if raw_block {
                            merged
                        } else {
                            escape(&merged, false)
                        }
                    } else {
                        autolink(&merged, raw_block)
                    });
                }
                if paragraph {
                    paragraph_last = end;
                }
            }
            Event::Code(text) => {
                out.push_str(&format!("<code>{}</code>", escape(&text, true)));
                if paragraph {
                    paragraph_last = range.end;
                }
            }
            Event::Html(text) | Event::InlineHtml(text) => {
                let lower = text.to_ascii_lowercase();
                if lower.starts_with("<a ") {
                    raw_link = true;
                } else if lower.starts_with("</a>") {
                    raw_link = false;
                }
                for tag in ["pre", "code", "kbd", "script"] {
                    if lower.strip_prefix(&format!("<{tag}")).is_some_and(|tail| {
                        tail.starts_with('>') || tail.starts_with(char::is_whitespace)
                    }) {
                        raw_block = true;
                    }
                    if lower.strip_prefix(&format!("</{tag}")).is_some_and(|tail| {
                        tail.starts_with('>') || tail.starts_with(char::is_whitespace)
                    }) {
                        raw_block = false;
                    }
                }
                out.push_str(&text);
                if paragraph {
                    paragraph_last = range.end;
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                out.push_str("<br>");
                if list_depth == 0 && quote_depth == 0 {
                    let tail = &source[range.end..];
                    let n = tail.bytes().take_while(|b| *b == b' ').count();
                    out.push_str(&tail[..n]);
                }
                if paragraph {
                    paragraph_last = range.end;
                }
            }
            Event::Rule => out.push_str("<hr>\n"),
            Event::TaskListMarker(checked) => {
                out.push_str(if checked {
                    "<input checked=\"\" disabled=\"\" type=\"checkbox\"> "
                } else {
                    "<input disabled=\"\" type=\"checkbox\"> "
                });
                if paragraph {
                    out.push('\n');
                }
            }
            _ => return Err("Markdown construct has no compatible Copy HTML renderer".into()),
        }
        if out.len() > MAX_HTML_BYTES {
            return Err(
                "Generated HTML exceeds the 8 MiB output limit; nothing was truncated".into(),
            );
        }
    }
    Ok(out)
}
