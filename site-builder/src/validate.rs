use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use png::ColorType;
use regex::Regex;

const REQUIRED_FILES: &[&str] = &[
    "index.html",
    "cv/index.html",
    "research/index.html",
    "writings/index.html",
    "updates/index.html",
    "updates/research/index.html",
    "updates/academia/index.html",
    "updates/writing/index.html",
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
    ensure!(
        root.is_dir(),
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
            html.contains("id=\"appearance-toggle\"")
                && html.contains("aria-label=\"Switch to dark mode\""),
            "{} has no accessible appearance control",
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
    if reference.starts_with("http://")
        || reference.starts_with("https://")
        || reference.starts_with("mailto:")
        || reference.starts_with("data:")
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
    for phrase in [
        "Hello! I'm Ryujin, a second-year master's student",
        "Department of Computer and Mathematical Sciences",
        "Foundations of Software Science",
        "Professor Eijiro Sumii",
        "Oleg Kiselyov",
        "compositional descriptions of probabilistic models and staged inference code generation",
        "My broader interests lie in modal and categorical logic, and in the conditions of intelligibility of formal reasoning.",
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
            if entry.file_type()?.is_dir() {
                collect(&path, output)?;
            } else {
                output.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, &mut files)?;
    files.sort();
    Ok(files)
}
