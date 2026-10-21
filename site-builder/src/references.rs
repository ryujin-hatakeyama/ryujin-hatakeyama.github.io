//! The list of the works a text cites, as the Writing Studio publishes it
//! (studio/docs/citations.md). The Studio resolves the text's citations
//! (`[@key, p. 4]`, Pandoc's syntax) against the document's bibliography and
//! sets them in its citation style (CSL, by citeproc-js); what reaches this
//! repository is already set:
//!
//! ```text
//! references:                                   (front matter)
//!   - {"id":"ref-Landini2021","text":"Landini, G. (2021). *Title*. …"}
//!
//! … as shown [(Landini, 2021)](#ref-Landini2021).   (the text)
//!
//! ## References
//!
//! ::: {#refs}
//! :::
//! ```
//!
//! Here the `::: {#refs}` block becomes the list, each entry's Markdown
//! rendered as one line of inline text (emphasis, links, escapes; never
//! HTML), with the identifiers carrying the passage's prefix as theorem
//! identifiers do; and every link to `#ref-…` is checked against the list
//! and given the same prefix. Only the works the text cites are in the file:
//! the rest of the Studio's bibliography never leaves it.

use std::collections::HashSet;

use anyhow::{Result, bail, ensure};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd, html};
use serde::Deserialize;

use crate::model::validate_markdown_url;
use crate::theorems::CodeFences;

/// One work cited, under `references:`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    /// `ref-` and the citation key, made safe for an identifier.
    pub id: String,
    /// The entry as the style writes it, as one line of Markdown.
    pub text: String,
}

/// The line the `::: {#refs}` block becomes before parsing: an HTML block the
/// renderer replaces with the list (raw HTML is otherwise refused).
pub(crate) const MARK: &str = "<!-- references -->";

pub fn validate(references: &[Reference]) -> Result<()> {
    let mut seen = HashSet::new();
    for reference in references {
        let key = reference.id.strip_prefix("ref-").unwrap_or_default();
        ensure!(
            !key.is_empty()
                && key
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "reference id {:?} must be ref- and letters, digits, _ or -",
            reference.id
        );
        ensure!(
            seen.insert(reference.id.as_str()),
            "reference id {:?} is given twice",
            reference.id
        );
        ensure!(
            !reference.text.trim().is_empty() && !reference.text.contains('\n'),
            "reference {:?} must be one line of text",
            reference.id
        );
    }
    Ok(())
}

fn is_opener(line: &str) -> bool {
    let trimmed = line.trim_end();
    let Some(rest) = trimmed.strip_prefix(":::") else {
        return false;
    };
    let rest = rest.trim_start_matches(':').trim();
    rest.strip_prefix('{')
        .and_then(|r| r.strip_suffix('}'))
        .is_some_and(|inner| inner.trim() == "#refs")
}

fn is_closer(line: &str) -> bool {
    let trimmed = line.trim_end();
    trimmed.len() >= 3 && trimmed.bytes().all(|b| b == b':')
}

/// The text with its `::: {#refs}` block (opening and closing lines, and
/// nothing between) as [`MARK`], when the file lists references; refused
/// when the list and the block do not go together.
pub(crate) fn place(source: &str, references: &[Reference]) -> Result<Option<String>> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut code = CodeFences::default();
    let mut found = None;
    for (n, line) in lines.iter().enumerate() {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if code.code(line) {
            continue;
        }
        ensure!(
            line.trim() != MARK,
            "line {}: {MARK} is reserved for the references",
            n + 1
        );
        if is_opener(line) {
            ensure!(
                found.is_none(),
                "line {}: the references are placed once (::: {{#refs}})",
                n + 1
            );
            let close = lines.get(n + 1).map(|l| l.strip_suffix('\r').unwrap_or(l));
            ensure!(
                close.is_some_and(is_closer),
                "line {}: ::: {{#refs}} is closed on the next line (:::), with nothing between",
                n + 1
            );
            found = Some(n);
        }
    }
    match (found, references.is_empty()) {
        (None, true) => Ok(None),
        (Some(n), true) => bail!(
            "line {}: ::: {{#refs}} places references, but the front matter lists none",
            n + 1
        ),
        (None, false) => {
            bail!("the front matter lists references, but the text has no ::: {{#refs}} for them")
        }
        (Some(n), false) => {
            let mut out: Vec<&str> = Vec::with_capacity(lines.len() - 1);
            out.extend(&lines[..n]);
            out.push(MARK);
            out.extend(&lines[n + 2..]);
            Ok(Some(out.join("\n")))
        }
    }
}

/// The identifier on the page: the passage's prefix, then the reference's.
pub(crate) fn element_id(prefix: Option<&str>, id: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}-{id}"),
        None => id.to_owned(),
    }
}

/// Where a link to `#ref-…` goes on the page; refused when the list has no such entry.
pub(crate) fn link_target(
    references: &[Reference],
    prefix: Option<&str>,
    url: &str,
) -> Result<Option<String>> {
    let Some(id) = url.strip_prefix('#').filter(|id| id.starts_with("ref-")) else {
        return Ok(None);
    };
    ensure!(
        references.iter().any(|r| r.id == id),
        "the link to #{id} names no entry of the references"
    );
    Ok(Some(format!("#{}", element_id(prefix, id))))
}

/// One entry's Markdown as inline HTML: emphasis, links (checked as the
/// text's are), code, and escapes; anything else is refused.
fn entry_html(text: &str) -> Result<String> {
    let mut events = Vec::new();
    for event in Parser::new_ext(text, Options::ENABLE_STRIKETHROUGH) {
        match &event {
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph) => continue,
            Event::Start(Tag::Link { dest_url, .. }) => validate_markdown_url(dest_url)?,
            Event::Start(Tag::Emphasis | Tag::Strong | Tag::Strikethrough)
            | Event::End(
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link,
            )
            | Event::Text(_)
            | Event::Code(_)
            | Event::SoftBreak => {}
            other => bail!("a reference is one line of text, emphasis, and links; found {other:?}"),
        }
        events.push(event);
    }
    let mut out = String::new();
    html::push_html(&mut out, events.into_iter());
    Ok(out)
}

/// The list, as Pandoc writes it (div#refs.references, div.csl-entry per work).
pub(crate) fn list_html(references: &[Reference], prefix: Option<&str>) -> Result<String> {
    let mut out = format!(
        "<div id=\"{}\" class=\"references csl-bib-body\" role=\"list\">\n",
        element_id(prefix, "refs")
    );
    for reference in references {
        out.push_str(&format!(
            "<div id=\"{}\" class=\"csl-entry\" role=\"listitem\">{}</div>\n",
            element_id(prefix, &reference.id),
            entry_html(&reference.text)?
        ));
    }
    out.push_str("</div>\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(id: &str, text: &str) -> Reference {
        Reference {
            id: id.to_owned(),
            text: text.to_owned(),
        }
    }

    #[test]
    fn places_the_list_where_the_text_asks_and_nowhere_else() {
        let refs = [reference("ref-A", "A, B. (2021). *T*.")];
        let placed = place("Text.\n\n## References\n\n::: {#refs}\n:::\n", &refs)
            .unwrap()
            .unwrap();
        assert_eq!(placed, format!("Text.\n\n## References\n\n{MARK}\n"));
        assert!(place("Text.\n", &[]).unwrap().is_none());
        assert!(
            place("```\n::: {#refs}\n:::\n```\n", &[])
                .unwrap()
                .is_none()
        );
        assert!(place("Text.\n", &refs).is_err());
        assert!(place("::: {#refs}\n:::\n", &[]).is_err());
        assert!(place("::: {#refs}\nx\n:::\n", &refs).is_err());
        assert!(place("::: {#refs}\n:::\n\n::: {#refs}\n:::\n", &refs).is_err());
        assert!(place(&format!("{MARK}\n"), &[]).is_err());
    }

    #[test]
    fn writes_each_entry_as_text_with_its_prefixed_id_and_refuses_html() {
        let refs = [
            reference("ref-A", "A, B. (2021). *T & U*. <https://example.org/a>"),
            reference(
                "ref-B_3a_x",
                "2\\. B C. [doi:10.1000/x](https://doi.org/10.1000/x)",
            ),
        ];
        let html = list_html(&refs, Some("ja")).unwrap();
        assert_eq!(
            html,
            "<div id=\"ja-refs\" class=\"references csl-bib-body\" role=\"list\">\n\
             <div id=\"ja-ref-A\" class=\"csl-entry\" role=\"listitem\">A, B. (2021). <em>T &amp; U</em>. <a href=\"https://example.org/a\">https://example.org/a</a></div>\n\
             <div id=\"ja-ref-B_3a_x\" class=\"csl-entry\" role=\"listitem\">2. B C. <a href=\"https://doi.org/10.1000/x\">doi:10.1000/x</a></div>\n\
             </div>\n"
        );
        assert!(list_html(&[reference("ref-A", "A <b>B</b>")], None).is_err());
        assert!(list_html(&[reference("ref-A", "[x](javascript:alert)")], None).is_err());
        assert!(validate(&[reference("ref-A", "x"), reference("ref-A", "y")]).is_err());
        assert!(validate(&[reference("ref-a b", "x")]).is_err());
        assert!(validate(&[reference("A", "x")]).is_err());
    }

    #[test]
    fn links_to_entries_carry_the_prefix_and_must_name_one() {
        let refs = [reference("ref-A", "A.")];
        assert_eq!(
            link_target(&refs, Some("2026-10-10-ja"), "#ref-A").unwrap(),
            Some("#2026-10-10-ja-ref-A".to_owned())
        );
        assert_eq!(link_target(&refs, None, "#thm-a").unwrap(), None);
        assert!(link_target(&refs, None, "#ref-B").is_err());
    }
}
