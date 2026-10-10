use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use png::ColorType;
use regex::Regex;

use crate::model::validate_markdown_url;

const REQUIRED_FILES: &[&str] = &[
    "index.html",
    "cv/index.html",
    "research/index.html",
    "writings/index.html",
    "updates/index.html",
    "updates/research/index.html",
    "updates/academia/index.html",
    "updates/writing/index.html",
    "miscellany/index.html",
    "miscellany/reading/index.html",
    "miscellany/notes/index.html",
    "miscellany/diary/index.html",
    "404.html",
    "rss.xml",
    "robots.txt",
    "sitemap-index.xml",
    "sitemap-0.xml",
    "favicon.svg",
    "favicon-32x32.png",
    "apple-touch-icon.png",
    "assets/site.css",
    "assets/site.js",
    ".nojekyll",
];

const PROTECTED_EMAIL: &str = "hatakeyama.ryujin.q7@dc.tohoku.ac.jp";

pub fn dist(root: &Path, source_root: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(root)
        .with_context(|| format!("failed to inspect built-site directory {}", root.display()))?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "built-site directory {} does not exist",
        root.display()
    );
    for required in REQUIRED_FILES {
        ensure!(
            root.join(required).is_file(),
            "built site is missing {required}"
        );
    }

    let files = collect_files(root)?;
    audit_public_files(root, source_root, &files)?;
    validate_html(root, &files)?;
    validate_css(root)?;
    validate_favicons(root)?;
    validate_expected_content(root)?;
    Ok(())
}

fn audit_public_files(root: &Path, source_root: &Path, files: &[PathBuf]) -> Result<()> {
    let allowed_extensions = [
        "css", "html", "js", "png", "svg", "ttf", "txt", "woff", "woff2", "xml",
    ];
    let source_path = source_root.display().to_string();
    for path in files {
        let relative = path.strip_prefix(root)?;
        let name = relative
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        ensure!(
            !relative.components().any(|part| part.as_os_str() == ".git"),
            "public artifact contains .git data"
        );
        ensure!(
            !name.ends_with(".map"),
            "public artifact contains source map {}",
            relative.display()
        );
        if name != ".nojekyll" {
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default();
            ensure!(
                allowed_extensions.contains(&extension),
                "unexpected public artifact {}",
                relative.display()
            );
        }
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("html" | "css" | "js" | "xml" | "txt" | "svg")
        ) {
            let text = fs::read_to_string(path)?;
            ensure!(
                !text.contains(PROTECTED_EMAIL),
                "{} exposes the complete email address",
                relative.display()
            );
            ensure!(
                !text.contains(&source_path),
                "{} exposes a local source path",
                relative.display()
            );
            ensure!(
                !text.contains("/Users/"),
                "{} exposes a local absolute path",
                relative.display()
            );
            ensure!(
                !text.contains("Fixture award") && !text.contains("Test record"),
                "{} exposes a test fixture",
                relative.display()
            );
        }
    }
    Ok(())
}

fn validate_html(root: &Path, files: &[PathBuf]) -> Result<()> {
    let attribute = Regex::new(r#"(?:href|src)=\"([^\"]+)\""#)?;
    let id_pattern = Regex::new(r#"id=\"([^\"]+)\""#)?;
    let html_files: Vec<_> = files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("html"))
        .collect();
    let mut documents = HashMap::new();
    let mut ids = HashMap::new();

    for path in &html_files {
        let relative = path.strip_prefix(root)?.to_owned();
        let html = fs::read_to_string(path)?;
        ensure!(
            !relative.starts_with("ja/"),
            "inactive Japanese route was published: {}",
            relative.display()
        );
        ensure!(
            html.contains("<html lang=\"en\">"),
            "{} has no English language attribute",
            relative.display()
        );
        ensure!(
            html.contains("<title>"),
            "{} has no title",
            relative.display()
        );
        ensure!(
            html.contains("name=\"description\""),
            "{} has no description",
            relative.display()
        );
        ensure!(
            html.contains("rel=\"canonical\""),
            "{} has no canonical URL",
            relative.display()
        );
        ensure!(
            html.contains("property=\"og:url\""),
            "{} has no Open Graph URL",
            relative.display()
        );
        ensure!(
            html.contains("href=\"/favicon.svg?v=2\""),
            "{} has no SVG favicon reference",
            relative.display()
        );
        ensure!(
            html.contains("href=\"/favicon-32x32.png?v=2\""),
            "{} has no PNG favicon reference",
            relative.display()
        );
        ensure!(
            html.contains("class=\"skip-link\" href=\"#main-content\""),
            "{} has no skip link",
            relative.display()
        );
        ensure!(
            html.contains("<main id=\"main-content\">"),
            "{} has no main landmark",
            relative.display()
        );
        ensure!(
            html.contains("<nav class=\"primary-nav\" aria-label=\"Primary navigation\">"),
            "{} has no labelled primary navigation",
            relative.display()
        );
        ensure!(
            html.contains("<a href=\"/updates/\"") && html.contains("<a href=\"/miscellany/\""),
            "{} has an incomplete primary navigation",
            relative.display()
        );
        if relative.starts_with("miscellany") {
            ensure!(
                html.contains("<a href=\"/miscellany/\" aria-current=\"page\">Miscellany</a>"),
                "{} does not mark Miscellany as the current section",
                relative.display()
            );
        }
        ensure!(
            html.matches("src=\"https://gc.zgo.at/count.js\"").count() == 1
                && html.matches("data-goatcounter=").count() == 1
                && html.contains(
                    "data-goatcounter=\"https://hatakeyama.goatcounter.com/count\" async src=\"https://gc.zgo.at/count.js\""
                ),
            "{} must load the GoatCounter script exactly once with the site endpoint",
            relative.display()
        );
        ensure!(
            html.contains("window.goatcounter={no_onload:true};</script><script data-goatcounter="),
            "{} must disable GoatCounter outside production before its script",
            relative.display()
        );
        ensure!(
            html.contains("id=\"appearance-toggle\"")
                && html.contains("aria-label=\"Dark mode\" aria-pressed=\"false\""),
            "{} has no accessible appearance control",
            relative.display()
        );
        ensure!(
            !html.contains("Academia"),
            "{} shows the retired category label Academia instead of Activities",
            relative.display()
        );
        ensure!(
            !html.contains("hreflang="),
            "{} exposes inactive translation metadata",
            relative.display()
        );

        let document_ids = id_pattern
            .captures_iter(&html)
            .map(|capture| capture[1].to_owned())
            .collect::<HashSet<_>>();
        ids.insert(relative.clone(), document_ids);
        documents.insert(relative, html);
    }

    for (relative, html) in &documents {
        for capture in attribute.captures_iter(html) {
            let reference = &capture[1];
            validate_reference(root, relative, reference, &ids)?;
        }
    }
    Ok(())
}

fn validate_reference(
    root: &Path,
    current: &Path,
    reference: &str,
    ids: &HashMap<PathBuf, HashSet<String>>,
) -> Result<()> {
    validate_markdown_url(reference)
        .with_context(|| format!("{} contains unsafe URL {reference:?}", current.display()))?;
    let lower_reference = reference.to_ascii_lowercase();
    if lower_reference.starts_with("http://")
        || lower_reference.starts_with("https://")
        || lower_reference.starts_with("mailto:")
    {
        return Ok(());
    }
    let reference = reference.split('?').next().unwrap_or(reference);
    let (path_part, fragment) = reference.split_once('#').unwrap_or((reference, ""));
    if path_part.is_empty() && fragment == "email" {
        return Ok(());
    }

    let target_relative = if path_part.is_empty() {
        current.to_owned()
    } else {
        let normalized = path_part.strip_prefix('/').unwrap_or(path_part);
        ensure!(
            Path::new(normalized)
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_))),
            "{} contains a local reference which escapes the output root: {reference}",
            current.display()
        );
        if normalized.ends_with('/') || normalized.is_empty() {
            PathBuf::from(normalized).join("index.html")
        } else {
            PathBuf::from(normalized)
        }
    };
    ensure!(
        root.join(&target_relative).is_file(),
        "{} references missing local file {reference}",
        current.display()
    );
    if !fragment.is_empty() {
        ensure!(
            ids.get(&target_relative)
                .is_some_and(|target_ids| target_ids.contains(fragment)),
            "{} references missing anchor {reference}",
            current.display()
        );
    }
    Ok(())
}

fn validate_css(root: &Path) -> Result<()> {
    let path = root.join("assets/site.css");
    let css = fs::read_to_string(&path)?;
    let urls = Regex::new(r#"url\((?:\"|')?([^\"')]+)"#)?;
    for capture in urls.captures_iter(&css) {
        let reference = &capture[1];
        if reference.starts_with("data:") || reference.starts_with("http") {
            continue;
        }
        ensure!(
            Path::new(reference)
                .components()
                .all(|component| matches!(component, std::path::Component::Normal(_))),
            "stylesheet reference escapes the output root: {reference}"
        );
        ensure!(
            path.parent().unwrap().join(reference).is_file(),
            "stylesheet references missing asset {reference}"
        );
    }
    Ok(())
}

fn validate_favicons(root: &Path) -> Result<()> {
    let svg = fs::read_to_string(root.join("favicon.svg"))?;
    ensure!(
        svg.contains("<title>Δ;Γ</title>"),
        "SVG favicon has the wrong title"
    );
    ensure!(
        !svg.contains("<rect"),
        "SVG favicon unexpectedly contains a background rectangle"
    );
    validate_png(&root.join("favicon-32x32.png"), 32, 32)?;
    validate_png(&root.join("apple-touch-icon.png"), 180, 180)?;
    Ok(())
}

fn validate_png(path: &Path, width: u32, height: u32) -> Result<()> {
    let file = fs::File::open(path)?;
    let decoder = png::Decoder::new(file);
    let reader = decoder.read_info()?;
    let info = reader.info();
    ensure!(
        info.width == width && info.height == height,
        "{} has unexpected dimensions",
        path.display()
    );
    ensure!(
        matches!(info.color_type, ColorType::Rgba | ColorType::GrayscaleAlpha),
        "{} lacks an alpha channel",
        path.display()
    );
    Ok(())
}

fn validate_expected_content(root: &Path) -> Result<()> {
    let home = fs::read_to_string(root.join("index.html"))?;
    // The biography's wording lives in content/home.json and is edited
    // there; the build checks only that it is present and inside the intro.
    let bio = home
        .find("<div class=\"home-bio-copy\"><p class=\"home-bio\">")
        .context("homepage has no biography")?;
    ensure!(
        bio < home
            .find("</section>")
            .context("homepage has no intro section")?,
        "the biography is outside the homepage introduction"
    );
    for phrase in [
        "畠山竜迅",
        "はたけやま りゅうじん",
        "hatakejama ɾʲɯːdʑiɴ",
        "Talks &amp; Presentations",
        "Recent Updates",
    ] {
        ensure!(
            home.contains(phrase),
            "homepage is missing required content: {phrase}"
        );
    }
    ensure!(
        home.contains("data-email-code="),
        "homepage has no protected email control"
    );
    ensure!(
        !home.contains("mailto:"),
        "homepage exposes a static mailto link"
    );

    let research = fs::read_to_string(root.join("research/index.html"))?;
    for phrase in [
        "代数的操作を用いた有限確率モデルの記述と多段階計算による推論コード生成",
        "Applicativeによるグラフィカルモデルの表現と厳密推論に向けた実装",
        "The 43rd Annual Conference of the Japan Society for Software Science and Technology (JSSST 2026)",
        "Oral presentation · Unrefereed proceedings paper",
        "https://jssst.or.jp/files/user/taikai/2026/papers/6b-1-R.pdf",
    ] {
        ensure!(
            research.contains(phrase),
            "research page is missing required content: {phrase}"
        );
    }
    let jssst = research
        .find("代数的操作")
        .context("missing JSSST record")?;
    let ppl = research.find("Applicative").context("missing PPL record")?;
    ensure!(jssst < ppl, "presentations are not ordered newest first");
    validate_publication_list(&research, "research page")?;

    let cv = fs::read_to_string(root.join("cv/index.html"))?;
    validate_publication_list(&cv, "CV")?;
    for phrase in [
        "Tohoku NLP Group, Tohoku University",
        "Undergraduate research participant, Step-QI School, Advanced Creative Engineering I and II.",
        "<strong>Best Award</strong>",
        "2022 academic-year Advanced Creative Engineering Training Poster Session, Step-QI School, Tohoku University.",
        "Nominated for Best System Paper at SemEval-2023.",
    ] {
        ensure!(
            cv.contains(phrase),
            "CV is missing required content: {phrase}"
        );
    }
    ensure!(
        !cv.contains("Excellence Award"),
        "CV uses the superseded award wording"
    );

    for (page, html) in [("research page", &research), ("CV", &cv)] {
        for activity in ["LLAL@GSIS", "Graham Priest"] {
            ensure!(
                !html.contains(activity),
                "{page} lists the activity report {activity} as a publication or presentation"
            );
        }
    }
    let activities = fs::read_to_string(root.join("updates/academia/index.html"))?;
    ensure!(
        activities.contains("<h1>Activities</h1>"),
        "the academia category page is not titled Activities"
    );

    let research_updates = fs::read_to_string(root.join("updates/research/index.html"))?;
    ensure!(
        !research_updates.contains("AIE English Training Session"),
        "the AIE English Training Session poster must be listed under Activities only"
    );

    let sitemap = fs::read_to_string(root.join("sitemap-0.xml"))?;
    ensure!(!sitemap.contains("/404/"), "sitemap includes the 404 page");
    ensure!(
        !sitemap.contains("/ja/"),
        "sitemap includes inactive Japanese pages"
    );
    let rss = fs::read_to_string(root.join("rss.xml"))?;
    ensure!(
        rss.contains("<language>en</language>"),
        "RSS language is missing"
    );
    ensure!(
        !rss.contains("/miscellany/"),
        "the Updates feed must not include Miscellany content"
    );
    Ok(())
}

/// Publications must form one newest-first list, with a language filter that
/// stays hidden until the script enables it, so every entry is shown without
/// JavaScript. The unrefereed JSSST proceedings paper precedes the SemEval
/// workshop paper, and the PPL poster remains a presentation only.
fn validate_publication_list(html: &str, page: &str) -> Result<()> {
    let section = html
        .split_once(">Publications</h2>")
        .and_then(|(_, rest)| rest.split_once("</section>"))
        .map(|(section, _)| section)
        .with_context(|| format!("{page} has no Publications section"))?;
    ensure!(
        section
            .matches("<ol class=\"publication-list\" id=\"publication-list\">")
            .count()
            == 1
            && !section.contains("In English")
            && !section.contains("In Japanese"),
        "{page} publications are not a single list"
    );
    ensure!(
        section.contains("<div class=\"publication-filter\" hidden data-publication-filter>")
            && section.contains("<option value=\"all\" selected>All</option>"),
        "{page} publication filter must be hidden by default and select All"
    );
    let jssst = section
        .find("<li data-language=\"ja\"><article><p class=\"publication-index\">2026</p><div><h3 lang=\"ja\">代数的操作")
        .with_context(|| format!("{page} is missing the JSSST 2026 proceedings paper"))?;
    let semeval = section
        .find("<li data-language=\"en\"><article><p class=\"publication-index\">2023</p><div><h3 lang=\"en\">TohokuNLP at SemEval-2023 Task 5")
        .with_context(|| format!("{page} is missing the SemEval-2023 paper"))?;
    ensure!(
        jssst < semeval,
        "{page} publications are not ordered newest first"
    );
    ensure!(
        section.contains("Unrefereed proceedings paper")
            && section.contains("Workshop paper · Published")
            && !section.contains("Applicative"),
        "{page} misclassifies publication records"
    );
    Ok(())
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn collect(directory: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
        let mut entries = fs::read_dir(directory)
            .with_context(|| format!("failed to read {}", directory.display()))?
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let file_type = entry.file_type()?;
            ensure!(
                !file_type.is_symlink(),
                "built site contains symbolic link {}",
                path.display()
            );
            if file_type.is_dir() {
                collect(&path, output)?;
            } else if file_type.is_file() {
                output.push(path);
            } else {
                anyhow::bail!("built site contains unsupported entry {}", path.display());
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, &mut files)?;
    files.sort();
    Ok(files)
}
