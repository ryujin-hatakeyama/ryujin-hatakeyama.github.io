use std::collections::HashSet;

use anyhow::{Context, Result, bail, ensure};
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub slug: String,
    pub title: String,
    pub authors: Vec<String>,
    pub year: i32,
    /// Language of the publication's own text, stated explicitly rather than
    /// inferred from its title, authors, or venue.
    pub language: Language,
    #[serde(rename = "type")]
    pub record_type: PublicationType,
    pub status: PublicationStatus,
    pub venue: Option<String>,
    pub note: Option<String>,
    pub presentation: Option<PresentationMetadata>,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub project_ids: Vec<String>,
    #[serde(default)]
    pub draft: bool,
}

impl Publication {
    pub fn validate(&self, source: &str) -> Result<()> {
        validate_slug(&self.slug).with_context(|| format!("{source}: invalid publication slug"))?;
        ensure!(
            !self.title.trim().is_empty(),
            "{source}: title must not be empty"
        );
        ensure!(
            !self.authors.is_empty(),
            "{source}: authors must not be empty"
        );
        ensure!(
            self.authors.iter().all(|author| !author.trim().is_empty()),
            "{source}: authors must not contain empty names"
        );
        ensure!(
            (1900..=2200).contains(&self.year),
            "{source}: implausible publication year"
        );
        for link in &self.links {
            link.validate(source)?;
        }
        for id in &self.project_ids {
            validate_slug(id).with_context(|| format!("{source}: invalid projectIds entry"))?;
        }

        match (&self.record_type, &self.presentation) {
            (PublicationType::Presentation, Some(metadata)) => ensure!(
                metadata.format == PresentationFormat::Oral,
                "{source}: presentation records must use the oral format"
            ),
            (PublicationType::Poster, Some(metadata)) => ensure!(
                metadata.format == PresentationFormat::Poster,
                "{source}: poster records must use the poster format"
            ),
            (PublicationType::Presentation | PublicationType::Poster, None) => {
                bail!("{source}: presentation metadata is required")
            }
            (_, Some(_)) => {
                bail!("{source}: presentation metadata is only valid for talks and posters")
            }
            (_, None) => {}
        }
        Ok(())
    }

    /// Whether the record is listed among bibliographic publications. A talk or
    /// poster is listed there only when it declares an accompanying
    /// proceedings paper; the review status is then shown with it.
    pub fn is_bibliographic(&self) -> bool {
        !self.is_presentation()
            || self
                .presentation
                .as_ref()
                .is_some_and(|metadata| metadata.proceedings_review.is_some())
    }

    pub const fn is_presentation(&self) -> bool {
        matches!(
            self.record_type,
            PublicationType::Presentation | PublicationType::Poster
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum PublicationType {
    ConferencePaper,
    JournalArticle,
    Preprint,
    ExtendedAbstract,
    WorkshopPaper,
    WorkshopContribution,
    StudentResearchCompetition,
    Presentation,
    Poster,
    Software,
}

impl PublicationType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConferencePaper => "Conference paper",
            Self::JournalArticle => "Journal article",
            Self::Preprint => "Preprint",
            Self::ExtendedAbstract => "Extended abstract",
            Self::WorkshopPaper => "Workshop paper",
            Self::WorkshopContribution => "Workshop contribution",
            Self::StudentResearchCompetition => "Student research competition",
            Self::Presentation => "Presentation",
            Self::Poster => "Poster",
            Self::Software => "Software",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PublicationStatus {
    Draft,
    Submitted,
    Accepted,
    Forthcoming,
    Published,
    Presented,
}

impl PublicationStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Submitted => "Submitted",
            Self::Accepted => "Accepted",
            Self::Forthcoming => "Forthcoming",
            Self::Published => "Published",
            Self::Presented => "Presented",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationMetadata {
    pub format: PresentationFormat,
    pub category: Option<String>,
    pub proceedings_review: Option<ProceedingsReview>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PresentationFormat {
    Oral,
    Poster,
}

impl PresentationFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Oral => "Oral presentation",
            Self::Poster => "Poster presentation",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProceedingsReview {
    Refereed,
    Unrefereed,
}

impl ProceedingsReview {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Refereed => "Refereed proceedings paper",
            Self::Unrefereed => "Unrefereed proceedings paper",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
    #[serde(rename = "type", default)]
    pub link_type: LinkType,
}

impl Link {
    fn validate(&self, source: &str) -> Result<()> {
        ensure!(
            !self.label.trim().is_empty(),
            "{source}: link label must not be empty"
        );
        validate_record_url(&self.url)
            .with_context(|| format!("{source}: invalid link URL {:?}", self.url))?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    #[default]
    External,
    Pdf,
    Doi,
    Preprint,
    Code,
    Slides,
    Bibtex,
    Audio,
}

impl LinkType {
    pub const fn label(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Pdf => "pdf",
            Self::Doi => "doi",
            Self::Preprint => "preprint",
            Self::Code => "code",
            Self::Slides => "slides",
            Self::Bibtex => "bibtex",
            Self::Audio => "audio",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    pub id: String,
    pub title: Localized,
    pub title_link: Option<TitleLink>,
    pub summary: Localized,
    pub date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub event_status: EventStatus,
    pub categories: Vec<Category>,
    pub kind: EventKind,
    #[serde(default)]
    pub links: Vec<Link>,
    pub related: Related,
    #[serde(default)]
    pub detail: bool,
    pub body: Option<Localized>,
}

impl Update {
    pub fn validate(&self, source: &str) -> Result<()> {
        validate_slug(&self.id).with_context(|| format!("{source}: invalid update id"))?;
        self.title.validate(source, "title")?;
        if let Some(link) = &self.title_link {
            let start = self.title.en.find(&link.text);
            ensure!(
                !link.text.is_empty()
                    && start.is_some()
                    && start == self.title.en.rfind(&link.text),
                "{source}: titleLink text must occur exactly once in the title"
            );
            validate_http_url(&link.url)
                .with_context(|| format!("{source}: invalid titleLink URL {:?}", link.url))?;
        }
        self.summary.validate(source, "summary")?;
        for segment in summary_segments(&self.summary.en) {
            if let SummarySegment::Link { url, .. } = segment {
                validate_http_url(url)
                    .with_context(|| format!("{source}: invalid summary link URL {url:?}"))?;
            }
        }
        ensure!(
            !self.categories.is_empty(),
            "{source}: categories must not be empty"
        );
        let unique: HashSet<_> = self.categories.iter().collect();
        ensure!(
            unique.len() == self.categories.len(),
            "{source}: duplicate update category"
        );
        for link in &self.links {
            link.validate(source)?;
            // The title link is the event's primary resource; `links` holds
            // only additional, distinct ones.
            if let Some(title_link) = &self.title_link {
                ensure!(
                    !same_page(&link.url, &title_link.url),
                    "{source}: link {:?} duplicates the titleLink destination {:?}",
                    link.label,
                    title_link.url
                );
            }
        }
        self.related.validate(source)?;
        ensure!(
            self.last_event_day() >= self.date,
            "{source}: endDate must not precede date"
        );
        if let Some(body) = &self.body {
            body.validate(source, "body")?;
        }
        match &self.kind {
            EventKind::Award { name, .. } => {
                ensure!(
                    !name.trim().is_empty(),
                    "{source}: award name must not be empty"
                );
            }
            EventKind::Other {
                detail: Some(detail),
            } => {
                ensure!(
                    !detail.trim().is_empty(),
                    "{source}: kind detail must not be empty"
                );
            }
            _ => {}
        }
        Ok(())
    }
}

impl Update {
    pub fn last_event_day(&self) -> NaiveDate {
        self.end_date.unwrap_or(self.date)
    }

    /// Most recently held first: by the event's final day, then its first day,
    /// both newest first, then by id for determinism.
    pub fn most_recent_first(left: &Self, right: &Self) -> std::cmp::Ordering {
        right
            .last_event_day()
            .cmp(&left.last_event_day())
            .then_with(|| right.date.cmp(&left.date))
            .then_with(|| left.id.cmp(&right.id))
    }
}

/// Whether two URLs name the same page; a trailing slash does not make a
/// different page.
fn same_page(left: &str, right: &str) -> bool {
    left.trim_end_matches('/') == right.trim_end_matches('/')
}

/// The part of an update title that names the event, linked to the event's
/// primary official resource. The title itself stays plain text for metadata
/// and RSS.
#[derive(Clone, Debug, Deserialize)]
pub struct TitleLink {
    pub text: String,
    pub url: String,
    pub lang: Option<Language>,
}

impl TitleLink {
    /// Splits `title` around the linked text, which validation guarantees
    /// occurs exactly once.
    pub fn split<'a>(&self, title: &'a str) -> Option<(&'a str, &'a str)> {
        let start = title.find(&self.text)?;
        Some((&title[..start], &title[start + self.text.len()..]))
    }
}

/// A run of an update summary: plain text, or an inline link written as
/// `[text](https://…)`. Anything that does not form such a link stays text, so
/// summaries without links render exactly as before.
#[derive(Debug, Eq, PartialEq)]
pub enum SummarySegment<'a> {
    Text(&'a str),
    Link { text: &'a str, url: &'a str },
}

pub fn summary_segments(summary: &str) -> Vec<SummarySegment<'_>> {
    let mut segments = Vec::new();
    let mut text_start = 0;
    let mut search_from = 0;
    while let Some(offset) = summary[search_from..].find('[') {
        let open = search_from + offset;
        let link = summary[open + 1..]
            .split_once("](")
            .and_then(|(text, tail)| Some((text, tail.split_once(')')?.0)))
            .filter(|(text, url)| {
                !text.trim().is_empty()
                    && !text.contains(['[', ']'])
                    && is_http_url(url)
                    && !url.contains(char::is_whitespace)
            });
        match link {
            Some((text, url)) => {
                if text_start < open {
                    segments.push(SummarySegment::Text(&summary[text_start..open]));
                }
                segments.push(SummarySegment::Link { text, url });
                text_start = open + text.len() + url.len() + 4;
                search_from = text_start;
            }
            None => search_from = open + 1,
        }
    }
    if text_start < summary.len() {
        segments.push(SummarySegment::Text(&summary[text_start..]));
    }
    segments
}

/// The summary with link markup removed, for meta descriptions and RSS.
pub fn summary_plain_text(summary: &str) -> String {
    summary_segments(summary)
        .into_iter()
        .map(|segment| match segment {
            SummarySegment::Text(text) | SummarySegment::Link { text, .. } => text,
        })
        .collect()
}

/// The record's editorial status: whether the recorded activity has taken place
/// or is still intended. This is independent of the record's draft/published
/// visibility and is never derived from the current date; a planned record
/// whose event has begun stops the build for editorial review instead.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Planned,
    Completed,
}

impl EventStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Planned => "Planned",
            Self::Completed => "Completed",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Localized {
    pub en: String,
    pub ja: Option<String>,
}

impl Localized {
    fn validate(&self, source: &str, field: &str) -> Result<()> {
        ensure!(
            !self.en.trim().is_empty(),
            "{source}: {field}.en must not be empty"
        );
        if let Some(ja) = &self.ja {
            ensure!(
                !ja.trim().is_empty(),
                "{source}: {field}.ja must not be empty"
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Research,
    Academia,
    Writing,
}

impl Category {
    pub const ALL: [Self; 3] = [Self::Research, Self::Academia, Self::Writing];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Research => "research",
            Self::Academia => "academia",
            Self::Writing => "writing",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Research => "Research",
            // Displayed as "Activities"; the slug stays "academia" so routes
            // and source records are unchanged.
            Self::Academia => "Activities",
            Self::Writing => "Writing",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum EventKind {
    Acceptance,
    Presentation,
    Publication,
    Participation,
    Visit,
    Award { outcome: AwardOutcome, name: String },
    Release,
    Other { detail: Option<String> },
}

impl EventKind {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Acceptance => "Acceptance",
            Self::Presentation => "Presentation",
            Self::Publication => "Publication",
            Self::Participation => "Participation",
            Self::Visit => "Visit",
            // The outcome is part of the label so that a nomination is never
            // presented as an award won.
            Self::Award {
                outcome: AwardOutcome::Nominated,
                ..
            } => "Award nomination",
            Self::Award {
                outcome: AwardOutcome::Shortlisted,
                ..
            } => "Award shortlist",
            Self::Award {
                outcome: AwardOutcome::Won,
                ..
            } => "Award",
            Self::Release => "Release",
            Self::Other { .. } => "Other",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AwardOutcome {
    Nominated,
    Shortlisted,
    Won,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Related {
    #[serde(default)]
    pub publications: Vec<String>,
    #[serde(default)]
    pub projects: Vec<String>,
    #[serde(default)]
    pub writings: Vec<String>,
}

impl Related {
    fn validate(&self, source: &str) -> Result<()> {
        for id in self
            .publications
            .iter()
            .chain(&self.projects)
            .chain(&self.writings)
        {
            validate_slug(id).with_context(|| format!("{source}: invalid related identifier"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WritingFrontMatter {
    pub slug: String,
    pub title: String,
    pub description: String,
    pub date: NaiveDate,
    pub lang: Language,
    pub kind: WritingKind,
    pub translation_key: Option<String>,
    pub publication: Option<String>,
    pub external_url: Option<String>,
    #[serde(default)]
    pub math: bool,
    #[serde(default = "default_true")]
    pub draft: bool,
}

impl WritingFrontMatter {
    pub fn validate(&self, source: &str) -> Result<()> {
        validate_slug(&self.slug).with_context(|| format!("{source}: invalid writing slug"))?;
        ensure!(
            !self.title.trim().is_empty(),
            "{source}: title must not be empty"
        );
        ensure!(
            !self.description.trim().is_empty(),
            "{source}: description must not be empty"
        );
        if let Some(key) = &self.translation_key {
            validate_slug(key).with_context(|| format!("{source}: invalid translationKey"))?;
        }
        if let Some(url) = &self.external_url {
            validate_http_url(url)
                .with_context(|| format!("{source}: invalid externalUrl {url:?}"))?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Ja,
}

impl Language {
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Ja => "ja",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WritingKind {
    Essay,
    Note,
    Literary,
    Prose,
    Fragment,
}

impl WritingKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Essay => "Essay",
            Self::Note => "Note",
            Self::Literary => "Literary work",
            Self::Prose => "Prose",
            Self::Fragment => "Fragment",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFrontMatter {
    pub slug: String,
    pub title: String,
    pub title_ja: Option<String>,
    pub summary: String,
    pub summary_ja: Option<String>,
    pub status: ProjectStatus,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default = "default_true")]
    pub draft: bool,
}

#[derive(Clone, Debug)]
pub struct Project {
    pub metadata: ProjectFrontMatter,
}

impl ProjectFrontMatter {
    pub fn validate(&self, source: &str) -> Result<()> {
        validate_slug(&self.slug).with_context(|| format!("{source}: invalid project slug"))?;
        ensure!(
            !self.title.trim().is_empty(),
            "{source}: title must not be empty"
        );
        ensure!(
            !self.summary.trim().is_empty(),
            "{source}: summary must not be empty"
        );
        for link in &self.links {
            link.validate(source)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Active,
    Completed,
    Paused,
}

const fn default_true() -> bool {
    true
}

pub fn validate_slug(value: &str) -> Result<()> {
    ensure!(!value.is_empty(), "slug must not be empty");
    ensure!(
        value.split('-').all(|part| !part.is_empty()
            && part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())),
        "slug must contain lowercase ASCII letters, digits, and single hyphens"
    );
    Ok(())
}

pub(crate) fn validate_markdown_url(value: &str) -> Result<()> {
    validate_url_characters(value)?;
    if is_http_url(value)
        || is_mailto_url(value)
        || is_safe_root_relative(value)
        || value.starts_with('#')
    {
        return Ok(());
    }

    let scheme_candidate = value.split(['/', '?', '#']).next().unwrap_or_default();
    ensure!(!scheme_candidate.contains(':'), "URL scheme is not allowed");
    ensure!(
        !value.starts_with("//"),
        "network-path URLs are not allowed"
    );
    Ok(())
}

fn validate_record_url(value: &str) -> Result<()> {
    validate_url_characters(value)?;
    ensure!(
        is_http_url(value) || is_mailto_url(value) || is_safe_root_relative(value),
        "link URL must use HTTP(S), mailto, or a root-relative path"
    );
    Ok(())
}

fn validate_http_url(value: &str) -> Result<()> {
    validate_url_characters(value)?;
    ensure!(is_http_url(value), "URL must use HTTP(S)");
    Ok(())
}

fn validate_url_characters(value: &str) -> Result<()> {
    ensure!(!value.is_empty(), "URL must not be empty");
    ensure!(value.trim() == value, "URL must not have surrounding space");
    ensure!(
        !value.chars().any(char::is_control),
        "URL must not contain control characters"
    );
    ensure!(
        !value.chars().any(char::is_whitespace),
        "URL must not contain whitespace"
    );
    ensure!(!value.contains('\\'), "URL must not contain backslashes");
    Ok(())
}

fn is_http_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    for prefix in ["https://", "http://"] {
        if let Some(remainder) = lower.strip_prefix(prefix) {
            let authority = remainder.split(['/', '?', '#']).next().unwrap_or_default();
            return !authority.is_empty();
        }
    }
    false
}

fn is_mailto_url(value: &str) -> bool {
    value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("mailto:"))
        && value.len() > 7
}

fn is_safe_root_relative(value: &str) -> bool {
    value.starts_with('/') && !value.starts_with("//")
}

#[cfg(test)]
mod tests {
    use super::{
        Category, EventKind, EventStatus, Language, Link, LinkType, Publication, SummarySegment,
        Update, summary_plain_text, summary_segments, validate_markdown_url, validate_slug,
    };

    fn publication(extra: &str) -> serde_json::Result<Publication> {
        serde_json::from_str(&format!(
            r#"{{"slug":"fixture","title":"Fixture","authors":["A"],"year":2026{extra}}}"#
        ))
    }

    fn update(date: &str, end: Option<&str>, status: &str) -> Update {
        let end = end.map_or(String::new(), |end| format!(r#","endDate":"{end}""#));
        serde_json::from_str(&format!(
            r#"{{"id":"fixture","title":{{"en":"x"}},"summary":{{"en":"x"}},"date":"{date}"{end},
            "eventStatus":"{status}","categories":["academia"],
            "kind":{{"type":"participation"}},"related":{{}}}}"#
        ))
        .unwrap()
    }

    #[test]
    fn academia_is_displayed_as_activities_at_its_original_route() {
        assert_eq!(Category::Academia.label(), "Activities");
        assert_eq!(Category::Academia.slug(), "academia");
        assert_eq!(Category::Research.label(), "Research");
        assert_eq!(Category::Writing.label(), "Writing");
    }

    #[test]
    fn award_labels_distinguish_nominations_from_awards_won() {
        let kind = |outcome| -> EventKind {
            serde_json::from_str(&format!(
                r#"{{"type":"award","outcome":"{outcome}","name":"Fixture"}}"#
            ))
            .unwrap()
        };
        assert_eq!(kind("nominated").label(), "Award nomination");
        assert_eq!(kind("shortlisted").label(), "Award shortlist");
        assert_eq!(kind("won").label(), "Award");
    }

    #[test]
    fn publication_language_is_explicit_and_typed() {
        assert!(publication(r#","type":"workshop-paper","status":"published""#).is_err());
        assert!(
            publication(r#","language":"fr","type":"workshop-paper","status":"published""#)
                .is_err()
        );
        let record =
            publication(r#","language":"ja","type":"workshop-paper","status":"published""#)
                .unwrap();
        assert_eq!(record.language, Language::Ja);
        assert_eq!(record.record_type.label(), "Workshop paper");
    }

    #[test]
    fn only_presentations_with_proceedings_are_bibliographic() {
        let paper = r#","language":"ja","type":"presentation","status":"presented","presentation":{"format":"oral","proceedingsReview":"unrefereed"}"#;
        let poster = r#","language":"ja","type":"poster","status":"presented","presentation":{"format":"poster"}"#;
        let workshop = r#","language":"en","type":"workshop-paper","status":"published""#;
        assert!(publication(paper).unwrap().is_bibliographic());
        assert!(!publication(poster).unwrap().is_bibliographic());
        assert!(publication(workshop).unwrap().is_bibliographic());
    }

    #[test]
    fn event_dates_must_not_end_before_they_begin() {
        let planned = update("2026-10-16", Some("2026-10-18"), "planned");
        assert_eq!(planned.event_status, EventStatus::Planned);
        assert!(planned.validate("fixture").is_ok());
        for status in ["planned", "completed"] {
            assert!(
                update("2026-10-18", Some("2026-10-16"), status)
                    .validate("fixture")
                    .is_err(),
                "{status}"
            );
            assert!(
                update("2026-10-16", Some("2026-10-16"), status)
                    .validate("fixture")
                    .is_ok(),
                "{status}"
            );
        }
        assert!(
            update("2026-09-25", None, "completed")
                .validate("fixture")
                .is_ok()
        );
    }

    #[test]
    fn links_must_not_duplicate_the_title_link_destination() {
        let with_links = |links: &str| -> Update {
            serde_json::from_str(&format!(
                r#"{{"id":"fixture","title":{{"en":"Attendance at LLAL@GSIS (XIII)"}},
                "titleLink":{{"text":"LLAL@GSIS (XIII)","url":"https://example.org/llal/"}},
                "summary":{{"en":"x"}},"date":"2026-03-23","eventStatus":"completed",
                "categories":["academia"],"kind":{{"type":"participation"}},"related":{{}},
                "links":{links}}}"#
            ))
            .unwrap()
        };
        for duplicate in ["https://example.org/llal/", "https://example.org/llal"] {
            let error = with_links(&format!(
                r#"[{{"label":"Program","url":"{duplicate}","type":"external"}}]"#
            ))
            .validate("fixture")
            .unwrap_err()
            .to_string();
            assert!(
                error.contains("duplicates the titleLink destination"),
                "{error}"
            );
        }
        // Distinct resources, and records without a title link, are accepted.
        assert!(
            with_links(
                r#"[{"label":"Program","url":"https://example.org/llal/program/","type":"external"},
                    {"label":"Slides","url":"https://example.org/slides.pdf","type":"slides"}]"#
            )
            .validate("fixture")
            .is_ok()
        );
        let mut untitled = with_links(r#"[{"label":"Program","url":"https://example.org/llal/"}]"#);
        untitled.title_link = None;
        assert!(untitled.validate("fixture").is_ok());
    }

    #[test]
    fn accepts_stable_slugs() {
        assert!(validate_slug("ppl-2025-poster").is_ok());
    }

    #[test]
    fn rejects_unsafe_slugs() {
        for value in ["", "Upper", "double--hyphen", "../escape", "end-"] {
            assert!(
                validate_slug(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn markdown_urls_reject_active_and_ambiguous_schemes() {
        for value in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            "data:text/html,<script>alert(1)</script>",
            "vbscript:msgbox(1)",
            "//example.com/path",
            "https://example.com/with space",
        ] {
            assert!(
                validate_markdown_url(value).is_err(),
                "{value:?} should be rejected"
            );
        }
    }

    #[test]
    fn markdown_urls_allow_normal_links() {
        for value in [
            "https://example.com/path?q=1#part",
            "mailto:person@example.com",
            "/research/",
            "notes/page.html",
            "#section",
        ] {
            assert!(
                validate_markdown_url(value).is_ok(),
                "{value:?} should be accepted"
            );
        }
    }

    #[test]
    fn content_record_links_reject_dangerous_schemes() {
        for url in [
            "javascript:alert(1)",
            "data:text/html,boom",
            "//example.com",
        ] {
            let link = Link {
                label: "unsafe".to_owned(),
                url: url.to_owned(),
                link_type: LinkType::External,
            };
            assert!(link.validate("fixture").is_err(), "{url:?}");
        }

        for url in ["https://example.com", "/research/", "mailto:a@example.com"] {
            let link = Link {
                label: "safe".to_owned(),
                url: url.to_owned(),
                link_type: LinkType::External,
            };
            assert!(link.validate("fixture").is_ok(), "{url:?}");
        }
    }

    #[test]
    fn summary_links_are_parsed_and_other_text_is_kept_verbatim() {
        let summary = "Visited [UW](https://www.washington.edu/visit/), [x] (y), and Micron.";
        assert_eq!(
            summary_segments(summary),
            [
                SummarySegment::Text("Visited "),
                SummarySegment::Link {
                    text: "UW",
                    url: "https://www.washington.edu/visit/"
                },
                SummarySegment::Text(", [x] (y), and Micron."),
            ]
        );
        assert_eq!(
            summary_plain_text(summary),
            "Visited UW, [x] (y), and Micron."
        );
        for literal in [
            "Plain text.",
            "[a](javascript:alert(1))",
            "[](https://x.org)",
            "[a](https://x.org",
        ] {
            assert_eq!(summary_segments(literal), [SummarySegment::Text(literal)]);
        }
    }

    #[test]
    fn title_links_must_name_a_unique_part_of_the_title_and_use_http() {
        let with_link = |link: &str| -> Update {
            serde_json::from_str(&format!(
                r#"{{"id":"fixture","title":{{"en":"Attendance at PPL Summer School 2026"}},"titleLink":{link},
                "summary":{{"en":"x"}},"date":"2026-09-07","eventStatus":"completed",
                "categories":["academia"],"kind":{{"type":"participation"}},"related":{{}}}}"#
            ))
            .unwrap()
        };
        let valid = with_link(
            r#"{"text":"PPL Summer School 2026","url":"https://jssst-ppl.org/wiki/ss2026"}"#,
        );
        assert!(valid.validate("fixture").is_ok());
        assert_eq!(
            valid.title_link.as_ref().unwrap().split(&valid.title.en),
            Some(("Attendance at ", ""))
        );
        for invalid in [
            r#"{"text":"PPL 2026","url":"https://example.org/"}"#,
            r#"{"text":"n","url":"https://example.org/"}"#,
            r#"{"text":"","url":"https://example.org/"}"#,
            r#"{"text":"PPL Summer School 2026","url":"/updates/"}"#,
            r#"{"text":"PPL Summer School 2026","url":"javascript:alert(1)"}"#,
        ] {
            assert!(with_link(invalid).validate("fixture").is_err(), "{invalid}");
        }
    }

    #[test]
    fn title_links_split_multibyte_titles_at_character_boundaries() {
        let update: Update = serde_json::from_str(
            r#"{"id":"fixture","title":{"en":"Planned attendance at 数学基礎論若手の会2026"},
            "titleLink":{"text":"数学基礎論若手の会2026","url":"https://sites.google.com/view/wakatenokai2026/","lang":"ja"},
            "summary":{"en":"x"},"date":"2026-10-16","eventStatus":"planned",
            "categories":["academia"],"kind":{"type":"participation"},"related":{}}"#,
        )
        .unwrap();
        assert!(update.validate("fixture").is_ok());
        let link = update.title_link.as_ref().unwrap();
        assert_eq!(link.lang, Some(Language::Ja));
        assert_eq!(
            link.split(&update.title.en),
            Some(("Planned attendance at ", ""))
        );
        // A substring that is not in the title (e.g. a partial character
        // sequence from another script) is rejected rather than sliced.
        let mut partial = update.clone();
        partial.title_link.as_mut().unwrap().text = "若手の会2027".to_owned();
        assert!(partial.validate("fixture").is_err());
    }
}
