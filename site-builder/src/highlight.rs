//! Fenced code, highlighted when the site is built, with irosashi: the
//! TextMate grammars of VS Code and Shiki, in Rust. A fence that names a
//! language becomes static HTML in Shiki's form, each token coloured by the
//! site's light theme and, in a CSS variable, its dark one, which
//! `assets/styles/site.css` switches to with the site's theme. Nothing is
//! highlighted in the browser.
//!
//! A fence with no language, or a plain-text one (`text`), stays plain code.
//! A name that is neither the site's own (below) nor a bundled grammar's
//! stops the build, so a mistyped language is never quietly left plain.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use anyhow::{Result, anyhow, bail};
use irosashi::{CodeToHtmlOptions, Highlighter, HighlighterBuilder};

/// The site's code themes, after its paper, ink, and teal (`site-builder/themes/`).
const LIGHT: &str = include_str!("../themes/code-light.json");
const DARK: &str = include_str!("../themes/code-dark.json");

/// The site's names for languages, where the bundled grammar has another.
const ALIASES: &[(&str, &str)] = &[
    ("hs", "haskell"),
    ("ml", "ocaml"),
    // BER MetaOCaml is OCaml with staging brackets: OCaml's grammar reads it.
    ("metaocaml", "ocaml"),
    ("ber-metaocaml", "ocaml"),
    // Rocq's grammar (from VsRocq, formerly VsCoq) keeps its former name.
    ("rocq", "coq"),
    ("lean4", "lean"),
    ("rs", "rust"),
    ("ts", "typescript"),
    ("js", "javascript"),
    ("py", "python"),
    ("bash", "shellscript"),
    ("sh", "shellscript"),
    ("shell", "shellscript"),
    ("tex", "latex"),
    ("yml", "yaml"),
];

/// Fence names that ask for plain text.
const PLAIN: &[&str] = &["text", "txt", "plain", "plaintext"];

/// One highlighter for the whole build: grammars are read once, when first
/// needed, and then shared by every page.
static HIGHLIGHTER: LazyLock<Result<Highlighter, String>> = LazyLock::new(|| {
    HighlighterBuilder::embedded()
        .theme("code-light", LIGHT.as_bytes())
        .and_then(|builder| builder.theme("code-dark", DARK.as_bytes()))
        .and_then(HighlighterBuilder::build)
        .map_err(|error| error.to_string())
});

fn highlighter() -> Result<&'static Highlighter> {
    HIGHLIGHTER
        .as_ref()
        .map_err(|error| anyhow!("the code highlighter could not start: {error}"))
}

/// The grammar a fence's info string names (its first word, in any case),
/// or `None` for plain text.
fn grammar(info: &str) -> Result<Option<String>> {
    let Some(word) = info.split_whitespace().next() else {
        return Ok(None);
    };
    let name = word.to_ascii_lowercase();
    if PLAIN.contains(&name.as_str()) {
        return Ok(None);
    }
    let name = ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .map_or(name.as_str(), |(_, grammar)| grammar);
    let registry = highlighter()?.registry();
    match registry.resolve_language(name) {
        Some(grammar) => Ok(Some(grammar.to_owned())),
        None => bail!(
            "unknown fenced-code language {word:?}; use a supported language name or `text` for intentionally plain code"
        ),
    }
}

/// A fenced code block as highlighted HTML, or `None` when it is plain text
/// (left as the Markdown renderer writes it). `source` is the block's text,
/// with its final newline; the HTML holds it unchanged, as escaped text.
pub(crate) fn highlight_fenced_code(info: &str, source: &str) -> Result<Option<String>> {
    let Some(grammar) = grammar(info)? else {
        return Ok(None);
    };
    let themes = BTreeMap::from([
        ("light".to_owned(), "code-light".to_owned()),
        ("dark".to_owned(), "code-dark".to_owned()),
    ]);
    let options = CodeToHtmlOptions::multi(&grammar, themes).shiki();
    // The lines, without the block's final newline (no empty last line).
    let code = source.strip_suffix('\n').unwrap_or(source);
    highlighter()?
        .code_to_html(code, &options)
        .map(Some)
        .map_err(|error| anyhow!("could not highlight {grammar} code: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASKELL: &str = include_str!("../tests/fixtures/highlight/haskell.hs");
    const OCAML: &str = include_str!("../tests/fixtures/highlight/ocaml.ml");
    const METAOCAML: &str = include_str!("../tests/fixtures/highlight/metaocaml.ml");
    const ROCQ: &str = include_str!("../tests/fixtures/highlight/rocq.v");

    fn highlighted(info: &str, source: &str) -> String {
        highlight_fenced_code(info, source)
            .unwrap()
            .unwrap_or_else(|| panic!("{info} was left plain"))
    }

    /// The text a reader copies from the block: the HTML without its tags,
    /// its character references decoded.
    fn text(html: &str) -> String {
        let mut out = String::new();
        let mut rest = html;
        while let Some(at) = rest.find(['<', '&']) {
            out.push_str(&rest[..at]);
            rest = &rest[at..];
            if rest.starts_with('<') {
                rest = &rest[rest.find('>').unwrap() + 1..];
            } else {
                let end = rest.find(';').unwrap();
                let entity = &rest[1..end];
                let c = match entity {
                    "lt" => '<',
                    "gt" => '>',
                    "amp" => '&',
                    "quot" => '"',
                    "apos" => '\'',
                    _ if entity.starts_with("#x") => {
                        char::from_u32(u32::from_str_radix(&entity[2..], 16).unwrap()).unwrap()
                    }
                    _ if entity.starts_with('#') => {
                        char::from_u32(entity[1..].parse().unwrap()).unwrap()
                    }
                    _ => panic!("unexpected entity &{entity};"),
                };
                out.push(c);
                rest = &rest[end + 1..];
            }
        }
        out.push_str(rest);
        out
    }

    /// The colours given in the light theme (inline) and the dark one (variables).
    fn colours(html: &str) -> (Vec<String>, Vec<String>) {
        let pick = |key: &str| {
            let mut found: Vec<String> = html
                .match_indices(key)
                .map(|(at, _)| html[at + key.len()..at + key.len() + 7].to_ascii_lowercase())
                .collect();
            found.sort();
            found.dedup();
            found
        };
        (
            pick(";color:")
                .into_iter()
                .chain(pick("\"color:"))
                .collect(),
            pick("--shiki-dark:"),
        )
    }

    /// The opening tag of the span whose text is `token` (Shiki folds the
    /// spaces before a token into it).
    fn span_with(html: &str, token: &str) -> String {
        html.split("<span ")
            .filter_map(|span| span.split_once('>'))
            .find(|(_, rest)| {
                rest.split("</span>")
                    .next()
                    .is_some_and(|t| t.trim() == token)
            })
            .map(|(open, _)| open.to_ascii_lowercase())
            .unwrap_or_else(|| panic!("no span holds {token:?}"))
    }

    fn assert_colour(html: &str, token: &str, light: &str, dark: &str) {
        let span = span_with(html, token);
        assert!(
            span.contains(&format!("color:{light}"))
                && span.contains(&format!("--shiki-dark:{dark}")),
            "{token:?}: {span}"
        );
    }

    #[test]
    fn haskell_is_highlighted_by_its_grammar_and_keeps_its_text() {
        let html = highlighted("haskell", HASKELL);
        assert!(html.starts_with("<pre class=\"shiki shiki-themes code-light code-dark\""));
        assert_eq!(text(&html), HASKELL.trim_end_matches('\n'));
        assert_colour(&html, "data", "#0d5962", "#9bd5d9");
        assert_colour(&html, "where", "#0d5962", "#9bd5d9");
        assert_colour(&html, "eval", "#176f78", "#73c1c7");
        assert_colour(&html, "Lit", "#176f78", "#73c1c7");
        assert_colour(&html, "Expr", "#176f78", "#73c1c7");
        assert_colour(&html, "\"hello, \"", "#6b5b3e", "#d4b77d");
        assert_colour(&html, "::", "#53616a", "#aeb9bd");
        assert_colour(&html, "&#x3C;>", "#53616a", "#aeb9bd");
        // `hs` is Haskell too.
        assert_eq!(highlighted("hs", HASKELL), html);
    }

    #[test]
    fn ocaml_is_highlighted_by_its_grammar_and_keeps_its_text() {
        let html = highlighted("ocaml", OCAML);
        assert_eq!(text(&html), OCAML.trim_end_matches('\n'));
        assert_colour(&html, "module", "#0d5962", "#9bd5d9");
        assert_colour(&html, "type", "#0d5962", "#9bd5d9");
        assert_colour(&html, "struct", "#0d5962", "#9bd5d9");
        assert_colour(&html, "Var", "#176f78", "#73c1c7");
        assert_colour(&html, "Printf", "#176f78", "#73c1c7");
        assert_colour(&html, "shift", "#176f78", "#73c1c7");
        assert_colour(&html, "\"(%s %s)\"", "#6b5b3e", "#d4b77d");
        assert_colour(&html, "1", "#7a4f5d", "#d4a8b4");
        assert_colour(&html, "|", "#53616a", "#aeb9bd");
        assert_eq!(highlighted("ml", OCAML), html);
    }

    #[test]
    fn metaocaml_is_read_as_ocaml_and_its_staging_brackets_stay_as_written() {
        let html = highlighted("metaocaml", METAOCAML);
        assert_eq!(text(&html), METAOCAML.trim_end_matches('\n'));
        assert_eq!(highlighted("ber-metaocaml", METAOCAML), html);
        assert_eq!(highlighted("ocaml", METAOCAML), html);
        // Quotation and escape are punctuation, and ordinary OCaml around them is highlighted.
        assert_colour(&html, ".&#x3C;", "#53616a", "#aeb9bd");
        assert_colour(&html, ".~", "#53616a", "#aeb9bd");
        assert_colour(&html, "let", "#0d5962", "#9bd5d9");
        assert_colour(&html, "Runnative", "#176f78", "#73c1c7");
        assert_eq!(
            html.matches("<span class=\"line\">").count(),
            METAOCAML.lines().count()
        );
    }

    #[test]
    fn rocq_is_read_by_the_coq_grammar_and_keeps_its_symbols() {
        let html = highlighted("rocq", ROCQ);
        assert_eq!(text(&html), ROCQ.trim_end_matches('\n'));
        assert!(text(&html).contains("x ≤ y"));
        assert_eq!(highlighted("coq", ROCQ), html);
        assert_eq!(highlighted("Rocq", ROCQ), html);
        for vernacular in ["Theorem", "Proof", "Qed", "Fixpoint", "Lemma"] {
            assert_colour(&html, vernacular, "#0d5962", "#9bd5d9");
        }
        for tactic in ["intros", "induction", "simpl", "rewrite", "reflexivity"] {
            assert_colour(&html, tactic, "#176f78", "#73c1c7");
        }
        assert_colour(&html, "forall", "#176f78", "#73c1c7");
        assert_colour(&html, "S", "#176f78", "#73c1c7");
        assert_colour(
            &html,
            "(* Addition on the naturals is commutative. *)",
            "#53616a",
            "#aeb9bd",
        );
    }

    #[test]
    fn common_languages_are_highlighted() {
        for (info, code) in [
            ("rust", "fn main() { let n: u32 = 1; println!(\"{n}\"); }\n"),
            ("ts", "export const id = <T,>(x: T): T => x; // identity\n"),
            (
                "python",
                "def area(r: float) -> float:\n    return 3.14159 * r ** 2\n",
            ),
            ("lean4", "theorem two_eq : 1 + 1 = 2 := by rfl\n"),
            ("sh", "for f in *.md; do wc -l \"$f\"; done\n"),
            (
                "latex",
                "\\begin{theorem} $\\forall x, x = x$ \\end{theorem}\n",
            ),
            (
                "json",
                "{\"name\": \"site\", \"draft\": false, \"order\": 10}\n",
            ),
            ("bibtex", "@article{key, title = {A Title}, year = 2026}\n"),
            ("yaml", "lang: ja\nmath: true\n"),
            ("toml", "[package]\nname = \"site-builder\"\n"),
            ("c", "int main(void) { return 0; }\n"),
            ("diff", "-old\n+new\n"),
        ] {
            let html = highlighted(info, code);
            assert_eq!(text(&html), code.trim_end_matches('\n'), "{info}");
            let (light, dark) = colours(&html);
            assert!(
                light.len() >= 3 && dark.len() >= 3,
                "{info}: {light:?} {dark:?}"
            );
        }
    }

    #[test]
    fn plain_and_unknown_fences() {
        for info in ["", "text", "TXT", "plain", "plaintext", "text title=\"x\""] {
            assert_eq!(
                highlight_fenced_code(info, "a <b>\n").unwrap(),
                None,
                "{info:?}"
            );
        }
        let error = highlight_fenced_code("definitely-not-a-language", "x\n")
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "unknown fenced-code language \"definitely-not-a-language\"; use a supported language name or `text` for intentionally plain code"
        );
        assert!(highlight_fenced_code("ocaml5", "x\n").is_err());
    }

    #[test]
    fn code_is_text_never_markup() {
        let code = "x = \"</span><script>alert(1)</script>\" & y > z\n-- ==not a mark== $a$ [^1]\n";
        let html = highlighted("haskell", code);
        assert!(!html.contains("<script"));
        assert!(!html.contains("</span><script"));
        assert!(!html.contains("<mark"));
        assert_eq!(text(&html), code.trim_end_matches('\n'));
        // Tabs, Unicode, and indentation are kept exactly.
        let code = "let\t(→) = fun a b ->\n    \"λ ∀ 日本語\"\n";
        assert_eq!(
            text(&highlighted("ocaml", code)),
            code.trim_end_matches('\n')
        );
    }

    #[test]
    fn the_themes_use_only_the_site_palette() {
        let mut all = Vec::new();
        let fixtures = [
            ("haskell", HASKELL),
            ("ocaml", OCAML),
            ("metaocaml", METAOCAML),
            ("rocq", ROCQ),
        ];
        for (info, code) in fixtures {
            let (light, dark) = colours(&highlighted(info, code));
            all.extend(light.into_iter().map(|c| (c, true)));
            all.extend(dark.into_iter().map(|c| (c, false)));
        }
        let light = [
            "#17242d", "#53616a", "#0d5962", "#176f78", "#6b5b3e", "#7a4f5d",
        ];
        let dark = [
            "#e8e5dc", "#aeb9bd", "#9bd5d9", "#73c1c7", "#d4b77d", "#d4a8b4",
        ];
        for (colour, is_light) in all {
            let palette = if is_light { &light } else { &dark };
            assert!(palette.contains(&colour.as_str()), "{colour}");
        }
        for (info, code) in fixtures {
            let html = highlighted(info, code);
            assert!(!html.contains("font-style") && !html.contains("font-weight"));
        }
    }
}
