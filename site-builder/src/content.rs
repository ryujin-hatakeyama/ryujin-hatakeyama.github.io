use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, ensure};
use katex::{Opts, render_with_opts};
use pulldown_cmark::{CowStr, Event, Options, Parser, html};
use serde::de::DeserializeOwned;

use crate::model::{
    Language, Project, ProjectFrontMatter, Publication, Update, Writing, WritingFrontMatter,
};

#[derive(Debug)]
pub struct SiteContent {
    pub publications: Vec<Publication>,
    pub updates: Vec<Update>,
    pub writings: Vec<Writing>,
    pub projects: Vec<Project>,
}

pub fn load(root: &Path) -> Result<SiteContent> {
    let publications = load_publications(&root.join("content/publications"))?;
    let updates = load_updates(&root.join("target/generated/updates.json"))?;
    let writings = load_writings(&root.join("content/writings"))?;
    let projects = load_projects(&root.join("content/projects"))?;

    validate_unique(
        publications.iter().map(|record| record.slug.as_str()),
        "publication slug",
    )?;
    validate_unique(updates.iter().map(|record| record.id.as_str()), "update id")?;
    validate_unique(
        writings.iter().map(|record| record.metadata.slug.as_str()),
        "writing slug",
    )?;
    validate_unique(
        projects.iter().map(|record| record.metadata.slug.as_str()),
        "project slug",
    )?;

    validate_relations(&publications, &updates, &writings, &projects)?;

    Ok(SiteContent {
        publications,
        updates,
        writings,
        projects,
    })
}

fn load_publications(directory: &Path) -> Result<Vec<Publication>> {
    let mut records = Vec::new();
    for path in files_with_extensions(directory, &["json"])? {
        let record: Publication = read_json(&path)?;
        record.validate(&path.display().to_string())?;
        records.push(record);
    }
    records.sort_by(|left, right| {
        right
            .year
            .cmp(&left.year)
            .then_with(|| left.slug.cmp(&right.slug))
    });
    Ok(records)
}

fn load_updates(path: &Path) -> Result<Vec<Update>> {
    ensure!(
        path.is_file(),
        "missing generated update data {}; run the OCaml generator first",
        path.display()
    );
    let mut records: Vec<Update> = read_json(path)?;
    for record in &records {
        record.validate(&path.display().to_string())?;
    }
    records.sort_by(|left, right| {
        right
            .announced_on
            .cmp(&left.announced_on)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(records)
}

fn load_writings(directory: &Path) -> Result<Vec<Writing>> {
    let mut records = Vec::new();
    for path in files_with_extensions(directory, &["md", "mdx"])? {
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (front_matter, body) = split_front_matter(&source, &path)?;
        let metadata: WritingFrontMatter = serde_yaml_ng::from_str(front_matter)
            .with_context(|| format!("invalid front matter in {}", path.display()))?;
        metadata.validate(&path.display().to_string())?;
        let rendered_body = render_markdown(body, metadata.math)
            .with_context(|| format!("failed to render {}", path.display()))?;
        records.push(Writing {
            metadata,
            rendered_body,
        });
    }
    records.sort_by(|left, right| {
        right
            .metadata
            .date
            .cmp(&left.metadata.date)
            .then_with(|| left.metadata.slug.cmp(&right.metadata.slug))
    });
    Ok(records)
}

fn load_projects(directory: &Path) -> Result<Vec<Project>> {
    let mut records = Vec::new();
    for path in files_with_extensions(directory, &["md", "mdx"])? {
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            continue;
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (front_matter, _) = split_front_matter(&source, &path)?;
        let metadata: ProjectFrontMatter = serde_yaml_ng::from_str(front_matter)
            .with_context(|| format!("invalid front matter in {}", path.display()))?;
        metadata.validate(&path.display().to_string())?;
        records.push(Project { metadata });
    }
    records.sort_by(|left, right| left.metadata.slug.cmp(&right.metadata.slug));
    Ok(records)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn files_with_extensions(directory: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    ensure!(
        directory.is_dir(),
        "missing content directory {}",
        directory.display()
    );
    let mut files = Vec::new();
    collect_files(directory, extensions, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(directory: &Path, extensions: &[&str], files: &mut Vec<PathBuf>) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("failed to read directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if metadata.is_dir() {
            collect_files(&path, extensions, files)?;
        } else if metadata.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|extension| extensions.contains(&extension))
        {
            files.push(path);
        }
    }
    Ok(())
}

fn split_front_matter<'a>(source: &'a str, path: &Path) -> Result<(&'a str, &'a str)> {
    let remainder = source
        .strip_prefix("---\n")
        .or_else(|| source.strip_prefix("---\r\n"))
        .with_context(|| format!("{} must begin with YAML front matter", path.display()))?;
    let (front_matter, body) = remainder
        .split_once("\n---\n")
        .or_else(|| remainder.split_once("\r\n---\r\n"))
        .with_context(|| format!("{} has unterminated YAML front matter", path.display()))?;
    Ok((front_matter, body))
}

fn render_markdown(source: &str, math_enabled: bool) -> Result<String> {
    let mut options = Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;
    if math_enabled {
        options.insert(Options::ENABLE_MATH);
    }

    let parser = Parser::new_ext(source, options).map(|event| match event {
        Event::InlineMath(formula) => render_math_event(&formula, false),
        Event::DisplayMath(formula) => render_math_event(&formula, true),
        other => Ok(other),
    });
    let mut output = String::new();
    let events = parser.collect::<Result<Vec<_>>>()?;
    html::push_html(&mut output, events.into_iter());
    Ok(output)
}

fn render_math_event<'a>(formula: &str, display: bool) -> Result<Event<'a>> {
    let options = Opts::builder()
        .display_mode(display)
        .throw_on_error(true)
        .build()
        .context("failed to configure KaTeX")?;
    let rendered = render_with_opts(formula, &options)
        .with_context(|| format!("invalid TeX expression: {formula}"))?;
    Ok(Event::Html(CowStr::Boxed(rendered.into_boxed_str())))
}

fn validate_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Result<()> {
    let mut seen = HashSet::new();
    for value in values {
        ensure!(seen.insert(value), "duplicate {label} {value:?}");
    }
    Ok(())
}

fn validate_relations(
    publications: &[Publication],
    updates: &[Update],
    writings: &[Writing],
    projects: &[Project],
) -> Result<()> {
    let publication_ids: HashSet<_> = publications
        .iter()
        .map(|record| record.slug.as_str())
        .collect();
    let writing_ids: HashSet<_> = writings
        .iter()
        .map(|record| record.metadata.slug.as_str())
        .collect();
    let project_ids: HashSet<_> = projects
        .iter()
        .map(|record| record.metadata.slug.as_str())
        .collect();

    for update in updates {
        for id in &update.related.publications {
            ensure!(
                publication_ids.contains(id.as_str()),
                "update {:?} references missing publication {id:?}",
                update.id
            );
        }
        for id in &update.related.writings {
            ensure!(
                writing_ids.contains(id.as_str()),
                "update {:?} references missing writing {id:?}",
                update.id
            );
        }
        for id in &update.related.projects {
            ensure!(
                project_ids.contains(id.as_str()),
                "update {:?} references missing project {id:?}",
                update.id
            );
        }
    }
    for publication in publications {
        for id in &publication.project_ids {
            ensure!(
                project_ids.contains(id.as_str()),
                "publication {:?} references missing project {id:?}",
                publication.slug
            );
        }
    }
    Ok(())
}

pub fn public_english_writings(writings: &[Writing]) -> impl Iterator<Item = &Writing> {
    writings
        .iter()
        .filter(|writing| !writing.metadata.draft && writing.metadata.lang == Language::En)
}

#[cfg(test)]
mod tests {
    use super::{render_markdown, split_front_matter};
    use std::path::Path;

    #[test]
    fn parses_front_matter() {
        let source = "---\ntitle: Example\n---\nBody";
        let (metadata, body) = split_front_matter(source, Path::new("example.md")).unwrap();
        assert_eq!(metadata, "title: Example");
        assert_eq!(body, "Body");
    }

    #[test]
    fn renders_accessible_math() {
        let html = render_markdown("Euler: $e^{i\\pi}+1=0$", true).unwrap();
        assert!(html.contains("class=\"katex\""));
        assert!(html.contains("<math"));
    }
}
