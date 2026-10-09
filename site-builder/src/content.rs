use std::collections::HashSet;
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use katex::{Opts, render_with_opts};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, html};
use serde::de::DeserializeOwned;

use chrono::NaiveDate;

use crate::model::{
    EventStatus, Language, Project, ProjectFrontMatter, Publication, Update, WritingFrontMatter,
    validate_markdown_url,
};

#[derive(Debug)]
pub(crate) struct ValidatedSiteContent {
    publications: Vec<Publication>,
    updates: Vec<Update>,
    writings: Vec<ValidatedWriting>,
    projects: Vec<Project>,
}

impl ValidatedSiteContent {
    pub(crate) fn publications(&self) -> &[Publication] {
        &self.publications
    }

    pub(crate) fn updates(&self) -> &[Update] {
        &self.updates
    }

    pub(crate) fn writings(&self) -> &[ValidatedWriting] {
        &self.writings
    }

    pub(crate) fn projects(&self) -> &[Project] {
        &self.projects
    }
}

#[derive(Debug)]
pub(crate) struct ValidatedWriting {
    metadata: WritingFrontMatter,
    rendered_body: RenderedMarkdown,
}

impl ValidatedWriting {
    pub(crate) fn metadata(&self) -> &WritingFrontMatter {
        &self.metadata
    }

    pub(crate) fn rendered_body(&self) -> &str {
        self.rendered_body.as_str()
    }
}

/// HTML emitted by the Markdown renderer after rejecting source HTML and
/// validating content-controlled URLs. Its constructor remains private to
/// this module; KaTeX is the only source of deliberately unescaped events.
#[derive(Debug)]
struct RenderedMarkdown(String);

impl RenderedMarkdown {
    fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn load(root: &Path) -> Result<ValidatedSiteContent> {
    let mut diagnostics = Diagnostics::default();
    let publications = diagnostics.capture(load_publications(&root.join("content/publications")));
    let updates = diagnostics.capture(load_updates(&root.join("target/generated/updates.json")));
    let writings = diagnostics.capture(load_writings(&root.join("content/writings")));
    let projects = diagnostics.capture(load_projects(&root.join("content/projects")));
    diagnostics.finish("content loading failed")?;

    let publications = publications.expect("successful diagnostics contain publications");
    let updates = updates.expect("successful diagnostics contain updates");
    let writings = writings.expect("successful diagnostics contain writings");
    let projects = projects.expect("successful diagnostics contain projects");

    let mut diagnostics = Diagnostics::default();
    diagnostics.extend(validate_unique(
        publications.iter().map(|record| record.slug.as_str()),
        "publication slug",
    ));
    diagnostics.extend(validate_unique(
        updates.iter().map(|record| record.id.as_str()),
        "update id",
    ));
    diagnostics.extend(validate_unique(
        writings.iter().map(|record| record.metadata.slug.as_str()),
        "writing slug",
    ));
    diagnostics.extend(validate_unique(
        projects.iter().map(|record| record.metadata.slug.as_str()),
        "project slug",
    ));

    diagnostics.extend(validate_relations(
        &publications,
        &updates,
        &writings,
        &projects,
    ));
    diagnostics.finish("content validation failed")?;

    Ok(ValidatedSiteContent {
        publications,
        updates,
        writings,
        projects,
    })
}

fn load_publications(directory: &Path) -> Result<Vec<Publication>> {
    load_records(files_with_extensions(directory, &["json"])?, |path| {
        let record: Publication = read_json(&path)?;
        record.validate(&path.display().to_string())?;
        Ok(record)
    })
    .map(|mut records| {
        records.sort_by(|left, right| {
            right
                .year
                .cmp(&left.year)
                .then_with(|| left.slug.cmp(&right.slug))
        });
        records
    })
}

fn load_updates(path: &Path) -> Result<Vec<Update>> {
    let metadata = fs::symlink_metadata(path).with_context(|| {
        format!(
            "missing generated update data {}; run the OCaml generator first",
            path.display()
        )
    })?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "generated update data must not be a symbolic link: {}",
        path.display()
    );
    ensure!(
        metadata.is_file(),
        "missing generated update data {}; run the OCaml generator first",
        path.display()
    );
    let mut records: Vec<Update> = read_json(path)?;
    let mut diagnostics = Diagnostics::default();
    for (index, record) in records.iter().enumerate() {
        let _ = diagnostics.capture(record.validate(&format!(
            "{} (record {})",
            path.display(),
            index + 1
        )));
    }
    diagnostics.finish("generated update validation failed")?;
    sort_updates(&mut records);
    Ok(records)
}

/// Orders updates by announcement, newest first. Updates announced on the same
/// day are ordered by event date, newest first, then by id for determinism.
fn sort_updates(records: &mut [Update]) {
    records.sort_by(|left, right| {
        right
            .announced_on
            .cmp(&left.announced_on)
            .then_with(|| right.date.cmp(&left.date))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn load_writings(directory: &Path) -> Result<Vec<ValidatedWriting>> {
    let paths = files_with_extensions(directory, &["md", "mdx"])?;
    load_records(paths, |path| {
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            return Ok(None);
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (front_matter, body) = split_front_matter(&source, &path)?;
        let metadata: WritingFrontMatter = serde_yaml_ng::from_str(front_matter)
            .with_context(|| format!("invalid front matter in {}", path.display()))?;
        metadata.validate(&path.display().to_string())?;
        let rendered_body = render_markdown(body, metadata.math)
            .with_context(|| format!("failed to render {}", path.display()))?;
        Ok(Some(ValidatedWriting {
            metadata,
            rendered_body,
        }))
    })
    .map(|records| records.into_iter().flatten().collect::<Vec<_>>())
    .map(|mut records| {
        records.sort_by(|left, right| {
            right
                .metadata
                .date
                .cmp(&left.metadata.date)
                .then_with(|| left.metadata.slug.cmp(&right.metadata.slug))
        });
        records
    })
}

fn load_projects(directory: &Path) -> Result<Vec<Project>> {
    let paths = files_with_extensions(directory, &["md", "mdx"])?;
    load_records(paths, |path| {
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            return Ok(None);
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (front_matter, _) = split_front_matter(&source, &path)?;
        let metadata: ProjectFrontMatter = serde_yaml_ng::from_str(front_matter)
            .with_context(|| format!("invalid front matter in {}", path.display()))?;
        metadata.validate(&path.display().to_string())?;
        Ok(Some(Project { metadata }))
    })
    .map(|records| records.into_iter().flatten().collect::<Vec<_>>())
    .map(|mut records| {
        records.sort_by(|left, right| left.metadata.slug.cmp(&right.metadata.slug));
        records
    })
}

fn load_records<T>(
    paths: Vec<PathBuf>,
    mut load: impl FnMut(PathBuf) -> Result<T>,
) -> Result<Vec<T>> {
    let mut diagnostics = Diagnostics::default();
    let mut records = Vec::new();
    for path in paths {
        if let Some(record) = diagnostics.capture(load(path)) {
            records.push(record);
        }
    }
    diagnostics.finish("one or more content records are invalid")?;
    Ok(records)
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&source).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn files_with_extensions(directory: &Path, extensions: &[&str]) -> Result<Vec<PathBuf>> {
    let metadata = fs::symlink_metadata(directory)
        .with_context(|| format!("missing content directory {}", directory.display()))?;
    ensure!(
        !metadata.file_type().is_symlink() && metadata.is_dir(),
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
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        ensure!(
            !file_type.is_symlink(),
            "content tree must not contain symbolic links: {}",
            path.display()
        );
        if file_type.is_dir() {
            collect_files(&path, extensions, files)?;
        } else if file_type.is_file()
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

fn render_markdown(source: &str, math_enabled: bool) -> Result<RenderedMarkdown> {
    let mut options = Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;
    if math_enabled {
        options.insert(Options::ENABLE_MATH);
    }

    let mut events = Vec::new();
    for event in Parser::new_ext(source, options) {
        match &event {
            Event::Html(_) | Event::InlineHtml(_) => {
                bail!("raw HTML is not permitted in Markdown content")
            }
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                validate_markdown_url(dest_url)
                    .with_context(|| format!("unsafe Markdown URL {dest_url:?}"))?;
            }
            _ => {}
        }
        events.push(match event {
            Event::InlineMath(formula) => render_math_event(&formula, false)?,
            Event::DisplayMath(formula) => render_math_event(&formula, true)?,
            other => other,
        });
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    Ok(RenderedMarkdown(output))
}

fn render_math_event<'a>(formula: &str, display: bool) -> Result<Event<'a>> {
    let options = Opts::builder()
        .display_mode(display)
        .throw_on_error(true)
        .trust(false)
        .build()
        .context("failed to configure KaTeX")?;
    let rendered = render_with_opts(formula, &options)
        .with_context(|| format!("invalid TeX expression: {formula}"))?;
    Ok(Event::Html(CowStr::Boxed(rendered.into_boxed_str())))
}

fn validate_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str) -> Vec<anyhow::Error> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    for value in values {
        if !seen.insert(value) {
            errors.push(anyhow::anyhow!("duplicate {label} {value:?}"));
        }
    }
    errors
}

fn validate_relations(
    publications: &[Publication],
    updates: &[Update],
    writings: &[ValidatedWriting],
    projects: &[Project],
) -> Vec<anyhow::Error> {
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

    let mut errors = Vec::new();
    for update in updates {
        for id in &update.related.publications {
            if !publication_ids.contains(id.as_str()) {
                errors.push(anyhow::anyhow!(
                    "update {:?} references missing publication {id:?}",
                    update.id
                ));
            }
        }
        for id in &update.related.writings {
            if !writing_ids.contains(id.as_str()) {
                errors.push(anyhow::anyhow!(
                    "update {:?} references missing writing {id:?}",
                    update.id
                ));
            }
        }
        for id in &update.related.projects {
            if !project_ids.contains(id.as_str()) {
                errors.push(anyhow::anyhow!(
                    "update {:?} references missing project {id:?}",
                    update.id
                ));
            }
        }
    }
    for publication in publications {
        for id in &publication.project_ids {
            if !project_ids.contains(id.as_str()) {
                errors.push(anyhow::anyhow!(
                    "publication {:?} references missing project {id:?}",
                    publication.slug
                ));
            }
        }
    }
    errors
}

/// Published updates that still describe a planned activity although the event
/// has begun by `today`. Their wording is a historical claim, so they are
/// reported for factual review rather than rewritten or reclassified.
pub(crate) fn planned_updates_needing_review(
    updates: &[Update],
    today: NaiveDate,
) -> impl Iterator<Item = &Update> {
    updates
        .iter()
        .filter(move |update| update.event_status == EventStatus::Planned && update.date <= today)
}

pub(crate) fn public_english_writings(
    writings: &[ValidatedWriting],
) -> impl Iterator<Item = &ValidatedWriting> {
    writings
        .iter()
        .filter(|writing| !writing.metadata.draft && writing.metadata.lang == Language::En)
}

#[derive(Default)]
struct Diagnostics {
    errors: Vec<anyhow::Error>,
}

impl Diagnostics {
    fn capture<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.errors.push(error);
                None
            }
        }
    }

    fn extend(&mut self, errors: impl IntoIterator<Item = anyhow::Error>) {
        self.errors.extend(errors);
    }

    fn finish(self, context: &'static str) -> Result<()> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(CollectedErrors {
                context,
                messages: self
                    .errors
                    .into_iter()
                    .map(|error| format!("{error:#}"))
                    .collect(),
            }
            .into())
        }
    }
}

#[derive(Debug)]
struct CollectedErrors {
    context: &'static str,
    messages: Vec<String>,
}

impl fmt::Display for CollectedErrors {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "{} with {} error(s):",
            self.context,
            self.messages.len()
        )?;
        for (index, message) in self.messages.iter().enumerate() {
            write!(formatter, "  {}. {message}", index + 1)?;
            if index + 1 < self.messages.len() {
                writeln!(formatter)?;
            }
        }
        Ok(())
    }
}

impl StdError for CollectedErrors {}

#[cfg(test)]
mod tests {
    use super::{
        ValidatedSiteContent, ValidatedWriting, load_records, planned_updates_needing_review,
        render_markdown, sort_updates, split_front_matter,
    };
    use crate::model::{
        Category, EventKind, EventStatus, Language, Localized, Related, Update, WritingFrontMatter,
        WritingKind,
    };
    use crate::render;
    use anyhow::bail;
    use chrono::NaiveDate;
    use std::path::{Path, PathBuf};
    use std::time::Instant;

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
        assert!(html.as_str().contains("class=\"katex\""));
        assert!(html.as_str().contains("<math"));
    }

    #[test]
    fn katex_trust_commands_are_not_rendered() {
        let html = render_markdown("$\\href{javascript:alert(1)}{x}$", true).unwrap();
        assert!(!html.as_str().contains("href=\"javascript:"));
        assert!(!html.as_str().contains("<a "));
    }

    #[test]
    fn escapes_ordinary_markdown_text() {
        let html = render_markdown("2 < 3 & 5 > 4", false).unwrap();
        assert_eq!(html.as_str(), "<p>2 &lt; 3 &amp; 5 &gt; 4</p>\n");
    }

    #[test]
    fn rejects_raw_html_and_event_handlers() {
        assert!(render_markdown("<script>alert(1)</script>", false).is_err());
        assert!(render_markdown("<img src=x onerror=\"alert(1)\">", false).is_err());
    }

    #[test]
    fn rejects_dangerous_markdown_urls() {
        for source in [
            "[bad](javascript:alert%281%29)",
            "[bad](data:text/html,boom)",
            "![bad](vbscript:boom)",
        ] {
            assert!(render_markdown(source, false).is_err(), "{source:?}");
        }
    }

    #[test]
    fn renders_normal_markdown_links() {
        let html = render_markdown("[Research](/research/)", false).unwrap();
        assert_eq!(
            html.as_str(),
            "<p><a href=\"/research/\">Research</a></p>\n"
        );
    }

    #[test]
    fn independent_record_errors_are_accumulated_in_path_order() {
        let error = load_records(
            vec![PathBuf::from("a.json"), PathBuf::from("b.json")],
            |path| -> anyhow::Result<()> { bail!("{} is invalid", path.display()) },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("2 error(s)"));
        assert!(error.find("a.json").unwrap() < error.find("b.json").unwrap());
    }

    fn dated_update(
        id: &str,
        date: (u32, u32),
        announced: (u32, u32),
        status: EventStatus,
    ) -> Update {
        let day = |(month, day)| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
        Update {
            id: id.to_owned(),
            title: Localized {
                en: id.to_owned(),
                ja: None,
            },
            title_link: None,
            summary: Localized {
                en: id.to_owned(),
                ja: None,
            },
            date: day(date),
            end_date: None,
            announced_on: day(announced),
            event_status: status,
            categories: vec![Category::Academia],
            kind: EventKind::Participation,
            links: Vec::new(),
            related: Related {
                publications: Vec::new(),
                projects: Vec::new(),
                writings: Vec::new(),
            },
            detail: false,
            body: None,
        }
    }

    #[test]
    fn updates_are_ordered_by_announcement_then_event_date() {
        let mut updates = vec![
            dated_update("pbl", (9, 25), (10, 9), EventStatus::Completed),
            dated_update("older", (8, 1), (8, 2), EventStatus::Completed),
            dated_update("wakate", (10, 16), (10, 9), EventStatus::Planned),
            dated_update("poster", (10, 9), (10, 9), EventStatus::Completed),
        ];
        sort_updates(&mut updates);
        let ids: Vec<_> = updates.iter().map(|update| update.id.as_str()).collect();
        assert_eq!(ids, ["wakate", "poster", "pbl", "older"]);
    }

    #[test]
    fn planned_updates_are_flagged_once_the_event_begins_without_being_changed() {
        let updates = vec![
            dated_update("wakate", (10, 16), (10, 9), EventStatus::Planned),
            dated_update("poster", (10, 9), (10, 9), EventStatus::Completed),
        ];
        let day = |day| NaiveDate::from_ymd_opt(2026, 10, day).unwrap();
        assert_eq!(planned_updates_needing_review(&updates, day(15)).count(), 0);
        let flagged: Vec<_> = planned_updates_needing_review(&updates, day(16)).collect();
        assert_eq!(flagged.len(), 1);
        assert_eq!(flagged[0].event_status, EventStatus::Planned);
    }

    #[test]
    #[ignore = "manual synthetic performance measurement"]
    fn synthetic_markdown_and_rendering_workload() {
        let writing_count = 100;
        let update_count = 1_000;
        let started = Instant::now();
        let writings = (0..writing_count)
            .map(|index| {
                let source = if index % 10 == 0 {
                    "A mathematical note: $e^{i\\pi}+1=0$."
                } else {
                    "A **synthetic** note with a [normal link](/research/)."
                };
                ValidatedWriting {
                    metadata: WritingFrontMatter {
                        slug: format!("synthetic-writing-{index}"),
                        title: format!("Synthetic writing {index}"),
                        description: "Synthetic performance fixture".to_owned(),
                        date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                        lang: Language::En,
                        kind: WritingKind::Note,
                        translation_key: None,
                        publication: None,
                        external_url: None,
                        math: index % 10 == 0,
                        draft: false,
                    },
                    rendered_body: render_markdown(source, index % 10 == 0).unwrap(),
                }
            })
            .collect();
        let markdown_time = started.elapsed();

        let updates = (0..update_count)
            .map(|index| Update {
                id: format!("synthetic-update-{index}"),
                title: Localized {
                    en: format!("Synthetic update {index}"),
                    ja: None,
                },
                title_link: None,
                summary: Localized {
                    en: "Synthetic performance fixture".to_owned(),
                    ja: None,
                },
                date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                end_date: None,
                announced_on: NaiveDate::from_ymd_opt(2026, 1, 2).unwrap(),
                event_status: EventStatus::Completed,
                categories: vec![Category::Research],
                kind: EventKind::Other { detail: None },
                links: Vec::new(),
                related: Related {
                    publications: Vec::new(),
                    projects: Vec::new(),
                    writings: Vec::new(),
                },
                detail: true,
                body: None,
            })
            .collect();
        let content = ValidatedSiteContent {
            publications: Vec::new(),
            updates,
            writings,
            projects: Vec::new(),
        };
        let started = Instant::now();
        let pages = render::pages(&content);
        let rendering_time = started.elapsed();
        let bytes = pages.iter().map(|page| page.html.len()).sum::<usize>();
        std::hint::black_box(&pages);

        assert_eq!(pages.len(), 1_109);
        eprintln!(
            "synthetic workload: writings={writing_count} (10 with KaTeX), updates={update_count}, pages={}, bytes={bytes}, markdown_ms={:.3}, render_ms={:.3}",
            pages.len(),
            markdown_time.as_secs_f64() * 1_000.0,
            rendering_time.as_secs_f64() * 1_000.0,
        );
    }
}
