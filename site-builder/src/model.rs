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
        ensure!(
            self.url.starts_with("https://")
                || self.url.starts_with("http://")
                || self.url.starts_with('/')
                || self.url.starts_with("mailto:"),
            "{source}: link URL must be absolute or root-relative: {}",
            self.url
        );
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
    pub summary: Localized,
    pub date: NaiveDate,
    pub announced_on: NaiveDate,
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
        self.summary.validate(source, "summary")?;
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
        }
        self.related.validate(source)?;
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
            Self::Academia => "Academia",
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
            Self::Award { .. } => "Award",
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

#[derive(Clone, Debug)]
pub struct Writing {
    pub metadata: WritingFrontMatter,
    pub rendered_body: String,
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
            ensure!(
                url.starts_with("https://") || url.starts_with("http://"),
                "{source}: externalUrl must be HTTP(S)"
            );
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

#[cfg(test)]
mod tests {
    use super::validate_slug;

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
}
