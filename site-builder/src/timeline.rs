//! Updates as one timeline: the author's update records (`content/updates`,
//! validated by the OCaml generator) together with one entry for each
//! published Miscellany article that states the day it was published.
//!
//! Article entries are never stored. They are derived from the published
//! articles on every build, so they follow the articles' metadata: a changed
//! publication day moves the entry, a changed title renames it, and neither
//! rebuilding nor publishing an article again can add a second entry. Their
//! ids come from the articles' own stable identities, never from titles.
//!
//! The homepage, the Updates pages, and the Miscellany overview all read the
//! same `Timeline`, so they cannot disagree about dates or order. The RSS
//! feed stays a feed of update records only: Miscellany is kept out of it
//! (the output validator checks this).

use std::cmp::Ordering;

use chrono::NaiveDate;

use crate::model::{Category, Update, UpdatePhase};

/// The Miscellany section a published article belongs to.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArticleSection {
    Reading,
    Notes,
    Diary,
    Sketches,
}

impl ArticleSection {
    /// The section's key, as in its route and in article update ids.
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Reading => "reading",
            Self::Notes => "notes",
            Self::Diary => "diary",
            Self::Sketches => "sketches",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Reading => "Reading",
            Self::Notes => "Notes",
            Self::Diary => "Diary",
            Self::Sketches => "Sketches",
        }
    }
}

/// What an article's entry calls it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ArticleName {
    /// The article's own title, exactly as written, with the title's
    /// language when it is known. It is never translated or summarised.
    Title { text: String, lang: Option<String> },
    /// An untitled diary entry, named by the day it records.
    DiaryDay(NaiveDate),
}

/// The entry in Updates for one published article.
#[derive(Clone, Debug)]
pub(crate) struct ArticleUpdate {
    id: String,
    section: ArticleSection,
    name: ArticleName,
    path: String,
    published: NaiveDate,
}

/// Every article entry is in Writing, and only there.
const ARTICLE_CATEGORIES: [Category; 1] = [Category::Writing];

impl ArticleUpdate {
    /// `identity` is the article's stable identity within its section: a
    /// note's or sketch's slug, a diary entry's date and slug, a reading
    /// item's id. `path` is the article's public address.
    pub(crate) fn new(
        section: ArticleSection,
        identity: &str,
        name: ArticleName,
        path: String,
        published: NaiveDate,
    ) -> Self {
        Self {
            id: Self::id_for(section, identity),
            section,
            name,
            path,
            published,
        }
    }

    /// The update id of the article with this identity. It is in the same
    /// namespace as the ids of update records, which must not use it.
    pub(crate) fn id_for(section: ArticleSection, identity: &str) -> String {
        format!("published-{}-{identity}", section.key())
    }

    pub(crate) fn id(&self) -> &str {
        &self.id
    }

    pub(crate) fn section(&self) -> ArticleSection {
        self.section
    }

    pub(crate) fn name(&self) -> &ArticleName {
        &self.name
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn published(&self) -> NaiveDate {
        self.published
    }

    /// Whether the article may be announced on `today`: an entry never
    /// appears before its publication day.
    pub(crate) fn is_announced_on(&self, today: NaiveDate) -> bool {
        self.published <= today
    }
}

/// One entry of the timeline: an update record, or a published article.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Entry<'a> {
    Activity(&'a Update),
    Article(&'a ArticleUpdate),
}

impl Entry<'_> {
    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Activity(update) => &update.id,
            Self::Article(article) => article.id(),
        }
    }

    pub(crate) fn categories(&self) -> &[Category] {
        match self {
            Self::Activity(update) => &update.categories,
            Self::Article(_) => &ARTICLE_CATEGORIES,
        }
    }

    /// The day the entry is placed by: an event's final day, or the day an
    /// article was published.
    fn last_day(&self) -> NaiveDate {
        match self {
            Self::Activity(update) => update.last_event_day(),
            Self::Article(article) => article.published,
        }
    }

    fn first_day(&self) -> NaiveDate {
        match self {
            Self::Activity(update) => update.date,
            Self::Article(article) => article.published,
        }
    }

    /// Most recent first: by the final day, then the first day, both newest
    /// first, then by id. Ids are unique across the timeline, so the order is
    /// total, and entries of one day keep the same order on every page and
    /// in every build. For two update records this is exactly
    /// `Update::most_recent_first`.
    pub(crate) fn most_recent_first(left: &Self, right: &Self) -> Ordering {
        right
            .last_day()
            .cmp(&left.last_day())
            .then_with(|| right.first_day().cmp(&left.first_day()))
            .then_with(|| left.id().cmp(right.id()))
    }
}

/// The timeline on a given day (see `site_date`), optionally within one
/// category. Every update record falls in exactly one group; an article
/// falls in `recent` from its publication day and nowhere before it.
pub(crate) struct Timeline<'a> {
    /// Planned events that have not ended, ongoing and upcoming, soonest
    /// first: by first day, then id.
    pub(crate) upcoming: Vec<&'a Update>,
    /// Completed records and published articles, most recent first (see
    /// `Entry::most_recent_first`).
    pub(crate) recent: Vec<Entry<'a>>,
    /// Planned events that have ended, most recently held first.
    pub(crate) past_plans: Vec<&'a Update>,
}

impl<'a> Timeline<'a> {
    pub(crate) fn on(
        updates: impl IntoIterator<Item = &'a Update>,
        articles: &'a [ArticleUpdate],
        today: NaiveDate,
        category: Option<Category>,
    ) -> Self {
        let in_category =
            |categories: &[Category]| category.is_none_or(|wanted| categories.contains(&wanted));
        let mut timeline = Self {
            upcoming: Vec::new(),
            recent: Vec::new(),
            past_plans: Vec::new(),
        };
        for update in updates {
            if !in_category(&update.categories) {
                continue;
            }
            match update.phase_on(today) {
                UpdatePhase::Upcoming | UpdatePhase::Ongoing => timeline.upcoming.push(update),
                UpdatePhase::Completed => timeline.recent.push(Entry::Activity(update)),
                UpdatePhase::PastPlan => timeline.past_plans.push(update),
            }
        }
        if in_category(&ARTICLE_CATEGORIES) {
            timeline.recent.extend(
                articles
                    .iter()
                    .filter(|article| article.is_announced_on(today))
                    .map(Entry::Article),
            );
        }
        timeline.upcoming.sort_by(|left, right| {
            left.date
                .cmp(&right.date)
                .then_with(|| left.id.cmp(&right.id))
        });
        timeline.recent.sort_by(Entry::most_recent_first);
        timeline
            .past_plans
            .sort_by(|left, right| Update::most_recent_first(left, right));
        timeline
    }
}
