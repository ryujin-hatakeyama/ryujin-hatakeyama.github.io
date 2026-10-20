//! The theorem fixtures shared with the Writing Studio
//! (`tests/fixtures/theorems`): each Markdown file with the meaning both
//! implementations must give it, in a JSON file beside it. The meaning is
//! read twice here, from the renderer's analysis and back from the HTML it
//! produced, so a difference in either is caught; the Studio reads the same
//! files with its own parser and its own DOM.
//!
//! `THEOREM_FIXTURES_PRINT=1 cargo test theorem_fixtures -- --nocapture`
//! prints what the renderer makes of each file, for writing a new one (the
//! expectation is then checked by hand, never copied blindly).

use std::fs;
use std::path::Path;

use pulldown_cmark::HeadingLevel;
use serde_json::{Value, json};

use crate::content::{MarkdownRules, render_markdown_analyzed};
use crate::theorems::{Analysis, TheoremNumbering, TheoremOptions};

/// What an environment shows, read back from the HTML.
fn environments_in_html(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(start) = rest.find("<div class=\"theorem theorem-") {
        rest = &rest[start..];
        let open_end = rest.find('>').unwrap_or(rest.len());
        let open = &rest[..open_end];
        let attribute = |name: &str| {
            let key = format!(" {name}=\"");
            open.find(&key).map(|i| {
                let value = &open[i + key.len()..];
                value[..value.find('"').unwrap_or(value.len())].to_owned()
            })
        };
        let end = rest.find("</div>\n").unwrap_or(rest.len());
        let body = &rest[..end];
        let head_html = body
            .split_once("<span class=\"theorem-head\">")
            .map_or("", |(_, h)| h.split_once("</span>").map_or(h, |(h, _)| h));
        let tag = if head_html.starts_with("<strong>") {
            "strong"
        } else {
            "em"
        };
        let close = format!("</{tag}>");
        let (head, after) = head_html
            .trim_start_matches(&format!("<{tag}>"))
            .split_once(&close)
            .unwrap_or(("", ""));
        let name = after
            .split_once(&format!("<{tag}>.</{tag}>"))
            .map(|(name, _)| name.trim())
            .filter(|name| !name.is_empty())
            .map(|name| unescape(name.trim_start_matches('(').trim_end_matches(')')));
        out.push(json!({
            "kind": attribute("data-kind"),
            "label": attribute("id"),
            "head": unescape(head),
            "name": name,
            "qed": body.matches('∎').count(),
        }));
        rest = &rest[end..];
    }
    out
}

/// The references, read back from the HTML: links, and those that resolve to nothing.
fn references_in_html(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut rest = html;
    loop {
        let link = rest.find("<a class=\"theorem-ref\" href=\"#");
        let missing = rest.find("<span class=\"theorem-ref-missing\"");
        let at = match (link, missing) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };
        rest = &rest[at..];
        if rest.starts_with("<a ") {
            let href = &rest["<a class=\"theorem-ref\" href=\"#".len()..];
            let label = &href[..href.find('"').unwrap_or(0)];
            let text = &href[href.find('>').map_or(0, |i| i + 1)..href.find("</a>").unwrap_or(0)];
            out.push(json!({ "label": label, "text": unescape(text) }));
        } else {
            let text =
                &rest[rest.find('>').map_or(0, |i| i + 1)..rest.find("</span>").unwrap_or(0)];
            out.push(json!({ "label": unescape(text.trim_start_matches('@')), "text": null }));
        }
        rest = &rest[1..];
    }
    out
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn render(source: &str, numbering: TheoremNumbering) -> (String, Analysis) {
    let rules = MarkdownRules {
        math: true,
        highest_heading: HeadingLevel::H2,
        footnotes: true,
        footnote_prefix: None,
        inline_languages: true,
        music: None,
        theorems: Some(TheoremOptions {
            numbering,
            id_prefix: None,
            first_line: 1,
        }),
    };
    let (html, analysis) = render_markdown_analyzed(source, rules).expect("renders");
    (html, analysis.expect("theorems are read"))
}

#[test]
fn theorem_fixtures_mean_what_their_json_says() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/theorems");
    let mut names: Vec<_> = fs::read_dir(&directory)
        .expect("fixtures")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "md").then_some(path)
        })
        .collect();
    names.sort();
    assert!(names.len() >= 20, "the fixtures are missing");
    let print = std::env::var_os("THEOREM_FIXTURES_PRINT").is_some();
    let mut failures = Vec::new();
    for path in names {
        let source = fs::read_to_string(&path).expect("fixture");
        let expected_path = path.with_extension("json");
        let expected: Value = fs::read_to_string(&expected_path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_else(|| json!({}));
        let numbering = match expected["numbering"].as_str() {
            Some("document") => TheoremNumbering::Document,
            _ => TheoremNumbering::Section,
        };
        let (html, analysis) = render(&source, numbering);
        let in_html = environments_in_html(&html);
        let environments: Vec<Value> = analysis
            .environments
            .iter()
            .zip(&in_html)
            .map(|(e, h)| {
                json!({
                    "line": e.line, "kind": e.kind, "label": e.label, "name": e.name,
                    "number": e.number, "head": e.head, "proofOf": e.proof_of, "qed": h["qed"],
                })
            })
            .collect();
        let references: Vec<Value> = analysis
            .references
            .iter()
            .map(|r| json!({ "line": r.line, "label": r.label, "text": r.text }))
            .collect();
        let diagnostics: Vec<Value> = analysis
            .diagnostics
            .iter()
            .map(|d| json!({ "line": d.line, "code": d.code }))
            .collect();
        let actual = json!({
            "numbering": if numbering == TheoremNumbering::Document { "document" } else { "section" },
            "environments": environments,
            "references": references,
            "diagnostics": diagnostics,
        });
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if print {
            println!(
                "== {name}\n{}",
                serde_json::to_string_pretty(&actual).unwrap_or_default()
            );
            continue;
        }
        // `linked: false` (a reference inside a link's text, which is not a link
        // itself) is read from the HTML below, not from the analysis.
        let mut comparable = expected.clone();
        for reference in comparable["references"]
            .as_array_mut()
            .into_iter()
            .flatten()
        {
            if let Some(object) = reference.as_object_mut() {
                object.remove("linked");
            }
        }
        if actual != comparable {
            failures.push(format!(
                "{name}: expected\n{}\nbut the renderer gives\n{}",
                serde_json::to_string_pretty(&expected).unwrap_or_default(),
                serde_json::to_string_pretty(&actual).unwrap_or_default()
            ));
            continue;
        }
        // The HTML says the same: every environment as analysed, and every reference.
        assert_eq!(
            in_html.len(),
            analysis.environments.len(),
            "{name}: environments in the HTML"
        );
        for (e, h) in analysis.environments.iter().zip(&in_html) {
            assert_eq!(h["kind"], e.kind, "{name}");
            assert_eq!(h["label"], json!(e.label), "{name}");
            assert_eq!(h["head"], e.head, "{name}");
            assert_eq!(h["name"], json!(e.name), "{name}");
        }
        let linked: Vec<Value> = references_in_html(&html);
        let expected_links: Vec<Value> = expected["references"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|r| r["linked"] != json!(false))
            .map(|r| json!({ "label": r["label"], "text": r["text"] }))
            .collect();
        assert_eq!(linked, expected_links, "{name}: references in the HTML");
        // A text without problems never shows a fence line (one with problems is refused).
        // (An escaped \::: is text by intent.)
        if analysis.diagnostics.is_empty() && !source.contains("\\:::") {
            let mut prose = html.clone();
            while let (Some(start), Some(end)) = (prose.find("<code"), prose.find("</code>")) {
                prose.replace_range(start..end + "</code>".len(), "");
            }
            assert!(
                !prose.contains(":::"),
                "{name}: a fence line was printed: {html}"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
#[ignore = "prints HTML for inspection"]
fn print_fixture_html() {
    let name = std::env::var("FIXTURE").unwrap_or_default();
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/theorems")
        .join(name);
    let (html, _) = render(
        &fs::read_to_string(path).expect("fixture"),
        TheoremNumbering::Section,
    );
    println!("{html}");
}
