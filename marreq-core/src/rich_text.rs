// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 Marreq

//! "Marreq statement Markdown": the small Markdown subset used for requirement
//! statements (issue #256). The SPA parses the same subset
//! (`frontend/src/utils/statementMarkdown.ts`); keep both in sync.
//!
//! Blocks: paragraphs (blank line between them; a single newline is a line
//! break), bulleted lists (`- ` / `* `), numbered lists (`1. `; the first
//! number sets the start). Inline: `**bold**`, `*italic*` / `_italic_`,
//! `` `code` ``, `[label](url)` with `http:`, `https:` or `mailto:` URLs only.
//! `\` escapes a marker character. Anything else is literal text.

use std::fmt::Write as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Code(String),
    Link { href: String, children: Vec<Inline> },
    LineBreak,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Paragraph(Vec<Inline>),
    Bullets(Vec<Vec<Inline>>),
    Numbered { start: u32, items: Vec<Vec<Inline>> },
}

const ESCAPABLE: &[char] = &['\\', '*', '_', '`', '[', ']', '(', ')', '-', '.', '#', '!'];

/// True for link targets that are safe to render (`http:`, `https:`, `mailto:`).
pub fn is_safe_href(href: &str) -> bool {
    let h = href.trim().to_ascii_lowercase();
    !href.chars().any(|c| c.is_whitespace() || c.is_control())
        && (h.starts_with("http://") || h.starts_with("https://") || h.starts_with("mailto:"))
}

fn bullet_item(line: &str) -> Option<&str> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    t.strip_prefix("- ").or_else(|| t.strip_prefix("* "))
}

fn numbered_item(line: &str) -> Option<(u32, &str)> {
    let t = line.trim_start_matches(' ');
    if line.len() - t.len() > 3 {
        return None;
    }
    let digits = t.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 || digits > 9 {
        return None;
    }
    let rest = &t[digits..];
    let rest = rest
        .strip_prefix(". ")
        .or_else(|| rest.strip_prefix(") "))?;
    Some((t[..digits].parse().ok()?, rest))
}

/// Parse statement Markdown into blocks.
pub fn parse(src: &str) -> Vec<Block> {
    let src = src.replace("\r\n", "\n");
    let mut blocks = Vec::new();
    let mut para: Vec<&str> = Vec::new();
    let mut list: Option<(bool, u32, Vec<Vec<Inline>>)> = None; // (numbered, start, items)

    fn flush_para(para: &mut Vec<&str>, blocks: &mut Vec<Block>) {
        if !para.is_empty() {
            blocks.push(Block::Paragraph(parse_inline(&para.join("\n"))));
            para.clear();
        }
    }
    fn flush_list(list: &mut Option<(bool, u32, Vec<Vec<Inline>>)>, blocks: &mut Vec<Block>) {
        if let Some((numbered, start, items)) = list.take() {
            blocks.push(if numbered {
                Block::Numbered { start, items }
            } else {
                Block::Bullets(items)
            });
        }
    }

    for line in src.split('\n') {
        if line.trim().is_empty() {
            flush_para(&mut para, &mut blocks);
            flush_list(&mut list, &mut blocks);
        } else if let Some(item) = bullet_item(line) {
            flush_para(&mut para, &mut blocks);
            if matches!(list, Some((true, _, _))) {
                flush_list(&mut list, &mut blocks);
            }
            list.get_or_insert((false, 1, Vec::new()))
                .2
                .push(parse_inline(item));
        } else if let Some((n, item)) = numbered_item(line) {
            flush_para(&mut para, &mut blocks);
            if matches!(list, Some((false, _, _))) {
                flush_list(&mut list, &mut blocks);
            }
            list.get_or_insert((true, n, Vec::new()))
                .2
                .push(parse_inline(item));
        } else {
            flush_list(&mut list, &mut blocks);
            para.push(line);
        }
    }
    flush_para(&mut para, &mut blocks);
    flush_list(&mut list, &mut blocks);
    blocks
}

fn push_text(out: &mut Vec<Inline>, s: &str) {
    if s.is_empty() {
        return;
    }
    if let Some(Inline::Text(prev)) = out.last_mut() {
        prev.push_str(s);
    } else {
        out.push(Inline::Text(s.to_string()));
    }
}

/// Index of the closing delimiter `delim` in `chars[from..]`, skipping escapes.
fn find_closing(chars: &[char], from: usize, delim: &[char]) -> Option<usize> {
    let mut i = from;
    while i + delim.len() <= chars.len() {
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        if chars[i..i + delim.len()] == *delim {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn is_word(c: Option<&char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric())
}

/// Parse inline markup (a paragraph or list item; `\n` becomes a line break).
pub fn parse_inline(src: &str) -> Vec<Inline> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            '\\' if chars.get(i + 1).is_some_and(|n| ESCAPABLE.contains(n)) => {
                push_text(&mut out, &chars[i + 1].to_string());
                i += 2;
                continue;
            }
            '\n' => {
                out.push(Inline::LineBreak);
                i += 1;
                continue;
            }
            '`' => {
                if let Some(end) = chars[i + 1..].iter().position(|&x| x == '`') {
                    if end > 0 {
                        out.push(Inline::Code(chars[i + 1..i + 1 + end].iter().collect()));
                        i += end + 2;
                        continue;
                    }
                }
            }
            '*' if chars.get(i + 1) == Some(&'*') => {
                if let Some(end) = find_closing(&chars, i + 2, &['*', '*']) {
                    if end > i + 2 {
                        let inner: String = chars[i + 2..end].iter().collect();
                        out.push(Inline::Strong(parse_inline(&inner)));
                        i = end + 2;
                        continue;
                    }
                }
            }
            '*' | '_' => {
                let opens = chars
                    .get(i + 1)
                    .is_some_and(|n| !n.is_whitespace() && *n != c)
                    && (c == '*' || !is_word(i.checked_sub(1).and_then(|p| chars.get(p))));
                if opens {
                    let mut j = i + 1;
                    let mut close = None;
                    while let Some(end) = find_closing(&chars, j, &[c]) {
                        let before_ws = chars[end - 1].is_whitespace();
                        let doubled = chars.get(end + 1) == Some(&c);
                        let word_after = c == '_' && is_word(chars.get(end + 1));
                        if !before_ws && !doubled && !word_after {
                            close = Some(end);
                            break;
                        }
                        j = end + if doubled { 2 } else { 1 };
                    }
                    if let Some(end) = close {
                        let inner: String = chars[i + 1..end].iter().collect();
                        out.push(Inline::Emphasis(parse_inline(&inner)));
                        i = end + 1;
                        continue;
                    }
                }
            }
            '[' => {
                if let Some(close) = find_closing(&chars, i + 1, &[']']) {
                    if chars.get(close + 1) == Some(&'(') {
                        if let Some(paren) = chars[close + 2..].iter().position(|&x| x == ')') {
                            let href: String = chars[close + 2..close + 2 + paren].iter().collect();
                            let label: String = chars[i + 1..close].iter().collect();
                            let end = close + 2 + paren + 1;
                            if close > i + 1 && is_safe_href(&href) {
                                out.push(Inline::Link {
                                    href: href.trim().to_string(),
                                    children: parse_inline(&label),
                                });
                            } else {
                                // Unsafe or empty link: keep the source visible as text.
                                push_text(&mut out, &chars[i..end].iter().collect::<String>());
                            }
                            i = end;
                            continue;
                        }
                    }
                }
            }
            _ => {}
        }
        push_text(&mut out, &c.to_string());
        i += 1;
    }
    out
}

fn escape_xml(s: &str, out: &mut String) {
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
}

fn inline_xhtml(nodes: &[Inline], out: &mut String) {
    for node in nodes {
        match node {
            Inline::Text(t) => escape_xml(t, out),
            Inline::LineBreak => out.push_str("<xhtml:br/>"),
            Inline::Code(t) => {
                out.push_str("<xhtml:code>");
                escape_xml(t, out);
                out.push_str("</xhtml:code>");
            }
            Inline::Strong(c) => {
                out.push_str("<xhtml:strong>");
                inline_xhtml(c, out);
                out.push_str("</xhtml:strong>");
            }
            Inline::Emphasis(c) => {
                out.push_str("<xhtml:em>");
                inline_xhtml(c, out);
                out.push_str("</xhtml:em>");
            }
            Inline::Link { href, children } => {
                out.push_str("<xhtml:a href=\"");
                escape_xml(href, out);
                out.push_str("\">");
                inline_xhtml(children, out);
                out.push_str("</xhtml:a>");
            }
        }
    }
}

/// Render statement Markdown as a ReqIF XHTML fragment (`<xhtml:div>…</xhtml:div>`).
/// The `xhtml` prefix must be bound to `http://www.w3.org/1999/xhtml` by the caller.
pub fn to_xhtml(src: &str) -> String {
    let mut out = String::from("<xhtml:div>");
    for block in parse(src) {
        match block {
            Block::Paragraph(inl) => {
                out.push_str("<xhtml:p>");
                inline_xhtml(&inl, &mut out);
                out.push_str("</xhtml:p>");
            }
            Block::Bullets(items) => {
                out.push_str("<xhtml:ul>");
                for item in items {
                    out.push_str("<xhtml:li>");
                    inline_xhtml(&item, &mut out);
                    out.push_str("</xhtml:li>");
                }
                out.push_str("</xhtml:ul>");
            }
            Block::Numbered { start, items } => {
                if start == 1 {
                    out.push_str("<xhtml:ol>");
                } else {
                    let _ = write!(out, "<xhtml:ol start=\"{start}\">");
                }
                for item in items {
                    out.push_str("<xhtml:li>");
                    inline_xhtml(&item, &mut out);
                    out.push_str("</xhtml:li>");
                }
                out.push_str("</xhtml:ol>");
            }
        }
    }
    out.push_str("</xhtml:div>");
    out
}

fn inline_plain(nodes: &[Inline], out: &mut String) {
    for node in nodes {
        match node {
            Inline::Text(t) | Inline::Code(t) => out.push_str(t),
            Inline::LineBreak => out.push('\n'),
            Inline::Strong(c) | Inline::Emphasis(c) => inline_plain(c, out),
            Inline::Link { children, .. } => inline_plain(children, out),
        }
    }
}

/// Statement text without markup (for search/embeddings and one-line previews).
/// List items keep their own lines, prefixed with `•` or their number.
pub fn to_plain_text(src: &str) -> String {
    let mut out = String::new();
    for (i, block) in parse(src).into_iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match block {
            Block::Paragraph(inl) => inline_plain(&inl, &mut out),
            Block::Bullets(items) => {
                for (k, item) in items.iter().enumerate() {
                    if k > 0 {
                        out.push('\n');
                    }
                    out.push_str("• ");
                    inline_plain(item, &mut out);
                }
            }
            Block::Numbered { start, items } => {
                for (k, item) in items.iter().enumerate() {
                    if k > 0 {
                        out.push('\n');
                    }
                    let _ = write!(out, "{}. ", start as usize + k);
                    inline_plain(item, &mut out);
                }
            }
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    /// Start of output, or right after a newline.
    LineStart,
    /// Text or a closing inline marker (a following word needs a space).
    Content,
    /// An opening inline marker (no space before the next word).
    Opener,
}

enum ListCtx {
    Bullets,
    Numbered(u32),
}

/// Builds statement Markdown from a stream of XHTML events (ReqIF import).
///
/// Feed element names without namespace prefix. Unknown elements are ignored
/// (their text is kept). Text runs are assumed trimmed; spaces are re-inserted
/// between runs.
pub struct MarkdownBuilder {
    out: String,
    last: Last,
    pending_block: bool,
    lists: Vec<ListCtx>,
    links: Vec<Option<String>>,
}

impl Default for MarkdownBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownBuilder {
    pub fn new() -> Self {
        Self {
            out: String::new(),
            last: Last::LineStart,
            pending_block: false,
            lists: Vec::new(),
            links: Vec::new(),
        }
    }

    fn block_break(&mut self) {
        if !self.out.is_empty() {
            while self.out.ends_with(' ') {
                self.out.pop();
            }
            if !self.out.ends_with("\n\n") {
                if self.out.ends_with('\n') {
                    self.out.push('\n');
                } else {
                    self.out.push_str("\n\n");
                }
            }
        }
        self.last = Last::LineStart;
        self.pending_block = false;
    }

    fn line_break(&mut self) {
        while self.out.ends_with(' ') {
            self.out.pop();
        }
        if !self.out.is_empty() && !self.out.ends_with('\n') {
            self.out.push('\n');
        }
        self.last = Last::LineStart;
    }

    fn before_inline(&mut self) {
        if self.pending_block {
            self.block_break();
        }
        if self.last == Last::Content {
            self.out.push(' ');
        }
    }

    fn marker(&mut self, m: &str, opening: bool) {
        if opening {
            self.before_inline();
            self.out.push_str(m);
            self.last = Last::Opener;
        } else {
            self.out.push_str(m);
            self.last = Last::Content;
        }
    }

    pub fn open(&mut self, tag: &str, href: Option<&str>) {
        match tag.to_ascii_lowercase().as_str() {
            "p" | "div" | "blockquote" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if self.lists.is_empty() {
                    self.pending_block = true;
                }
            }
            "ul" => {
                if self.lists.is_empty() {
                    self.block_break();
                }
                self.lists.push(ListCtx::Bullets);
            }
            "ol" => {
                if self.lists.is_empty() {
                    self.block_break();
                }
                self.lists.push(ListCtx::Numbered(1));
            }
            "li" => {
                self.pending_block = false;
                self.line_break();
                let prefix = match self.lists.last_mut() {
                    Some(ListCtx::Numbered(n)) => {
                        let p = format!("{n}. ");
                        *n += 1;
                        p
                    }
                    _ => "- ".to_string(),
                };
                self.out.push_str(&prefix);
                self.last = Last::Opener;
            }
            "br" => self.line_break(),
            // Tables are not part of the subset: keep one row per line.
            "tr" => {
                if self.pending_block {
                    self.block_break();
                }
                self.line_break();
            }
            "strong" | "b" => self.marker("**", true),
            "em" | "i" => self.marker("*", true),
            "code" | "tt" => self.marker("`", true),
            "a" => {
                let safe = href.filter(|h| is_safe_href(h)).map(str::to_string);
                if safe.is_some() {
                    self.marker("[", true);
                }
                self.links.push(safe);
            }
            _ => {}
        }
    }

    /// Set the start number of the innermost numbered list (`<ol start="n">`).
    pub fn set_list_start(&mut self, start: u32) {
        if let Some(ListCtx::Numbered(n)) = self.lists.last_mut() {
            *n = start;
        }
    }

    pub fn close(&mut self, tag: &str) {
        match tag.to_ascii_lowercase().as_str() {
            "p" | "div" | "blockquote" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                if self.lists.is_empty() {
                    self.pending_block = true;
                }
            }
            "ul" | "ol" => {
                self.lists.pop();
                if self.lists.is_empty() {
                    self.pending_block = true;
                }
            }
            "strong" | "b" => self.marker("**", false),
            "em" | "i" => self.marker("*", false),
            "code" | "tt" => self.marker("`", false),
            "a" => {
                if let Some(Some(href)) = self.links.pop() {
                    self.marker(&format!("]({href})"), false);
                }
            }
            _ => {}
        }
    }

    pub fn text(&mut self, text: &str) {
        // XHTML whitespace is not significant: collapse runs to single spaces.
        let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let text = collapsed.as_str();
        if text.is_empty() {
            return;
        }
        let starts_with_punct = text.starts_with(['.', ',', ';', ':', '!', '?', ')']);
        if self.pending_block {
            self.block_break();
        }
        if self.last == Last::Content && !starts_with_punct {
            self.out.push(' ');
        }
        let mut prev: Option<char> = self.out.chars().last();
        for c in text.chars() {
            let at_word_start = !prev.is_some_and(|p| p.is_alphanumeric());
            if matches!(c, '\\' | '*' | '`' | '[' | ']') || (c == '_' && at_word_start) {
                self.out.push('\\');
            }
            self.out.push(c);
            prev = Some(c);
        }
        self.last = Last::Content;
    }

    /// The Markdown built so far, without trailing whitespace.
    pub fn finish(self) -> String {
        self.out.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> Inline {
        Inline::Text(s.into())
    }

    #[test]
    fn plain_text_is_one_paragraph_with_line_breaks() {
        assert_eq!(
            parse("The system shall start.\nIt shall stop."),
            vec![Block::Paragraph(vec![
                t("The system shall start."),
                Inline::LineBreak,
                t("It shall stop.")
            ])]
        );
        assert_eq!(parse("  \n\n "), vec![]);
    }

    #[test]
    fn lists_and_paragraphs() {
        let blocks = parse("Power modes:\n1. Off\n2. **Safe**\n3. Nominal\n\n- a\n* b\n\nEnd");
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0], Block::Paragraph(vec![t("Power modes:")]));
        assert_eq!(
            blocks[1],
            Block::Numbered {
                start: 1,
                items: vec![
                    vec![t("Off")],
                    vec![Inline::Strong(vec![t("Safe")])],
                    vec![t("Nominal")]
                ]
            }
        );
        assert_eq!(blocks[2], Block::Bullets(vec![vec![t("a")], vec![t("b")]]));
        assert_eq!(blocks[3], Block::Paragraph(vec![t("End")]));
        assert!(matches!(
            parse("3. x\n4. y")[0],
            Block::Numbered { start: 3, .. }
        ));
    }

    #[test]
    fn inline_markup() {
        assert_eq!(
            parse_inline("a **b** *c* _d_ `e*f` [g](https://x.test/p)"),
            vec![
                t("a "),
                Inline::Strong(vec![t("b")]),
                t(" "),
                Inline::Emphasis(vec![t("c")]),
                t(" "),
                Inline::Emphasis(vec![t("d")]),
                t(" "),
                Inline::Code("e*f".into()),
                t(" "),
                Inline::Link {
                    href: "https://x.test/p".into(),
                    children: vec![t("g")]
                },
            ]
        );
    }

    #[test]
    fn literal_cases_stay_text() {
        assert_eq!(parse_inline("POWER_MODE_ONE"), vec![t("POWER_MODE_ONE")]);
        assert_eq!(parse_inline("2 * 3 * 4"), vec![t("2 * 3 * 4")]);
        assert_eq!(parse_inline("\\*not italic\\*"), vec![t("*not italic*")]);
        assert_eq!(parse_inline("**unclosed"), vec![t("**unclosed")]);
        assert_eq!(parse_inline("`"), vec![t("`")]);
    }

    #[test]
    fn unsafe_links_are_not_links() {
        for src in [
            "[x](javascript:alert(1))",
            "[x](data:text/html,hi)",
            "[x](vbscript:y)",
            "[](https://a.test)",
        ] {
            assert!(
                !parse_inline(src)
                    .iter()
                    .any(|n| matches!(n, Inline::Link { .. })),
                "{src}"
            );
        }
        assert!(is_safe_href("mailto:ops@example.com"));
        assert!(!is_safe_href("https://a.test/ x"));
    }

    #[test]
    fn xhtml_is_escaped_and_structured() {
        assert_eq!(
            to_xhtml("Modes <b>&\n1. **Off**\n2. [Doc](https://d.test/?a=1&b=\"2\")"),
            "<xhtml:div><xhtml:p>Modes &lt;b&gt;&amp;</xhtml:p><xhtml:ol><xhtml:li><xhtml:strong>Off</xhtml:strong></xhtml:li><xhtml:li><xhtml:a href=\"https://d.test/?a=1&amp;b=&quot;2&quot;\">Doc</xhtml:a></xhtml:li></xhtml:ol></xhtml:div>"
        );
        assert_eq!(
            to_xhtml("x\ny\n\n3. a"),
            "<xhtml:div><xhtml:p>x<xhtml:br/>y</xhtml:p><xhtml:ol start=\"3\"><xhtml:li>a</xhtml:li></xhtml:ol></xhtml:div>"
        );
        assert!(!to_xhtml("[x](javascript:alert(1))").contains("href"));
    }

    #[test]
    fn plain_text_drops_markers() {
        assert_eq!(
            to_plain_text(
                "The **system** shall:\n- boot\n- run `self-test`\n\nSee [spec](https://s.test)."
            ),
            "The system shall:\n• boot\n• run self-test\nSee spec."
        );
        assert_eq!(to_plain_text("2. a\n3. b"), "2. a\n3. b");
    }

    fn build(events: &[(&str, &str)]) -> String {
        let mut b = MarkdownBuilder::new();
        for (kind, value) in events {
            match *kind {
                "open" => b.open(value, None),
                "close" => b.close(value),
                "text" => b.text(value),
                other => {
                    let (tag, href) = other.split_once('=').unwrap();
                    b.open(tag, Some(href));
                    b.text(value);
                    b.close(tag);
                }
            }
        }
        b.finish()
    }

    #[test]
    fn builder_converts_xhtml_structure() {
        let md = build(&[
            ("open", "div"),
            ("open", "p"),
            ("text", "The system shall support"),
            ("open", "strong"),
            ("text", "three"),
            ("close", "strong"),
            ("text", "modes:"),
            ("close", "p"),
            ("open", "ol"),
            ("open", "li"),
            ("text", "Off"),
            ("close", "li"),
            ("open", "li"),
            ("text", "Safe"),
            ("close", "li"),
            ("close", "ol"),
            ("open", "p"),
            ("text", "See"),
            ("a=https://s.test", "spec"),
            ("text", "."),
            ("close", "p"),
            ("close", "div"),
        ]);
        assert_eq!(
            md,
            "The system shall support **three** modes:\n\n1. Off\n2. Safe\n\nSee [spec](https://s.test)."
        );
        assert_eq!(parse(&md).len(), 3);
    }

    #[test]
    fn builder_escapes_markers_and_drops_unsafe_links() {
        let md = build(&[
            ("open", "p"),
            ("text", "a*b [c] _d POWER_MODE"),
            ("a=javascript:x", "click"),
            ("close", "p"),
        ]);
        assert_eq!(md, "a\\*b \\[c\\] \\_d POWER_MODE click");
        assert_eq!(to_plain_text(&md), "a*b [c] _d POWER_MODE click");
    }

    #[test]
    fn builder_handles_line_breaks_and_bullets() {
        let md = build(&[
            ("text", "Line one"),
            ("open", "br"),
            ("close", "br"),
            ("text", "Line two"),
            ("open", "ul"),
            ("open", "li"),
            ("text", "x"),
            ("close", "li"),
            ("close", "ul"),
        ]);
        assert_eq!(md, "Line one\nLine two\n\n- x");
    }
}
