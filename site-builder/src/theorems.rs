//! Theorem environments: theorems, lemmas, definitions, proofs, and their
//! kin, written as Quarto/Pandoc fenced divs, numbered from the text's
//! structure, and referred to by label.
//!
//! ```text
//! ::: {#thm-soundness name="Soundness"}      Theorem 2.3 (Soundness).
//! Every derivable equation …
//! :::
//!
//! ::: {.proof}                               Proof. … ∎
//! By induction on the derivation.
//! :::
//!
//! As shown in @thm-soundness, …              As shown in Theorem 2.3, …
//! ```
//!
//! The Writing Studio has the same rules (its `src/publish/theorems.ts`);
//! the shared fixtures in `tests/fixtures/theorems` hold both to one meaning:
//! the same kinds, numbers, references, ∎ and problems for the same text.
//!
//! The environments are found line by line, as Pandoc finds fenced divs: an
//! opening line at the very start of a line, outside fenced code, up to the
//! next `:::` line outside fenced code. Each opening and closing line is then
//! given to the CommonMark parser as an HTML comment line, which ends a
//! paragraph, list, or quotation exactly where a fenced div does, so the
//! whole text is parsed once (link definitions and footnotes are shared) and
//! the environment's boundaries come back as events at known offsets. The
//! author's own HTML is still refused: only events at those offsets are
//! taken as boundaries.
//!
//! Numbers are never written into the text: all numbered kinds share one
//! counter, by section (the text's highest heading level) or through the
//! text, as the file's `theoremNumbering` says. A text that has a problem (a
//! malformed opening line, an environment never closed or opened inside
//! another, a label given twice, a reference to no environment) is refused,
//! with the line and the reason, rather than published wrongly.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::ops::Range;

use pulldown_cmark::{CowStr, Event, LinkType, Tag, TagEnd};
use serde::Deserialize;

/// How a text's environments are numbered (`theoremNumbering` in its front
/// matter): within sections (2.1, 2.2, 3.1) or through the text (1, 2, 3).
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TheoremNumbering {
    #[default]
    Section,
    Document,
}

/// amsthm's three styles, and the proof's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Style {
    /// Bold heading, italic body: theorems and their kin.
    Plain,
    /// Bold heading, upright body: definitions, examples, …
    Definition,
    /// Italic heading, upright body.
    Remark,
    /// *Proof.*, upright, ∎ at the end; never numbered.
    Proof,
}

impl Style {
    const fn class(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Definition => "definition",
            Self::Remark => "remark",
            Self::Proof => "proof",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Kind {
    /// The class that names it (`.theorem`).
    pub(crate) name: &'static str,
    /// The label prefix (Quarto's where it has one); none for proofs.
    pub(crate) prefix: Option<&'static str>,
    pub(crate) label: &'static str,
    pub(crate) style: Style,
    /// Whether it is numbered (all numbered kinds share one counter).
    pub(crate) numbered: bool,
}

const fn kind(name: &'static str, prefix: &'static str, label: &'static str, style: Style) -> Kind {
    Kind {
        name,
        prefix: Some(prefix),
        label,
        style,
        numbered: true,
    }
}

/// The kinds, as the Studio knows them (`THEOREM_KINDS`).
pub(crate) const KINDS: [Kind; 13] = [
    kind("theorem", "thm", "Theorem", Style::Plain),
    kind("lemma", "lem", "Lemma", Style::Plain),
    kind("proposition", "prp", "Proposition", Style::Plain),
    kind("corollary", "cor", "Corollary", Style::Plain),
    kind("conjecture", "cnj", "Conjecture", Style::Plain),
    kind("claim", "clm", "Claim", Style::Plain),
    kind("observation", "obs", "Observation", Style::Plain),
    kind("definition", "def", "Definition", Style::Definition),
    kind("assumption", "asm", "Assumption", Style::Definition),
    kind("example", "exm", "Example", Style::Definition),
    kind("exercise", "exr", "Exercise", Style::Definition),
    kind("remark", "rem", "Remark", Style::Remark),
    Kind {
        name: "proof",
        prefix: None,
        label: "Proof",
        style: Style::Proof,
        numbered: false,
    },
];

fn by_prefix(prefix: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.prefix == Some(prefix))
}

fn by_name(name: &str) -> Option<&'static Kind> {
    KINDS.iter().find(|kind| kind.name == name)
}

const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// What follows `prefix-` in a label: runs of letters, digits and `_`,
/// joined by single `-`, `:` or `.`.
fn is_label_rest(rest: &str) -> bool {
    !rest.is_empty()
        && rest
            .split(['-', ':', '.'])
            .all(|run| !run.is_empty() && run.bytes().all(is_word))
}

/// A whole label: a known prefix of a kind that takes labels, then the rest.
fn label_kind(label: &str) -> Option<&'static Kind> {
    let (prefix, rest) = label.split_once('-')?;
    let kind = by_prefix(prefix)?;
    is_label_rest(rest).then_some(kind)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Spec {
    pub(crate) kind: &'static Kind,
    /// The full label (`thm-soundness`), if any.
    pub(crate) label: Option<String>,
    /// The name in parentheses after the number, as written.
    pub(crate) name: Option<String>,
    pub(crate) numbered: bool,
    /// Proofs: the label of what it proves (`of="thm-soundness"`).
    pub(crate) of: Option<String>,
}

/// Whether the attributes mention a theorem at all, so a line that is
/// something else (a box, say) is not called a malformed environment.
fn looks_like_theorem(inner: &str) -> bool {
    let bytes = inner.as_bytes();
    (0..bytes.len()).any(|i| {
        let after = &inner[i + 1..];
        match bytes[i] {
            b'#' | b'.' if i == 0 || bytes[i - 1].is_ascii_whitespace() => KINDS
                .iter()
                .filter_map(|kind| kind.prefix)
                .any(|prefix| after.starts_with(prefix) && after[prefix.len()..].starts_with('-')),
            _ => false,
        }
    }) || (0..bytes.len()).any(|i| {
        bytes[i] == b'.'
            && KINDS.iter().any(|kind| {
                inner[i + 1..].starts_with(kind.name)
                    && !inner[i + 1 + kind.name.len()..]
                        .bytes()
                        .next()
                        .is_some_and(is_word)
            })
    })
}

/// A theorem environment's opening line: `Some(Ok(spec))`, `Some(Err(why))`
/// for a line that is meant as one but is not right, or `None` for a line
/// that is not a theorem's opener at all. Attributes as Pandoc writes them:
/// `#id`, `.class`, and `key="value"` (with `\"` and `\\` escaped).
pub(crate) fn parse_opener(line: &str) -> Option<Result<Spec, String>> {
    let rest = line.strip_prefix(":::")?;
    let rest = rest.trim_start_matches([' ', '\t']).strip_prefix('{')?;
    let inner = rest.trim_end_matches([' ', '\t']).strip_suffix('}')?;
    let problem = |why: String| looks_like_theorem(inner).then_some(Err(why));

    let mut id: Option<&str> = None;
    let mut classes: Vec<&str> = Vec::new();
    let mut attributes: Vec<(&str, String)> = Vec::new();
    let bytes = inner.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' {
            i += 1;
            continue;
        }
        if c == b'#' || c == b'.' {
            let start = i + 1;
            let mut end = start;
            if end < bytes.len() && is_word(bytes[end]) {
                end += 1;
                while end < bytes.len()
                    && (is_word(bytes[end]) || matches!(bytes[end], b':' | b'.' | b'-'))
                {
                    end += 1;
                }
            }
            if end == start {
                return problem(format!("“{}” must be followed by a name.", c as char));
            }
            let word = &inner[start..end];
            if c == b'#' {
                if id.is_some() {
                    return Some(Err("An environment has one label (one #…).".to_owned()));
                }
                id = Some(word);
            } else {
                classes.push(word);
            }
            i = end;
            continue;
        }
        // key="value"
        let key_end = if c.is_ascii_alphabetic() {
            let mut end = i + 1;
            while end < bytes.len() && (is_word(bytes[end]) || bytes[end] == b'-') {
                end += 1;
            }
            (inner[end..].starts_with("=\"")).then_some(end)
        } else {
            None
        };
        if let Some(key_end) = key_end {
            let key = &inner[i..key_end];
            let mut value = String::new();
            let mut closed = None;
            let mut chars = inner[key_end + 2..].char_indices();
            while let Some((offset, d)) = chars.next() {
                if d == '\\' {
                    let next = inner[key_end + 2 + offset + 1..].chars().next();
                    if matches!(next, Some('"' | '\\')) {
                        value.push(next.unwrap_or_default());
                        chars.next();
                        continue;
                    }
                }
                if d == '"' {
                    closed = Some(key_end + 2 + offset + 1);
                    break;
                }
                value.push(d);
            }
            let Some(after) = closed else {
                return problem(format!("The value of {key} is not closed with \"."));
            };
            attributes.retain(|(existing, _)| *existing != key);
            attributes.push((key, value));
            i = after;
            continue;
        }
        let shown: String = inner[i..].chars().take(20).collect();
        return problem(format!(
            "“{shown}” is not an attribute (#label, .class, or name=\"…\")."
        ));
    }

    if classes.contains(&"box") {
        return None;
    }
    let from_class: Vec<&'static Kind> = classes.iter().filter_map(|c| by_name(c)).collect();
    let mut from_id = None;
    if let Some(id) = id {
        let (prefix, rest) = id.split_once('-').unwrap_or(("", id));
        match by_prefix(prefix) {
            Some(kind) => {
                if !is_label_rest(rest) {
                    return Some(Err(format!(
                        "“{id}”: after “{prefix}-”, a label has letters, digits and _, joined by single -, : or ."
                    )));
                }
                from_id = Some(kind);
            }
            None => {
                let first = from_class.first()?;
                return Some(Err(match first.prefix {
                    Some(prefix) => {
                        format!("The label “{id}” must begin with the kind’s prefix ({prefix}-).")
                    }
                    None => format!("The label “{id}”: proofs take no label."),
                }));
            }
        }
    }
    if from_id.is_none() && from_class.is_empty() {
        return None;
    }
    if from_class.len() > 1 {
        let names: Vec<String> = from_class.iter().map(|k| format!(".{}", k.name)).collect();
        return Some(Err(format!(
            "One kind per environment ({}).",
            names.join(", ")
        )));
    }
    let kind = from_id.or_else(|| from_class.first().copied())?;
    if let (Some(from_id), Some(from_class)) = (from_id, from_class.first()) {
        if from_id != *from_class {
            return Some(Err(format!(
                "The label says {}, the class says {}.",
                from_id.label, from_class.label
            )));
        }
    }
    if kind.style == Style::Proof && id.is_some() {
        return Some(Err(
            "A proof takes no label; refer to what it proves instead.".to_owned(),
        ));
    }
    if let Some((key, _)) = attributes
        .iter()
        .find(|(key, _)| *key != "name" && *key != "of")
    {
        return Some(Err(format!(
            "Unknown attribute {key}=\"…\" (name=\"…\"{} are read).",
            if kind.style == Style::Proof {
                " and of=\"…\""
            } else {
                ""
            }
        )));
    }
    let attribute = |key: &str| {
        attributes
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
    };
    let of = attribute("of");
    if let Some(of) = &of {
        if kind.style != Style::Proof {
            return Some(Err("Only a proof says what it is of (of=\"…\").".to_owned()));
        }
        if label_kind(of).is_none() {
            return Some(Err(format!(
                "of=\"{of}\" must be a label, as of=\"thm-soundness\"."
            )));
        }
    }
    let unknown: Vec<String> = classes
        .iter()
        .filter(|c| by_name(c).is_none() && **c != "unnumbered")
        .map(|c| format!(".{c}"))
        .collect();
    if !unknown.is_empty() {
        return Some(Err(format!("Unknown class {}.", unknown.join(", "))));
    }
    let name = attribute("name").filter(|name| !name.is_empty());
    Some(Ok(Spec {
        kind,
        label: id.map(str::to_owned),
        name,
        numbered: kind.numbered && !classes.contains(&"unnumbered"),
        of,
    }))
}

/// A reference `@prefix-label` at byte `at` (the `@`) of `text`, up to
/// `end`: its label, where it ends, and whether a label follows the prefix
/// at all. As in Quarto, only known prefixes are references (`@smith2020`
/// is a citation key, `[@x]` a citation), never after a letter or digit (an
/// address); a sentence's final `.` or `:` is not part of the label.
pub(crate) fn cross_ref_at(text: &str, at: usize, end: usize) -> Option<(String, usize, bool)> {
    let bytes = text.as_bytes();
    if bytes.get(at) != Some(&b'@') {
        return None;
    }
    if at > 0 && (is_word(bytes[at - 1]) || matches!(bytes[at - 1], b'[' | b'@')) {
        return None;
    }
    let mut i = at + 1;
    while i < end && bytes[i].is_ascii_lowercase() {
        i += 1;
    }
    if i >= end || bytes[i] != b'-' || by_prefix(&text[at + 1..i]).is_none() {
        return None;
    }
    i += 1;
    let start = i;
    let mut last = i;
    let mut j = i;
    loop {
        let run = j;
        while j < end && is_word(bytes[j]) {
            j += 1;
        }
        if j == run {
            break;
        }
        last = j;
        if j + 1 < end && matches!(bytes[j], b'-' | b':' | b'.') && is_word(bytes[j + 1]) {
            j += 1;
        } else {
            break;
        }
    }
    Some((text[at + 1..last].to_owned(), last, last > start))
}

// ------------------------------------------------------------ the text's structure

/// One problem, by line (from 1), with a code the Studio uses too.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Diagnostic {
    pub(crate) line: usize,
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

#[derive(Debug)]
struct Environment {
    spec: Spec,
    /// Lines (from 1) of the opening and closing fences.
    open: usize,
    close: usize,
}

/// The environments of a text, and the text given to the parser: each
/// fence line replaced by an HTML comment line of its own.
#[derive(Debug)]
pub(crate) struct Prepared {
    pub(crate) source: String,
    environments: Vec<Environment>,
    /// Byte offsets, in `source`, of the opening and closing comment lines.
    opens: Vec<usize>,
    closes: Vec<usize>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

const OPEN_MARK: &str = "<!-- theorem -->";
const CLOSE_MARK: &str = "<!-- /theorem -->";

/// A fenced code block's opening or closing line (as CommonMark reads it at
/// the margin): its character, length, and the rest of the line.
fn code_fence(line: &str) -> Option<(u8, usize, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let first = *trimmed.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let length = trimmed.bytes().take_while(|&b| b == first).count();
    (length >= 3).then(|| (first, length, &trimmed[length..]))
}

/// Tracks fenced code line by line, as the Studio's scanners do.
#[derive(Default)]
struct CodeFences(Option<(u8, usize)>);

impl CodeFences {
    /// Whether the line is code (or a fence of it).
    fn code(&mut self, line: &str) -> bool {
        let fence = code_fence(line);
        if let Some((char, length)) = self.0 {
            if let Some((c, l, rest)) = fence {
                if c == char && l >= length && rest.trim().is_empty() {
                    self.0 = None;
                }
            }
            return true;
        }
        if let Some((c, l, rest)) = fence {
            if !(c == b'`' && rest.contains('`')) {
                self.0 = Some((c, l));
                return true;
            }
        }
        false
    }
}

const fn is_closer(line: &str) -> bool {
    let bytes = line.as_bytes();
    if bytes.len() < 3 || bytes[0] != b':' || bytes[1] != b':' || bytes[2] != b':' {
        return false;
    }
    let mut i = 3;
    while i < bytes.len() {
        if bytes[i] != b' ' && bytes[i] != b'\t' {
            return false;
        }
        i += 1;
    }
    true
}

/// Finds the environments (an opener at the margin, outside fenced code, up
/// to the next `:::` line outside fenced code) and every fence problem.
pub(crate) fn prepare(source: &str) -> Prepared {
    let lines: Vec<&str> = source.split('\n').collect();
    let line_text = |n: usize| lines[n].strip_suffix('\r').unwrap_or(lines[n]);
    let mut diagnostics = Vec::new();
    let mut environments = Vec::new();
    let mut code = CodeFences::default();
    let mut n = 0;
    while n < lines.len() {
        let line = line_text(n);
        if code.code(line) || !line.starts_with(":::") {
            n += 1;
            continue;
        }
        // A malformed opening line is still meant as an environment: it is
        // reported, and its body and closing line are its own (not reported
        // again as stray), but it is drawn as the text it is.
        let spec = match parse_opener(line) {
            Some(Ok(spec)) => Some(spec),
            Some(Err(why)) => {
                diagnostics.push(Diagnostic {
                    line: n + 1,
                    code: "malformed-opener",
                    message: why,
                });
                None
            }
            None if is_closer(line) => {
                diagnostics.push(Diagnostic {
                    line: n + 1,
                    code: "stray-closer",
                    message: "This ::: closes no environment.".to_owned(),
                });
                n += 1;
                continue;
            }
            None => {
                diagnostics.push(Diagnostic {
                    line: n + 1,
                    code: "unsupported-fence",
                    message: format!(
                        "“{}” is not a theorem environment; the website shows no other ::: blocks.",
                        line.chars().take(60).collect::<String>()
                    ),
                });
                n += 1;
                continue;
            }
        };
        // Its body, up to the next ::: line outside fenced code.
        let mut inner = CodeFences::default();
        let mut close = None;
        let mut nested = Vec::new();
        for m in n + 1..lines.len() {
            let text = line_text(m);
            if inner.code(text) {
                continue;
            }
            if is_closer(text) {
                close = Some(m);
                break;
            }
            if text.starts_with(":::") {
                nested.push(m);
            }
        }
        for m in &nested {
            diagnostics.push(Diagnostic {
                line: m + 1,
                code: "nested",
                message: "An environment cannot start inside another; close the first with ::: on its own line."
                    .to_owned(),
            });
        }
        let Some(close) = close else {
            diagnostics.push(Diagnostic {
                line: n + 1,
                code: "unclosed",
                message: "This environment is never closed; end it with ::: on its own line."
                    .to_owned(),
            });
            break;
        };
        if let Some(spec) = spec {
            environments.push(Environment {
                spec,
                open: n + 1,
                close: close + 1,
            });
        }
        n = close + 1;
    }

    // The text for the parser: each fence line as a comment line.
    let mut prepared = String::with_capacity(source.len() + 16 * environments.len());
    let mut opens = Vec::new();
    let mut closes = Vec::new();
    let mut fences = environments
        .iter()
        .flat_map(|e| [(e.open, true), (e.close, false)])
        .peekable();
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            prepared.push('\n');
        }
        match fences.peek() {
            Some(&(fence, open)) if fence == index + 1 => {
                fences.next();
                if open {
                    opens.push(prepared.len());
                    prepared.push_str(OPEN_MARK);
                } else {
                    closes.push(prepared.len());
                    prepared.push_str(CLOSE_MARK);
                }
            }
            _ => prepared.push_str(line),
        }
    }
    Prepared {
        source: prepared,
        environments,
        opens,
        closes,
        diagnostics,
    }
}

impl Prepared {
    /// Whether an HTML event at this offset is one of the fences.
    pub(crate) fn is_fence(&self, offset: usize) -> bool {
        self.opens.binary_search(&offset).is_ok() || self.closes.binary_search(&offset).is_ok()
    }
}

// ------------------------------------------------------------ numbering and references

struct Numbered {
    number: Option<String>,
    /// How a reference reads: "Theorem 2.3", or "Theorem (Soundness)" unnumbered.
    reference: String,
}

/// Numbers the environments, given the heading levels before each (in text
/// order: `outline` holds `Some(level)` for a heading and `None` for an
/// environment). Sections are the text's top level of headings; an
/// environment before the first is in section 0, as LaTeX numbers it; a
/// text without headings is numbered straight through.
fn number(outline: &[Option<u8>], specs: &[&Spec], numbering: TheoremNumbering) -> Vec<Numbered> {
    let section_level = outline.iter().flatten().min().copied();
    let by_section = numbering == TheoremNumbering::Section && section_level.is_some();
    let mut section = 0;
    let mut counter = 0;
    let mut specs = specs.iter();
    let mut out = Vec::new();
    for entry in outline {
        match entry {
            Some(level) => {
                if by_section && Some(*level) == section_level {
                    section += 1;
                    counter = 0;
                }
            }
            None => {
                let Some(spec) = specs.next() else { break };
                let number = spec.numbered.then(|| {
                    counter += 1;
                    if by_section {
                        format!("{section}.{counter}")
                    } else {
                        counter.to_string()
                    }
                });
                let reference = match (&number, &spec.name) {
                    (Some(number), _) => format!("{} {number}", spec.kind.label),
                    (None, Some(name)) => format!("{} ({name})", spec.kind.label),
                    (None, None) => spec.kind.label.to_owned(),
                };
                out.push(Numbered { number, reference });
            }
        }
    }
    out
}

enum Target<'a> {
    Found(&'a Numbered),
    Duplicate,
    Missing,
}

/// What the text's environments and references came to, for the shared
/// fixtures: the same facts the Studio's analysis gives.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Analysis {
    pub(crate) environments: Vec<EnvironmentFacts>,
    pub(crate) references: Vec<ReferenceFacts>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct EnvironmentFacts {
    pub(crate) line: usize,
    pub(crate) kind: &'static str,
    pub(crate) label: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) number: Option<String>,
    /// The heading as the reader sees it, without the name: "Theorem 2.3", "Proof of Lemma 1".
    pub(crate) head: String,
    pub(crate) proof_of: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ReferenceFacts {
    pub(crate) line: usize,
    pub(crate) label: String,
    /// What it reads as, or None when it resolves to nothing.
    pub(crate) text: Option<String>,
}

/// How the environments are drawn on a page.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TheoremOptions<'a> {
    pub(crate) numbering: TheoremNumbering,
    /// Put before each label in the page's identifiers, keeping them unique
    /// when several texts share a page (as footnotes are).
    pub(crate) id_prefix: Option<&'a str>,
    /// The file's line on which the text begins (after its front matter),
    /// so a problem is reported by the file's line.
    pub(crate) first_line: usize,
}

/// How a name is set after the number: in Japanese (or Chinese) in full-width
/// parentheses, "Definition 2.3 （定義的拡張の保守性）" (the site's CSS sets them
/// proportionally); otherwise "Definition 2.3 (Conservativity)". Decided by the name's
/// first character; the same rule as the Studio's (studio/src/publish/theorems.ts,
/// `nameParentheses`).
pub fn name_parentheses(name: &str) -> (&'static str, &'static str, &'static str) {
    let wide = name.chars().next().is_some_and(|c| {
        matches!(c, '\u{3000}'..='\u{30ff}' | '\u{3400}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{ff00}'..='\u{ffef}')
    });
    if wide {
        (" ", "（", "）")
    } else {
        (" ", "(", ")")
    }
}

/// A part of an environment's name: text, or a formula's TeX.
#[derive(Debug, PartialEq, Eq)]
pub enum NamePart<'a> {
    Text(String),
    Math(&'a str),
}

/// A name's text and formulas, in order: "Conservativity of $T'$ over $T$" is
/// text, math "T'", text, math "T". A formula is $…$ with something other
/// than a space just inside each $ (as in the text's own formulas); \$ is a
/// dollar sign; $$, and an unclosed $, stay text. The name is the attribute's
/// value, already unescaped; KaTeX alone reads the TeX. The same rule as the
/// Studio's (studio/src/publish/theorems.ts, `nameParts`).
pub fn name_parts_of(name: &str) -> Vec<NamePart<'_>> {
    let bytes = name.as_bytes();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0;
    let space = |b: u8| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r';
    while i < bytes.len() {
        if bytes[i] == b'\\' && bytes.get(i + 1) == Some(&b'$') {
            text.push('$');
            i += 2;
            continue;
        }
        if bytes[i] == b'$' {
            if bytes.get(i + 1) == Some(&b'$') {
                text.push_str("$$");
                i += 2;
                continue;
            }
            if bytes.get(i + 1).is_some_and(|&b| !space(b)) {
                let mut j = i + 1;
                let mut close = None;
                while j < bytes.len() {
                    if bytes[j] == b'\\' {
                        j += 2;
                        continue;
                    }
                    if bytes[j] == b'$' {
                        if !space(bytes[j - 1]) && bytes.get(j + 1) != Some(&b'$') {
                            close = Some(j);
                        }
                        break;
                    }
                    j += 1;
                }
                if let Some(close) = close.filter(|&c| c > i + 1) {
                    if !text.is_empty() {
                        out.push(NamePart::Text(std::mem::take(&mut text)));
                    }
                    out.push(NamePart::Math(&name[i + 1..close]));
                    i = close + 1;
                    continue;
                }
            }
        }
        let c = name[i..].chars().next().unwrap_or_default();
        text.push(c);
        i += c.len_utf8().max(1);
    }
    if !text.is_empty() {
        out.push(NamePart::Text(text));
    }
    out
}

/// A name as HTML between its parentheses: its text escaped, its formulas
/// typeset by KaTeX (as the text's are); a formula KaTeX refuses is shown as
/// written, marked, and the rest of the name stays. A formula at either end
/// is kept on one line with its parenthesis (span.theorem-name-tail), so a
/// long name never leaves ")" alone at the start of a line. The same as the
/// Studio's (`nameGlued`).
fn name_html(name: &str, open: &str, close: &str) -> String {
    let parts = name_parts_of(name);
    let first_math = matches!(parts.first(), Some(NamePart::Math(_)));
    let last_math = matches!(parts.last(), Some(NamePart::Math(_)));
    let mut html = name_parts_html(parts);
    if html.len() == 1 && first_math {
        return format!(
            r#"<span class="theorem-name-tail">{open}{}{close}</span>"#,
            html[0]
        );
    }
    if first_math {
        html[0] = format!(
            r#"<span class="theorem-name-tail">{open}{}</span>"#,
            html[0]
        );
    } else {
        html.insert(0, open.to_owned());
    }
    if last_math {
        let last = html.len() - 1;
        html[last] = format!(
            r#"<span class="theorem-name-tail">{}{close}</span>"#,
            html[last]
        );
    } else {
        html.push(close.to_owned());
    }
    html.concat()
}

fn name_parts_html(parts: Vec<NamePart<'_>>) -> Vec<String> {
    let options = katex::Opts::builder()
        .display_mode(false)
        .throw_on_error(true)
        .trust(false)
        .build();
    parts
        .into_iter()
        .map(|part| match part {
            NamePart::Text(text) => escape(&text),
            NamePart::Math(tex) => options
                .as_ref()
                .ok()
                .and_then(|o| katex::render_with_opts(tex, o).ok())
                .unwrap_or_else(|| {
                    format!(
                        r#"<span class="theorem-name-error">{}</span>"#,
                        escape(&format!("${tex}$"))
                    )
                }),
        })
        .collect()
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

const QED: &str = r#"<span class="qed" role="img" aria-label="End of proof">∎</span>"#;

fn line_of(source: &str, offset: usize) -> usize {
    source.as_bytes()[..offset.min(source.len())]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

/// Whether a proof's last paragraph already ends with its mark: ∎, or
/// `\blacksquare` at the end of a final formula. (`\qed` written as text at
/// the very end is replaced by the mark instead: see `strip_text_qed`.)
fn ends_with_mark(paragraph: &str) -> bool {
    let mut text = paragraph.trim_end();
    for delimiter in ["$$", "\\]", "\\)", "$"] {
        if let Some(stripped) = text.strip_suffix(delimiter) {
            text = stripped.trim_end();
            break;
        }
    }
    text.ends_with('∎') || text.ends_with("\\blacksquare")
}

/// The text without a final `\qed` (or `\qedhere`), written as text.
fn strip_text_qed(text: &str) -> Option<&str> {
    let trimmed = text.trim_end();
    trimmed
        .strip_suffix("\\qedhere")
        .or_else(|| trimmed.strip_suffix("\\qed"))
        .map(str::trim_end)
}

/// Whether a character is Japanese or Chinese (which has no italic).
const fn is_cjk(c: char) -> bool {
    matches!(c, '\u{3000}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}' | '\u{ff00}'..='\u{ffef}')
}

/// Applies the environments to the parsed text: their boundaries, headings,
/// numbers, references, and ∎. `events` holds each event with its range in
/// `prepared.source` (none for events the renderer made itself).
pub(crate) fn apply<'a>(
    events: Vec<(Event<'a>, Option<Range<usize>>)>,
    prepared: &Prepared,
    options: TheoremOptions<'_>,
) -> (Vec<Event<'a>>, Analysis) {
    let source = prepared.source.as_str();
    let mut diagnostics = prepared.diagnostics.clone();
    // The comment lines' own block events: nothing of the author's.
    let events: Vec<_> = events
        .into_iter()
        .filter(|(event, range)| {
            !(matches!(
                event,
                Event::Start(Tag::HtmlBlock) | Event::End(TagEnd::HtmlBlock)
            ) && is_fence_at(prepared, range.as_ref()))
        })
        .collect();

    // The outline: headings outside environments, at the top level, and the environments.
    let mut outline = Vec::new();
    let mut depth = 0_usize;
    let mut inside = false;
    for (event, range) in &events {
        match event {
            Event::Html(_) if is_fence_at(prepared, range.as_ref()) => {
                if prepared
                    .opens
                    .binary_search(&range_start(range.as_ref()))
                    .is_ok()
                {
                    outline.push(None);
                    inside = true;
                } else {
                    inside = false;
                }
            }
            Event::Start(tag) => {
                if let Tag::Heading { level, .. } = tag {
                    if depth == 0 && !inside {
                        outline.push(Some(*level as u8));
                    }
                }
                depth += 1;
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let specs: Vec<&Spec> = prepared.environments.iter().map(|e| &e.spec).collect();
    let numbered = number(&outline, &specs, options.numbering);

    // Labels: a label given twice resolves to neither.
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut duplicates: HashSet<&str> = HashSet::new();
    for (index, environment) in prepared.environments.iter().enumerate() {
        if let Some(label) = &environment.spec.label {
            if seen.insert(label.as_str(), index).is_some() {
                duplicates.insert(label.as_str());
            }
        }
    }
    for environment in &prepared.environments {
        if let Some(label) = &environment.spec.label {
            if duplicates.contains(label.as_str()) {
                diagnostics.push(Diagnostic {
                    line: environment.open,
                    code: "duplicate-label",
                    message: format!(
                        "“{label}” labels more than one environment; give each its own label."
                    ),
                });
            }
        }
    }
    let target = |label: &str| {
        if duplicates.contains(label) {
            Target::Duplicate
        } else {
            match seen.get(label) {
                Some(&index) => Target::Found(&numbered[index]),
                None => Target::Missing,
            }
        }
    };
    let id_of = |label: &str| match options.id_prefix {
        Some(prefix) => format!("{prefix}-{label}"),
        None => label.to_owned(),
    };

    let mut analysis = Analysis::default();
    let mut heads = Vec::new();
    for (index, environment) in prepared.environments.iter().enumerate() {
        let spec = &environment.spec;
        let head = if spec.kind.style == Style::Proof {
            match &spec.of {
                Some(of) => match target(of) {
                    Target::Found(n) => format!("Proof of {}", n.reference),
                    Target::Duplicate | Target::Missing => {
                        diagnostics.push(Diagnostic {
                            line: environment.open,
                            code: "undefined-proof-target",
                            message: format!(
                                "of=\"{of}\" names no environment (or one labelled twice)."
                            ),
                        });
                        format!("Proof of {of}")
                    }
                },
                None => "Proof".to_owned(),
            }
        } else {
            match &numbered[index].number {
                Some(number) => format!("{} {number}", spec.kind.label),
                None => spec.kind.label.to_owned(),
            }
        };
        analysis.environments.push(EnvironmentFacts {
            line: environment.open,
            kind: spec.kind.name,
            label: spec.label.clone(),
            name: spec.name.clone(),
            number: numbered[index].number.clone(),
            head: head.clone(),
            proof_of: spec.of.clone(),
        });
        let tag = if matches!(spec.kind.style, Style::Plain | Style::Definition) {
            "strong"
        } else {
            "em"
        };
        let name = spec
            .name
            .as_ref()
            .map(|name| {
                let (before, open, close) = name_parentheses(name);
                format!(
                    r#"{before}<span class="theorem-name" data-name="{}">{}</span>"#,
                    escape(name),
                    name_html(name, open, close)
                )
            })
            .unwrap_or_default();
        heads.push(format!(
            r#"<span class="theorem-head"><{tag}>{}</{tag}>{name}<{tag}>.</{tag}></span> "#,
            escape(&head)
        ));
    }

    // The events: boundaries, headings, references, and ∎.
    let mut out: Vec<Event<'a>> = Vec::with_capacity(events.len() + 8 * heads.len());
    let mut environment: Option<usize> = None;
    let mut env_count = 0;
    let mut depth = 0_usize;
    let mut link_depth = 0_usize;
    let mut verbatim_depth = 0_usize;
    // The depths of the autolinks open (their ends look like any link's).
    let mut autolinks: Vec<usize> = Vec::new();
    // Inside an environment: where its last top-level paragraph began in `out` and in the source.
    let mut last_paragraph: Option<(usize, Range<usize>)> = None;
    let mut iter = events.into_iter().peekable();
    while let Some((event, range)) = iter.next() {
        let at_fence = matches!(event, Event::Html(_)) && is_fence_at(prepared, range.as_ref());
        if at_fence {
            if prepared
                .opens
                .binary_search(&range_start(range.as_ref()))
                .is_ok()
            {
                let index = env_count;
                env_count += 1;
                let spec = &prepared.environments[index].spec;
                let id = spec
                    .label
                    .as_ref()
                    .map(|label| format!(r#" id="{}""#, escape(&id_of(label))))
                    .unwrap_or_default();
                out.push(Event::Html(
                    format!(
                        "<div class=\"theorem theorem-{}\" data-kind=\"{}\"{id}>\n",
                        spec.kind.style.class(),
                        spec.kind.name
                    )
                    .into(),
                ));
                // Run in: the heading begins the first paragraph; otherwise a paragraph of its own.
                let empty = matches!(iter.peek(), Some((Event::Html(_), next)) if is_fence_at(prepared, next.as_ref()));
                if matches!(iter.peek(), Some((Event::Start(Tag::Paragraph), _))) {
                    let (start, range) =
                        iter.next().unwrap_or((Event::Start(Tag::Paragraph), None));
                    out.push(start);
                    out.push(Event::InlineHtml(heads[index].clone().into()));
                    depth += 1;
                    if let Some(range) = range {
                        last_paragraph = Some((out.len(), range));
                    }
                } else {
                    let qed = if empty && spec.kind.style == Style::Proof {
                        QED
                    } else {
                        ""
                    };
                    out.push(Event::Html(
                        format!("<p>{}{qed}</p>\n", heads[index]).into(),
                    ));
                    last_paragraph = None;
                }
                environment = Some(index);
            } else if let Some(index) = environment.take() {
                let spec = &prepared.environments[index].spec;
                let empty = matches!(out.last(), Some(Event::Html(html)) if html.starts_with("<p><span class=\"theorem-head\">"));
                if spec.kind.style == Style::Proof && !empty {
                    close_proof(&mut out, last_paragraph.take(), source);
                }
                out.push(Event::Html("</div>\n".into()));
            }
            continue;
        }
        match &event {
            Event::Start(tag) => {
                if environment.is_some() && depth == 0 {
                    last_paragraph = match (tag, &range) {
                        (Tag::Paragraph, Some(range)) => Some((out.len() + 1, range.clone())),
                        _ => None,
                    };
                }
                match tag {
                    // An address written <…> is the address, never a reference.
                    Tag::Link {
                        link_type: LinkType::Autolink | LinkType::Email,
                        ..
                    } => {
                        verbatim_depth += 1;
                        autolinks.push(depth + 1);
                    }
                    Tag::Link { .. } => link_depth += 1,
                    Tag::CodeBlock(_) | Tag::Image { .. } => verbatim_depth += 1,
                    _ => {}
                }
                depth += 1;
            }
            Event::End(end) => {
                match end {
                    TagEnd::Link if autolinks.last() == Some(&depth) => {
                        autolinks.pop();
                        verbatim_depth = verbatim_depth.saturating_sub(1);
                    }
                    TagEnd::Link => link_depth = link_depth.saturating_sub(1),
                    TagEnd::CodeBlock | TagEnd::Image => {
                        verbatim_depth = verbatim_depth.saturating_sub(1);
                    }
                    _ => {}
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
        // References: in plain text, as written (not escaped, not in code or an image description).
        if let (Event::Text(text), Some(range)) = (&event, &range) {
            if verbatim_depth == 0
                && text.contains('@')
                && source.get(range.clone()) == Some(text.as_ref())
            {
                let mut rest_start = 0;
                let mut i = 0;
                let bytes = text.as_bytes();
                let mut pieces: Vec<Event<'a>> = Vec::new();
                while i < bytes.len() {
                    if bytes[i] != b'@' || escaped(source, range.start + i) {
                        i += 1;
                        continue;
                    }
                    let Some((label, to, valid)) = cross_ref_at(source, range.start + i, range.end)
                    else {
                        i += 1;
                        continue;
                    };
                    let line = line_of(source, range.start + i);
                    let resolved = if valid { Some(target(&label)) } else { None };
                    let (html, read) = match resolved {
                        Some(Target::Found(n)) => {
                            let html = if link_depth > 0 {
                                escape(&n.reference)
                            } else {
                                format!(
                                    r##"<a class="theorem-ref" href="#{}">{}</a>"##,
                                    escape(&id_of(&label)),
                                    escape(&n.reference)
                                )
                            };
                            (html, Some(n.reference.clone()))
                        }
                        other => {
                            let (code, message) = match other {
                                None => (
                                    "incomplete-reference",
                                    format!(
                                        "“@{label}” is not a whole reference: a label follows the prefix (as @thm-soundness)."
                                    ),
                                ),
                                Some(Target::Duplicate) => (
                                    "ambiguous-reference",
                                    format!(
                                        "“{label}” labels more than one environment; give each its own label."
                                    ),
                                ),
                                _ => (
                                    "undefined-reference",
                                    format!("No environment is labelled “{label}”."),
                                ),
                            };
                            diagnostics.push(Diagnostic {
                                line,
                                code,
                                message: message.clone(),
                            });
                            (
                                format!(
                                    r#"<span class="theorem-ref-missing" title="{}">@{}</span>"#,
                                    escape(&message),
                                    escape(&label)
                                ),
                                None,
                            )
                        }
                    };
                    analysis.references.push(ReferenceFacts {
                        line,
                        label,
                        text: read,
                    });
                    if i > rest_start {
                        pieces.push(Event::Text(text[rest_start..i].to_owned().into()));
                    }
                    pieces.push(Event::InlineHtml(html.into()));
                    i = to - range.start;
                    rest_start = i;
                }
                if !pieces.is_empty() {
                    if rest_start < text.len() {
                        pieces.push(Event::Text(text[rest_start..].to_owned().into()));
                    }
                    out.extend(pieces);
                    continue;
                }
            }
        }
        out.push(event);
    }

    diagnostics.sort_by_key(|d| d.line);
    analysis.diagnostics = diagnostics;
    (out, analysis)
}

fn range_start(range: Option<&Range<usize>>) -> usize {
    range.map_or(usize::MAX, |r| r.start)
}

fn is_fence_at(prepared: &Prepared, range: Option<&Range<usize>>) -> bool {
    range.is_some_and(|r| prepared.is_fence(r.start))
}

/// Whether the character at `offset` follows an odd number of backslashes.
fn escaped(source: &str, offset: usize) -> bool {
    source.as_bytes()[..offset]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        == 1
}

/// ∎ at the end of a proof: at the end of its last paragraph, unless written
/// there already; otherwise (a list, a quotation, code, a table) on a line of
/// its own after it.
fn close_proof(
    out: &mut Vec<Event<'_>>,
    last_paragraph: Option<(usize, Range<usize>)>,
    source: &str,
) {
    let ends_paragraph = matches!(out.last(), Some(Event::End(TagEnd::Paragraph)));
    match last_paragraph {
        Some((_, range)) if ends_paragraph => {
            if ends_with_mark(&source[range]) {
                return;
            }
            let end = out.len() - 1;
            if let Some(Event::Text(text)) = out.get(end - 1) {
                if let Some(stripped) = strip_text_qed(text) {
                    let stripped = stripped.to_owned();
                    out[end - 1] = Event::Text(stripped.into());
                }
            }
            out.insert(end, Event::InlineHtml(QED.into()));
        }
        _ => out.push(Event::Html(
            format!("<p class=\"qed-line\">{QED}</p>\n").into(),
        )),
    }
}

/// Wraps runs of Japanese and Chinese text in plain-style environments in
/// `<span class="upright">`.
pub(crate) fn upright_cjk(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut out = Vec::with_capacity(events.len());
    let mut plain = false;
    let mut verbatim = 0_usize;
    for event in events {
        match &event {
            Event::Html(html) if html.starts_with("<div class=\"theorem theorem-") => {
                plain = html.starts_with("<div class=\"theorem theorem-plain\"");
            }
            Event::Html(html) if html.as_ref() == "</div>\n" => plain = false,
            Event::Start(Tag::CodeBlock(_) | Tag::Image { .. }) => verbatim += 1,
            Event::End(TagEnd::CodeBlock | TagEnd::Image) => verbatim = verbatim.saturating_sub(1),
            Event::Text(text) if plain && verbatim == 0 && text.chars().any(is_cjk) => {
                let mut html = String::new();
                let mut run = false;
                for c in text.chars() {
                    if is_cjk(c) != run {
                        html.push_str(if run {
                            "</span>"
                        } else {
                            "<span class=\"upright\">"
                        });
                        run = !run;
                    }
                    let _ = write!(html, "{}", escape(c.encode_utf8(&mut [0; 4])));
                }
                if run {
                    html.push_str("</span>");
                }
                out.push(Event::InlineHtml(CowStr::from(html)));
                continue;
            }
            _ => {}
        }
        out.push(event);
    }
    out
}

#[cfg(test)]
mod name_tests {
    use super::{NamePart, name_html, name_parts_of};

    /// The same cases as the Studio's (studio/test/theorems.test.ts, "formulas in a name").
    #[test]
    fn names_split_like_the_studio() {
        let text = |s: &str| NamePart::Text(s.to_owned());
        let cases: Vec<(&str, Vec<NamePart>)> = vec![
            (
                "Conservativity of $T'$ over $T$",
                vec![
                    text("Conservativity of "),
                    NamePart::Math("T'"),
                    text(" over "),
                    NamePart::Math("T"),
                ],
            ),
            (
                "The relation $R \\subseteq X \\times X$",
                vec![
                    text("The relation "),
                    NamePart::Math("R \\subseteq X \\times X"),
                ],
            ),
            (
                "$\\lambda$計算の合流性",
                vec![NamePart::Math("\\lambda"), text("計算の合流性")],
            ),
            (
                "it's \\$5 and $\\{x\\}$",
                vec![text("it's $5 and "), NamePart::Math("\\{x\\}")],
            ),
            (
                "costs $5 and $ 6 $ or $$x$$",
                vec![text("costs $5 and $ 6 $ or $$x$$")],
            ),
            ("an unclosed $x", vec![text("an unclosed $x")]),
            ("$a$$b$", vec![text("$a$$b$")]),
        ];
        for (name, parts) in cases {
            assert_eq!(name_parts_of(name), parts, "{name}");
        }
    }

    #[test]
    fn a_formula_katex_refuses_stays_as_written() {
        let html = name_html("Broken $\\frac{1}{$ stays", "(", ")");
        assert_eq!(
            html,
            r#"(Broken <span class="theorem-name-error">$\frac{1}{$</span> stays)"#
        );
        let tail = name_html("Of $T'$", "(", ")");
        assert!(
            tail.starts_with(r#"(Of <span class="theorem-name-tail"><span class="katex">"#)
                && tail.ends_with(")</span>"),
            "{tail}"
        );
    }
}
