use std::collections::{BTreeSet, HashSet};
use std::error::Error as StdError;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail, ensure};
use katex::{Opts, render_with_opts};
use pulldown_cmark::{CowStr, Event, HeadingLevel, Options, Parser, Tag, TagEnd, html};
use serde::de::DeserializeOwned;

use chrono::NaiveDate;

use crate::model::{
    DiaryFrontMatter, DisplayLanguage, EventStatus, HomeSource, Language, LanguageTag,
    NoteFrontMatter, Paragraph, PassageFrontMatter, PassageLanguage, Project, ProjectFrontMatter,
    Publication, ReadingComment, ReadingItem, ReadingList, Update, WritingFrontMatter,
    validate_markdown_url,
};

#[derive(Debug)]
pub(crate) struct ValidatedSiteContent {
    publications: Vec<Publication>,
    updates: Vec<Update>,
    writings: Vec<ValidatedWriting>,
    projects: Vec<Project>,
    miscellany: Miscellany,
    home: ValidatedHome,
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

    pub(crate) fn miscellany(&self) -> &Miscellany {
        &self.miscellany
    }

    pub(crate) fn home(&self) -> &ValidatedHome {
        &self.home
    }

    /// Site content with only the given Miscellany, for rendering tests.
    #[cfg(test)]
    pub(crate) fn with_miscellany(miscellany: Miscellany) -> Self {
        Self {
            publications: Vec::new(),
            updates: Vec::new(),
            writings: Vec::new(),
            projects: Vec::new(),
            miscellany,
            home: ValidatedHome::from_json(r#"{"bio":["Fixture biography."]}"#)
                .expect("the fixture biography is valid"),
        }
    }
}

/// The homepage biography from `content/home.json`: paragraphs of text and
/// checked links, in the authored order.
#[derive(Debug)]
pub(crate) struct ValidatedHome {
    bio: Vec<Paragraph>,
}

impl ValidatedHome {
    pub(crate) fn bio(&self) -> &[Paragraph] {
        &self.bio
    }

    fn parse(path: &Path, source: &str) -> Result<Self> {
        let home: HomeSource = serde_json::from_str(source)
            .with_context(|| format!("invalid JSON in {}", path.display()))?;
        ensure!(
            !home.bio.is_empty(),
            "{}: bio must contain at least one paragraph",
            path.display()
        );
        let mut diagnostics = Diagnostics::default();
        let bio: Vec<_> =
            home.bio
                .iter()
                .enumerate()
                .filter_map(|(index, paragraph)| {
                    diagnostics.capture(Paragraph::parse(paragraph).with_context(|| {
                        format!("{}: bio paragraph {}", path.display(), index + 1)
                    }))
                })
                .collect();
        diagnostics.finish("homepage biography is invalid")?;
        Ok(Self { bio })
    }

    /// A biography parsed from in-memory JSON through the same validation.
    #[cfg(test)]
    pub(crate) fn from_json(source: &str) -> Result<Self> {
        Self::parse(Path::new("home.json"), source)
    }
}

fn load_home(path: &Path) -> Result<ValidatedHome> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("missing homepage content {}", path.display()))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "missing homepage content {}",
        path.display()
    );
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    ValidatedHome::parse(path, &source)
}

/// Reading records, study notes, and diary entries. Each has its own small
/// type because their requirements differ: reading items have no body or
/// date, notes have no date, and diary entries need no title.
#[derive(Debug, Default)]
pub(crate) struct Miscellany {
    reading: Vec<ValidatedReadingItem>,
    notes: Vec<ValidatedNote>,
    diary: Vec<ValidatedDiaryEntry>,
}

impl Miscellany {
    /// The reading record in the author's order.
    pub(crate) fn reading(&self) -> &[ValidatedReadingItem] {
        &self.reading
    }

    /// Published notes in editorial order.
    pub(crate) fn public_notes(&self) -> impl Iterator<Item = &ValidatedNote> {
        self.notes.iter().filter(|note| !note.metadata.draft)
    }

    /// Published diary entries, newest first.
    pub(crate) fn public_diary(&self) -> impl Iterator<Item = &ValidatedDiaryEntry> {
        self.diary.iter().filter(|entry| !entry.metadata.draft)
    }

    /// The display languages of published, separately authored passages and
    /// reading commentary, in display order. Languages that appear only in
    /// inline phrases or mixed passages are not included.
    pub(crate) fn display_languages(&self) -> Vec<DisplayLanguage> {
        let languages: BTreeSet<_> = self
            .public_passage_language_sets()
            .into_iter()
            .flatten()
            .collect();
        languages.into_iter().collect()
    }

    /// Whether any published note, diary entry, or reading item has
    /// separately authored texts in more than one language.
    pub(crate) fn has_multilingual_entry(&self) -> bool {
        self.public_passage_language_sets()
            .iter()
            .any(|languages| languages.len() > 1)
    }

    fn public_passage_language_sets(&self) -> Vec<Vec<DisplayLanguage>> {
        let reading = self.reading.iter().map(|item| {
            item.comments
                .iter()
                .filter_map(|comment| comment.language.designated())
                .collect()
        });
        let notes = self
            .public_notes()
            .map(|note| note.passages.designated_languages());
        let diary = self
            .public_diary()
            .map(|entry| entry.passages.designated_languages());
        reading.chain(notes).chain(diary).collect()
    }
}

impl Miscellany {
    /// Note slugs and diary anchors must be unique, drafts included, so that
    /// publishing a draft can never collide with an existing URL.
    fn validate_identifiers(&self) -> Vec<anyhow::Error> {
        let mut errors = validate_unique(
            self.notes
                .iter()
                .map(|record| record.metadata.slug.as_str()),
            "note slug",
        );
        let anchors: Vec<_> = self
            .diary
            .iter()
            .map(|record| record.metadata.anchor())
            .collect();
        errors.extend(
            validate_unique(anchors.iter().map(String::as_str), "diary entry")
                .into_iter()
                .map(|error| error.context("entries sharing a date need distinct slugs")),
        );
        errors
    }

    /// Miscellany parsed from in-memory sources through the same validation
    /// as the content directory, for tests that must not publish fixtures.
    /// Companion passages are given among the sources of their section.
    #[cfg(test)]
    pub(crate) fn from_sources(reading: &str, notes: &[&str], diary: &[&str]) -> Result<Self> {
        fn sources(kind: &str, sources: &[&str]) -> Result<Vec<SourceFile>> {
            sources
                .iter()
                .enumerate()
                .map(|(index, source)| {
                    let path = PathBuf::from(format!("{kind}-{index}.md"));
                    let (front_matter, body) = split_front_matter(source, &path)?;
                    Ok(SourceFile {
                        front_matter: front_matter.to_owned(),
                        body: body.to_owned(),
                        path,
                    })
                })
                .collect()
        }
        let mut miscellany = Self {
            reading: parse_reading(Path::new("reading.yaml"), reading)?,
            notes: assemble_articles(sources("note", notes)?)?,
            diary: assemble_articles(sources("diary", diary)?)?,
        };
        sort_notes(&mut miscellany.notes);
        sort_diary(&mut miscellany.diary);
        let mut diagnostics = Diagnostics::default();
        diagnostics.extend(miscellany.validate_identifiers());
        diagnostics.finish("miscellany validation failed")?;
        Ok(miscellany)
    }
}

/// A reading item: the bibliographic record of one work, and the author's
/// commentary on it as separately authored passages.
#[derive(Debug)]
pub(crate) struct ValidatedReadingItem {
    work: ReadingItem,
    comments: Vec<ReadingComment>,
}

impl ValidatedReadingItem {
    pub(crate) fn work(&self) -> &ReadingItem {
        &self.work
    }

    pub(crate) fn comments(&self) -> &[ReadingComment] {
        &self.comments
    }
}

/// One logical article: its identity and shared metadata, kept apart from the
/// separately authored passages that make up its text. However many
/// languages it is written in, it has one identity, one URL or anchor, and
/// one entry in every listing.
#[derive(Debug)]
pub(crate) struct Article<M> {
    metadata: M,
    passages: Passages,
}

impl<M> Article<M> {
    pub(crate) fn metadata(&self) -> &M {
        &self.metadata
    }

    pub(crate) fn passages(&self) -> &Passages {
        &self.passages
    }
}

pub(crate) type ValidatedNote = Article<NoteFrontMatter>;
pub(crate) type ValidatedDiaryEntry = Article<DiaryFrontMatter>;

/// A separately authored passage of an article, in one language or
/// deliberately mixed, with an optional title of its own.
#[derive(Debug)]
pub(crate) struct Passage {
    language: PassageLanguage,
    title: Option<String>,
    body: RenderedMarkdown,
}

impl Passage {
    pub(crate) fn language(&self) -> PassageLanguage {
        self.language
    }

    pub(crate) fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub(crate) fn rendered_body(&self) -> &str {
        self.body.as_str()
    }
}

/// An article's passages: at least one, at most one per passage language,
/// with designated languages in display order followed by any mixed passage.
#[derive(Debug)]
pub(crate) struct Passages(Vec<Passage>);

impl Passages {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Passage> {
        self.0.iter()
    }

    pub(crate) fn designated_languages(&self) -> Vec<DisplayLanguage> {
        self.0
            .iter()
            .filter_map(|passage| passage.language.designated())
            .collect()
    }

    pub(crate) fn has_mixed(&self) -> bool {
        self.0
            .iter()
            .any(|passage| passage.language == PassageLanguage::Mixed)
    }

    /// Orders the passages, rejecting two passages in the same language.
    fn assemble(mut passages: Vec<(PathBuf, Passage)>) -> Result<Self> {
        passages.sort_by_key(|(_, passage)| passage.language);
        for pair in passages.windows(2) {
            ensure!(
                pair[0].1.language != pair[1].1.language,
                "{} and {} are both {} passages of the same article",
                pair[0].0.display(),
                pair[1].0.display(),
                pair[0].1.language.code()
            );
        }
        Ok(Self(
            passages.into_iter().map(|(_, passage)| passage).collect(),
        ))
    }
}

/// What the shared article assembly needs to know about a kind of article.
trait ArticleMetadata: DeserializeOwned {
    /// How errors name this kind of article.
    const KIND: &'static str;
    /// The most prominent heading an untitled passage body may contain; a
    /// passage title takes this level and pushes its body one level down.
    const HEADING: HeadingLevel;
    /// Whether passage bodies may contain footnotes.
    const FOOTNOTES: bool;

    fn validate(&self, source: &str) -> Result<()>;
    /// The identity that companion passages name with `of`.
    fn id(&self) -> String;
    fn language(&self) -> PassageLanguage;
    fn math(&self) -> bool;
}

impl ArticleMetadata for NoteFrontMatter {
    const KIND: &'static str = "note";
    const HEADING: HeadingLevel = HeadingLevel::H2;
    const FOOTNOTES: bool = true;

    fn validate(&self, source: &str) -> Result<()> {
        NoteFrontMatter::validate(self, source)
    }
    fn id(&self) -> String {
        self.slug.clone()
    }
    fn language(&self) -> PassageLanguage {
        self.lang
    }
    fn math(&self) -> bool {
        self.math
    }
}

impl ArticleMetadata for DiaryFrontMatter {
    const KIND: &'static str = "diary entry";
    // Entries share one page beneath the h1 "Diary" and an h2 per entry, and
    // footnote identifiers there would need to be unique across entries.
    const HEADING: HeadingLevel = HeadingLevel::H3;
    const FOOTNOTES: bool = false;

    fn validate(&self, source: &str) -> Result<()> {
        DiaryFrontMatter::validate(self, source)
    }
    fn id(&self) -> String {
        DiaryFrontMatter::id(self)
    }
    fn language(&self) -> PassageLanguage {
        self.lang
    }
    fn math(&self) -> bool {
        self.math
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
    let miscellany = diagnostics.capture(load_miscellany(&root.join("content/miscellany")));
    let home = diagnostics.capture(load_home(&root.join("content/home.json")));
    diagnostics.finish("content loading failed")?;

    let publications = publications.expect("successful diagnostics contain publications");
    let updates = updates.expect("successful diagnostics contain updates");
    let writings = writings.expect("successful diagnostics contain writings");
    let projects = projects.expect("successful diagnostics contain projects");
    let miscellany = miscellany.expect("successful diagnostics contain miscellany");
    let home = home.expect("successful diagnostics contain the homepage");

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
    diagnostics.extend(miscellany.validate_identifiers());

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
        miscellany,
        home,
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

/// Orders updates by event date, most recently held first (see
/// `Update::most_recent_first`). This deterministic order is the RSS feed's;
/// pages separate planned from completed records and put Upcoming soonest
/// first (see `render::split_upcoming`).
fn sort_updates(records: &mut [Update]) {
    records.sort_by(Update::most_recent_first);
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

fn load_miscellany(directory: &Path) -> Result<Miscellany> {
    let mut diagnostics = Diagnostics::default();
    let reading = diagnostics.capture(load_reading(&directory.join("reading.yaml")));
    let notes = diagnostics.capture(load_notes(&directory.join("notes")));
    let diary = diagnostics.capture(load_diary(&directory.join("diary")));
    diagnostics.finish("miscellany loading failed")?;
    Ok(Miscellany {
        reading: reading.expect("successful diagnostics contain reading"),
        notes: notes.expect("successful diagnostics contain notes"),
        diary: diary.expect("successful diagnostics contain diary"),
    })
}

fn load_reading(path: &Path) -> Result<Vec<ValidatedReadingItem>> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("missing reading list {}", path.display()))?;
    ensure!(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "missing reading list {}",
        path.display()
    );
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    parse_reading(path, &source)
}

fn parse_reading(path: &Path, source: &str) -> Result<Vec<ValidatedReadingItem>> {
    let list: ReadingList = serde_yaml_ng::from_str(source)
        .with_context(|| format!("invalid reading list {}", path.display()))?;
    list.validate(&path.display().to_string())?;
    list.items
        .into_iter()
        .enumerate()
        .map(|(index, work)| {
            let source = format!("{} (item {})", path.display(), index + 1);
            Ok(ValidatedReadingItem {
                comments: work.comments(&source)?,
                work,
            })
        })
        .collect()
}

/// A Markdown source file split into its front matter and body.
struct SourceFile {
    path: PathBuf,
    front_matter: String,
    body: String,
}

/// Markdown files with front matter in `directory`, skipping its README.
fn load_markdown_sources(directory: &Path) -> Result<Vec<SourceFile>> {
    let paths = files_with_extensions(directory, &["md"])?;
    load_records(paths, |path| {
        if path.file_name().and_then(|name| name.to_str()) == Some("README.md") {
            return Ok(None);
        }
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let (front_matter, body) = split_front_matter(&source, &path)?;
        Ok(Some(SourceFile {
            front_matter: front_matter.to_owned(),
            body: body.to_owned(),
            path,
        }))
    })
    .map(|records| records.into_iter().flatten().collect())
}

fn load_notes(directory: &Path) -> Result<Vec<ValidatedNote>> {
    let mut notes = assemble_articles(load_markdown_sources(directory)?)?;
    sort_notes(&mut notes);
    Ok(notes)
}

/// A source file is either an article's primary file, holding its identity,
/// shared metadata, and first passage, or a companion passage naming its
/// article with `of`.
enum ArticleSource<M> {
    Primary {
        path: PathBuf,
        metadata: M,
        passage: Passage,
    },
    Companion {
        path: PathBuf,
        of: String,
        passage: Passage,
    },
}

fn parse_article_source<M: ArticleMetadata>(file: SourceFile) -> Result<ArticleSource<M>> {
    let path = file.path;
    let invalid = || format!("invalid front matter in {}", path.display());
    let fields: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&file.front_matter).with_context(invalid)?;
    if fields.get("of").is_some() {
        let metadata: PassageFrontMatter =
            serde_yaml_ng::from_value(fields).with_context(invalid)?;
        metadata.validate(&path.display().to_string())?;
        let passage = render_passage::<M>(
            &path,
            &file.body,
            metadata.lang,
            metadata.title,
            metadata.math,
        )?;
        Ok(ArticleSource::Companion {
            of: metadata.of,
            passage,
            path,
        })
    } else {
        let metadata: M = serde_yaml_ng::from_value(fields).with_context(invalid)?;
        metadata.validate(&path.display().to_string())?;
        let passage = render_passage::<M>(
            &path,
            &file.body,
            metadata.language(),
            None,
            metadata.math(),
        )?;
        Ok(ArticleSource::Primary {
            path,
            metadata,
            passage,
        })
    }
}

fn render_passage<M: ArticleMetadata>(
    path: &Path,
    body: &str,
    language: PassageLanguage,
    title: Option<String>,
    math: bool,
) -> Result<Passage> {
    ensure!(
        !body.trim().is_empty(),
        "{}: the passage text must not be empty",
        path.display()
    );
    let highest_heading = if title.is_some() {
        next_heading_level(M::HEADING)
    } else {
        M::HEADING
    };
    let rules = MarkdownRules {
        math,
        highest_heading,
        footnotes: M::FOOTNOTES,
        // Footnote identifiers are prefixed by passage language so that the
        // passages of one page never share an identifier.
        footnote_prefix: Some(language.code()),
        inline_languages: true,
    };
    let body = render_markdown_with(body, rules)
        .with_context(|| format!("failed to render {}", path.display()))?;
    Ok(Passage {
        language,
        title,
        body,
    })
}

const fn next_heading_level(level: HeadingLevel) -> HeadingLevel {
    match level {
        HeadingLevel::H1 => HeadingLevel::H2,
        HeadingLevel::H2 => HeadingLevel::H3,
        HeadingLevel::H3 => HeadingLevel::H4,
        HeadingLevel::H4 => HeadingLevel::H5,
        HeadingLevel::H5 | HeadingLevel::H6 => HeadingLevel::H6,
    }
}

/// Gathers each article's primary file and companion passages into one
/// article, reporting every invalid file, companion naming no article, and
/// duplicated passage language.
fn assemble_articles<M: ArticleMetadata>(files: Vec<SourceFile>) -> Result<Vec<Article<M>>> {
    let mut diagnostics = Diagnostics::default();
    let mut primaries = Vec::new();
    let mut companions = Vec::new();
    for file in files {
        match diagnostics.capture(parse_article_source::<M>(file)) {
            Some(ArticleSource::Primary {
                path,
                metadata,
                passage,
            }) => primaries.push((metadata.id(), metadata, vec![(path, passage)])),
            Some(ArticleSource::Companion { path, of, passage }) => {
                companions.push((path, of, passage));
            }
            None => {}
        }
    }
    for (path, of, passage) in companions {
        match primaries.iter_mut().find(|(id, _, _)| *id == of) {
            Some((_, _, passages)) => passages.push((path, passage)),
            None => diagnostics.extend([anyhow::anyhow!(
                "{}: of {of:?} names no {}",
                path.display(),
                M::KIND
            )]),
        }
    }
    let articles = primaries
        .into_iter()
        .filter_map(|(_, metadata, passages)| {
            let passages = diagnostics.capture(Passages::assemble(passages))?;
            Some(Article { metadata, passages })
        })
        .collect();
    diagnostics.finish("one or more content records are invalid")?;
    Ok(articles)
}

/// Editorial order: notes with an `order` come first, lowest first; the rest
/// follow. Ties are broken by slug.
fn sort_notes(notes: &mut [ValidatedNote]) {
    notes.sort_by(|left, right| {
        let key = |note: &ValidatedNote| (note.metadata.order.is_none(), note.metadata.order);
        key(left)
            .cmp(&key(right))
            .then_with(|| left.metadata.slug.cmp(&right.metadata.slug))
    });
}

fn load_diary(directory: &Path) -> Result<Vec<ValidatedDiaryEntry>> {
    let mut entries = assemble_articles(load_markdown_sources(directory)?)?;
    sort_diary(&mut entries);
    Ok(entries)
}

/// Newest first by the entry's own date; entries sharing a date follow their
/// anchors for determinism.
fn sort_diary(entries: &mut [ValidatedDiaryEntry]) {
    entries.sort_by(|left, right| {
        right
            .metadata
            .date
            .cmp(&left.metadata.date)
            .then_with(|| left.metadata.anchor().cmp(&right.metadata.anchor()))
    });
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

/// Constraints on a Markdown body beyond the shared safety rules.
#[derive(Clone, Copy)]
struct MarkdownRules {
    math: bool,
    /// The most prominent heading the body may contain, so that it fits
    /// beneath the headings of the page it appears on.
    highest_heading: HeadingLevel,
    footnotes: bool,
    /// Prefixed to footnote labels, keeping identifiers unique when several
    /// bodies share one page.
    footnote_prefix: Option<&'static str>,
    /// Whether `[phrase]{lang=xx}` marks the language of a phrase.
    inline_languages: bool,
}

fn render_markdown(source: &str, math_enabled: bool) -> Result<RenderedMarkdown> {
    render_markdown_with(
        source,
        MarkdownRules {
            math: math_enabled,
            highest_heading: HeadingLevel::H1,
            footnotes: true,
            footnote_prefix: None,
            inline_languages: false,
        },
    )
}

fn render_markdown_with(source: &str, rules: MarkdownRules) -> Result<RenderedMarkdown> {
    let mut options = Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION;
    if rules.math {
        options.insert(Options::ENABLE_MATH);
    }

    let mut events = Vec::new();
    for event in Parser::new_ext(source, options) {
        match &event {
            Event::Html(_) | Event::InlineHtml(_) => {
                bail!("raw HTML is not permitted in Markdown content")
            }
            Event::Start(Tag::Heading { level, .. }) if *level < rules.highest_heading => {
                bail!(
                    "headings in this content must be level {} (`{}`) or lower",
                    rules.highest_heading as usize,
                    "#".repeat(rules.highest_heading as usize)
                )
            }
            Event::FootnoteReference(_) | Event::Start(Tag::FootnoteDefinition(_))
                if !rules.footnotes =>
            {
                bail!("footnotes are not supported in this content")
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
            Event::FootnoteReference(label) => Event::FootnoteReference(prefixed(label, &rules)),
            Event::Start(Tag::FootnoteDefinition(label)) => {
                Event::Start(Tag::FootnoteDefinition(prefixed(label, &rules)))
            }
            other => other,
        });
    }
    if rules.inline_languages {
        events = mark_inline_languages(events)?;
    }
    let mut output = String::new();
    html::push_html(&mut output, events.into_iter());
    Ok(RenderedMarkdown(output))
}

fn prefixed<'a>(label: CowStr<'a>, rules: &MarkdownRules) -> CowStr<'a> {
    match rules.footnote_prefix {
        Some(prefix) => format!("{prefix}-{label}").into(),
        None => label,
    }
}

/// Marks phrases written `[phrase]{lang=xx}` with `<span lang="xx">`. This
/// works on the parsed event stream, never on HTML: adjacent text events are
/// joined, code and image descriptions are left alone, and the phrase must be
/// plain text within one run of text. The language tag is validated, so the
/// only markup produced is the span itself; the phrase stays escaped text.
fn mark_inline_languages(events: Vec<Event<'_>>) -> Result<Vec<Event<'_>>> {
    let mut output = Vec::with_capacity(events.len());
    let mut text = String::new();
    let mut verbatim_depth = 0_usize;
    for event in events {
        match event {
            Event::Text(run) if verbatim_depth == 0 => text.push_str(&run),
            other => {
                push_marked_text(std::mem::take(&mut text), &mut output)?;
                match &other {
                    Event::Start(Tag::CodeBlock(_) | Tag::Image { .. }) => verbatim_depth += 1,
                    Event::End(TagEnd::CodeBlock | TagEnd::Image) => verbatim_depth -= 1,
                    _ => {}
                }
                output.push(other);
            }
        }
    }
    push_marked_text(text, &mut output)?;
    Ok(output)
}

fn push_marked_text(text: String, output: &mut Vec<Event<'_>>) -> Result<()> {
    const MARKER: &str = "]{lang=";
    let mut rest = text.as_str();
    while let Some(marker) = rest.find(MARKER) {
        let open = rest[..marker].rfind('[').with_context(|| {
            format!(
                "an inline language mark must follow plain text in brackets, as in [Stimme]{{lang=de}}, near {:?}",
                &rest[marker..]
            )
        })?;
        let phrase = &rest[open + 1..marker];
        ensure!(
            !phrase.trim().is_empty() && !phrase.contains(']'),
            "an inline language mark needs a plain phrase in brackets, near {:?}",
            &rest[open..]
        );
        let after = &rest[marker + MARKER.len()..];
        let close = after
            .find('}')
            .with_context(|| format!("unclosed inline language mark near {:?}", &rest[open..]))?;
        let tag = LanguageTag::parse(&after[..close])?;
        if open > 0 {
            output.push(Event::Text(rest[..open].to_owned().into()));
        }
        output.push(Event::InlineHtml(
            format!("<span lang=\"{}\">", tag.as_str()).into(),
        ));
        output.push(Event::Text(phrase.to_owned().into()));
        output.push(Event::InlineHtml("</span>".into()));
        rest = &after[close + 1..];
    }
    if !rest.is_empty() {
        output.push(Event::Text(rest.to_owned().into()));
    }
    Ok(())
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
        Miscellany, ValidatedHome, ValidatedSiteContent, ValidatedWriting, load_miscellany,
        load_records, planned_updates_needing_review, render_markdown, sort_updates,
        split_front_matter,
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
        end: Option<(u32, u32)>,
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
            end_date: end.map(day),
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
    fn updates_are_ordered_by_final_event_day_then_start_then_id() {
        let mut updates = vec![
            dated_update("pbl", (9, 25), None, EventStatus::Completed),
            dated_update("older", (8, 1), None, EventStatus::Completed),
            dated_update("wakate", (10, 16), Some((10, 18)), EventStatus::Planned),
            dated_update("camp", (9, 20), Some((9, 25)), EventStatus::Completed),
            dated_update("alpha", (9, 25), None, EventStatus::Completed),
            dated_update("poster", (10, 9), None, EventStatus::Completed),
        ];
        sort_updates(&mut updates);
        let ids: Vec<_> = updates.iter().map(|update| update.id.as_str()).collect();
        assert_eq!(ids, ["wakate", "poster", "alpha", "pbl", "camp", "older"]);
    }

    #[test]
    fn planned_updates_are_flagged_once_the_event_begins_without_being_changed() {
        let updates = vec![
            dated_update("wakate", (10, 16), None, EventStatus::Planned),
            dated_update("poster", (10, 9), None, EventStatus::Completed),
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
            miscellany: Miscellany::default(),
            home: ValidatedHome::from_json(r#"{"bio":["Synthetic biography."]}"#).unwrap(),
        };
        let started = Instant::now();
        let pages = render::pages(&content);
        let rendering_time = started.elapsed();
        let bytes = pages.iter().map(|page| page.html.len()).sum::<usize>();
        std::hint::black_box(&pages);

        assert_eq!(pages.len(), 1_113);
        eprintln!(
            "synthetic workload: writings={writing_count} (10 with KaTeX), updates={update_count}, pages={}, bytes={bytes}, markdown_ms={:.3}, render_ms={:.3}",
            pages.len(),
            markdown_time.as_secs_f64() * 1_000.0,
            rendering_time.as_secs_f64() * 1_000.0,
        );
    }

    /// A note source with the given extra front matter and body. Fixtures are
    /// drafts unless a test publishes them explicitly.
    fn note(front_matter: &str, body: &str) -> String {
        format!("---\nslug: fixture-note\ntitle: Fixture note\nlang: en\n{front_matter}---\n{body}")
    }

    fn diary(front_matter: &str, body: &str) -> String {
        format!("---\ndate: 2026-10-10\nlang: en\n{front_matter}---\n{body}")
    }

    fn error_of(result: anyhow::Result<Miscellany>) -> String {
        format!("{:#}", result.unwrap_err())
    }

    #[test]
    fn miscellany_loads_from_its_directory_and_skips_readmes() {
        let root =
            std::env::temp_dir().join(format!("site-builder-miscellany-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |path: &str, contents: &str| {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, contents).unwrap();
        };
        write("reading.yaml", "items:\n  - title: Fixture work\n");
        write("notes/README.md", "# Not front matter\n");
        write("diary/README.md", "# Not front matter\n");
        write("notes/a.md", &note("draft: false\n", "Body.\n"));
        write("diary/a.md", &diary("", "Body.\n"));
        let loaded = load_miscellany(&root);
        std::fs::remove_file(root.join("reading.yaml")).unwrap();
        let missing = load_miscellany(&root);
        std::fs::remove_dir_all(&root).unwrap();

        let miscellany = loaded.unwrap();
        assert_eq!(miscellany.reading().len(), 1);
        assert_eq!(miscellany.public_notes().count(), 1);
        // Diary entries are drafts unless published explicitly.
        assert_eq!(miscellany.public_diary().count(), 0);
        assert!(format!("{:#}", missing.unwrap_err()).contains("missing reading list"));
    }

    #[test]
    fn notes_and_diary_entries_default_to_drafts() {
        let miscellany =
            Miscellany::from_sources("items: []", &[&note("", "x")], &[&diary("", "x")]).unwrap();
        assert_eq!(miscellany.public_notes().count(), 0);
        assert_eq!(miscellany.public_diary().count(), 0);
    }

    #[test]
    fn invalid_miscellany_metadata_is_rejected() {
        let reading = |items: &str| Miscellany::from_sources(items, &[], &[]);
        for (source, expected) in [
            ("items:\n  - title: ''\n", "title must not be empty"),
            (
                "items:\n  - title: X\n    author: ' '\n",
                "author must not be empty",
            ),
            (
                "items:\n  - title: X\n    noteLang: ja\n",
                "noteLang requires a note",
            ),
            (
                "items:\n  - title: X\n    lang: Japanese\n",
                "language tag \"Japanese\"",
            ),
            (
                "items:\n  - title: X\n    note: x\n    noteLang: fr\n",
                "not supported",
            ),
            (
                "items:\n  - title: X\n    lang: ja-\n",
                "language tag \"ja-\"",
            ),
            (
                "items:\n  - title: X\n    url: javascript:alert(1)\n",
                "invalid url",
            ),
            ("items:\n  - title: X\n    url: /relative/\n", "invalid url"),
            ("items:\n  - title: X\n    rating: 5\n", "unknown field"),
            ("items:\n  - author: Anonymous\n", "missing field `title`"),
        ] {
            let error = error_of(reading(source));
            assert!(error.contains(expected), "{source}: {error}");
        }
        assert!(
            reading(
                "items:\n  - title: 竹取物語\n    lang: ja\n  - title: Tractatus\n    lang: de-AT\n"
            )
            .is_ok()
        );

        let notes = |source: &str| Miscellany::from_sources("items: []", &[source], &[]);
        for (source, expected) in [
            (note("progress: 50\n", ""), "unknown field"),
            (note("lang: fr\n", ""), "duplicate entry with key \"lang\""),
            (
                note("description: ''\n", ""),
                "description must not be empty",
            ),
            (
                "---\nslug: Bad_Slug\ntitle: X\nlang: en\n---\n".to_owned(),
                "invalid note slug",
            ),
            (
                "---\nslug: x\ntitle: X\nlang: fr\n---\n".to_owned(),
                "is not supported (expected en, ja, de or mixed)",
            ),
            (
                "---\nslug: x\nlang: en\n---\n".to_owned(),
                "missing field `title`",
            ),
        ] {
            let error = error_of(notes(&source));
            assert!(error.contains(expected), "{source}: {error}");
        }

        let entries = |source: &str| Miscellany::from_sources("items: []", &[], &[source]);
        for (source, expected) in [
            ("---\nlang: en\n---\nx".to_owned(), "missing field `date`"),
            (
                "---\ndate: 2026-02-30\nlang: en\n---\nx".to_owned(),
                "invalid front matter",
            ),
            (diary("title: ''\n", "x"), "title must not be empty"),
            (diary("slug: Two Words\n", "x"), "invalid diary slug"),
            (diary("mood: calm\n", "x"), "unknown field"),
        ] {
            let error = error_of(entries(&source));
            assert!(error.contains(expected), "{source}: {error}");
        }
    }

    #[test]
    fn miscellany_identifiers_must_be_unique_including_drafts() {
        let published = note("draft: false\n", "x");
        let error = error_of(Miscellany::from_sources(
            "items: []",
            &[&published, &note("", "y")],
            &[],
        ));
        assert!(
            error.contains("duplicate note slug \"fixture-note\""),
            "{error}"
        );

        let error = error_of(Miscellany::from_sources(
            "items: []",
            &[],
            &[&diary("", "a"), &diary("", "b")],
        ));
        assert!(
            error.contains("entries sharing a date need distinct slugs"),
            "{error}"
        );
        assert!(
            Miscellany::from_sources(
                "items: []",
                &[],
                &[&diary("", "a"), &diary("slug: evening\n", "b")],
            )
            .is_ok()
        );
    }

    #[test]
    fn miscellany_markdown_keeps_the_shared_safety_rules() {
        let notes = |body: &str| Miscellany::from_sources("items: []", &[&note("", body)], &[]);
        let entries = |body: &str| Miscellany::from_sources("items: []", &[], &[&diary("", body)]);
        for body in ["<script>alert(1)</script>\n", "Text <b>bold</b>.\n"] {
            assert!(error_of(notes(body)).contains("raw HTML is not permitted"));
            assert!(error_of(entries(body)).contains("raw HTML is not permitted"));
        }
        for body in [
            "[x](javascript:alert(1))\n",
            "[x](//example.org/)\n",
            "![x](data:image/png;base64,AAAA)\n",
        ] {
            assert!(
                error_of(notes(body)).contains("unsafe Markdown URL"),
                "{body}"
            );
            assert!(
                error_of(entries(body)).contains("unsafe Markdown URL"),
                "{body}"
            );
        }
        // Headings must sit beneath the page's own: the note title is the
        // only h1, and diary entries share a page beneath per-entry h2s.
        assert!(error_of(notes("# Second title\n")).contains("level 2 (`##`)"));
        assert!(notes("## Section\n\n### Part\n").is_ok());
        assert!(error_of(entries("## Heading\n")).contains("level 3 (`###`)"));
        assert!(entries("### Heading\n").is_ok());
        // Footnote identifiers would collide between entries on one page.
        let footnote = "Text.[^1]\n\n[^1]: Note.\n";
        assert!(notes(footnote).is_ok());
        assert!(error_of(entries(footnote)).contains("footnotes are not supported"));
        // TeX is checked at build time when math is enabled.
        let math =
            |body: &str| Miscellany::from_sources("items: []", &[&note("math: true\n", body)], &[]);
        assert!(math("$x^2$ and $$\\int_0^1 f$$\n").is_ok());
        assert!(error_of(math("$\\notacommand{x}$\n")).contains("invalid TeX expression"));
    }

    #[test]
    fn notes_follow_editorial_order_and_diary_is_newest_first() {
        let published = |slug: &str, order: &str| {
            format!("---\nslug: {slug}\ntitle: {slug}\nlang: en\n{order}draft: false\n---\nx")
        };
        let notes = [
            published("zeta", "order: 1\n"),
            published("alpha", ""),
            published("beta", "order: 2\n"),
            published("gamma", ""),
        ];
        let dated = |date: &str, slug: &str| {
            format!("---\ndate: {date}\nlang: en\n{slug}draft: false\n---\nx")
        };
        let diary = [
            dated("2026-03-01", ""),
            dated("2026-10-10", "slug: morning\n"),
            dated("2025-12-31", ""),
            dated("2026-10-10", ""),
        ];
        let miscellany = Miscellany::from_sources(
            "items: []",
            &notes.iter().map(String::as_str).collect::<Vec<_>>(),
            &diary.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .unwrap();
        let slugs: Vec<_> = miscellany
            .public_notes()
            .map(|note| note.metadata().slug.as_str())
            .collect();
        assert_eq!(slugs, ["zeta", "beta", "alpha", "gamma"]);
        let anchors: Vec<_> = miscellany
            .public_diary()
            .map(|entry| entry.metadata().anchor())
            .collect();
        assert_eq!(
            anchors,
            [
                "diary-2026-10-10",
                "diary-2026-10-10-morning",
                "diary-2026-03-01",
                "diary-2025-12-31"
            ]
        );
    }

    #[test]
    fn invalid_homepage_content_is_rejected_with_file_diagnostics() {
        for (source, expected) in [
            ("{\"bio\": [", "invalid JSON in home.json"),
            ("{}", "missing field `bio`"),
            (
                "{\"bio\": []}",
                "home.json: bio must contain at least one paragraph",
            ),
            ("{\"bio\": [\"Fine.\", \"\"]}", "home.json: bio paragraph 2"),
            (
                "{\"bio\": [\"Fine.\"], \"title\": \"x\"}",
                "unknown field `title`",
            ),
            (
                "{\"bio\": [\"See [x](javascript:alert(1)).\"]}",
                "home.json: bio paragraph 1",
            ),
            ("{\"bio\": [\"An [unclosed link.\"]}", "unclosed link"),
        ] {
            let error = format!("{:#}", ValidatedHome::from_json(source).unwrap_err());
            assert!(error.contains(expected), "{source}: {error}");
        }
        // Every invalid paragraph is reported, not only the first.
        let error = format!(
            "{:#}",
            ValidatedHome::from_json("{\"bio\": [\"[a](x)\", \"ok\", \"b]\"]}").unwrap_err()
        );
        assert!(
            error.contains("2 error(s)") && error.contains("paragraph 3"),
            "{error}"
        );
    }

    #[test]
    fn multilingual_sources_are_validated_per_file() {
        let primary = "---\nslug: moon\ntitle: Moon\nlang: ja\n---\n本文。\n";
        let notes = |extra: &str| Miscellany::from_sources("items: []", &[primary, extra], &[]);
        for (companion, expected) in [
            // A companion names an article that does not exist.
            (
                "---\nof: sun\nlang: en\n---\nText.\n",
                "note-1.md: of \"sun\" names no note",
            ),
            // Two passages in one language.
            (
                "---\nof: moon\nlang: ja\n---\n別の本文。\n",
                "note-0.md and note-1.md are both ja passages",
            ),
            // Shared metadata belongs to the article, not to a passage.
            (
                "---\nof: moon\nlang: en\nslug: moon-en\n---\nText.\n",
                "unknown field `slug`",
            ),
            (
                "---\nof: moon\nlang: en\ndraft: false\n---\nText.\n",
                "unknown field `draft`",
            ),
            (
                "---\nof: moon\nlang: en\norder: 1\n---\nText.\n",
                "unknown field `order`",
            ),
            // Unsupported passage languages and empty passages.
            (
                "---\nof: moon\nlang: fr\n---\nTexte.\n",
                "\"fr\" is not supported",
            ),
            (
                "---\nof: moon\nlang: en\n---\n\n",
                "the passage text must not be empty",
            ),
            // A passage title is its heading, so its body starts one level lower.
            (
                "---\nof: moon\nlang: en\ntitle: Moon\n---\n## Too high\n",
                "level 3 (`###`)",
            ),
            // Inline language marks are validated.
            (
                "---\nof: moon\nlang: en\n---\nA [phrase]{lang=Deutsch}.\n",
                "language tag \"Deutsch\"",
            ),
            (
                "---\nof: moon\nlang: en\n---\nA [phrase]{lang=de\n",
                "unclosed inline language mark",
            ),
            (
                "---\nof: moon\nlang: en\n---\nA *phrase*]{lang=de}.\n",
                "plain text in brackets",
            ),
            (
                "---\nof: moon\nlang: en\n---\nA [x](javascript:alert(1)).\n",
                "unsafe Markdown URL",
            ),
            (
                "---\nof: moon\nlang: en\n---\n<span lang=\"de\">x</span>\n",
                "raw HTML is not permitted",
            ),
        ] {
            let error = format!("{:#}", notes(companion).unwrap_err());
            assert!(error.contains(expected), "{companion:?}: {error}");
        }
        let entry = "---\ndate: 2026-10-10\nlang: en\n---\nText.\n";
        let error = format!(
            "{:#}",
            Miscellany::from_sources(
                "items: []",
                &[],
                &[entry, "---\nof: 2026-10-11\nlang: ja\n---\n本文。\n"],
            )
            .unwrap_err()
        );
        assert!(
            error.contains("of \"2026-10-11\" names no diary entry"),
            "{error}"
        );
        // A reading note in an unsupported language, and both note forms.
        for (reading, expected) in [
            (
                "items:\n  - title: X\n    notes:\n      fr: Texte.\n",
                "invalid notes language \"fr\"",
            ),
            (
                "items:\n  - title: X\n    note: a\n    notes:\n      ja: b\n",
                "either note or notes",
            ),
            (
                "items:\n  - title: X\n    notes:\n      en: ''\n",
                "en note must not be empty",
            ),
            (
                "items:\n  - title: X\n    notes:\n      en: a\n      en: b\n",
                "duplicate entry",
            ),
        ] {
            let error = format!(
                "{:#}",
                Miscellany::from_sources(reading, &[], &[]).unwrap_err()
            );
            assert!(error.contains(expected), "{reading:?}: {error}");
        }
    }

    #[test]
    fn inline_language_marks_are_safe_and_leave_code_alone() {
        let body = "Ein [A & <B>]{lang=de} und `[x]{lang=fr}`.\n\n```\n[y]{lang=fr}\n```\n\n![alt [z]{lang=fr}](/a.png)\n";
        let note = format!("---\nslug: m\ntitle: M\nlang: mixed\ndraft: false\n---\n{body}");
        let error = format!(
            "{:#}",
            Miscellany::from_sources("items: []", &[&note], &[]).unwrap_err()
        );
        // Raw HTML stays rejected even inside a marked phrase.
        assert!(error.contains("raw HTML is not permitted"), "{error}");
        let note = note.replace(" & <B>", " &amp; B");
        let miscellany = Miscellany::from_sources("items: []", &[&note], &[]).unwrap();
        let html = miscellany
            .public_notes()
            .next()
            .unwrap()
            .passages()
            .iter()
            .next()
            .unwrap()
            .rendered_body()
            .to_owned();
        assert!(
            html.contains(r#"Ein <span lang="de">A &amp; B</span> und <code>[x]{lang=fr}</code>."#),
            "{html}"
        );
        assert!(
            html.contains("<pre><code>[y]{lang=fr}\n</code></pre>"),
            "{html}"
        );
        assert!(html.contains(r#"alt="alt [z]{lang=fr}""#), "{html}");
        assert_eq!(html.matches("<span").count(), 1);
    }
}
