use std::fmt::Write as _;
use std::sync::LazyLock;

use chrono::{Datelike, NaiveDate};
use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::content::{
    Miscellany, Passage, Passages, ValidatedDiaryEntry, ValidatedHome, ValidatedNote,
    ValidatedReadingItem, ValidatedSiteContent, ValidatedSketch, ValidatedWriting,
    public_english_writings, reading_anchor,
};
use crate::model::{
    Category, DisplayLanguage, EventStatus, Inline, Language, LanguageTag, Link, Paragraph,
    PassageLanguage, Project, Publication, ReadingComment, SummarySegment, Update, UpdatePhase,
    summary_plain_text, summary_segments,
};
use crate::timeline::{ArticleName, ArticleUpdate, Entry, Timeline};

const SITE_ORIGIN: &str = "https://ryujin-hatakeyama.github.io";
const SITE_NAME: &str = "Ryujin Hatakeyama";
/// The date of the last substantive content revision, shown in the footer.
/// It is editorial metadata, kept in `content/last-updated.txt` (one line,
/// `YYYY-MM-DD`): change it deliberately with each such revision, by hand or
/// when the Writing Studio publishes. It is never taken from the clock, Git,
/// or deployment, so builds stay reproducible.
static LAST_UPDATED: LazyLock<NaiveDate> = LazyLock::new(|| {
    parse_last_updated(include_str!("../../content/last-updated.txt"))
        .expect("content/last-updated.txt must hold one date, YYYY-MM-DD")
});

fn parse_last_updated(text: &str) -> Option<NaiveDate> {
    let line = text.strip_suffix('\n').unwrap_or(text);
    if line.len() != 10 || line.contains(char::is_whitespace) {
        return None;
    }
    NaiveDate::parse_from_str(line, "%Y-%m-%d").ok()
}
/// GoatCounter's public counting endpoint and official script. These are
/// public configuration values, not credentials.
const GOATCOUNTER_ENDPOINT: &str = "https://hatakeyama.goatcounter.com/count";
const GOATCOUNTER_SCRIPT: &str = "https://gc.zgo.at/count.js";
/// Pageviews are counted only on this exact hostname. Elsewhere (local
/// previews, test servers, other deployments) GoatCounter's documented
/// `no_onload` option is set before its script can run, so nothing is sent.
const PRODUCTION_HOSTNAME: &str = "ryujin-hatakeyama.github.io";
const HOME_DESCRIPTION: &str = "Ryujin Hatakeyama is a second-year master's student at Tohoku University studying programming language theory and staged computation.";

#[derive(Debug)]
pub struct GeneratedPage {
    pub output_path: String,
    pub public_path: String,
    pub include_in_sitemap: bool,
    pub html: String,
}

#[derive(Clone, Copy)]
struct PageMetadata<'a> {
    title: &'a str,
    description: &'a str,
    current_path: &'a str,
    noindex: bool,
    article: bool,
}

/// The site's pages as of `today`, a calendar day in Asia/Tokyo that places
/// each update under Upcoming, Recent Updates, or Previously announced.
pub(crate) fn pages(content: &ValidatedSiteContent, today: NaiveDate) -> Vec<GeneratedPage> {
    let public_publications: Vec<_> = content
        .publications()
        .iter()
        .filter(|record| !record.draft && record.is_bibliographic())
        .collect();
    let presentations: Vec<_> = content
        .publications()
        .iter()
        .filter(|record| !record.draft && record.is_presentation())
        .collect();
    let public_projects: Vec<_> = content
        .projects()
        .iter()
        .filter(|project| !project.metadata.draft)
        .collect();
    let writings: Vec<_> = public_english_writings(content.writings()).collect();
    // The one timeline every list of updates is drawn from (src/timeline.rs).
    let articles = content.article_updates();
    let timeline = Timeline::on(content.updates(), articles, today, None);

    let mut pages = vec![
        page(
            "index.html",
            "/",
            true,
            PageMetadata {
                title: SITE_NAME,
                description: HOME_DESCRIPTION,
                current_path: "/",
                noindex: false,
                article: false,
            },
            home(content.home(), &timeline, today),
        ),
        page(
            "cv/index.html",
            "/cv/",
            true,
            PageMetadata {
                title: "CV",
                description: "Curriculum vitae of Ryujin Hatakeyama, a second-year master's student in the Graduate School of Information Sciences at Tohoku University.",
                current_path: "/cv/",
                noindex: false,
                article: false,
            },
            cv(&public_publications, &presentations),
        ),
        page(
            "research/index.html",
            "/research/",
            true,
            PageMetadata {
                title: "Research",
                description: "Research in programming language theory and staged computation by Ryujin Hatakeyama.",
                current_path: "/research/",
                noindex: false,
                article: false,
            },
            research(&public_publications, &presentations, &public_projects),
        ),
        page(
            "updates/index.html",
            "/updates/",
            true,
            PageMetadata {
                title: "Updates",
                description: "Research, academic, and writing updates from Ryujin Hatakeyama.",
                current_path: "/updates/",
                noindex: false,
                article: false,
            },
            updates_page(&timeline, None, today),
        ),
        page(
            "writings/index.html",
            "/writings/",
            true,
            PageMetadata {
                title: "Writings",
                description: "Writings by Ryujin Hatakeyama.",
                current_path: "/writings/",
                noindex: false,
                article: false,
            },
            writings_page(&writings),
        ),
        page(
            "miscellany/index.html",
            "/miscellany/",
            true,
            PageMetadata {
                title: "Miscellany",
                description: "Reading, study notes, occasional diary entries, and sketches by Ryujin Hatakeyama.",
                current_path: "/miscellany/",
                noindex: false,
                article: false,
            },
            miscellany_overview(content.miscellany(), &timeline.recent),
        ),
        page(
            "miscellany/reading/index.html",
            "/miscellany/reading/",
            true,
            PageMetadata {
                title: "Reading",
                description: "Books and papers read by Ryujin Hatakeyama.",
                current_path: "/miscellany/reading/",
                noindex: false,
                article: false,
            },
            html! { (reading_page(content.miscellany())) (message_form(content.miscellany(), "reading")) },
        ),
        page(
            "miscellany/notes/index.html",
            "/miscellany/notes/",
            true,
            PageMetadata {
                title: "Notes",
                description: "Study notes by Ryujin Hatakeyama.",
                current_path: "/miscellany/notes/",
                noindex: false,
                article: false,
            },
            html! { (notes_page(content.miscellany())) (message_form(content.miscellany(), "notes")) },
        ),
        page(
            "miscellany/diary/index.html",
            "/miscellany/diary/",
            true,
            PageMetadata {
                title: "Diary",
                description: "Occasional diary entries by Ryujin Hatakeyama.",
                current_path: "/miscellany/diary/",
                noindex: false,
                article: false,
            },
            html! { (diary_page(content.miscellany())) (message_form(content.miscellany(), "diary")) },
        ),
        page(
            "miscellany/sketches/index.html",
            "/miscellany/sketches/",
            true,
            PageMetadata {
                title: "Sketches",
                description: "Sketches by Ryujin Hatakeyama.",
                current_path: "/miscellany/sketches/",
                noindex: false,
                article: false,
            },
            html! { (sketches_page(content.miscellany())) (message_form(content.miscellany(), "sketches")) },
        ),
        // Results depend on the query; the page itself is not for search
        // engines and is not in the sitemap.
        page(
            "miscellany/search/index.html",
            SEARCH_PATH,
            false,
            PageMetadata {
                title: "Search",
                description: "Search Reading, Notes, Diary, and Sketches by Ryujin Hatakeyama.",
                current_path: SEARCH_PATH,
                noindex: true,
                article: false,
            },
            search_page(),
        ),
        page(
            "privacy/index.html",
            "/privacy/",
            true,
            PageMetadata {
                title: "Privacy",
                description: "How this site counts visits.",
                current_path: "/privacy/",
                noindex: false,
                article: false,
            },
            privacy(),
        ),
        page(
            "404.html",
            "/404/",
            false,
            PageMetadata {
                title: "Page not found",
                description: "The requested page could not be found.",
                current_path: "/404/",
                noindex: true,
                article: false,
            },
            not_found(),
        ),
    ];

    for category in Category::ALL {
        let category_timeline = Timeline::on(content.updates(), articles, today, Some(category));
        let title = format!("{} updates", category.label());
        let description = format!("{} updates from Ryujin Hatakeyama.", category.label());
        let path = format!("/updates/{}/", category.slug());
        pages.push(page_owned(
            format!("updates/{}/index.html", category.slug()),
            path.clone(),
            true,
            &title,
            &description,
            &path,
            updates_page(&category_timeline, Some(category), today),
        ));
    }

    for update in content.updates().iter().filter(|update| update.detail) {
        let path = update_detail_path(update);
        pages.push(page_owned(
            format!("updates/item/{}/index.html", update.id),
            path.clone(),
            true,
            &update.title.en,
            &summary_plain_text(&update.summary.en),
            &path,
            update_detail(update),
        ));
    }

    for writing in writings {
        if writing.metadata().external_url.is_some() {
            continue;
        }
        let path = format!("/writings/{}/", writing.metadata().slug);
        pages.push(page_owned(
            format!("writings/{}/index.html", writing.metadata().slug),
            path.clone(),
            true,
            &writing.metadata().title,
            &writing.metadata().description,
            &path,
            writing_detail(writing),
        ));
    }

    for note in content.miscellany().public_notes() {
        let metadata = note.metadata();
        let path = note_path(&metadata.slug);
        let description = metadata
            .description
            .clone()
            .unwrap_or_else(|| format!("A study note by Ryujin Hatakeyama: {}.", metadata.title));
        pages.push(page_owned(
            format!("miscellany/notes/{}/index.html", metadata.slug),
            path.clone(),
            true,
            &metadata.title,
            &description,
            &path,
            note_detail(note, &content.miscellany().display_languages()),
        ));
    }

    for sketch in content.miscellany().public_sketches() {
        let metadata = sketch.metadata();
        let path = sketch_path(&metadata.slug);
        let description = metadata
            .description
            .clone()
            .unwrap_or_else(|| format!("A sketch by Ryujin Hatakeyama: {}.", metadata.title));
        pages.push(page_owned(
            format!("miscellany/sketches/{}/index.html", metadata.slug),
            path.clone(),
            true,
            &metadata.title,
            &description,
            &path,
            sketch_detail(sketch, &content.miscellany().display_languages()),
        ));
    }

    // Each month with published entries has its own page; months are linked
    // to their neighbours with entries, skipping empty months.
    let months = diary_months(content.miscellany());
    for (index, month) in months.iter().enumerate() {
        let path = month.path();
        let title = format!("Diary, {}", month.name());
        let description = format!("Diary entries by Ryujin Hatakeyama, {}.", month.name());
        let languages = month_languages(month);
        pages.push(page_owned(
            month.output_path(),
            path.clone(),
            true,
            &title,
            &description,
            &path,
            diary_month_page(
                month,
                months.get(index + 1),
                index.checked_sub(1).and_then(|newer| months.get(newer)),
                &languages,
            ),
        ));
    }

    pages.sort_by(|left, right| left.output_path.cmp(&right.output_path));
    pages
}

fn page(
    output_path: &str,
    public_path: &str,
    include_in_sitemap: bool,
    metadata: PageMetadata<'_>,
    body: Markup,
) -> GeneratedPage {
    GeneratedPage {
        output_path: output_path.to_owned(),
        public_path: public_path.to_owned(),
        include_in_sitemap,
        html: layout(metadata, body).into_string(),
    }
}

fn page_owned(
    output_path: String,
    public_path: String,
    include_in_sitemap: bool,
    title: &str,
    description: &str,
    current_path: &str,
    body: Markup,
) -> GeneratedPage {
    let html = layout(
        PageMetadata {
            title,
            description,
            current_path,
            noindex: false,
            article: true,
        },
        body,
    )
    .into_string();
    GeneratedPage {
        output_path,
        public_path,
        include_in_sitemap,
        html,
    }
}

fn layout(metadata: PageMetadata<'_>, body: Markup) -> Markup {
    let full_title = if metadata.title == SITE_NAME {
        SITE_NAME.to_owned()
    } else {
        format!("{} — {SITE_NAME}", metadata.title)
    };
    let canonical = format!("{SITE_ORIGIN}{}", metadata.current_path);
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="UTF-8";
                meta name="viewport" content="width=device-width";
                meta name="generator" content="site-builder (Rust/Maud)";
                meta name="description" content=(metadata.description);
                meta name="theme-color" content="#f4f0e8" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#111820" media="(prefers-color-scheme: dark)";
                @if metadata.noindex {
                    meta name="robots" content="noindex, nofollow";
                }
                link rel="canonical" href=(canonical);
                link rel="icon" href="/favicon.svg?v=2" type="image/svg+xml";
                link rel="icon" href="/favicon-32x32.png?v=2" type="image/png" sizes="32x32";
                link rel="apple-touch-icon" href="/apple-touch-icon.png?v=2" sizes="180x180";
                link rel="alternate" type="application/rss+xml" title="Ryujin Hatakeyama — Updates" href="/rss.xml";
                link rel="stylesheet" href="/assets/site.css";
                @if metadata.article {
                    meta property="og:type" content="article";
                } @else {
                    meta property="og:type" content="website";
                }
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(metadata.description);
                meta property="og:locale" content="en_US";
                meta property="og:url" content=(canonical);
                title { (full_title) }
                script src="/assets/site.js" {}
                (analytics())
            }
            body {
                a.skip-link href="#main-content" { "Skip to content" }
                div.site-shell {
                    (header(metadata.current_path))
                    main id="main-content" { (body) }
                    (footer())
                }
            }
        }
    }
}

/// Cookie-free, aggregate visit counting with GoatCounter. The inline guard
/// runs while the document is parsed, before the asynchronous script can
/// execute, and disables the automatic pageview outside production.
fn analytics() -> Markup {
    let guard = format!(
        "if(location.hostname!=='{PRODUCTION_HOSTNAME}')window.goatcounter={{no_onload:true}};"
    );
    html! {
        script { (PreEscaped(guard)) }
        script data-goatcounter=(GOATCOUNTER_ENDPOINT) async src=(GOATCOUNTER_SCRIPT) {}
    }
}

fn header(current_path: &str) -> Markup {
    html! {
        header.site-header {
            a.wordmark href="/" aria-label="Ryujin Hatakeyama home" { "Ryujin Hatakeyama" }
            nav.primary-nav aria-label="Primary navigation" {
                (nav_link("/", "Home", current_path == "/"))
                (nav_link("/research/", "Research", current_path.starts_with("/research/")))
                (nav_link("/miscellany/", "Miscellany", current_path.starts_with("/miscellany/")))
            }
            div.header-tools {
                // A single toggle button with a stable name; aria-pressed and the
                // knob position both follow the root data-theme attribute.
                button.theme-toggle
                    id="appearance-toggle"
                    type="button"
                    aria-label="Dark mode"
                    aria-pressed="false" {
                    span.theme-toggle-knob aria-hidden="true" {}
                    svg.theme-icon.theme-icon-sun aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="16" height="16" {
                        circle cx="12" cy="12" r="3.6" fill="none" stroke="currentColor" stroke-width="1.7" {}
                        path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M18.7 5.3l-1.4 1.4M6.7 17.3l-1.4 1.4" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" {}
                    }
                    svg.theme-icon.theme-icon-moon aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="16" height="16" {
                        path d="M20.2 15.3A8.5 8.5 0 0 1 8.7 3.8 8.5 8.5 0 1 0 20.2 15.3Z" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" {}
                    }
                }
            }
        }
    }
}

fn nav_link(href: &str, label: &str, active: bool) -> Markup {
    if active {
        html! { a href=(href) aria-current="page" { (label) } }
    } else {
        html! { a href=(href) { (label) } }
    }
}

fn footer() -> Markup {
    html! {
        footer.site-footer {
            p.footer-name { "© 2026 Ryujin Hatakeyama" }
            p.footer-links { a href="/privacy/" { "Privacy" } }
            // The feed stays discoverable through the <head> link.
            p.footer-updated {
                "Last updated "
                time datetime=(LAST_UPDATED.format("%Y-%m-%d")) { (LAST_UPDATED.format("%-d %B %Y")) }
            }
        }
    }
}

/// The homepage previews each section separately, so a busy Upcoming section
/// can never crowd completed updates out of Recent Updates.
const HOME_UPCOMING_LIMIT: usize = 3;
const HOME_RECENT_LIMIT: usize = 5;

/// Planned events that have ended are listed only on the Updates pages, under
/// Previously announced, until the author records what happened.
fn home(home: &ValidatedHome, timeline: &Timeline<'_>, today: NaiveDate) -> Markup {
    let upcoming = &timeline.upcoming[..timeline.upcoming.len().min(HOME_UPCOMING_LIMIT)];
    let recent = &timeline.recent[..timeline.recent.len().min(HOME_RECENT_LIMIT)];
    let email = "hatakeyama.ryujin.q7@dc.tohoku.ac.jp";
    let encoded_email = email
        .chars()
        .map(|character| u32::from(character).to_string())
        .collect::<Vec<_>>()
        .join("-");
    html! {
        section.home-intro aria-labelledby="home-name" {
            div.identity-block {
                // Both forms of the name form one heading; the space keeps
                // them separate words when read aloud or copied, while the
                // layout itself comes from CSS.
                h1.person-name id="home-name" {
                    span.name-latin lang="en" { "Ryujin Hatakeyama" }
                    " "
                    span.name-japanese lang="ja" { "畠山竜迅" }
                }
                p.name-pronunciation {
                    span lang="ja" { "はたけやま りゅうじん" }
                    span aria-hidden="true" { " · " }
                    span.ipa { "/hatakejama ɾʲɯːdʑiɴ/" }
                }
            }
            div.home-bio-copy {
                @for paragraph in home.bio() {
                    p.home-bio { (inline_content(paragraph)) }
                }
            }
            nav.home-links aria-label="Academic and contact links" {
                ul.home-academic-links {
                    li { (home_link("/cv/", cv_icon(), "CV", None)) }
                    li { (home_link("/research/#presentations", presentation_icon(), "Talks & Presentations", None)) }
                    li { (home_link("https://github.com/ryujin-hatakeyama", github_icon(), "GitHub", None)) }
                    li { (home_link("#email", email_icon(), "Email", Some(&encoded_email))) }
                }
            }
        }
        @if !upcoming.is_empty() {
            section.home-news aria-labelledby="upcoming-heading" {
                header.home-news-heading {
                    h2 id="upcoming-heading" { "Upcoming" }
                }
                (upcoming_list(upcoming, true, today))
            }
        }
        @if !recent.is_empty() || upcoming.is_empty() {
            section.home-news aria-labelledby="updates-heading" {
                header.home-news-heading {
                    h2 id="updates-heading" { "Recent Updates" }
                }
                (update_list(recent, true))
            }
        }
        // One link to the complete records, after both previews, instead of a
        // "read more" link on every entry.
        p.home-all-updates { a href="/updates/" { "All updates" } }
    }
}

/// A validated paragraph as phrasing content. Text is escaped by Maud, and
/// link targets have already been checked.
fn inline_content(paragraph: &Paragraph) -> Markup {
    html! {
        @for inline in paragraph.inlines() {
            @match inline {
                Inline::Text(text) => (text),
                Inline::Link { text, target } => a href=(target.as_str()) { (text) },
            }
        }
    }
}

fn home_link(href: &str, icon: Markup, label: &str, email_code: Option<&str>) -> Markup {
    if let Some(code) = email_code {
        html! { a href=(href) data-email-code=(code) { (icon) span { (label) } } }
    } else {
        html! { a href=(href) { (icon) span { (label) } } }
    }
}

fn cv_icon() -> Markup {
    html! { svg.home-link-icon aria-hidden="true" viewBox="0 0 16 16" { path d="M3.5 1.5h6l3 3v10h-9zM9.5 1.5v3h3M5.5 7.5h5M5.5 10h5M5.5 12.5h3.5" {} } }
}

fn presentation_icon() -> Markup {
    html! { svg.home-link-icon aria-hidden="true" viewBox="0 0 16 16" { path d="M1.5 2.5h13M14 2.5v7.25A1.25 1.25 0 0 1 12.75 11h-9.5A1.25 1.25 0 0 1 2 9.75V2.5M4.8 14 8 11l3.2 3" {} } }
}

fn github_icon() -> Markup {
    html! { svg.home-link-icon aria-hidden="true" viewBox="0 0 16 16" { path d="M8 1.3a6.7 6.7 0 0 0-2.1 13.1c.3.1.4-.1.4-.3v-1.3c-1.8.4-2.2-.8-2.2-.8-.3-.8-.7-1-1-1.2-.6-.4 0-.4 0-.4.7.1 1.1.7 1.1.7.6 1.1 1.7.8 2.1.6.1-.5.2-.8.5-1-1.5-.2-3-.7-3-3.3 0-.7.3-1.3.7-1.8-.1-.2-.3-.8.1-1.8 0 0 .6-.2 1.8.7A6.4 6.4 0 0 1 8 4.3c.6 0 1.1.1 1.6.2 1.3-.9 1.8-.7 1.8-.7.4 1 .2 1.6.1 1.8.5.5.7 1.1.7 1.8 0 2.6-1.6 3.1-3 3.3.3.2.5.6.5 1.2v2.2c0 .2.1.4.5.3A6.7 6.7 0 0 0 8 1.3Z" fill="currentColor" stroke="none" {} } }
}

fn email_icon() -> Markup {
    html! { svg.home-link-icon aria-hidden="true" viewBox="0 0 16 16" { path d="M1.5 3.5h13v9h-13zM2 4l6 4.5L14 4" {} } }
}

fn cv(publications: &[&Publication], presentations: &[&Publication]) -> Markup {
    html! {
        article.cv-page {
            header.cv-header {
                h1 { "Curriculum vitae" }
                p { "Ryujin Hatakeyama " span lang="ja" { "（畠山竜迅）" } }
            }
            section aria-labelledby="education-heading" {
                h2 id="education-heading" { "Education" }
                div.cv-entry {
                    time datetime="2025" { "2025–present" }
                    div {
                        p { strong { "Master's program, Graduate School of Information Sciences, Tohoku University" } }
                        p { a href="https://www.is.tohoku.ac.jp/en/laboratory/list_dept/a11.html" { "Foundations of Software Science" } }
                        p { "Current status: second-year master's student (M2), academic year 2026." }
                        p { "Supervisor: " a href="https://www.kb.ecei.tohoku.ac.jp/~sumii/" { "Prof. Eijiro Sumii" } "." }
                    }
                }
                div.cv-entry {
                    time datetime="2025" { "2025" }
                    div {
                        p { strong { "Bachelor of Engineering, Tohoku University" } }
                        p { "School of Engineering, Department of Electrical, Information and Physics Engineering" }
                        p { "Computer Science Course" }
                    }
                }
            }
            section aria-labelledby="programs-heading" {
                h2 id="programs-heading" { "Academic programs" }
                div.cv-entry {
                    time datetime="2025" { "2025–present" }
                    p { "Selected participant, " a href="https://www.aie.tohoku.ac.jp/english/" { "WISE Program for AI Electronics (AIE)" } ", Tohoku University." }
                }
            }
            section aria-labelledby="research-experience-heading" {
                h2 id="research-experience-heading" { "Research experience" }
                div.cv-entry {
                    time datetime="2022-10" { "Oct 2022 – Mar 2023" }
                    div {
                        p { strong { "Tohoku NLP Group, Tohoku University" } }
                        p { "Undergraduate research participant, Step-QI School, Advanced Creative Engineering I and II." }
                        p { a href="https://www.nlp.ecei.tohoku.ac.jp/about-us/former-members/" { "Laboratory record" } }
                    }
                }
            }
            section aria-labelledby="teaching-experience-heading" {
                h2 id="teaching-experience-heading" { "Teaching experience" }
                div.cv-entry {
                    time datetime="2025" { "Spring 2025" }
                    div {
                        p { strong { "Teaching Assistant — " span lang="ja" { "プログラミング演習B" } " (Programming B; F#)" } }
                        p { "Department of Electrical, Information and Physics Engineering, Tohoku University." }
                        p { "Assisted with F# programming exercises taught by " a href="https://www.riec.tohoku.ac.jp/~asada/" { "Kazuyuki Asada" } " and " a href="https://www.r-info.tohoku.ac.jp/en/246db5ad378b8edbec312635d74650b4.html" { "Kentaro Kikuchi" } "." }
                    }
                }
            }
            section aria-labelledby="publications-heading" {
                h2 id="publications-heading" { "Publications" }
                (filterable_publications(publications))
            }
            section aria-labelledby="presentations-heading" {
                h2 id="presentations-heading" { "Presentations" }
                (publication_list(presentations, true))
            }
            section aria-labelledby="awards-heading" {
                h2 id="awards-heading" { "Honors and awards" }
                div.cv-entry {
                    time datetime="2023" { "2023" }
                    div {
                        p { strong { "Best Award" } }
                        p { "2022 academic-year Advanced Creative Engineering Training Poster Session, Step-QI School, Tohoku University." }
                    }
                }
            }
        }
    }
}

fn research(
    publications: &[&Publication],
    presentations: &[&Publication],
    projects: &[&Project],
) -> Markup {
    html! {
        header.plain-page-header {
            h1 { "Research" }
            // The CV has no primary navigation item, so Research links to it
            // after its own sections.
            nav.jump-links aria-label="Research contents" {
                a href="#publications" { "Publications" }
                @if !projects.is_empty() { a href="#projects" { "Projects" } }
                a href="#presentations" { "Talks & Presentations" }
                a href="/cv/" { "CV" }
            }
        }
        div.research-page {
            section id="publications" class="content-section" {
                h2 { "Publications" }
                (filterable_publications(publications))
            }
            @if !projects.is_empty() {
                section id="projects" class="content-section" {
                    h2 { "Projects" }
                    @for project in projects {
                        article.project-entry {
                            h3 { (&project.metadata.title) }
                            p { (&project.metadata.summary) }
                        }
                    }
                }
            }
            section id="presentations" class="content-section" {
                h2 { "Talks & Presentations" }
                (publication_list(presentations, true))
            }
        }
    }
}

/// All bibliographic publications in one newest-first list, preceded by a
/// language filter. The filter is hidden until the browser script enables it,
/// so the complete list remains visible without JavaScript and in print.
fn filterable_publications(publications: &[&Publication]) -> Markup {
    if publications.is_empty() {
        return html! { p.empty-state { "No publications yet." } };
    }
    html! {
        div.publication-filter hidden data-publication-filter {
            label for="publication-language" { "Language" }
            select id="publication-language" autocomplete="off" aria-controls="publication-list" {
                option value="all" selected { "All" }
                option value="en" { "English" }
                option value="ja" { "Japanese" }
            }
            p.sr-only aria-live="polite" data-publication-filter-status {}
        }
        (publication_entries(publications, false, Some("publication-list")))
    }
}

fn publication_list(publications: &[&Publication], presentations: bool) -> Markup {
    if publications.is_empty() {
        return html! { p.empty-state { "No publications yet." } };
    }
    publication_entries(publications, presentations, None)
}

fn publication_entries(
    publications: &[&Publication],
    presentations: bool,
    filterable_id: Option<&str>,
) -> Markup {
    html! {
        ol.publication-list id=[filterable_id] {
            @for publication in publications {
                li data-language=[filterable_id.map(|_| publication.language.code())] {
                    article {
                        p.publication-index { (publication.year) }
                        div {
                            h3 lang=(publication.language.code()) { (&publication.title) }
                            p.authors { (publication.authors.join(", ")) }
                            p.venue {
                                @if let Some(venue) = &publication.venue { span { (venue) ". " } }
                                span { (classification(publication, presentations)) }
                            }
                            @if let Some(note) = &publication.note { p.publication-note { (note) } }
                            @if !publication.links.is_empty() { (record_links(&publication.links, true)) }
                        }
                    }
                }
            }
        }
    }
}

fn classification(publication: &Publication, presentations: bool) -> String {
    if !presentations {
        if let Some(review) = publication
            .presentation
            .as_ref()
            .and_then(|metadata| metadata.proceedings_review)
        {
            return review.label().to_owned();
        }
    }
    if presentations {
        if let Some(metadata) = &publication.presentation {
            let mut format = metadata.format.label().to_owned();
            if let Some(category) = &metadata.category {
                format.push_str(" (");
                format.push_str(category);
                format.push(')');
            }
            if let Some(review) = metadata.proceedings_review {
                format.push_str(" · ");
                format.push_str(review.label());
            }
            return format;
        }
    }
    format!(
        "{} · {}",
        publication.record_type.label(),
        publication.status.label()
    )
}

/// An Updates page from the timeline on the build day, already limited to
/// `active` when it names a category.
fn updates_page(timeline: &Timeline<'_>, active: Option<Category>, today: NaiveDate) -> Markup {
    let Timeline {
        upcoming,
        recent,
        past_plans,
    } = timeline;
    let past_plans: Vec<_> = past_plans
        .iter()
        .map(|update| Entry::Activity(update))
        .collect();
    html! {
        (page_intro(active.map_or("Updates", Category::label)))
        (update_filters(active))
        @if !upcoming.is_empty() {
            section.update-section.upcoming-updates aria-labelledby="upcoming-heading" {
                h2 id="upcoming-heading" { "Upcoming" }
                (upcoming_list(upcoming, false, today))
            }
        }
        @if !recent.is_empty() {
            section.update-section aria-labelledby="recent-heading" {
                h2 id="recent-heading" { "Recent Updates" }
                (update_list(recent, false))
            }
        } @else if upcoming.is_empty() && past_plans.is_empty() {
            (update_list(recent, false))
        }
        // Plans whose events have ended keep their original wording and
        // anchors until the author records what happened; they are neither
        // presented as completed nor removed.
        @if !past_plans.is_empty() {
            section.update-section.past-plans aria-labelledby="past-plans-heading" {
                h2 id="past-plans-heading" { "Previously announced" }
                (update_list(&past_plans, false))
            }
        }
    }
}

/// Planned events lead with their event date; one under way on `today` is
/// marked Ongoing, which describes the event's dates, not attendance.
fn upcoming_list(updates: &[&Update], compact: bool, today: NaiveDate) -> Markup {
    let class = if compact {
        "update-list upcoming-list compact"
    } else {
        "update-list upcoming-list"
    };
    html! {
        ol class=(class) {
            @for update in updates {
                li {
                    article id=[(!compact).then(|| format!("update-{}", update.id))] {
                        div.update-meta {
                            (event_dates(update))
                            @if update.phase_on(today) == UpdatePhase::Ongoing {
                                span.phase-label { "Ongoing" }
                            }
                            span.kind-label { (update.kind.label()) }
                            @if let Some(label) = visible_category_labels(&update.categories) {
                                span.category-label { (label) }
                            }
                        }
                        div.update-copy {
                            h3 { (update_title(update, true)) }
                            @if !compact {
                                p { (update_summary(&update.summary.en)) }
                            }
                            (update_links(update, !compact, true))
                        }
                    }
                }
            }
        }
    }
}

fn update_detail_path(update: &Update) -> String {
    format!("/updates/item/{}/", update.id)
}

/// An update title as phrasing content. A record's `titleLink` turns only the
/// event's name into a link to its official page; otherwise a record with a
/// detail page links its whole title there (when `link_detail` is set). The two
/// never combine, so anchors are never nested; `update_links` then offers the
/// detail page separately.
fn update_title(update: &Update, link_detail: bool) -> Markup {
    let title = &update.title.en;
    if let Some(link) = &update.title_link {
        if let Some((before, after)) = link.split(title) {
            return html! {
                (before)
                a href=(&link.url) lang=[link.lang.map(Language::code)] { (&link.text) }
                (after)
            };
        }
    }
    if link_detail && update.detail {
        html! { a href=(update_detail_path(update)) { (title) } }
    } else {
        html! { (title) }
    }
}

/// The links beneath an update: its detail page when the title links to the
/// event instead, then (in full views) its own links. Validation guarantees
/// these are distinct from the title's event link, so all are shown.
fn update_links(update: &Update, include_record_links: bool, include_detail: bool) -> Markup {
    let links: &[Link] = if include_record_links {
        &update.links
    } else {
        &[]
    };
    let detail = include_detail && update.detail && update.title_link.is_some();
    html! {
        @if detail || !links.is_empty() {
            p.record-links {
                @if detail {
                    a href=(update_detail_path(update)) {
                        "Details"
                        span.sr-only { ": " (&update.title.en) }
                    }
                }
                @for link in links { (record_link(link, true)) }
            }
        }
    }
}

fn update_summary(summary: &str) -> Markup {
    html! {
        @for segment in summary_segments(summary) {
            @match segment {
                SummarySegment::Text(text) => (text),
                SummarySegment::Link { text, url } => a href=(url) { (text) },
            }
        }
    }
}

/// The category labels shown on an entry. Activities (the `academia`
/// category) is the default context of most entries and is not labelled, so
/// only explicit categories such as Research appear; an entry with none shows
/// no label at all.
fn visible_category_labels(categories: &[Category]) -> Option<String> {
    let labels: Vec<_> = categories
        .iter()
        .filter(|category| category.is_labelled())
        .map(|category| category.label())
        .collect();
    (!labels.is_empty()).then(|| labels.join(" · "))
}

fn page_intro(title: &str) -> Markup {
    html! { header.page-intro { h1 { (title) } } }
}

fn update_filters(active: Option<Category>) -> Markup {
    html! {
        nav.update-filters aria-label="Update categories" {
            (filter_link("/updates/", "All", active.is_none()))
            // Activities has no filter of its own; /updates/academia/ is still
            // generated so that existing links keep working.
            @for category in Category::ALL.into_iter().filter(|category| category.is_labelled()) {
                (filter_link(
                    &format!("/updates/{}/", category.slug()),
                    category.label(),
                    active == Some(category),
                ))
            }
        }
    }
}

fn filter_link(href: &str, label: &str, active: bool) -> Markup {
    if active {
        html! { a href=(href) aria-current="page" { (label) } }
    } else {
        html! { a href=(href) { (label) } }
    }
}

fn update_list(entries: &[Entry<'_>], compact: bool) -> Markup {
    if entries.is_empty() {
        return html! { p.empty-state { "No updates yet." } };
    }
    let class = if compact {
        "update-list compact"
    } else {
        "update-list"
    };
    html! {
        ol class=(class) {
            @for entry in entries {
                li {
                    article id=[(!compact).then(|| format!("update-{}", entry.id()))] {
                        div.update-meta {
                            (entry_dates(entry))
                            @if let Entry::Activity(update) = entry {
                                span.kind-label { (update.kind.label()) }
                            }
                            // Categories are plain, muted metadata rather than
                            // links; the category filters provide navigation.
                            @if let Some(label) = visible_category_labels(entry.categories()) {
                                span.category-label { (label) }
                            }
                        }
                        div.update-copy {
                            @match entry {
                                Entry::Activity(update) => {
                                    h3 { (update_title(update, true)) }
                                    @if !compact {
                                        p { (update_summary(&update.summary.en)) }
                                    }
                                    (update_links(update, !compact, true))
                                }
                                // An article's entry is its one sentence; the
                                // article itself is a click away.
                                Entry::Article(article) => h3 { (article_sentence(article)) },
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The dates an entry is listed under: an activity's event dates, or the day
/// an article was published.
fn entry_dates(entry: &Entry<'_>) -> Markup {
    match entry {
        Entry::Activity(update) => event_dates(update),
        Entry::Article(article) => {
            let day = article.published();
            html! { time datetime=(day.format("%Y-%m-%d")) { (format_date(day)) } }
        }
    }
}

/// `Published “Title” in Notes.`, the title exactly as written and linked to
/// the article, in its own language. An untitled diary entry is named by the
/// day it records.
fn article_sentence(article: &ArticleUpdate) -> Markup {
    let section = article.section().label();
    match article.name() {
        ArticleName::Title { text, lang } => html! {
            "Published “" a href=(article.path()) lang=[lang] { (text) } "” in " (section) "."
        },
        ArticleName::DiaryDay(day) => html! {
            "Published an entry for " a href=(article.path()) { (day.format("%-d %B %Y")) } " in " (section) "."
        },
    }
}

/// An entry as one line of text: an activity's title (its event's name
/// linked, as everywhere), or an article's sentence.
fn entry_text(entry: &Entry<'_>) -> Markup {
    match entry {
        Entry::Activity(update) => update_title(update, true),
        Entry::Article(article) => article_sentence(article),
    }
}

fn update_detail(update: &Update) -> Markup {
    html! {
        article.detail-sheet.update-detail {
            header {
                p.longform-kicker { "Update " span aria-hidden="true" { "/" } " " (update.kind.label()) }
                h1 { (update_title(update, false)) }
                p.summary { (update_summary(&update.summary.en)) }
            }
            dl {
                div.detail-meta { dt { "Event date" } dd { (event_dates(update)) } }
                @if update.event_status == EventStatus::Planned {
                    div.detail-meta { dt { "Status" } dd { "Planned" } }
                }
                @if let Some(label) = visible_category_labels(&update.categories) {
                    div.detail-meta { dt { "Categories" } dd { (label) } }
                }
            }
            @if let Some(body) = &update.body { div.prose { p { (&body.en) } } }
            (update_links(update, true, false))
        }
    }
}

fn event_dates(update: &Update) -> Markup {
    let start = update.date.format("%Y-%m-%d").to_string();
    match update.end_date {
        Some(end) if end != update.date => html! {
            time datetime=(start) { (format_date_range(update.date, end)) }
        },
        _ => html! { time datetime=(start) { (format_date(update.date)) } },
    }
}

fn record_links(links: &[Link], include_type: bool) -> Markup {
    html! {
        p.record-links {
            @for link in links { (record_link(link, include_type)) }
        }
    }
}

fn record_link(link: &Link, include_type: bool) -> Markup {
    html! {
        a href=(&link.url) {
            (&link.label)
            @if include_type { span.sr-only { " (" (link.link_type.label()) ")" } }
        }
    }
}

fn writings_page(writings: &[&ValidatedWriting]) -> Markup {
    html! {
        (page_intro("Writings"))
        @if writings.is_empty() {
            p.empty-state { "No writings yet." }
        } @else {
            ol.writing-list {
                @for writing in writings { (writing_list_item(writing)) }
            }
        }
    }
}

fn writing_list_item(writing: &ValidatedWriting) -> Markup {
    let metadata = writing.metadata();
    let href = metadata
        .external_url
        .clone()
        .unwrap_or_else(|| format!("/writings/{}/", metadata.slug));
    html! {
        li {
            article {
                p.writing-meta {
                    span { (metadata.kind.label()) }
                    time datetime=(metadata.date.format("%Y-%m-%d")) { (format_date(metadata.date)) }
                }
                h3 { a href=(href) lang=(metadata.lang.code()) { (&metadata.title) } }
                p { (&metadata.description) }
            }
        }
    }
}

fn writing_detail(writing: &ValidatedWriting) -> Markup {
    let metadata = writing.metadata();
    let class = if metadata.lang == Language::Ja {
        "longform longform-ja"
    } else {
        "longform"
    };
    html! {
        article class=(class) lang=(metadata.lang.code()) {
            header.longform-header {
                p.longform-kicker {
                    (metadata.kind.label()) " " span aria-hidden="true" { "/" } " "
                    time datetime=(metadata.date.format("%Y-%m-%d")) { (format_date(metadata.date)) }
                }
                h1 { (&metadata.title) }
                p.dek { (&metadata.description) }
                @if let Some(publication) = &metadata.publication { p.publication-context { (publication) } }
            }
            div.prose { (PreEscaped(writing.rendered_body())) }
        }
    }
}

/// One Miscellany section: its route, label, functional description (empty
/// when the author has given none), and the value its pages carry in the
/// search index's `section` filter and in `/miscellany/search/?section=`.
struct MiscellanySection {
    path: &'static str,
    label: &'static str,
    key: &'static str,
}

/// The four Miscellany sections, in the order they are listed.
const MISCELLANY_SECTIONS: [MiscellanySection; 4] = [
    MiscellanySection {
        path: "/miscellany/reading/",
        label: "Reading",
        key: "reading",
    },
    MiscellanySection {
        path: "/miscellany/notes/",
        label: "Notes",
        key: "notes",
    },
    MiscellanySection {
        path: "/miscellany/diary/",
        label: "Diary",
        key: "diary",
    },
    MiscellanySection {
        path: "/miscellany/sketches/",
        label: "Sketches",
        key: "sketches",
    },
];

const READING: &MiscellanySection = &MISCELLANY_SECTIONS[0];
const NOTES: &MiscellanySection = &MISCELLANY_SECTIONS[1];
const DIARY: &MiscellanySection = &MISCELLANY_SECTIONS[2];
const SKETCHES: &MiscellanySection = &MISCELLANY_SECTIONS[3];

/// The Writing Studio's category icons (studio/src/page.ts), drawn on the
/// same 24-unit grid and stroke, in `MISCELLANY_SECTIONS` order: an open
/// book, a notebook, a calendar, and a pencil.
const MISCELLANY_ICONS: [&str; 4] = [
    // Reading: an open book.
    r#"<path d="M12 6.5C10.2 5.2 7.7 4.5 4 4.5v13c3.7 0 6.2.7 8 2 1.8-1.3 4.3-2 8-2v-13c-3.7 0-6.2.7-8 2Z"/><path d="M12 6.5v13"/>"#,
    // Notes: a written page.
    r#"<rect x="5" y="3.5" width="14" height="17" rx="1.5"/><path d="M8.5 8.5h7M8.5 12h7M8.5 15.5h4"/>"#,
    // Diary: a calendar's page.
    r#"<rect x="4" y="5" width="16" height="15.5" rx="1.5"/><path d="M4 9.5h16M8.5 3v4M15.5 3v4"/>"#,
    // Sketches: two forms laid over each other, a thought taking shape.
    r#"<circle cx="9.5" cy="9.5" r="5.5"/><rect x="10" y="10" width="9.5" height="9.5" rx="1"/>"#,
];

/// The search page, which reads the static index built from the published
/// Reading, Notes, Diary, and Sketches pages (docs/search.md).
const SEARCH_PATH: &str = "/miscellany/search/";

fn note_path(slug: &str) -> String {
    format!("/miscellany/notes/{slug}/")
}

fn sketch_path(slug: &str) -> String {
    format!("/miscellany/sketches/{slug}/")
}

/// The search index reads only an element marked `data-pagefind-body`,
/// with its section as a filter. Only the published Reading, Notes, Diary
/// month, and Sketches pages render one, and the indexer reads only files
/// below `miscellany/` (scripts/search-index.mjs); everything outside the
/// marked element (navigation, controls) is left out.
fn search_filter(section: &MiscellanySection) -> String {
    format!("section:{}", section.label)
}

/// The one editorial note about multilingual writing, shown on the overview
/// only once some published entry has texts in more than one language.
const MULTILINGUAL_NOTE: &str = "An entry may contain texts in different languages that are not necessarily translations of one another.";

/// How many of the latest updates the overview shows beside the sections:
/// three keep the column about as tall as the four sections.
const MISCELLANY_RECENT_LIMIT: usize = 3;

/// The overview: its name, the search, and the four sections as a compact
/// table of contents, one row (icon, name) each; then the latest entries of
/// the one Updates timeline, `recent` (see `Timeline`), activities and
/// published articles alike. On a wide screen the sections stand at the
/// left and the search and the updates at the right; on a narrow one the
/// sections follow the search and the updates are left to the Updates page
/// (assets/styles/site.css). The search is there even when nothing is
/// published.
fn miscellany_overview(miscellany: &Miscellany, recent: &[Entry<'_>]) -> Markup {
    let recent = &recent[..recent.len().min(MISCELLANY_RECENT_LIMIT)];
    html! {
        div.miscellany-home {
            header.miscellany-header {
                div.miscellany-title { h1 { "Miscellany" } }
                @if miscellany.has_multilingual_entry() { p.miscellany-description { (MULTILINGUAL_NOTE) } }
            }
            (search_form(None))
            (miscellany_contents())
            @if !recent.is_empty() {
                section.miscellany-updates aria-labelledby="miscellany-updates-heading" {
                    h2 id="miscellany-updates-heading" { "Recent Updates" }
                    ol.recent-updates {
                        @for entry in recent {
                            li {
                                (entry_dates(entry))
                                p { (entry_text(entry)) }
                            }
                        }
                    }
                    p.miscellany-updates-all { a href="/updates/" { "All updates" } }
                }
            }
        }
    }
}

/// The four sections as rows; the whole row is the link.
fn miscellany_contents() -> Markup {
    html! {
        nav.miscellany-overview aria-label="Miscellany sections" {
            ul.miscellany-contents {
                @for (section, icon) in MISCELLANY_SECTIONS.iter().zip(MISCELLANY_ICONS) {
                    li {
                        // The whole row is the link: the icon and the chevron are decorative.
                        a href=(section.path) {
                            svg.miscellany-icon aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" {
                                (PreEscaped(icon))
                            }
                            span.miscellany-contents-label { (section.label) }
                            svg.miscellany-chevron aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" {
                                path d="m9.5 6 6 6-6 6" {}
                            }
                        }
                    }
                }
            }
        }
    }
}

/// A quiet search field that opens `/miscellany/search/`. From a section's
/// pages it searches that section first; the search page offers the others.
/// It is an ordinary form, so it works as a link to the search page even
/// before the search script has loaded.
fn search_form(section: Option<&MiscellanySection>) -> Markup {
    let (label, placeholder) = match section {
        Some(section) => (
            format!("Search {}", section.label),
            format!("Search {}", section.label),
        ),
        None => (
            "Search Miscellany".to_owned(),
            "Search Miscellany".to_owned(),
        ),
    };
    html! {
        form.miscellany-search action=(SEARCH_PATH) method="get" role="search" aria-label=(label) {
            label.sr-only for="miscellany-search-query" { (label) }
            input #miscellany-search-query type="search" name="q" placeholder=(placeholder) autocomplete="off" spellcheck="false" enterkeyhint="search";
            @if let Some(section) = section {
                input type="hidden" name="section" value=(section.key);
            }
            button type="submit" aria-label="Search" {
                (search_icon())
            }
        }
    }
}

fn search_icon() -> Markup {
    html! {
        svg.search-icon aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" {
            circle cx="10.5" cy="10.5" r="6" {}
            path d="M15 15l5 5" {}
        }
    }
}

/// The way back to the overview, then any section, as links separated
/// by slashes. On a section's own page it is followed by the section's name
/// as the page heading, so that the two read as one line.
fn miscellany_trail(section: Option<&MiscellanySection>) -> Markup {
    html! {
        nav.miscellany-trail aria-label="Breadcrumb" {
            a href="/miscellany/" { "Miscellany" }
            span.miscellany-trail-separator aria-hidden="true" { "/" }
            @if let Some(section) = section {
                a href=(section.path) { (section.label) }
                span.miscellany-trail-separator aria-hidden="true" { "/" }
            }
        }
    }
}

/// The four sections as one group, the current one marked, shown on the
/// section pages, where readers move between them.
fn miscellany_sections(current_path: &str) -> Markup {
    html! {
        nav.miscellany-sections aria-label="Miscellany sections" {
            @for section in &MISCELLANY_SECTIONS {
                (nav_link(section.path, section.label, current_path == section.path))
            }
        }
    }
}

/// A section page's top: `Miscellany / Reading` as its heading, with the
/// section's search beside it (below it on a phone, always there, entries or
/// none), then the sections.
fn miscellany_section_header(section: &MiscellanySection, languages: &[DisplayLanguage]) -> Markup {
    html! {
        header.miscellany-header {
            div.miscellany-heading {
                div.miscellany-title {
                    (miscellany_trail(None))
                    h1 { (section.label) }
                }
                (search_form(Some(section)))
            }
            (miscellany_sections(section.path))
            (language_filter(languages))
        }
    }
}

/// The reader's choice among the languages of separately authored passages:
/// All, then each display language actually used, by its own name. It is
/// rendered only when there is a choice to make, and stays hidden until the
/// script enables it, so every passage is shown without JavaScript.
fn language_filter(languages: &[DisplayLanguage]) -> Markup {
    html! {
        @if languages.len() > 1 {
            div.language-filter role="group" aria-label="Language" hidden data-language-filter {
                button type="button" data-language="all" aria-pressed="true" { "All" }
                @for language in languages {
                    button type="button" data-language=(language.code()) lang=(language.code()) aria-pressed="false" {
                        (language.endonym())
                    }
                }
            }
        }
    }
}

/// The passages of one article, which the language filter shows or hides
/// together. When some offered language would hide every passage, a hidden
/// notice names the languages of the text and offers to show it; the script
/// reveals the notice only when the reader's choice does hide them all.
/// Mixed passages are never hidden, so articles with one need no notice.
fn passage_group(
    designated: &[DisplayLanguage],
    has_mixed: bool,
    offered: &[DisplayLanguage],
    passages: Markup,
) -> Markup {
    let can_be_emptied = offered.len() > 1
        && !has_mixed
        && offered
            .iter()
            .any(|language| !designated.contains(language));
    html! {
        div.passages data-passages {
            (passages)
            @if can_be_emptied { (language_notice(designated)) }
        }
    }
}

/// The notice is the site's own wording, not the author's text, so the
/// search index leaves it out.
fn language_notice(languages: &[DisplayLanguage]) -> Markup {
    let names: Vec<_> = languages.iter().map(|l| l.english_name()).collect();
    let listed = match names.as_slice() {
        [only] => (*only).to_owned(),
        [first, second] => format!("{first} and {second}"),
        [init @ .., last] => format!("{}, and {last}", init.join(", ")),
        [] => String::new(),
    };
    let action = match languages {
        [only] => format!("Read in {}", only.english_name()),
        _ => "Show the text".to_owned(),
    };
    html! {
        p.language-notice data-language-notice data-pagefind-ignore="all" hidden {
            "Written in " (listed) ". "
            button type="button" data-language-reveal { (action) }
        }
    }
}

/// One separately authored passage of a note, sketch, or diary entry. A
/// designated passage carries its language; a mixed passage carries none of
/// its own, so phrases inside it keep whatever languages are marked inline.
/// A passage title, when given, is a heading at `title_level`.
fn markdown_passage(passage: &Passage, title_level: u8) -> Markup {
    let designated = passage.language().designated();
    let class = if designated == Some(DisplayLanguage::Ja) {
        "passage longform-ja"
    } else {
        "passage"
    };
    html! {
        div class=(class) data-passage=(passage.language().code()) lang=[designated.map(DisplayLanguage::code)] {
            @if let Some(title) = passage.title() {
                @match title_level {
                    2 => h2.passage-title { (title) },
                    3 => h3.passage-title { (title) },
                    _ => h4.passage-title { (title) },
                }
            }
            div.prose.miscellany-prose { (PreEscaped(passage.rendered_body())) }
        }
    }
}

fn article_passages(passages: &Passages, title_level: u8, offered: &[DisplayLanguage]) -> Markup {
    passage_group(
        &passages.designated_languages(),
        passages.has_mixed(),
        offered,
        html! { @for passage in passages.iter() { (markdown_passage(passage, title_level)) } },
    )
}

/// A stable fragment identifier for each reading item: its `id` when it has
/// one, and otherwise a hash of its title and author, so it changes only if
/// they do. Items with the same title and author are told apart by their
/// order.
fn reading_anchors(items: &[ValidatedReadingItem]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    items
        .iter()
        .map(|item| {
            let work = item.work();
            // An item with an id is named by it, so its anchor survives a
            // corrected title; its entry in Updates links here.
            if let Some(id) = &work.id {
                return reading_anchor(id);
            }
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
            let key = format!(
                "{}\u{1f}{}",
                work.title,
                work.author.as_deref().unwrap_or_default()
            );
            for byte in key.bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
            let base = format!("reading-{:08x}", hash >> 32);
            let repeats = seen.iter().filter(|anchor| **anchor == base).count();
            seen.push(base.clone());
            if repeats == 0 {
                base
            } else {
                format!("{base}-{}", repeats + 1)
            }
        })
        .collect()
}

/// The reading record in the author's order. Titles and names are shown as
/// written, marked with their language when it is not English. The language
/// filter applies to the author's commentary, never to the record itself.
/// Each work has an anchor, so a search can lead to it.
fn reading_page(miscellany: &Miscellany) -> Markup {
    let languages = miscellany.display_languages();
    let items = miscellany.reading();
    let anchors = reading_anchors(items);
    html! {
        (miscellany_section_header(READING, &languages))
        // Nothing published: the one quiet line every category says alike.
        @if items.is_empty() {
            (no_entries())
        } @else {
            ol.reading-list data-pagefind-body data-pagefind-filter=(search_filter(READING)) data-pagefind-meta="title:Reading" {
                @for (item, anchor) in items.iter().zip(&anchors) {
                    @let work = item.work();
                    @let lang = work.lang.as_ref().map(LanguageTag::as_str);
                    li data-reading-article=(reading_key(READING, anchor.trim_start_matches("reading-"))) data-reading-seen {
                        h2.reading-title id=(anchor) data-reading-target {
                            cite lang=[lang] {
                                @if let Some(url) = &work.url {
                                    a href=(url) { (&work.title) }
                                } @else {
                                    (&work.title)
                                }
                            }
                        }
                        @if let Some(author) = &work.author {
                            p.reading-author lang=[lang] { (author) }
                        }
                        @if !item.comments().is_empty() {
                            (reading_comments(item.comments(), &languages))
                        }
                    }
                }
            }
        }
    }
}

fn reading_comments(comments: &[ReadingComment], offered: &[DisplayLanguage]) -> Markup {
    let designated: Vec<_> = comments
        .iter()
        .filter_map(|comment| comment.language.designated())
        .collect();
    let has_mixed = comments
        .iter()
        .any(|comment| comment.language == PassageLanguage::Mixed);
    passage_group(
        &designated,
        has_mixed,
        offered,
        html! {
            @for comment in comments {
                p.reading-note data-passage=(comment.language.code())
                    lang=[comment.language.designated().map(DisplayLanguage::code)] {
                    (&comment.text)
                }
            }
        },
    )
}

/// Each logical note once, under its one title, whatever languages it is
/// written in.
fn notes_page(miscellany: &Miscellany) -> Markup {
    let notes: Vec<_> = miscellany.public_notes().collect();
    html! {
        (miscellany_section_header(NOTES, &[]))
        // Nothing published: the one quiet line every category says alike.
        @if notes.is_empty() {
            (no_entries())
        } @else {
            ol.miscellany-list data-reading-list {
                @for note in notes {
                    @let metadata = note.metadata();
                    li lang=[metadata.title_language()] {
                        h2 { a href=(note_path(&metadata.slug)) { (&metadata.title) } }
                        @if let Some(description) = &metadata.description {
                            p { (description) }
                        }
                    }
                }
            }
        }
    }
}

fn note_detail(note: &ValidatedNote, languages: &[DisplayLanguage]) -> Markup {
    let metadata = note.metadata();
    standalone_article(
        languages,
        NOTES,
        &metadata.slug,
        html! {
            header {
                h1 lang=[metadata.title_language()] data-pagefind-meta="title" { (&metadata.title) }
                @if let Some(description) = &metadata.description {
                    p.summary lang=[metadata.title_language()] { (description) }
                }
            }
            (article_passages(note.passages(), 2, languages))
        },
    )
}

/// The page of one note or sketch: the way back to its section and the
/// language choice, then the article, which alone is read by the search
/// index. The sections are not repeated above the text.
fn standalone_article(
    languages: &[DisplayLanguage],
    section: &MiscellanySection,
    slug: &str,
    article: Markup,
) -> Markup {
    html! {
        div.miscellany-header.miscellany-article-header {
            (miscellany_trail(Some(section)))
            (language_filter(languages))
        }
        article.miscellany-note data-pagefind-body data-pagefind-filter=(search_filter(section)) data-reading-article=(reading_key(section, slug)) {
            (article)
        }
        (reading_control())
    }
}

/// The key under which a visitor's own browser remembers having opened or
/// read an article: its section and its stable identity, the same identity
/// its address and its entry in Updates are made from. The script derives
/// the same key from any link to the article (client/site.ts).
fn reading_key(section: &MiscellanySection, identity: &str) -> String {
    format!("{}/{identity}", section.key)
}

/// The visitor's own "Mark as read" choice for an article, kept in their
/// browser only. It is hidden until the script finds that it can remember
/// the choice, so without JavaScript or storage there is nothing to press.
/// The search index never reads it.
fn reading_control() -> Markup {
    html! {
        p.reading-control data-reading-control data-pagefind-ignore="all" hidden {
            button type="button" aria-pressed="false" { "Mark as read" }
        }
    }
}

/// Sketches newest first, each with its title, date, and any description.
fn sketches_page(miscellany: &Miscellany) -> Markup {
    let sketches: Vec<_> = miscellany.public_sketches().collect();
    html! {
        (miscellany_section_header(SKETCHES, &[]))
        // Nothing published: the one quiet line every category says alike.
        @if sketches.is_empty() {
            (no_entries())
        } @else {
            ol.miscellany-list.sketch-list data-reading-list {
                @for sketch in sketches {
                    @let metadata = sketch.metadata();
                    li lang=[metadata.title_language()] {
                        h2 { a href=(sketch_path(&metadata.slug)) { (&metadata.title) } }
                        p.miscellany-date lang="en" {
                            time datetime=(metadata.date.format("%Y-%m-%d")) { (metadata.date.format("%-d %B %Y")) }
                        }
                        @if let Some(description) = &metadata.description {
                            p { (description) }
                        }
                    }
                }
            }
        }
    }
}

fn sketch_detail(sketch: &ValidatedSketch, languages: &[DisplayLanguage]) -> Markup {
    let metadata = sketch.metadata();
    standalone_article(
        languages,
        SKETCHES,
        &metadata.slug,
        html! {
            header {
                h1 lang=[metadata.title_language()] data-pagefind-meta="title" { (&metadata.title) }
                // The date is in the index's metadata, not in its text.
                p.article-date lang="en" data-pagefind-ignore {
                    time datetime=(metadata.date.format("%Y-%m-%d")) data-pagefind-meta="date[datetime]" { (metadata.date.format("%-d %B %Y")) }
                }
                @if let Some(description) = &metadata.description {
                    p.summary lang=[metadata.title_language()] { (description) }
                }
            }
            (article_passages(sketch.passages(), 2, languages))
        },
    )
}

/// The published diary entries of one calendar month, oldest first (entries
/// of one day in the order of their anchors).
struct DiaryMonth<'a> {
    first_day: NaiveDate,
    entries: Vec<&'a ValidatedDiaryEntry>,
}

impl DiaryMonth<'_> {
    /// The reading keys of the month's entries, separated by spaces.
    fn reading_keys(&self) -> String {
        self.entries
            .iter()
            .map(|entry| reading_key(DIARY, &entry.metadata().id()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn path(&self) -> String {
        format!("/miscellany/diary/{}/", self.first_day.format("%Y/%m"))
    }

    fn output_path(&self) -> String {
        format!(
            "miscellany/diary/{}/index.html",
            self.first_day.format("%Y/%m")
        )
    }

    fn name(&self) -> String {
        self.first_day.format("%B %Y").to_string()
    }
}

/// The months that have published entries, newest first. A month without
/// one has no page and is not listed.
fn diary_months(miscellany: &Miscellany) -> Vec<DiaryMonth<'_>> {
    let mut months: Vec<DiaryMonth<'_>> = Vec::new();
    for entry in miscellany.public_diary() {
        let date = entry.metadata().date;
        let first_day = date.with_day(1).expect("every month has a first day");
        match months.iter_mut().find(|month| month.first_day == first_day) {
            Some(month) => month.entries.push(entry),
            None => months.push(DiaryMonth {
                first_day,
                entries: vec![entry],
            }),
        }
    }
    for month in &mut months {
        month.entries.sort_by(|left, right| {
            let key =
                |entry: &ValidatedDiaryEntry| (entry.metadata().date, entry.metadata().anchor());
            key(left).cmp(&key(right))
        });
    }
    months.sort_by(|left, right| right.first_day.cmp(&left.first_day));
    months
}

/// The Diary's front page: the months that have entries, by year, newest
/// first. Each month is its own page.
fn diary_page(miscellany: &Miscellany) -> Markup {
    let months = diary_months(miscellany);
    let mut years: Vec<(i32, Vec<&DiaryMonth<'_>>)> = Vec::new();
    for month in &months {
        let year = month.first_day.year();
        match years.last_mut() {
            Some((last, list)) if *last == year => list.push(month),
            _ => years.push((year, vec![month])),
        }
    }
    html! {
        (miscellany_section_header(DIARY, &[]))
        // Nothing published: the one quiet line every category says alike.
        @if months.is_empty() {
            (no_entries())
        } @else {
            div.diary-archive data-diary-archive {
                @for (year, list) in &years {
                    section.diary-year aria-labelledby=(format!("diary-year-{year}")) {
                        h2 id=(format!("diary-year-{year}")) { (year) }
                        ol.diary-months {
                            @for month in list {
                                li {
                                    a href=(month.path()) aria-label=(month.name()) data-reading-entries=(month.reading_keys()) {
                                        (month.first_day.format("%B"))
                                    }
                                    " "
                                    span.diary-month-count aria-hidden="true" {
                                        (month.entries.len())
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The display languages of one month's passages, in display order: the
/// language choice is offered on a month's page only when that month has a
/// choice to make.
fn month_languages(month: &DiaryMonth<'_>) -> Vec<DisplayLanguage> {
    let mut languages: Vec<_> = month
        .entries
        .iter()
        .flat_map(|entry| entry.passages().designated_languages())
        .collect();
    languages.sort();
    languages.dedup();
    languages
}

/// One month of the Diary: its entries in full, oldest first, under a
/// heading and the way to the months before and after it.
fn diary_month_page(
    month: &DiaryMonth<'_>,
    previous: Option<&DiaryMonth<'_>>,
    next: Option<&DiaryMonth<'_>>,
    languages: &[DisplayLanguage],
) -> Markup {
    let name = month.name();
    html! {
        header.miscellany-header.diary-month-header {
            (miscellany_trail(Some(DIARY)))
            h1 { time datetime=(month.first_day.format("%Y-%m")) { (name) } }
            @if previous.is_some() || next.is_some() {
                nav.diary-month-nav aria-label="Months" {
                    @if let Some(previous) = previous {
                        a.diary-previous href=(previous.path()) rel="prev" {
                            span aria-hidden="true" { "← " } (previous.name())
                        }
                    }
                    @if let Some(next) = next {
                        a.diary-next href=(next.path()) rel="next" {
                            (next.name()) span aria-hidden="true" { " →" }
                        }
                    }
                }
            }
            @if month.entries.len() > 1 {
                nav.diary-days aria-label=(format!("Days in {name}")) {
                    @for (index, entry) in month.entries.iter().enumerate() {
                        @let date = entry.metadata().date;
                        @if index == 0 || month.entries[index - 1].metadata().date != date {
                            a href=(format!("#{}", entry.metadata().anchor())) aria-label=(date.format("%-d %B").to_string()) {
                                (date.format("%-d"))
                            }
                        }
                    }
                }
            }
            (language_filter(languages))
        }
        (search_form(Some(DIARY)))
        div.diary data-pagefind-body data-pagefind-filter=(search_filter(DIARY)) data-pagefind-meta=(format!("title:{name}")) {
            @for (index, entry) in month.entries.iter().enumerate() {
                @let same_day = index > 0 && month.entries[index - 1].metadata().date == entry.metadata().date;
                (diary_entry(entry, same_day, languages))
            }
        }
        @if previous.is_some() || next.is_some() {
            nav.diary-month-nav.diary-month-nav-end aria-label="Months, at the end of the page" {
                @if let Some(previous) = previous {
                    a.diary-previous href=(previous.path()) rel="prev" {
                        span aria-hidden="true" { "← " } (previous.name())
                    }
                }
                @if let Some(next) = next {
                    a.diary-next href=(next.path()) rel="next" {
                        (next.name()) span aria-hidden="true" { " →" }
                    }
                }
            }
        }
    }
}

/// One diary entry: the day of the month and its weekday beside the text.
/// The heading carries the entry's stable anchor; its accessible name is
/// the full date (and the title, when there is one). The search index takes
/// the date from the anchor, so the day and weekday are not indexed.
fn diary_entry(entry: &ValidatedDiaryEntry, same_day: bool, offered: &[DisplayLanguage]) -> Markup {
    let metadata = entry.metadata();
    let anchor = metadata.anchor();
    let date = metadata.date;
    let class = match (&metadata.title, same_day) {
        (Some(_), true) => "diary-entry has-title same-day",
        (Some(_), false) => "diary-entry has-title",
        (None, true) => "diary-entry same-day",
        (None, false) => "diary-entry",
    };
    html! {
        article class=(class) data-reading-article=(reading_key(DIARY, &metadata.id())) data-reading-seen {
            // The visitor's reading mark goes beside the title, or the day
            // when there is none (client/site.ts).
            h2.diary-heading id=(anchor) {
                a.diary-date href=(format!("#{anchor}")) aria-label=(date.format("%A %-d %B %Y").to_string()) data-pagefind-ignore="all" {
                    time datetime=(date.format("%Y-%m-%d")) {
                        span.diary-day data-reading-target=[metadata.title.is_none().then_some("")] { (date.format("%-d")) }
                        " "
                        span.diary-weekday { (date.format("%a")) }
                    }
                }
                @if let Some(title) = &metadata.title {
                    span.diary-title lang=[metadata.title_language()] data-reading-target { (title) }
                }
            }
            div.diary-body { (article_passages(entry.passages(), 3, offered)) }
            (reading_control())
        }
    }
}

/// The search page. The form works without the script (it reloads this
/// page with the query in its address); the results need the script, which
/// reads the static index and keeps the query and section in the address.
fn search_page() -> Markup {
    html! {
        header.miscellany-header {
            div.miscellany-heading {
                div.miscellany-title {
                    (miscellany_trail(None))
                    h1 { "Search" }
                }
            }
        }
        div.search-page data-search-page {
            form.search-form action=(SEARCH_PATH) method="get" role="search" aria-label="Search Miscellany" data-search-form {
                div.search-field {
                    label.sr-only for="search-query" { "Search Miscellany" }
                    input #search-query type="search" name="q" autocomplete="off" spellcheck="false" enterkeyhint="search" data-search-query;
                    button type="submit" aria-label="Search" { (search_icon()) }
                }
                fieldset.search-scope {
                    legend.sr-only { "Search in" }
                    label { input type="radio" name="section" value="" checked; span { "All" } }
                    @for section in &MISCELLANY_SECTIONS {
                        label { input type="radio" name="section" value=(section.key); span { (section.label) } }
                    }
                }
            }
            p.search-status role="status" data-search-status {}
            noscript { p.search-status { "Searching needs JavaScript. The sections can be read from the " a href="/miscellany/" { "overview" } "." } }
            ol.search-results data-search-results data-reading-list {}
            p.search-more { button type="button" data-search-more hidden { "More results" } }
        }
    }
}

/// What the site's analytics actually do: GoatCounter, loaded on every page
/// and counting only on the production hostname (see `analytics`).
fn privacy() -> Markup {
    html! {
        header.plain-page-header { h1 { "Privacy" } }
        div.privacy-page {
            p { "This site uses GoatCounter for aggregate visit statistics. GoatCounter does not use tracking cookies." }
            p {
                "For what it collects and how, see GoatCounter's "
                a href="https://www.goatcounter.com/help/privacy" { "privacy policy" }
                "."
            }
            h2 { "What you have read" }
            p { "In Miscellany, your browser remembers which notes, sketches, diary entries, and reading records you have opened, and which you have marked as read, so that lists can show them apart. This record is kept only in your browser’s storage on this device. It is never sent to this site, to GoatCounter, or to anyone else, and the site keeps no copy." }
            p { "It is not kept in private browsing, and it is lost when you clear this site’s data or use another browser or device. Safari may also remove it after about seven days of use without a visit to this site. Nothing else depends on it." }
        }
    }
}

fn not_found() -> Markup {
    html! {
        section.detail-sheet {
            p.longform-kicker { "404 / Not found" }
            h1 { "Nothing is filed here." }
            p.summary { "The address may have changed, or the page may not be public." }
            p { a href="/" { "Return home" } " " span aria-hidden="true" { "·" } " " a href="/research/" { "Browse research" } }
        }
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format("%d %b %Y").to_string()
}

fn format_date_range(start: NaiveDate, end: NaiveDate) -> String {
    if start.format("%Y-%m").to_string() == end.format("%Y-%m").to_string() {
        format!("{}–{}", start.format("%d"), format_date(end))
    } else if start.format("%Y").to_string() == end.format("%Y").to_string() {
        format!("{} – {}", start.format("%d %b"), format_date(end))
    } else {
        format!("{} – {}", format_date(start), format_date(end))
    }
}

pub fn rss(updates: &[Update]) -> String {
    let mut items = String::new();
    // Items carry no pubDate: records hold only event dates, which are not
    // publication dates, and a synthetic timestamp would misstate either.
    for update in updates {
        let link = if update.detail {
            format!("{SITE_ORIGIN}/updates/item/{}/", update.id)
        } else {
            // Each item needs a distinct link and guid; the anchor names the
            // update's entry on the full Updates page.
            format!("{SITE_ORIGIN}/updates/#update-{}", update.id)
        };
        let mut categories = String::new();
        for category in &update.categories {
            write!(categories, "<category>{}</category>", category.label())
                .expect("writing to a String cannot fail");
        }
        write!(
            items,
            "<item><title>{}</title><description>{}</description><link>{link}</link><guid>{link}</guid>{categories}</item>",
            escape_xml(&update.title.en),
            escape_xml(&summary_plain_text(&update.summary.en)),
        )
        .expect("writing to a String cannot fail");
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><rss version=\"2.0\"><channel><title>Ryujin Hatakeyama — Updates</title><description>Research, academic, and writing updates.</description><link>{SITE_ORIGIN}/</link><language>en</language>{items}</channel></rss>\n"
    )
}

pub fn sitemap(pages: &[GeneratedPage]) -> (String, String) {
    let mut urls = String::new();
    for page in pages.iter().filter(|page| page.include_in_sitemap) {
        write!(
            urls,
            "<url><loc>{SITE_ORIGIN}{}</loc></url>",
            page.public_path
        )
        .expect("writing to a String cannot fail");
    }
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{urls}</urlset>\n"
    );
    let index = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"><sitemap><loc>{SITE_ORIGIN}/sitemap-0.xml</loc></sitemap></sitemapindex>\n"
    );
    (document, index)
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The build day for tests that are not about it: before every planned
/// fixture event, so planned records are upcoming.
#[cfg(test)]
fn fixture_today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()
}

#[cfg(test)]
mod tests {
    use super::{
        HOME_RECENT_LIMIT, HOME_UPCOMING_LIMIT, classification, cv, escape_xml,
        filterable_publications, fixture_today, footer, format_date, format_date_range, header,
        publication_list, research, rss, update_detail, update_summary,
    };
    use crate::content::ValidatedHome;
    use crate::model::{Category, EventStatus, Publication, Update};
    use crate::timeline::{Entry, Timeline};
    use chrono::NaiveDate;
    use maud::Markup;

    // These tests are about update records alone: the views below are the
    // site's own, drawn from a timeline with no published articles (the
    // timeline's tests are in `timeline_tests`).

    fn home(home: &ValidatedHome, updates: &[Update], today: NaiveDate) -> Markup {
        super::home(home, &Timeline::on(updates, &[], today, None), today)
    }

    /// An Updates page of exactly these records; `active` only marks the
    /// filter, as the records are already the category's.
    fn updates_page_refs(
        updates: &[&Update],
        active: Option<Category>,
        today: NaiveDate,
    ) -> Markup {
        super::updates_page(
            &Timeline::on(updates.iter().copied(), &[], today, None),
            active,
            today,
        )
    }

    fn update_list(updates: &[&Update], compact: bool) -> Markup {
        let entries: Vec<_> = updates
            .iter()
            .map(|update| Entry::Activity(update))
            .collect();
        super::update_list(&entries, compact)
    }

    /// A small biography for tests that are not about its wording.
    fn fixture_home() -> ValidatedHome {
        ValidatedHome::from_json(r#"{"bio":["Fixture biography."]}"#).unwrap()
    }

    fn bio_section(html: &str) -> &str {
        let start = html.find(r#"<div class="home-bio-copy">"#).unwrap();
        &html[start..start + html[start..].find("</div>").unwrap()]
    }

    #[test]
    fn biography_paragraphs_render_in_order_with_escaped_text_and_links() {
        let home_content = ValidatedHome::from_json(
            r#"{"bio":[
                "First, with an [external link](https://example.org/a?b=1&c=2).",
                "I keep a [diary](/miscellany/diary/) and enjoy noticing birds and “bugs” on my walks. :-)",
                "Escaped <b>markup</b> & a literal \\[bracket\\]."
            ]}"#,
        )
        .unwrap();
        let html = home(&home_content, &[], fixture_today()).into_string();
        assert_eq!(
            bio_section(&html),
            concat!(
                r#"<div class="home-bio-copy">"#,
                r#"<p class="home-bio">First, with an <a href="https://example.org/a?b=1&amp;c=2">external link</a>.</p>"#,
                r#"<p class="home-bio">I keep a <a href="/miscellany/diary/">diary</a> and enjoy noticing birds and “bugs” on my walks. :-)</p>"#,
                r#"<p class="home-bio">Escaped &lt;b&gt;markup&lt;/b&gt; &amp; a literal [bracket].</p>"#,
            )
        );
        // The biography sits between the name and the contact links.
        assert!(html.find("</h1>").unwrap() < html.find("home-bio-copy").unwrap());
        assert!(html.find("home-bio-copy").unwrap() < html.find("home-links").unwrap());
    }

    /// The real biography renders one paragraph per JSON string, in order,
    /// with every authored link. Only its structure is checked, so editing
    /// the wording in content/home.json needs no change here.
    #[test]
    fn the_authored_biography_renders_one_paragraph_per_entry() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/home.json");
        let source = std::fs::read_to_string(path).unwrap();
        let raw: serde_json::Value = serde_json::from_str(&source).unwrap();
        let home_content = ValidatedHome::from_json(&source).unwrap();
        assert_eq!(
            home_content.bio().len(),
            raw["bio"].as_array().unwrap().len()
        );
        let html = home(&home_content, &[], fixture_today()).into_string();
        let section = bio_section(&html);
        assert_eq!(
            section.matches(r#"<p class="home-bio">"#).count(),
            home_content.bio().len()
        );
        let mut position = 0;
        for paragraph in home_content.bio() {
            for inline in paragraph.inlines() {
                let expected = match inline {
                    crate::model::Inline::Text(text) => maud::html! { (text) }.into_string(),
                    crate::model::Inline::Link { text, target } => {
                        maud::html! { a href=(target.as_str()) { (text) } }.into_string()
                    }
                };
                let found = section[position..]
                    .find(&expected)
                    .unwrap_or_else(|| panic!("missing or out of order: {expected}"));
                position += found + expected.len();
            }
        }
    }

    fn publication(slug: &str, year: i32, language: &str, kind: &str) -> Publication {
        serde_json::from_str(&format!(
            r#"{{"slug":"{slug}","title":"{slug}","authors":["A"],"year":{year},"language":"{language}"{kind}}}"#
        ))
        .unwrap()
    }

    const PAPER: &str = r#","type":"workshop-paper","status":"published""#;
    const POSTER: &str =
        r#","type":"poster","status":"presented","presentation":{"format":"poster"}"#;
    const PROCEEDINGS_TALK: &str = r#","type":"presentation","status":"presented","presentation":{"format":"oral","proceedingsReview":"unrefereed"}"#;

    #[test]
    fn publications_form_one_newest_first_list_with_a_language_filter() {
        let records = [
            publication("jssst", 2026, "ja", PROCEEDINGS_TALK),
            publication("semeval", 2023, "en", PAPER),
        ];
        let refs: Vec<_> = records.iter().collect();
        let html = filterable_publications(&refs).into_string();
        assert!(!html.contains("In English") && !html.contains("In Japanese"));
        assert_eq!(html.matches("<ol").count(), 1);
        let jssst = html.find(r#"<li data-language="ja"><article><p class="publication-index">2026</p><div><h3 lang="ja">jssst</h3>"#).unwrap();
        let semeval = html.find(r#"<li data-language="en"><article><p class="publication-index">2023</p><div><h3 lang="en">semeval</h3>"#).unwrap();
        assert!(jssst < semeval);
        // Hidden until the script enables it; "All" is the default selection.
        assert!(
            html.contains(r#"<div class="publication-filter" hidden data-publication-filter>"#)
        );
        assert!(html.contains(r#"<label for="publication-language">Language</label>"#));
        assert!(html.contains(r#"<option value="all" selected>All</option>"#));
        assert!(html.contains(r#"aria-controls="publication-list""#));
        assert!(html.contains(r#"<ol class="publication-list" id="publication-list">"#));
    }

    #[test]
    fn talks_are_not_filterable() {
        let records = [publication("ppl", 2025, "ja", POSTER)];
        let refs: Vec<_> = records.iter().collect();
        let html = publication_list(&refs, true).into_string();
        assert!(!html.contains("data-language") && !html.contains("publication-filter"));
    }

    #[test]
    fn proceedings_talks_are_unrefereed_papers_among_publications() {
        let record = publication("jssst", 2026, "ja", PROCEEDINGS_TALK);
        assert_eq!(
            classification(&record, false),
            "Unrefereed proceedings paper"
        );
        assert_eq!(
            classification(&record, true),
            "Oral presentation · Unrefereed proceedings paper"
        );
    }

    #[test]
    fn cv_lists_the_corrected_award_separately_from_publications() {
        let html = cv(&[], &[]).into_string();
        assert!(html.contains("<strong>Best Award</strong>"));
        assert!(!html.contains("Excellence Award"));
        assert!(
            html.find("Research experience").unwrap() < html.find("Honors and awards").unwrap()
        );
    }

    #[test]
    fn update_summary_links_are_escaped_anchors() {
        let html =
            update_summary("A <b> [Ai2 & co](https://allenai.org/about?a=1&b=2).").into_string();
        assert_eq!(
            html,
            r#"A &lt;b&gt; <a href="https://allenai.org/about?a=1&amp;b=2">Ai2 &amp; co</a>."#
        );
    }

    #[test]
    fn cv_places_teaching_experience_between_research_experience_and_publications() {
        let html = cv(&[], &[]).into_string();
        let teaching = html.find("Teaching experience").unwrap();
        assert!(html.find("Research experience").unwrap() < teaching);
        assert!(teaching < html.find("id=\"publications-heading\"").unwrap());
        assert!(html.contains(r#"<span lang="ja">プログラミング演習B</span> (Programming B; F#)"#));
    }

    #[test]
    fn theme_toggle_has_a_stable_name_and_decorative_icons() {
        let html = header("/").into_string();
        assert!(html.contains(r#"aria-label="Dark mode" aria-pressed="false""#));
        assert_eq!(html.matches(r#"aria-hidden="true""#).count(), 3);
        assert!(html.find("theme-icon-sun").unwrap() < html.find("theme-icon-moon").unwrap());
    }

    #[test]
    fn rss_items_without_detail_pages_have_distinct_guids() {
        let updates: Vec<crate::model::Update> = ["first", "second"]
            .iter()
            .map(|id| {
                serde_json::from_str(&format!(
                    r#"{{"id":"{id}","title":{{"en":"x"}},"summary":{{"en":"x"}},"date":"2026-10-09",
                    "eventStatus":"completed","categories":["academia"],
                    "kind":{{"type":"presentation"}},"related":{{}}}}"#
                ))
                .unwrap()
            })
            .collect();
        let feed = rss(&updates);
        assert!(
            feed.contains("<guid>https://ryujin-hatakeyama.github.io/updates/#update-first</guid>")
        );
        assert!(
            feed.contains(
                "<guid>https://ryujin-hatakeyama.github.io/updates/#update-second</guid>"
            )
        );
        assert!(feed.contains("<category>Activities</category>"));
    }

    #[test]
    fn update_links_are_shown_in_the_full_list_and_detail_pages() {
        let update: crate::model::Update = serde_json::from_str(
            r#"{"id":"poster","title":{"en":"Poster"},"summary":{"en":"x"},"date":"2026-10-09",
            "eventStatus":"completed","categories":["academia"],
            "kind":{"type":"presentation"},"related":{},
            "links":[{"label":"Program","url":"https://example.org/program.pdf","type":"pdf"}]}"#,
        )
        .unwrap();
        let link = r#"<a href="https://example.org/program.pdf">Program"#;
        assert!(update_list(&[&update], false).into_string().contains(link));
        assert!(!update_list(&[&update], true).into_string().contains(link));
        let detail = update_detail(&update).into_string();
        assert!(detail.contains(link));
        assert!(detail.contains(r#"<article class="detail-sheet update-detail">"#));
    }

    fn test_update(id: &str, date: &str, status: &str) -> crate::model::Update {
        test_update_in(id, date, "", status, "academia")
    }

    fn test_update_in(
        id: &str,
        date: &str,
        end: &str,
        status: &str,
        category: &str,
    ) -> crate::model::Update {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","title":{{"en":"title-{id}"}},"summary":{{"en":"x"}},"date":"{date}"{end},
            "eventStatus":"{status}","categories":["{category}"],
            "kind":{{"type":"participation"}},"related":{{}}}}"#
        ))
        .unwrap()
    }

    /// Records in the loader's most-recent-event-first order, with more
    /// planned records than the homepage shows and the planned ones first, so
    /// that limiting the combined list before separating it would leave no
    /// completed record in the preview.
    fn mixed_updates() -> Vec<crate::model::Update> {
        let mut updates: Vec<_> = (1..=4)
            .map(|n| {
                test_update(
                    &format!("planned-{n}"),
                    &format!("2026-11-{:02}", 10 - n),
                    "planned",
                )
            })
            .collect();
        updates.extend((1..=7).map(|n| {
            test_update(
                &format!("completed-{n}"),
                &format!("2026-09-{:02}", 20 - n),
                "completed",
            )
        }));
        updates
    }

    /// Byte offsets of a section's start and end in the rendered HTML.
    fn section_bounds(html: &str, heading_id: &str) -> (usize, usize) {
        let start = html
            .find(format!(r#"aria-labelledby="{heading_id}""#).as_str())
            .unwrap();
        let end = start + html[start..].find("</section>").unwrap();
        (start, end)
    }

    #[test]
    fn homepage_previews_upcoming_and_recent_updates_separately() {
        let updates = mixed_updates();
        let html = home(&fixture_home(), &updates, fixture_today()).into_string();

        assert!(html.contains(r#"<h2 id="upcoming-heading">Upcoming</h2>"#));
        assert!(html.contains(r#"<h2 id="updates-heading">Recent Updates</h2>"#));
        assert!(!html.contains(r#"<h2 class="sr-only""#));
        let upcoming = &html[{
            let (start, end) = section_bounds(&html, "upcoming-heading");
            start..end
        }];
        let recent = &html[{
            let (start, end) = section_bounds(&html, "updates-heading");
            start..end
        }];

        // Each section has its own limit, applied after the split; both are
        // ordered by event date.
        assert_eq!(upcoming.matches("<li>").count(), 3);
        assert_eq!(recent.matches("<li>").count(), 5);
        let at = |section: &str, id: &str| section.find(format!(">title-{id}<").as_str());
        assert!(at(upcoming, "planned-4").unwrap() < at(upcoming, "planned-3").unwrap());
        assert!(at(upcoming, "planned-3").unwrap() < at(upcoming, "planned-2").unwrap());
        assert!(at(upcoming, "planned-1").is_none());
        assert!(at(recent, "completed-1").unwrap() < at(recent, "completed-5").unwrap());
        assert!(at(recent, "completed-6").is_none());
        assert!(!upcoming.contains("completed-") && !recent.contains("planned-"));
        // Planned events lead with their event date.
        assert!(upcoming.contains(r#"<time datetime="2026-11-06">06 Nov 2026</time>"#));
        assert!(!html.contains("Announced"));

        // No standalone Writings entry point on the homepage.
        assert!(!html.contains(r#"href="/writings/""#) && !html.contains("Writings"));
    }

    #[test]
    fn homepage_keeps_recent_updates_without_upcoming_events() {
        let updates: Vec<_> = mixed_updates()
            .into_iter()
            .filter(|update| update.id.starts_with("completed-"))
            .collect();
        let html = home(&fixture_home(), &updates, fixture_today()).into_string();
        assert!(!html.contains("Upcoming"));
        assert!(html.contains(r#"<h2 id="updates-heading">Recent Updates</h2>"#));
        assert!(html.contains(r#"<a href="/updates/">All updates</a>"#));
        let (start, end) = section_bounds(&html, "updates-heading");
        assert_eq!(html[start..end].matches("<li>").count(), 5);
    }

    #[test]
    fn updates_page_shows_upcoming_and_recent_updates_in_full() {
        let updates = mixed_updates();
        let refs: Vec<_> = updates.iter().collect();
        let html = updates_page_refs(&refs, None, fixture_today()).into_string();

        assert!(html.contains(r#"<h2 id="upcoming-heading">Upcoming</h2>"#));
        assert!(html.contains(r#"<h2 id="recent-heading">Recent Updates</h2>"#));
        assert!(!html.contains(r#"<h2 class="sr-only""#) && !html.contains("Other updates"));
        let (upcoming_start, upcoming_end) = section_bounds(&html, "upcoming-heading");
        let (recent_start, recent_end) = section_bounds(&html, "recent-heading");
        assert!(upcoming_end < recent_start);
        for update in &updates {
            let at = html
                .find(format!(r#"id="update-{}""#, update.id).as_str())
                .unwrap();
            if update.event_status == EventStatus::Planned {
                assert!(upcoming_start < at && at < upcoming_end, "{}", update.id);
            } else {
                assert!(recent_start < at && at < recent_end, "{}", update.id);
            }
        }
        // Upcoming is soonest event first.
        assert!(
            html.find(r#"id="update-planned-4""#).unwrap()
                < html.find(r#"id="update-planned-1""#).unwrap()
        );
        assert!(!html.contains("Announced"));
        assert!(!html.contains("Completed") && !html.contains("· Planned"));

        let completed: Vec<_> = refs[4..].to_vec();
        let without_plans = updates_page_refs(&completed, None, fixture_today()).into_string();
        assert!(!without_plans.contains("Upcoming"));
        assert!(without_plans.contains(r#"<h2 id="recent-heading">Recent Updates</h2>"#));
        assert_eq!(without_plans.matches(r#"<article id="update-"#).count(), 7);

        let without_records = updates_page_refs(&[], None, fixture_today()).into_string();
        assert!(without_records.contains("No updates yet.") && !without_records.contains("<h2"));
    }

    #[test]
    fn category_pages_keep_both_sections_and_the_event_date_range() {
        let ranged = test_update_in(
            "meeting",
            "2026-10-16",
            r#","endDate":"2026-10-18""#,
            "planned",
            "research",
        );
        let talk = test_update_in("talk", "2026-09-10", "", "completed", "research");
        let html = updates_page_refs(&[&ranged, &talk], Some(Category::Research), fixture_today())
            .into_string();
        assert!(html.contains("<h1>Research</h1>"));
        assert!(html.contains(r#"<a href="/updates/research/" aria-current="page">Research</a>"#));
        assert!(html.contains(r#"<h2 id="upcoming-heading">Upcoming</h2>"#));
        assert!(html.contains(r#"<h2 id="recent-heading">Recent Updates</h2>"#));
        assert!(html.contains(
            r#"<div class="update-meta"><time datetime="2026-10-16">16–18 Oct 2026</time>"#
        ));

        // A filter that leaves only planned records shows no empty Recent Updates.
        let only_planned =
            updates_page_refs(&[&ranged], Some(Category::Research), fixture_today()).into_string();
        assert!(
            !only_planned.contains("Recent Updates") && !only_planned.contains("No updates yet.")
        );
    }

    /// Completed records in an order unrelated to when the activities took
    /// place.
    fn completed_out_of_order() -> Vec<crate::model::Update> {
        vec![
            test_update_in("talk", "2026-09-07", "", "completed", "research"),
            test_update_in(
                "visit",
                "2026-01-10",
                r#","endDate":"2026-01-18""#,
                "completed",
                "academia",
            ),
            test_update_in(
                "camp",
                "2026-09-01",
                r#","endDate":"2026-09-07""#,
                "completed",
                "research",
            ),
            test_update_in("poster", "2026-10-09", "", "completed", "academia"),
            test_update_in("seminar", "2026-03-24", "", "completed", "research"),
            test_update_in("alpha", "2026-09-07", "", "completed", "research"),
        ]
    }

    fn ids_in_order(html: &str) -> Vec<&str> {
        html.split(r#"<h3>title-"#)
            .skip(1)
            .map(|rest| &rest[..rest.find('<').unwrap()])
            .collect()
    }

    #[test]
    fn completed_updates_show_and_sort_by_event_date() {
        let updates = completed_out_of_order();
        let refs: Vec<_> = updates.iter().collect();
        // Newest final event day first; a range ending on the same day as a
        // one-day event follows it (earlier start), and remaining ties go by id.
        let expected = ["poster", "alpha", "talk", "camp", "seminar", "visit"];

        let full = updates_page_refs(&refs, None, fixture_today()).into_string();
        assert_eq!(ids_in_order(&full), expected);
        let home_html = home(&fixture_home(), &updates, fixture_today()).into_string();
        let (start, end) = section_bounds(&home_html, "updates-heading");
        assert_eq!(ids_in_order(&home_html[start..end]), expected[..5]);
        let research: Vec<_> = refs
            .iter()
            .copied()
            .filter(|update| update.categories.contains(&Category::Research))
            .collect();
        let category =
            updates_page_refs(&research, Some(Category::Research), fixture_today()).into_string();
        assert_eq!(
            ids_in_order(&category),
            ["alpha", "talk", "camp", "seminar"]
        );

        for html in [&full, &home_html, &category] {
            assert!(!html.contains("Announced") && !html.contains("Event: "));
        }
        assert!(full.contains(
            r#"<article id="update-talk"><div class="update-meta"><time datetime="2026-09-07">07 Sep 2026</time>"#
        ));
        // Ranges are rendered whole, in both full and compact lists.
        for html in [&full, &home_html] {
            assert!(html.contains(r#"<time datetime="2026-09-01">01–07 Sep 2026</time>"#));
        }
        assert!(full.contains(r#"<time datetime="2026-01-10">10–18 Jan 2026</time>"#));
        // Each entry shows its date once, in the metadata.
        assert_eq!(full.matches("<time ").count(), updates.len());
    }

    #[test]
    fn detail_pages_show_the_event_date() {
        let mut update = test_update_in(
            "meeting",
            "2026-10-16",
            r#","endDate":"2026-10-18""#,
            "planned",
            "research",
        );
        update.detail = true;
        let html = update_detail(&update).into_string();
        assert!(html.contains(
            r#"<dt>Event date</dt><dd><time datetime="2026-10-16">16–18 Oct 2026</time></dd>"#
        ));
        assert!(html.contains("<dt>Status</dt><dd>Planned</dd>"));
        assert!(!html.contains("Announced"));
        assert_eq!(html.matches("<time ").count(), 1);
    }

    #[test]
    fn rss_keeps_the_given_order_without_dates() {
        let updates = completed_out_of_order();
        let feed = rss(&updates);
        assert!(!feed.contains("pubDate") && !feed.contains("<time"));
        let positions: Vec<_> = updates
            .iter()
            .map(|update| feed.find(&format!("#update-{}</guid>", update.id)).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn rss_lists_planned_and_completed_updates() {
        let updates = mixed_updates();
        let feed = rss(&updates);
        assert_eq!(feed.matches("<item>").count(), updates.len());
    }

    fn update_source(file: &str) -> serde_json::Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../content/updates")
            .join(file);
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    /// The AIE and LLAL records: the activity phrase stays text, only the
    /// event's name links to its primary resource, the link appears once, and
    /// the event dates (not any publication date) are shown.
    #[test]
    fn aie_and_llal_titles_link_only_the_event_name_in_every_view() {
        const AIE_PDF: &str = "https://www.aie.tohoku.ac.jp/data/news/20261009_english.pdf";
        const LLAL_PAGE: &str = "https://sites.google.com/view/llal-at-gsis/meetings/llalgsis-13";
        let aie = update_from_source("2026-10-aie-english-training-poster.json");
        let llal = update_from_source("2026-03-llal-gsis-13.json");
        for (update, text, url) in [
            (
                &aie,
                "Effective Presentation & Communication in English",
                AIE_PDF,
            ),
            (&llal, "LLAL@GSIS (XIII)", LLAL_PAGE),
        ] {
            let link = update.title_link.as_ref().unwrap();
            assert_eq!((link.text.as_str(), link.url.as_str()), (text, url));
            assert!(update.links.is_empty(), "{}", update.id);
        }
        assert_eq!(
            aie.summary.en,
            "Presented a research poster on compositional hidden Markov models and staged inference at the AIE English Training Session."
        );
        assert_eq!(
            llal.summary.en,
            "Attended LLAL@GSIS (XIII), a workshop on nonclassical and philosophical logic at Tohoku University."
        );

        let aie_heading = linked_heading(&aie);
        let llal_heading = linked_heading(&llal);
        let titles = [escape_xml(&aie.title.en), escape_xml(&llal.title.en)];
        let updates = [aie, llal];
        let refs: Vec<_> = updates.iter().collect();
        let views = [
            (
                "home",
                home(&fixture_home(), &updates, fixture_today()).into_string(),
            ),
            (
                "updates",
                updates_page_refs(&refs, None, fixture_today()).into_string(),
            ),
            (
                "activities",
                updates_page_refs(&refs, Some(Category::Academia), fixture_today()).into_string(),
            ),
        ];
        for (view, html) in &views {
            for (heading, url, dates) in [
                (
                    &aie_heading,
                    AIE_PDF,
                    r#"<time datetime="2026-10-09">09 Oct 2026</time>"#,
                ),
                (
                    &llal_heading,
                    LLAL_PAGE,
                    r#"<time datetime="2026-03-23">23–24 Mar 2026</time>"#,
                ),
            ] {
                assert_eq!(html.matches(heading.as_str()).count(), 1, "{view}");
                assert_eq!(
                    html.matches(&format!(r#"href="{url}""#)).count(),
                    1,
                    "{view}"
                );
                assert_eq!(html.matches(dates).count(), 1, "{view}");
            }
            // The AIE record lists first: its event is the more recent.
            assert!(html.find(&aie_heading).unwrap() < html.find(&llal_heading).unwrap());
            assert!(!html.contains(">Program<"), "{view}");
            assert_no_nested_anchors(html);
        }
        let feed = rss(&updates);
        for title in titles {
            assert!(feed.contains(&format!("<item><title>{title}</title>")));
        }
        assert!(!feed.contains("<a ") && !feed.contains("](") && !feed.contains("aie.tohoku"));
    }

    #[test]
    fn aie_update_titles_are_english_only() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/updates");
        for file in [
            "2026-10-aie-english-training-poster.json",
            "2026-09-aie-pbl-symposium.json",
        ] {
            let record: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(root.join(file)).unwrap()).unwrap();
            let title = record["title"]["en"].as_str().unwrap();
            assert!(
                !title.contains('(') && title.is_ascii(),
                "{file} has a non-English title: {title}"
            );
        }
    }

    /// Every published record, as the OCaml generator exports it, in the
    /// loader's order.
    fn source_updates() -> Vec<crate::model::Update> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/updates");
        let mut files: Vec<_> = std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .filter(|name| name.ends_with(".json"))
            .collect();
        files.sort();
        let mut updates: Vec<_> = files
            .iter()
            .filter(|file| update_source(file)["status"] == "published")
            .map(|file| update_from_source(file))
            .collect();
        updates.sort_by(crate::model::Update::most_recent_first);
        updates
    }

    fn october(day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, day).unwrap()
    }

    /// The section of a full Updates page that holds a record's entry.
    fn section_of<'h>(html: &'h str, id: &str) -> &'h str {
        let at = html.find(&format!(r#"id="update-{id}""#)).unwrap();
        let start = html[..at].rfind("<section").unwrap();
        let heading = &html[start..at];
        if heading.contains("upcoming-heading") {
            "upcoming"
        } else if heading.contains("recent-heading") {
            "recent"
        } else if heading.contains("past-plans-heading") {
            "past-plans"
        } else {
            panic!("{id} is outside every section")
        }
    }

    /// A record's entry on a full Updates page, from its anchor to the end of
    /// its article.
    fn entry_of<'h>(html: &'h str, id: &str) -> &'h str {
        let entry = &html[html.find(&format!(r#"id="update-{id}""#)).unwrap()..];
        &entry[..entry.find("</article>").unwrap()]
    }

    /// A planned event (made up, 16–18 October) across its dates, listed
    /// beside every published record: upcoming, then ongoing, then a past
    /// plan awaiting review — never completed and never dropped, whatever the
    /// build day. A completed event with the same dates is listed as done.
    /// The published records' own statuses do not matter here.
    #[test]
    fn a_planned_record_moves_by_date_without_changing_status() {
        const PLANNED: &str = "phase-fixture-planned";
        const DONE: &str = "phase-fixture-completed";
        let dates = r#","endDate":"2026-10-18""#;
        let fixtures = vec![
            test_update_in(PLANNED, "2026-10-16", dates, "planned", "academia"),
            test_update_in(DONE, "2026-10-16", dates, "completed", "academia"),
        ];
        let mut updates = source_updates();
        updates.extend(fixtures.iter().cloned());
        updates.sort_by(crate::model::Update::most_recent_first);
        let refs: Vec<_> = updates.iter().collect();
        let page = |today| updates_page_refs(&refs, None, today).into_string();
        // The homepage is checked with the made-up records alone, so that
        // published records can never crowd them out of its limits.
        let homepage = |today| home(&fixture_home(), &fixtures, today).into_string();
        let ongoing = r#"<span class="phase-label">Ongoing</span>"#;
        let planned_title = format!("title-{PLANNED}");

        let before = page(october(15));
        assert_eq!(section_of(&before, PLANNED), "upcoming");
        assert!(!entry_of(&before, PLANNED).contains(ongoing));

        for day in [16, 17, 18] {
            let during = page(october(day));
            assert_eq!(section_of(&during, PLANNED), "upcoming", "{day}");
            assert!(entry_of(&during, PLANNED).contains(ongoing), "{day}");
            let home_html = homepage(october(day));
            assert!(home_html.contains(&planned_title), "{day}");
            assert!(home_html.contains(ongoing), "{day}");
        }

        for after in [october(19), NaiveDate::from_ymd_opt(2027, 4, 1).unwrap()] {
            let html = page(after);
            assert_eq!(section_of(&html, PLANNED), "past-plans");
            assert!(html.contains(r#"<h2 id="past-plans-heading">Previously announced</h2>"#));
            let entry = entry_of(&html, PLANNED);
            assert!(!entry.contains(ongoing) && !entry.contains("Needs review"));
            // The past plan is not presented as completed: it stays apart
            // from Recent Updates, with its original wording.
            let (recent_start, recent_end) = section_bounds(&html, "recent-heading");
            assert!(!html[recent_start..recent_end].contains(PLANNED));
            assert!(entry.contains(&planned_title));
            // A completed record is a past result, under Recent Updates.
            assert_eq!(section_of(&html, DONE), "recent");
            assert!(!entry_of(&html, DONE).contains(ongoing));
            // The homepage previews only upcoming and completed records.
            let home_html = homepage(after);
            assert!(!home_html.contains(&planned_title));
            assert!(!home_html.contains("Previously announced"));
            assert!(home_html.contains(&format!("title-{DONE}")));
        }

        // Rendering never changes a record's status.
        let planned = updates.iter().find(|update| update.id == PLANNED).unwrap();
        assert_eq!(planned.event_status, EventStatus::Planned);
    }

    /// Whatever the build day, every record keeps exactly one anchor on the
    /// full Updates page and on each of its category pages, the anchors that
    /// RSS guids name keep resolving, and the homepage previews never repeat a
    /// record or exceed their limits.
    #[test]
    fn date_phases_never_drop_or_duplicate_records() {
        let updates = source_updates();
        let refs: Vec<_> = updates.iter().collect();
        let feed = rss(&updates);
        let days = [
            october(1),
            october(15),
            october(16),
            october(18),
            october(19),
            NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
        ];
        for today in days {
            let html = updates_page_refs(&refs, None, today).into_string();
            assert_eq!(
                html.matches(r#"<article id="update-"#).count(),
                updates.len()
            );
            for update in &updates {
                let anchor = format!(r#"id="update-{}""#, update.id);
                assert_eq!(html.matches(&anchor).count(), 1, "{} on {today}", update.id);
                // Detail pages are linked rather than anchored; every other
                // guid names the record's anchor here.
                if !update.detail {
                    assert!(feed.contains(&format!(
                        "<guid>https://ryujin-hatakeyama.github.io/updates/#update-{}</guid>",
                        update.id
                    )));
                }
            }
            for category in Category::ALL {
                let members: Vec<_> = refs
                    .iter()
                    .copied()
                    .filter(|update| update.categories.contains(&category))
                    .collect();
                let page = updates_page_refs(&members, Some(category), today).into_string();
                assert_eq!(
                    page.matches(r#"<article id="update-"#).count(),
                    members.len()
                );
                assert!(members.is_empty() || !page.contains("No updates yet."));
            }

            let home_html = home(&fixture_home(), &updates, today).into_string();
            let preview = |heading| {
                let html = &home_html;
                html.find(&format!(r#"aria-labelledby="{heading}""#))
                    .map_or(String::new(), |start| {
                        html[start..start + html[start..].find("</section>").unwrap()].to_owned()
                    })
            };
            let (upcoming, recent) = (preview("upcoming-heading"), preview("updates-heading"));
            assert!(upcoming.matches("<li>").count() <= 3);
            assert!(recent.matches("<li>").count() <= 5);
            for update in &updates {
                let shown = [&upcoming, &recent]
                    .iter()
                    .filter(|section| section.contains(&update.title.en))
                    .count();
                assert!(shown <= 1, "{} twice on {today}", update.id);
                if update.event_status == EventStatus::Planned {
                    assert!(!recent.contains(&update.title.en));
                }
            }
        }
        // The feed does not depend on the build day at all.
        assert!(!feed.contains("Ongoing") && !feed.contains("Previously announced"));
    }

    #[test]
    fn past_plans_keep_category_pages_from_looking_empty() {
        let past = test_update_in("workshop", "2026-09-01", "", "planned", "research");
        let html = updates_page_refs(&[&past], Some(Category::Research), october(19)).into_string();
        assert!(html.contains(r#"<h2 id="past-plans-heading">Previously announced</h2>"#));
        assert!(!html.contains("No updates yet.") && !html.contains("Recent Updates"));
        assert!(!html.contains("Upcoming"));
        let empty = updates_page_refs(&[], Some(Category::Research), october(19)).into_string();
        assert!(empty.contains("No updates yet.") && !empty.contains("Previously announced"));
    }

    #[test]
    fn ongoing_and_upcoming_events_share_the_upcoming_section_soonest_first() {
        let ongoing = test_update_in(
            "meeting",
            "2026-10-16",
            r#","endDate":"2026-10-18""#,
            "planned",
            "academia",
        );
        let single = test_update("talk", "2026-10-17", "planned");
        let later = test_update("visit", "2026-11-02", "planned");
        let html = updates_page_refs(&[&later, &single, &ongoing], None, october(17)).into_string();
        let at = |id: &str| html.find(&format!(r#"id="update-{id}""#)).unwrap();
        assert!(at("meeting") < at("talk") && at("talk") < at("visit"));
        // Both events under way that day are marked; the later one is not.
        assert_eq!(html.matches(">Ongoing<").count(), 2);
        assert!(!html[at("visit")..].contains(">Ongoing<"));
    }

    #[test]
    fn formats_event_date_ranges() {
        let day = |month, day| NaiveDate::from_ymd_opt(2026, month, day).unwrap();
        assert_eq!(
            format_date_range(day(10, 16), day(10, 18)),
            "16–18 Oct 2026"
        );
        assert_eq!(
            format_date_range(day(9, 30), day(10, 2)),
            "30 Sep – 02 Oct 2026"
        );
    }

    #[test]
    fn formats_dates_like_the_previous_site() {
        assert_eq!(
            format_date(NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()),
            "07 Sep 2026"
        );
    }

    #[test]
    fn escapes_xml_text() {
        assert_eq!(escape_xml("A & <B>"), "A &amp; &lt;B&gt;");
    }

    /// A source record as the OCaml generator exports it to the Rust build.
    fn update_from_source(file: &str) -> crate::model::Update {
        let mut record = update_source(file);
        let fields = record.as_object_mut().unwrap();
        for (from, to) in [
            ("title_link", "titleLink"),
            ("end_date", "endDate"),
            ("event_status", "eventStatus"),
        ] {
            if let Some(value) = fields.remove(from) {
                fields.insert(to.to_owned(), value);
            }
        }
        fields.remove("status");
        let update: crate::model::Update = serde_json::from_value(record).unwrap();
        update.validate(file).unwrap();
        update
    }

    /// Records whose titles link the event's name, with each official page.
    const LINKED_EVENTS: [(&str, &str); 6] = [
        (
            "2026-10-aie-english-training-poster.json",
            "https://www.aie.tohoku.ac.jp/data/news/20261009_english.pdf",
        ),
        (
            "2026-03-llal-gsis-13.json",
            "https://sites.google.com/view/llal-at-gsis/meetings/llalgsis-13",
        ),
        (
            "2026-10-graham-priest-welcome.json",
            "https://sites.google.com/view/welcome-graham/",
        ),
        (
            "2026-10-wakate-no-kai.json",
            "https://sites.google.com/view/wakatenokai2026/",
        ),
        (
            "2026-09-ppl-summer-school.json",
            "https://jssst-ppl.org/wiki/ss2026",
        ),
        (
            "2026-09-jssst-2026-presentation.json",
            "https://jssst2026.wordpress.com/",
        ),
    ];

    /// The heading a record's title renders as: the event's name, and only
    /// that, linked to its page; the rest of the title plain, escaped text.
    /// Built from the record itself, so titles can be reworded freely.
    fn linked_heading(update: &crate::model::Update) -> String {
        let link = update.title_link.as_ref().unwrap();
        let (before, after) = link.split(&update.title.en).unwrap();
        let escape = |text: &str| maud::html! { (text) }.into_string();
        let lang = link
            .lang
            .map(|lang| format!(r#" lang="{}""#, lang.code()))
            .unwrap_or_default();
        format!(
            r#"<h3>{}<a href="{}"{lang}>{}</a>{}</h3>"#,
            escape(before),
            link.url,
            escape(&link.text),
            escape(after)
        )
    }

    /// Fails if any anchor opens before the previous one has closed.
    fn assert_no_nested_anchors(html: &str) {
        let mut open = false;
        let mut rest = html;
        while let Some(index) = rest.find("<a ").into_iter().chain(rest.find("</a>")).min() {
            if rest[index..].starts_with("<a ") {
                assert!(
                    !open,
                    "nested anchor near {}",
                    &rest[index..(index + 80).min(rest.len())]
                );
                open = true;
                rest = &rest[index + 3..];
            } else {
                open = false;
                rest = &rest[index + 4..];
            }
        }
    }

    #[test]
    fn every_page_loads_goatcounter_once_and_only_counts_in_production() {
        let content = crate::content::ValidatedSiteContent::with_miscellany(
            crate::content::Miscellany::default(),
        );
        let guard = "<script>if(location.hostname!=='ryujin-hatakeyama.github.io')window.goatcounter={no_onload:true};</script>";
        let tag = r#"<script data-goatcounter="https://hatakeyama.goatcounter.com/count" async src="https://gc.zgo.at/count.js"></script>"#;
        for page in super::pages(&content, fixture_today()) {
            let html = &page.html;
            assert_eq!(html.matches("gc.zgo.at").count(), 1, "{}", page.output_path);
            assert_eq!(html.matches("data-goatcounter=").count(), 1);
            assert_eq!(html.matches(guard).count(), 1);
            // The guard precedes the asynchronous script, inside <head>, and
            // after the theme script, which it leaves untouched.
            let site = html
                .find(r#"<script src="/assets/site.js"></script>"#)
                .unwrap();
            let guard_at = html.find(guard).unwrap();
            assert!(site < guard_at && guard_at < html.find(tag).unwrap());
            assert!(html.find(tag).unwrap() < html.find("</head>").unwrap());
            assert!(html.contains(&format!(
                r#"<link rel="canonical" href="https://ryujin-hatakeyama.github.io{}">"#,
                page.public_path
            )));
        }
        // Only the documented option: no events, identifiers, or counters.
        let html = super::layout(
            super::PageMetadata {
                title: "T",
                description: "D",
                current_path: "/",
                noindex: false,
                article: false,
            },
            maud::html! {},
        )
        .into_string();
        for absent in [
            "goatcounter.count(",
            "data-goatcounter-settings",
            "visit_count",
            "cookie=",
        ] {
            assert!(!html.contains(absent), "{absent}");
        }
    }

    #[test]
    fn homepage_heading_holds_both_forms_of_the_name() {
        let html = home(&fixture_home(), &[], fixture_today()).into_string();
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains(r#"<h1 class="person-name" id="home-name"><span class="name-latin" lang="en">Ryujin Hatakeyama</span> <span class="name-japanese" lang="ja">畠山竜迅</span></h1>"#));
        assert_eq!(html.matches("畠山竜迅").count(), 1);
        // The pronunciation follows the heading unchanged.
        assert!(html.contains(r#"</h1><p class="name-pronunciation"><span lang="ja">はたけやま りゅうじん</span><span aria-hidden="true"> · </span><span class="ipa">/hatakejama ɾʲɯːdʑiɴ/</span></p>"#));
        assert!(html.contains(r#"<section class="home-intro" aria-labelledby="home-name">"#));
    }

    #[test]
    fn only_explicit_categories_are_labelled_and_activities_has_no_filter() {
        // A title saying "Presented" does not make an entry research; only
        // its category does.
        let mut activity = test_update_in("activity", "2026-09-01", "", "completed", "academia");
        activity.title.en = "Presented at a symposium".to_owned();
        let research = test_update_in("talk", "2026-09-02", "", "completed", "research");
        let mut both = test_update_in("poster", "2026-09-03", "", "completed", "academia");
        both.categories.push(Category::Research);
        let refs = [&activity, &research, &both];
        let pages = [
            updates_page_refs(&refs, None, fixture_today()).into_string(),
            update_list(&refs, true).into_string(),
        ];
        for html in &pages {
            let entry = |title: &str| {
                let start = html.find(&format!(">{title}<")).unwrap();
                let meta = html[..start].rfind(r#"<div class="update-meta">"#).unwrap();
                html[meta..start].to_owned()
            };
            assert!(!entry(&activity.title.en).contains("category-label"));
            for labelled in [&research, &both] {
                assert!(
                    entry(&labelled.title.en)
                        .contains(r#"<span class="category-label">Research</span>"#)
                );
            }
            assert!(!html.contains("Activities") && !html.contains(r#"category-label"></span>"#));
        }
        let filters = &pages[0][..pages[0].find("</nav>").unwrap()];
        assert!(filters.contains(r#"href="/updates/research/""#));
        assert!(!filters.contains("/updates/academia/"));
        // The Activities page remains for existing links, under its title.
        let academia = updates_page_refs(
            &[&activity, &both],
            Some(Category::Academia),
            fixture_today(),
        )
        .into_string();
        assert!(academia.contains("<h1>Activities</h1>"));
        assert!(!academia.contains(r#"aria-current="page""#));
        // A detail page has no empty Categories row.
        let mut detailed = activity.clone();
        detailed.detail = true;
        assert!(
            !update_detail(&detailed)
                .into_string()
                .contains("Categories")
        );
    }

    #[test]
    fn footer_links_to_the_privacy_page_beside_the_name_before_the_date() {
        let html = footer().into_string();
        let privacy = html
            .find(r#"<p class="footer-links"><a href="/privacy/">Privacy</a></p>"#)
            .unwrap();
        assert!(html.find("© 2026 Ryujin Hatakeyama").unwrap() < privacy);
        assert!(privacy < html.find("Last updated <time").unwrap());
        assert!(!html.contains("goatcounter.com"));
    }

    #[test]
    fn privacy_page_is_a_short_ordinary_page_linking_goatcounter() {
        let content = crate::content::ValidatedSiteContent::with_miscellany(
            crate::content::Miscellany::default(),
        );
        let page = super::pages(&content, fixture_today())
            .into_iter()
            .find(|page| page.output_path == "privacy/index.html")
            .unwrap();
        assert_eq!(page.public_path, "/privacy/");
        assert!(page.include_in_sitemap);
        let html = &page.html;
        assert!(html.contains("<title>Privacy — Ryujin Hatakeyama</title>"));
        assert!(html.contains(
            r#"<link rel="canonical" href="https://ryujin-hatakeyama.github.io/privacy/">"#
        ));
        let main = &html[html.find("<main").unwrap()..html.find("</main>").unwrap()];
        assert_eq!(main.matches("<h1>Privacy</h1>").count(), 1);
        assert!(main.contains(r#"<a href="https://www.goatcounter.com/help/privacy">"#));
        // Not a primary section.
        assert!(!header("/privacy/").into_string().contains("/privacy/"));
    }

    #[test]
    fn primary_navigation_has_three_sections() {
        for path in ["/", "/research/", "/cv/", "/updates/", "/miscellany/"] {
            let html = header(path).into_string();
            let nav = &html[html.find("<nav").unwrap()..html.find("</nav>").unwrap()];
            let hrefs: Vec<_> = nav
                .match_indices("href=\"")
                .map(|(at, _)| &nav[at + 6..])
                .collect();
            assert_eq!(hrefs.len(), 3, "{path}");
            for (href, expected) in hrefs.iter().zip(["/\"", "/research/\"", "/miscellany/\""]) {
                assert!(href.starts_with(expected), "{path}");
            }
        }
        // CV and Updates stay reachable from Home and Research rather than
        // the header, so neither page marks a primary section as current.
        for path in ["/cv/", "/updates/", "/updates/item/x/"] {
            assert!(
                !header(path).into_string().contains("aria-current"),
                "{path}"
            );
        }
    }

    #[test]
    fn research_links_to_the_cv_after_its_sections() {
        let html = research(&[], &[], &[]).into_string();
        assert!(!html.contains(r##"href="#cv""##));
        assert!(!html.contains(r#"id="cv""#));
        assert_eq!(html.matches(r#"href="/cv/""#).count(), 1);
        let nav = &html[html.find("<nav").unwrap()..html.find("</nav>").unwrap()];
        assert!(
            nav.find("#presentations").unwrap() < nav.find(r#"<a href="/cv/">CV</a>"#).unwrap()
        );
    }

    #[test]
    fn homepage_has_one_all_updates_link_after_both_previews() {
        let html = home(&fixture_home(), &mixed_updates(), fixture_today()).into_string();
        assert_eq!(
            html.matches(r#"<a href="/updates/">All updates</a>"#)
                .count(),
            1
        );
        let link = html.find("All updates").unwrap();
        assert!(html.find(r#"id="upcoming-heading""#).unwrap() < link);
        assert!(html.rfind("</ol>").unwrap() < link);
        assert!(html.contains(r#"<a href="/cv/">"#), "homepage CV link");
    }

    #[test]
    fn event_names_link_to_their_official_pages_everywhere_they_are_listed() {
        let updates: Vec<_> = LINKED_EVENTS
            .iter()
            .map(|(file, _)| update_from_source(file))
            .collect();
        let refs: Vec<_> = updates.iter().collect();
        let research: Vec<_> = refs
            .iter()
            .copied()
            .filter(|u| u.categories.contains(&Category::Research))
            .collect();
        let academia: Vec<_> = refs
            .iter()
            .copied()
            .filter(|u| u.categories.contains(&Category::Academia))
            .collect();
        let research_page =
            updates_page_refs(&research, Some(Category::Research), fixture_today()).into_string();
        let academia_page =
            updates_page_refs(&academia, Some(Category::Academia), fixture_today()).into_string();
        let pages = [
            home(&fixture_home(), &updates, fixture_today()).into_string(),
            updates_page_refs(&refs, None, fixture_today()).into_string(),
            research_page.clone(),
            academia_page.clone(),
        ];
        for (update, (file, url)) in updates.iter().zip(LINKED_EVENTS) {
            assert_eq!(update.title_link.as_ref().unwrap().url, url, "{file}");
        }
        // Each record is listed once on the full page and once on the page of
        // each of its categories; on the homepage, once if its preview
        // includes it (which depends on the records' statuses and dates).
        for update in &updates {
            let heading = linked_heading(update);
            for (category, page) in [
                (Category::Research, &research_page),
                (Category::Academia, &academia_page),
            ] {
                let expected = usize::from(update.categories.contains(&category));
                assert_eq!(page.matches(&heading).count(), expected, "{}", update.id);
            }
        }
        let Timeline {
            mut upcoming,
            mut recent,
            ..
        } = Timeline::on(refs.iter().copied(), &[], fixture_today(), None);
        upcoming.truncate(HOME_UPCOMING_LIMIT);
        recent.truncate(HOME_RECENT_LIMIT);
        let previewed: Vec<_> = upcoming
            .iter()
            .map(|u| u.id.as_str())
            .chain(recent.iter().map(Entry::id))
            .collect();
        assert!(!previewed.is_empty());
        for update in &updates {
            let on_home = usize::from(previewed.contains(&update.id.as_str()));
            for (page, expected) in [(&pages[0], on_home), (&pages[1], 1)] {
                assert_eq!(
                    page.matches(&linked_heading(update)).count(),
                    expected,
                    "{}",
                    update.id
                );
            }
        }
        for page in &pages {
            assert_no_nested_anchors(page);
            assert!(
                !page.contains("サマースクール")
                    && !page.contains("日本ソフトウェア科学会第43回大会")
            );
        }
        // Only the event name is linked: the activity phrase stays text, and
        // the title has exactly one link.
        for update in &updates {
            let link = update.title_link.as_ref().unwrap();
            let (before, after) = link.split(&update.title.en).unwrap();
            assert!(
                !before.trim().is_empty() || !after.trim().is_empty(),
                "{}",
                update.id
            );
            assert_eq!(linked_heading(update).matches("<a ").count(), 1);
        }
        // Each primary resource is linked once, from the title: the records
        // no longer repeat it as a resource link. The distinct JSSST program
        // stays available, and every record link is rendered.
        let full = &pages[1];
        assert!(!full.contains("Event website") && !full.contains("Event information"));
        for update in &updates {
            let url = &update.title_link.as_ref().unwrap().url;
            assert_eq!(
                full.matches(&format!(r#"href="{url}""#)).count(),
                1,
                "{url}"
            );
            for link in &update.links {
                assert!(full.contains(&format!(r#"<a href="{}">{}"#, link.url, link.label)));
            }
        }
        assert!(!full.contains("Workshop welcoming"));
        assert!(full.contains(r#"<a href="https://jssst2026.wordpress.com/program/">Program"#));
        let resource_links: usize = updates.iter().map(|u| u.links.len()).sum();
        assert_eq!(resource_links, 1);
    }

    #[test]
    fn titles_without_event_links_remain_escaped_plain_text() {
        let mut update = test_update("plain", "2026-09-01", "completed");
        update.title.en = "A <b> & \"C\"".to_owned();
        let html = update_list(&[&update], false).into_string();
        assert!(html.contains("<h3>A &lt;b&gt; &amp; &quot;C&quot;</h3>"));
    }

    #[test]
    fn linked_title_text_is_escaped() {
        let update: crate::model::Update = serde_json::from_str(
            r#"{"id":"escaped","title":{"en":"At <Event> & Co"},"titleLink":{"text":"<Event> & Co","url":"https://example.org/?a=1&b=2"},
            "summary":{"en":"x"},"date":"2026-09-01","eventStatus":"completed",
            "categories":["academia"],"kind":{"type":"participation"},"related":{}}"#,
        )
        .unwrap();
        assert!(update_list(&[&update], true).into_string().contains(
            r#"<h3>At <a href="https://example.org/?a=1&amp;b=2">&lt;Event&gt; &amp; Co</a></h3>"#
        ));
    }

    #[test]
    fn detail_pages_stay_reachable_beside_event_links() {
        let source = |title_link: &str| -> crate::model::Update {
            serde_json::from_str(&format!(
                r#"{{"id":"talk","title":{{"en":"Talk at Conf 2026"}}{title_link},"summary":{{"en":"x"}},
                "date":"2026-09-01","eventStatus":"completed","categories":["research"],
                "kind":{{"type":"presentation"}},"related":{{}},"detail":true,
                "links":[{{"label":"Program","url":"https://conf.example/program/","type":"external"}},
                         {{"label":"Slides","url":"https://example.org/slides.pdf","type":"slides"}}]}}"#
            ))
            .unwrap()
        };
        let plain = source("");
        let html = update_list(&[&plain], true).into_string();
        assert!(html.contains(r#"<h3><a href="/updates/item/talk/">Talk at Conf 2026</a></h3>"#));

        let linked = source(r#","titleLink":{"text":"Conf 2026","url":"https://conf.example/"}"#);
        assert!(linked.validate("fixture").is_ok());
        for compact in [true, false] {
            let html = update_list(&[&linked], compact).into_string();
            assert!(
                html.contains(r#"<h3>Talk at <a href="https://conf.example/">Conf 2026</a></h3>"#)
            );
            assert!(html.contains(r#"<a href="/updates/item/talk/">Details<span class="sr-only">: Talk at Conf 2026</span></a>"#));
            assert_no_nested_anchors(&html);
            assert_eq!(html.matches(r#"href="https://conf.example/""#).count(), 1);
            assert_eq!(html.contains("slides.pdf"), !compact);
            assert_eq!(
                html.contains(r#"<a href="https://conf.example/program/">Program"#),
                !compact
            );
        }
        let detail = update_detail(&linked).into_string();
        assert!(
            detail.contains(r#"<h1>Talk at <a href="https://conf.example/">Conf 2026</a></h1>"#)
        );
        assert!(!detail.contains("/updates/item/talk/"));
        assert!(detail.contains("slides.pdf") && detail.contains("conf.example/program/"));
        let feed = rss(&[linked]);
        assert!(feed.contains("<title>Talk at Conf 2026</title>"));
        assert!(
            feed.contains("<guid>https://ryujin-hatakeyama.github.io/updates/item/talk/</guid>")
        );
    }

    #[test]
    fn rss_items_keep_plain_titles_and_internal_destinations() {
        let updates: Vec<_> = LINKED_EVENTS
            .iter()
            .map(|(file, _)| update_from_source(file))
            .chain([update_from_source("2026-01-aie-study-visit.json")])
            .collect();
        let feed = rss(&updates);
        // Each item carries its record's whole title as plain text.
        for update in &updates {
            assert!(feed.contains(&format!(
                "<item><title>{}</title>",
                escape_xml(&update.title.en)
            )));
        }
        assert!(feed.contains(
            "<description>Attended PPL Summer School 2026, a summer school on quantum program compilation.</description>"
        ));
        for update in &updates {
            let url = format!(
                "https://ryujin-hatakeyama.github.io/updates/#update-{}",
                update.id
            );
            assert!(feed.contains(&format!("<link>{url}</link><guid>{url}</guid>")));
        }
        for external in [
            "sites.google.com",
            "jssst-ppl.org",
            "jssst2026",
            "allenai.org",
            "](",
            "<a ",
        ] {
            assert!(!feed.contains(external), "{external}");
        }
        assert!(feed.contains("the Allen Institute for AI (Ai2), and Micron.</description>"));
    }

    #[test]
    fn study_visit_institutions_render_as_links() {
        let update = update_from_source("2026-01-aie-study-visit.json");
        let html = update_list(&[&update], false).into_string();
        for (name, url) in [
            (
                "University of Washington",
                "https://www.washington.edu/about/seattle-campus/",
            ),
            (
                "Google",
                "https://www.google.com/about/careers/applications/locations/seattle-kirkland-bellevue-redmond/",
            ),
            ("Amazon", "https://www.amazon.jobs/content/en/locations/hq1"),
            (
                "Microsoft",
                "https://careers.microsoft.com/v2/global/en/locations/seattle-area.html",
            ),
            (
                "Allen Institute for AI (Ai2)",
                "https://allenai.org/contact",
            ),
            ("Micron", "https://www.micron.com/about/careers/idaho"),
        ] {
            assert!(
                html.contains(&format!(r#"<a href="{url}">{name}</a>"#)),
                "{name}"
            );
        }
        assert!(!html.contains("]("));
        // The title is plain text: the institutions are linked in the summary.
        assert!(html.contains(&format!(
            "<h3>{}</h3>",
            maud::html! { (update.title.en) }.into_string()
        )));
        let summary = &html[html.find("<p>Selected").unwrap()..];
        let summary = &summary[..summary.find("</p>").unwrap()];
        assert_eq!(summary.matches("<a ").count(), 6);
        assert!(!summary.contains("target="));
    }

    #[test]
    fn last_updated_is_one_exact_date_line() {
        assert_eq!(
            super::parse_last_updated("2026-10-10\n"),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 10)
        );
        assert_eq!(
            super::parse_last_updated("2026-10-10"),
            chrono::NaiveDate::from_ymd_opt(2026, 10, 10)
        );
        for bad in [
            "",
            "2026-10-1\n",
            " 2026-10-10\n",
            "2026-10-10\n\n",
            "2026-02-30\n",
            "10 October 2026\n",
        ] {
            assert_eq!(super::parse_last_updated(bad), None, "{bad:?}");
        }
        // The file in the repository is valid.
        assert_eq!(
            *super::LAST_UPDATED,
            super::parse_last_updated(include_str!("../../content/last-updated.txt")).unwrap()
        );
    }

    #[test]
    fn footer_shows_the_editorial_last_updated_date_and_no_rss_link() {
        let html = footer().into_string();
        assert!(html.contains(r#"<p class="footer-name">© 2026 Ryujin Hatakeyama</p>"#));
        // The date is whatever content/last-updated.txt says (the Writing
        // Studio may set it when publishing), so it is read from there.
        let line = include_str!("../../content/last-updated.txt").trim_end();
        let date = chrono::NaiveDate::parse_from_str(line, "%Y-%m-%d").unwrap();
        assert!(html.contains(&format!(
            r#"<p class="footer-updated">Last updated <time datetime="{line}">{}</time></p>"#,
            date.format("%-d %B %Y")
        )));
        assert!(!html.contains("rss.xml") && !html.contains(">RSS<"));
    }

    #[test]
    fn rss_items_have_no_synthetic_publication_dates() {
        let updates = mixed_updates();
        let feed = rss(&updates);
        assert!(!feed.contains("pubDate") && !feed.contains("00:00:00"));
        assert_eq!(feed.matches("<guid>").count(), updates.len());
        assert!(feed.contains(
            "<link>https://ryujin-hatakeyama.github.io/updates/#update-completed-1</link><guid>https://ryujin-hatakeyama.github.io/updates/#update-completed-1</guid><category>Activities</category></item>"
        ));
    }
}

#[cfg(test)]
mod miscellany_tests {
    use super::{
        MISCELLANY_ICONS, diary_months, diary_page, fixture_today, header, miscellany_overview,
        note_detail, notes_page, pages, reading_page,
    };
    use crate::content::{Miscellany, ValidatedSiteContent};

    /// Published fixtures parsed through the real validation; they exist only
    /// in memory and are never written to the content directory.
    fn miscellany(reading: &str, notes: &[&str], diary: &[&str]) -> Miscellany {
        Miscellany::from_sources(reading, notes, diary).unwrap()
    }

    fn main_of(html: &str) -> &str {
        let start = html.find("<main id=\"main-content\">").unwrap();
        &html[start..html.find("</main>").unwrap()]
    }

    fn page_html(content: &ValidatedSiteContent, output_path: &str) -> String {
        pages(content, fixture_today())
            .into_iter()
            .find(|page| page.output_path == output_path)
            .unwrap_or_else(|| panic!("missing {output_path}"))
            .html
    }

    const NOTE_EN: &str = "---\nslug: staged-interpreters\ntitle: Staged <interpreters> & you\nlang: en\ndescription: How a staged interpreter becomes a compiler.\norder: 1\nmath: true\ndraft: false\n---\n\n## Background\n\nA paragraph with `inline code` and a [reference](https://example.org/paper).\n\n- first\n- second\n\n> A quotation.\n\n```ocaml\nlet rec power n x = if n = 0 then .<1>. else .<.~x * .~(power (n - 1) x)>.\n```\n\nInline $x^2$ and displayed:\n\n$$\\sum_{i=0}^{n} i = \\frac{n(n+1)}{2}$$\n\nText.[^1]\n\n[^1]: A footnote.\n";
    const NOTE_JA: &str = "---\nslug: kripke-semantics\ntitle: クリプキ意味論の覚え書き\nlang: ja\ndraft: false\n---\n\n## 可能世界\n\n本文の段落です。\n";
    const NOTE_DRAFT: &str =
        "---\nslug: unfinished\ntitle: Unfinished note\nlang: en\n---\n\nNot public.\n";
    const DIARY_OLD: &str = "---\ndate: 2026-03-01\nlang: en\ndraft: false\n---\n\nFirst paragraph.\n\nSecond paragraph.\n";
    const DIARY_NEW: &str = "---\ndate: 2026-10-10\nlang: ja\ntitle: 仙台にて\ndraft: false\n---\n\n今日は図書館で一日を過ごした。\n";
    const DIARY_DRAFT: &str = "---\ndate: 2026-10-11\nlang: en\n---\n\nNot public.\n";
    const DIARY_NEW_EARLIER: &str =
        "---\ndate: 2026-10-02\nlang: en\ndraft: false\n---\n\nEarlier in October.\n";
    const DIARY_LAST_YEAR: &str =
        "---\ndate: 2025-12-31\nlang: en\ndraft: false\n---\n\nLast year.\n";

    #[test]
    fn miscellany_is_the_third_primary_section_and_current_below_its_route() {
        for path in [
            "/miscellany/",
            "/miscellany/reading/",
            "/miscellany/notes/",
            "/miscellany/notes/staged-interpreters/",
            "/miscellany/diary/",
        ] {
            let html = header(path).into_string();
            let order: Vec<_> = ["Home<", "Research<", "Miscellany<"]
                .iter()
                .map(|label| html.find(label).unwrap())
                .collect();
            assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{path}");
            assert!(html.contains(r#"<a href="/miscellany/" aria-current="page">Miscellany</a>"#));
            assert_eq!(html.matches("aria-current").count(), 1, "{path}");
        }
        for path in ["/", "/research/", "/cv/", "/updates/", "/writings/"] {
            let html = header(path).into_string();
            assert!(
                html.contains(r#"<a href="/miscellany/">Miscellany</a>"#),
                "{path}"
            );
        }
    }

    #[test]
    fn overview_has_one_heading_four_plain_rows_and_a_quiet_search() {
        let html = miscellany_overview(&Miscellany::default(), &[]).into_string();
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains("<h1>Miscellany</h1>"));
        // The overview is the top: no way back, and no section is current.
        assert!(!html.contains("Breadcrumb") && !html.contains("aria-current"));
        let sections = [
            ("reading", "Reading", MISCELLANY_ICONS[0]),
            ("notes", "Notes", MISCELLANY_ICONS[1]),
            ("diary", "Diary", MISCELLANY_ICONS[2]),
            ("sketches", "Sketches", MISCELLANY_ICONS[3]),
        ];
        let positions: Vec<_> = sections
            .iter()
            .map(|(slug, label, icon)| {
                let start = html
                    .find(&format!(r#"<li><a href="/miscellany/{slug}/"><svg"#))
                    .unwrap();
                let section = &html[start..];
                // One link per row: the decorative icon, the name, and a quiet
                // chevron; no description beside it (each section's own page
                // keeps its line).
                let expected = format!(
                    r#"<li><a href="/miscellany/{slug}/"><svg class="miscellany-icon" aria-hidden="true" focusable="false" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">{icon}</svg><span class="miscellany-contents-label">{label}</span><svg class="miscellany-chevron" aria-hidden="true""#
                );
                assert!(section.starts_with(&expected), "{section}");
                start
            })
            .collect();
        assert!(!html.contains("miscellany-contents-description"));
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(html.matches("<li>").count(), 4);
        assert!(html.contains(r#"<nav class="miscellany-overview" aria-label="Miscellany sections"><ul class="miscellany-contents">"#));
        assert_eq!(
            html.matches("<svg").count(),
            9,
            "four icons, four chevrons, and the search icon"
        );
        // The search is one plain form that opens the search page, after the
        // heading and before the sections, and is there with nothing
        // published.
        assert!(html.contains(r#"<div class="miscellany-home"><header class="miscellany-header"><div class="miscellany-title"><h1>Miscellany</h1></div></header><form class="miscellany-search" action="/miscellany/search/" method="get" role="search" aria-label="Search Miscellany"><label class="sr-only" for="miscellany-search-query">Search Miscellany</label><input id="miscellany-search-query" type="search" name="q""#));
        assert!(!html.contains(r#"name="section""#));
        assert_eq!(html.matches("role=").count(), 1);
        assert!(!html.contains("guides"));
        for decoration in [
            "card",
            "badge",
            "tab",
            "<img",
            "No entries",
            "data-pagefind",
        ] {
            assert!(!html.contains(decoration), "{decoration}");
        }
    }

    #[test]
    fn empty_sections_are_quiet_and_contain_no_entries() {
        let content = ValidatedSiteContent::with_miscellany(Miscellany::default());
        for (path, title) in [
            ("miscellany/reading/index.html", "Reading"),
            ("miscellany/notes/index.html", "Notes"),
            ("miscellany/diary/index.html", "Diary"),
            ("miscellany/sketches/index.html", "Sketches"),
        ] {
            let html = page_html(&content, path);
            let main = main_of(&html);
            // `Miscellany / Reading` is the heading: the way back, then the
            // section's name as the page's one h1.
            assert!(main.contains(&format!(r#"<div class="miscellany-title"><nav class="miscellany-trail" aria-label="Breadcrumb"><a href="/miscellany/">Miscellany</a><span class="miscellany-trail-separator" aria-hidden="true">/</span></nav><h1>{title}</h1></div>"#)), "{path}");
            assert_eq!(main.matches("<h1").count(), 1);
            // Every category says the same one line for having nothing, and no description.
            assert!(main.contains(r#"</header><p class="empty-state miscellany-empty">No entries yet.</p><section class="site-message""#), "{path}");
            for said in [
                "miscellany-description",
                "No notes yet",
                "No sketches yet",
                "Books and papers",
                "Study notes",
                "Occasional entries",
            ] {
                assert!(!main.contains(said), "{path}: {said}");
            }
            // The search, always.
            assert!(
                main.contains(r#"<form class="miscellany-search""#),
                "{path}"
            );
            // (The message form at the end has its own heading, apart from the section's entries.)
            let entries = main
                .split(r#"<section class="site-message""#)
                .next()
                .unwrap();
            assert!(
                !entries.contains("<article")
                    && !entries.contains("<li")
                    && !entries.contains("<h2")
            );
            assert!(main.contains("data-site-message"), "{path}");
            // All four sections, in order, one tap away, even when empty.
            let sections = &main[main
                .find(r#"<nav class="miscellany-sections" aria-label="Miscellany sections">"#)
                .unwrap()..];
            let order: Vec<_> = ["/reading/", "/notes/", "/diary/", "/sketches/"]
                .iter()
                .map(|slug| {
                    sections
                        .find(&format!(r#"<a href="/miscellany{slug}""#))
                        .unwrap()
                })
                .collect();
            assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(main.contains(&format!(r#"aria-current="page">{title}</a>"#)));
            assert_eq!(main.matches(r#"aria-current="page""#).count(), 1);
            // The search comes before the sections and the list.
            assert!(
                main.find("miscellany-search").unwrap() < main.find("miscellany-sections").unwrap()
            );
            // Each section offers a search limited to it.
            assert!(main.contains(r#"<input type="hidden" name="section" value=""#));
            assert!(!main.contains("data-pagefind-body"), "{path}");
        }
        assert!(
            !pages(&content, fixture_today())
                .iter()
                .any(|page| page.output_path.starts_with("miscellany/diary/")
                    && page.output_path != "miscellany/diary/index.html"),
            "an empty Diary has no month pages"
        );
        assert!(
            !pages(&content, fixture_today())
                .iter()
                .any(|page| page.output_path.starts_with("miscellany/notes/")
                    && page.output_path != "miscellany/notes/index.html")
        );
    }

    #[test]
    fn reading_keeps_the_authors_order_original_titles_and_languages() {
        let reading = "items:\n  - author: 夏目漱石\n    title: こころ\n    lang: ja\n    note: Read in the original.\n  - author: Saul A. Kripke\n    title: Naming and Necessity\n    url: https://example.org/naming\n  - title: 竹取物語\n    lang: ja\n    note: 二度目。\n    noteLang: ja\n  - author: A <b> & \"C\"\n    title: Title <script>alert(1)</script>\n";
        let html = reading_page(&miscellany(reading, &[], &[])).into_string();
        let order: Vec<_> = [
            "こころ",
            "Naming and Necessity",
            "竹取物語",
            "Title &lt;script&gt;",
        ]
        .iter()
        .map(|title| html.find(title).unwrap())
        .collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
        // The work's language marks its title and author; the commentary is
        // the author's own passage, in its own language.
        assert!(html.contains(r#"<cite lang="ja">こころ</cite></h2><p class="reading-author" lang="ja">夏目漱石</p><div class="passages" data-passages><p class="reading-note" data-passage="en" lang="en">Read in the original.</p><p class="language-notice" data-language-notice data-pagefind-ignore="all" hidden>Written in English. <button type="button" data-language-reveal>Read in English</button></p></div>"#));
        assert!(html.contains(r#"<cite><a href="https://example.org/naming">Naming and Necessity</a></cite></h2><p class="reading-author">Saul A. Kripke</p>"#));
        // An anonymous work has no author line; a Japanese note is marked.
        assert!(html.contains(
            r#"<cite lang="ja">竹取物語</cite></h2><div class="passages" data-passages><p class="reading-note" data-passage="ja" lang="ja">二度目。</p><p class="language-notice" data-language-notice data-pagefind-ignore="all" hidden>Written in Japanese. <button type="button" data-language-reveal>Read in Japanese</button></p></div>"#
        ));
        assert!(html.contains("A &lt;b&gt; &amp; &quot;C&quot;"));
        assert!(!html.contains("<script>") && !html.contains("<b>"));
        for absent in ["rating", "progress", "%", "★", "<time"] {
            assert!(!html.contains(absent), "{absent}");
        }
    }

    #[test]
    fn notes_have_stable_pages_with_markdown_and_math() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            "items: []",
            &[NOTE_JA, NOTE_DRAFT, NOTE_EN],
            &[],
        ));
        let index = page_html(&content, "miscellany/notes/index.html");
        // The ordered note comes first; the draft is not listed.
        let english = index
            .find(r#"<li lang="en"><h2><a href="/miscellany/notes/staged-interpreters/">Staged &lt;interpreters&gt; &amp; you</a></h2><p>How a staged interpreter becomes a compiler.</p></li>"#)
            .unwrap();
        let japanese = index
            .find(r#"<li lang="ja"><h2><a href="/miscellany/notes/kripke-semantics/">クリプキ意味論の覚え書き</a></h2></li>"#)
            .unwrap();
        assert!(english < japanese);
        assert!(!index.contains("Unfinished") && !index.contains("/unfinished/"));

        let paths: Vec<_> = pages(&content, fixture_today())
            .into_iter()
            .map(|page| page.output_path)
            .collect();
        assert!(paths.contains(&"miscellany/notes/staged-interpreters/index.html".to_owned()));
        assert!(paths.contains(&"miscellany/notes/kripke-semantics/index.html".to_owned()));
        assert!(!paths.iter().any(|path| path.contains("unfinished")));

        let html = page_html(&content, "miscellany/notes/staged-interpreters/index.html");
        assert!(html.contains(r#"<link rel="canonical" href="https://ryujin-hatakeyama.github.io/miscellany/notes/staged-interpreters/">"#));
        assert!(
            html.contains(
                "<title>Staged &lt;interpreters&gt; &amp; you — Ryujin Hatakeyama</title>"
            )
        );
        assert!(html.contains(r#"<a href="/miscellany/" aria-current="page">Miscellany</a>"#));
        let main = main_of(&html);
        assert_eq!(main.matches("<h1").count(), 1);
        assert!(main.contains(r#"<article class="miscellany-note" data-pagefind-body data-pagefind-filter="section:Notes" data-reading-article="notes/staged-interpreters"><header><h1 lang="en" data-pagefind-meta="title">Staged &lt;interpreters&gt; &amp; you</h1><p class="summary" lang="en">How a staged interpreter becomes a compiler.</p></header><div class="passages" data-passages><div class="passage" data-passage="en" lang="en"><div class="prose miscellany-prose">"#));
        for fragment in [
            "<h2>Background</h2>",
            "<code>inline code</code>",
            r#"<a href="https://example.org/paper">reference</a>"#,
            "<ul>\n<li>first</li>",
            "<blockquote>\n<p>A quotation.</p>",
            r#"<pre><code class="language-ocaml">let rec power n x = if n = 0 then .&lt;1&gt;."#,
            r#"<span class="katex">"#,
            r#"<span class="katex-display">"#,
            "<annotation encoding=\"application/x-tex\">x^2</annotation>",
            r#"class="footnote-definition""#,
        ] {
            assert!(main.contains(fragment), "{fragment}");
        }
        // The way back to Notes, outside the language-marked article; the
        // sections are not repeated above the text.
        assert!(main.contains(r#"<nav class="miscellany-trail" aria-label="Breadcrumb"><a href="/miscellany/">Miscellany</a><span class="miscellany-trail-separator" aria-hidden="true">/</span><a href="/miscellany/notes/">Notes</a>"#));
        assert!(main.find("miscellany-trail").unwrap() < main.find("<article").unwrap());
        assert!(!main.contains("miscellany-sections") && !main.contains("miscellany-search"));
    }

    const NOTE_THEOREMS: &str = "---\nslug: normalization\ntitle: Normalization\nlang: en\nmath: true\ntheoremNumbering: section\ndraft: false\n---\n\n## Preliminaries\n\n::: {#def-value name=\"Value\"}\nA term $v$ with $\\llbracket v\\rrbracket_\\rho$ defined.\n:::\n\n::: {#lem-step}\nEvery step shrinks the measure.\n:::\n\n::: {.proof of=\"lem-step\"}\nBy @def-value.\n:::\n\n## Results\n\n::: {#thm-normal}\nEvery term normalizes, by @lem-step.\n:::\n";

    #[test]
    fn notes_number_and_link_theorem_environments() {
        let content =
            ValidatedSiteContent::with_miscellany(miscellany("items: []", &[NOTE_THEOREMS], &[]));
        let html = page_html(&content, "miscellany/notes/normalization/index.html");
        let main = main_of(&html);
        for fragment in [
            r#"<div class="theorem theorem-definition" data-kind="definition" id="en-def-value">"#,
            r#"<span class="theorem-head"><strong>Definition 1.1</strong> <span class="theorem-name" data-name="Value">(Value)</span><strong>.</strong></span> A term"#,
            r#"<div class="theorem theorem-plain" data-kind="lemma" id="en-lem-step">"#,
            r##"<span class="theorem-head"><em>Proof of Lemma 1.2</em><em>.</em></span> By <a class="theorem-ref" href="#en-def-value">Definition 1.1</a>."##,
            r#"<span class="qed" role="img" aria-label="End of proof">∎</span>"#,
            r##"<span class="theorem-head"><strong>Theorem 2.1</strong><strong>.</strong></span> Every term normalizes, by <a class="theorem-ref" href="#en-lem-step">Lemma 1.2</a>."##,
        ] {
            assert!(main.contains(fragment), "{fragment}\n{main}");
        }
        assert!(!main.contains(":::"));
        assert_eq!(main.matches('∎').count(), 1);

        // Through the text instead: the same environments, 1, 2, 3.
        let through =
            NOTE_THEOREMS.replace("theoremNumbering: section", "theoremNumbering: document");
        let content =
            ValidatedSiteContent::with_miscellany(miscellany("items: []", &[&through], &[]));
        let html = page_html(&content, "miscellany/notes/normalization/index.html");
        assert!(
            html.contains("<strong>Definition 1</strong>")
                && html.contains("<strong>Lemma 2</strong>")
                && html.contains("<strong>Theorem 3</strong>")
        );
        // Without the field: by section.
        let default = NOTE_THEOREMS.replace("theoremNumbering: section\n", "");
        let content =
            ValidatedSiteContent::with_miscellany(miscellany("items: []", &[&default], &[]));
        assert!(
            page_html(&content, "miscellany/notes/normalization/index.html")
                .contains("<strong>Theorem 2.1</strong>")
        );
    }

    #[test]
    fn theorem_problems_stop_the_build_with_their_lines() {
        let error = |body: &str| {
            let note = NOTE_THEOREMS.replace("## Results", body);
            format!(
                "{:#}",
                Miscellany::from_sources("items: []", &[&note], &[]).unwrap_err()
            )
        };
        let undefined = error("See @thm-nowhere.");
        assert!(
            undefined.contains("line 24: No environment is labelled “thm-nowhere”"),
            "{undefined}"
        );
        let unclosed = format!(
            "{:#}",
            Miscellany::from_sources(
                "items: []",
                &[&format!(
                    "{NOTE_THEOREMS}\n::: {{#thm-open}}\nNever closed.\n"
                )],
                &[]
            )
            .unwrap_err()
        );
        assert!(
            unclosed.contains("line 30: This environment is never closed"),
            "{unclosed}"
        );
        let nested = error("::: {#thm-open}\nNever closed here.");
        assert!(
            nested.contains("line 27: An environment cannot start inside another"),
            "{nested}"
        );
        let duplicate = error("::: {#lem-step}\nAgain.\n:::");
        assert!(
            duplicate.contains("labels more than one environment"),
            "{duplicate}"
        );
        let malformed = error("::: {#thm-a .lemma}\nDisagrees.\n:::");
        assert!(
            malformed.contains("The label says Theorem, the class says Lemma."),
            "{malformed}"
        );
        let box_fence = error("::: {.box}\nA box.\n:::");
        assert!(
            box_fence.contains("shows no other ::: blocks"),
            "{box_fence}"
        );
        let wrong = NOTE_THEOREMS.replace("theoremNumbering: section", "theoremNumbering: chapter");
        assert!(Miscellany::from_sources("items: []", &[&wrong], &[]).is_err());
    }

    #[test]
    fn diary_entries_on_one_page_keep_their_theorem_anchors_apart() {
        let entry = |date: &str| {
            format!(
                "---\ndate: {date}\nlang: en\ndraft: false\n---\n\n::: {{#thm-day}}\nA fact.\n:::\n\nBy @thm-day.\n"
            )
        };
        let (first, second) = (entry("2026-10-02"), entry("2026-10-10"));
        let content =
            ValidatedSiteContent::with_miscellany(miscellany("items: []", &[], &[&first, &second]));
        let html = page_html(&content, "miscellany/diary/2026/10/index.html");
        for date in ["2026-10-02", "2026-10-10"] {
            assert!(
                html.contains(&format!(r#"id="{date}-en-thm-day""#)),
                "{date}"
            );
            assert!(
                html.contains(&format!(r##"href="#{date}-en-thm-day""##)),
                "{date}"
            );
        }
    }

    #[test]
    fn japanese_notes_are_marked_as_japanese() {
        let notes = miscellany("items: []", &[NOTE_JA], &[]);
        let note = notes.public_notes().next().unwrap();
        let html = note_detail(note, &[]).into_string();
        assert!(html.contains(r#"<article class="miscellany-note" data-pagefind-body data-pagefind-filter="section:Notes" data-reading-article="notes/kripke-semantics"><header><h1 lang="ja" data-pagefind-meta="title">クリプキ意味論の覚え書き</h1></header><div class="passages" data-passages><div class="passage longform-ja" data-passage="ja" lang="ja">"#));
        assert!(html.contains("<h2>可能世界</h2>"));
        assert!(!html.contains("summary"));
        assert!(
            notes_page(&notes)
                .into_string()
                .contains(r#"<li lang="ja">"#)
        );
    }

    #[test]
    fn diary_lists_months_newest_first_and_each_month_has_its_page() {
        let entries = miscellany(
            "items: []",
            &[],
            &[
                DIARY_OLD,
                DIARY_DRAFT,
                DIARY_NEW,
                DIARY_NEW_EARLIER,
                DIARY_LAST_YEAR,
            ],
        );
        let html = diary_page(&entries).into_string();
        let year = html
            .find(r#"<section class="diary-year" aria-labelledby="diary-year-2026"><h2 id="diary-year-2026">2026</h2><ol class="diary-months"><li><a href="/miscellany/diary/2026/10/" aria-label="October 2026" data-reading-entries="diary/2026-10-02 diary/2026-10-10">October</a> <span class="diary-month-count" aria-hidden="true">2</span></li><li><a href="/miscellany/diary/2026/03/" aria-label="March 2026" data-reading-entries="diary/2026-03-01">March</a> <span class="diary-month-count" aria-hidden="true">1</span></li></ol></section>"#)
            .unwrap();
        let earlier_year = html.find(r#"<h2 id="diary-year-2025">2025</h2>"#).unwrap();
        assert!(year < earlier_year);
        // The front page lists months; it shows no entry and is not indexed.
        assert!(!html.contains("<article") && !html.contains("data-pagefind-body"));
        assert!(!html.contains("Not public"));
        // Months without entries (every month between March and October,
        // and the draft's) have no page.
        let months: Vec<_> = diary_months(&entries)
            .iter()
            .map(super::DiaryMonth::path)
            .collect();
        assert_eq!(
            months,
            [
                "/miscellany/diary/2026/10/",
                "/miscellany/diary/2026/03/",
                "/miscellany/diary/2025/12/"
            ]
        );
    }

    #[test]
    fn a_month_shows_its_entries_oldest_first_with_stable_anchors_and_neighbours() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            "items: []",
            &[],
            &[
                DIARY_OLD,
                DIARY_DRAFT,
                DIARY_NEW,
                DIARY_NEW_EARLIER,
                DIARY_LAST_YEAR,
            ],
        ));
        let html = page_html(&content, "miscellany/diary/2026/10/index.html");
        assert!(html.contains("<title>Diary, October 2026 — Ryujin Hatakeyama</title>"));
        assert!(html.contains(r#"<link rel="canonical" href="https://ryujin-hatakeyama.github.io/miscellany/diary/2026/10/">"#));
        let main = main_of(&html);
        assert!(main.contains(r#"<h1><time datetime="2026-10">October 2026</time></h1>"#));
        // The previous month with entries is March, not September; there is
        // no later month.
        assert!(main.contains(r#"<a class="diary-previous" href="/miscellany/diary/2026/03/" rel="prev"><span aria-hidden="true">← </span>March 2026</a>"#));
        assert!(!main.contains("diary-next"));
        assert!(main.contains(r##"<nav class="diary-days" aria-label="Days in October 2026"><a href="#diary-2026-10-02" aria-label="2 October">2</a><a href="#diary-2026-10-10" aria-label="10 October">10</a></nav>"##));
        let earlier = main
            .find(r##"<article class="diary-entry" data-reading-article="diary/2026-10-02" data-reading-seen><h2 class="diary-heading" id="diary-2026-10-02"><a class="diary-date" href="#diary-2026-10-02" aria-label="Friday 2 October 2026" data-pagefind-ignore="all"><time datetime="2026-10-02"><span class="diary-day" data-reading-target="">2</span> <span class="diary-weekday">Fri</span></time></a></h2><div class="diary-body"><div class="passages" data-passages><div class="passage" data-passage="en" lang="en"><div class="prose miscellany-prose"><p>Earlier in October.</p>"##)
            .unwrap();
        let later = main
            .find(r##"<article class="diary-entry has-title" data-reading-article="diary/2026-10-10" data-reading-seen><h2 class="diary-heading" id="diary-2026-10-10"><a class="diary-date" href="#diary-2026-10-10" aria-label="Saturday 10 October 2026" data-pagefind-ignore="all"><time datetime="2026-10-10"><span class="diary-day">10</span> <span class="diary-weekday">Sat</span></time></a><span class="diary-title" lang="ja" data-reading-target>仙台にて</span></h2><div class="diary-body"><div class="passages" data-passages><div class="passage longform-ja" data-passage="ja" lang="ja"><div class="prose miscellany-prose"><p>今日は図書館で一日を過ごした。</p>"##)
            .unwrap();
        assert!(earlier < later, "oldest first within the month");
        assert_eq!(main.matches("<article").count(), 2);
        assert!(!main.contains("Not public") && !main.contains("2026-10-11"));
        // Only the entries are read by the search index, filed under Diary.
        assert_eq!(main.matches("data-pagefind-body").count(), 1);
        assert!(main.contains(r#"<div class="diary" data-pagefind-body data-pagefind-filter="section:Diary" data-pagefind-meta="title:October 2026">"#));
        assert!(main.find("data-pagefind-body").unwrap() > main.find("diary-days").unwrap());
        // The section's search starts in the Diary.
        assert!(main.contains(r#"<input type="hidden" name="section" value="diary">"#));

        let march = page_html(&content, "miscellany/diary/2026/03/index.html");
        let main = main_of(&march);
        assert!(main.contains(r#"<a class="diary-next" href="/miscellany/diary/2026/10/" rel="next">October 2026<span aria-hidden="true"> →</span></a>"#));
        assert!(main.contains(r#"href="/miscellany/diary/2025/12/" rel="prev""#));
        // One entry: no list of days.
        assert!(!main.contains("diary-days"));
        assert!(main.contains("<p>First paragraph.</p>"));
    }

    #[test]
    fn existing_routes_and_feed_are_unchanged_by_miscellany() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            "items:\n  - title: Fixture work\n",
            &[NOTE_EN],
            &[DIARY_OLD],
        ));
        let paths: Vec<_> = pages(&content, fixture_today())
            .into_iter()
            .map(|page| page.output_path)
            .collect();
        for path in [
            "index.html",
            "research/index.html",
            "cv/index.html",
            "updates/index.html",
            "updates/academia/index.html",
            "writings/index.html",
            "404.html",
            "miscellany/index.html",
            "miscellany/reading/index.html",
            "miscellany/notes/index.html",
            "miscellany/diary/index.html",
            "miscellany/diary/2026/03/index.html",
            "miscellany/sketches/index.html",
            "miscellany/search/index.html",
            "miscellany/notes/staged-interpreters/index.html",
        ] {
            assert!(paths.contains(&path.to_owned()), "{path}");
        }
        // Miscellany is never announced as an update.
        let feed = super::rss(content.updates());
        assert!(!feed.contains("<item>") && !feed.contains("miscellany"));
        let updates = page_html(&content, "updates/index.html");
        assert!(
            !main_of(&updates).contains("Staged") && !main_of(&updates).contains("First paragraph")
        );
        let home = page_html(&content, "index.html");
        assert!(!main_of(&home).contains("miscellany"));
    }
}

#[cfg(test)]
mod multilingual_tests {
    use super::{
        MESSAGE_ENDPOINT, articles_json, fixture_today, message_form, miscellany_overview,
        note_detail, notes_page, pages, reading_page,
    };
    use crate::content::{Miscellany, ValidatedSiteContent};
    use crate::model::DisplayLanguage;

    // Fictional fixtures, parsed through the real validation and never
    // written to the content directory.
    const MOON_JA: &str = "---\nslug: moon\ntitle: 月について\nlang: ja\ndraft: false\n---\n\n## 見えるもの\n\n月の話。ここでは [Stimme]{lang=de} という語を借りる。[^1]\n\n[^1]: 日本語の注。\n";
    const MOON_EN: &str = "---\nof: moon\nlang: en\ntitle: On <the> moon\n---\n\nA longer English text with its own argument.[^1]\n\n### A section only here\n\nMore English prose, with `code` and a [link](https://example.org/).\n\n[^1]: An English note.\n";
    const ONLY_EN: &str =
        "---\nslug: only-en\ntitle: Only English\nlang: en\ndraft: false\n---\n\nEnglish text.\n";
    const ONLY_JA: &str =
        "---\nslug: only-ja\ntitle: 日本語だけ\nlang: ja\ndraft: false\n---\n\n日本語の本文。\n";
    const MIXED: &str = "---\nslug: mixed\ntitle: Between languages\nlang: mixed\ndraft: false\n---\n\nThis sentence begins in English, そして日本語で続き、[und endet auf Deutsch]{lang=de}.\n";
    const ONLY_EN_DE: &str =
        "---\nof: only-en\nlang: de\n---\n\nEin deutscher Text, keine Übersetzung.\n";
    const DIARY_JA: &str = "---\ndate: 2026-10-10\nlang: ja\ndraft: false\n---\n\n鳥を見た。\n";
    const DIARY_EN: &str =
        "---\nof: 2026-10-10\nlang: en\n---\n\nA different, shorter English entry.\n";
    const DIARY_OTHER_SAME_DAY: &str = "---\ndate: 2026-10-10\nslug: evening\nlang: en\ndraft: false\n---\n\nAn unrelated entry on the same date.\n";
    const READING: &str = "items:\n  - author: 作者\n    title: 架空の本\n    lang: ja\n    note: My English commentary on a Japanese book.\n  - title: Two Commentaries\n    notes:\n      ja: 日本語の感想。\n      en: An English comment, not a translation.\n  - author: Someone\n    title: No Commentary\n";

    #[test]
    fn a_message_form_ends_each_category_and_offers_only_published_articles() {
        let published =
            "---\nslug: shown\ntitle: A <shown> note\nlang: en\ndraft: false\n---\n\nText.\n";
        let draft = "---\nslug: private-draft\ntitle: Not yet\nlang: en\n---\n\nText.\n";
        let miscellany = miscellany(&[published, draft], &[DIARY_JA]);
        let form = message_form(&miscellany, "notes").into_string();
        assert!(form.contains(r#"data-category="notes""#), "{form}");
        assert!(form.contains(MESSAGE_ENDPOINT));
        assert!(form.contains(r#"value="shown""#));
        // Escaped as text; a draft is never offered.
        assert!(form.contains("A &lt;shown&gt; note"));
        assert!(!form.contains("private-draft") && !form.contains("Not yet"));
        // Hidden until the site's script shows it, with its field for robots.
        assert!(
            form.contains(" hidden>") || form.contains(" hidden "),
            "{form}"
        );
        assert!(form.contains(r#"name="website""#));
        // Its words: the heading, the two labels, Send; nothing explained, nothing like a public comment.
        assert!(
            form.contains(r#"<h2 id="site-message-heading">Messages welcome.</h2><form"#),
            "{form}"
        );
        assert!(form.contains(r#"<label for="site-message-body">Message</label><textarea id="site-message-body" name="body" rows="3""#), "{form}");
        assert!(
            form.contains(r#"Name <span class="site-message-optional">(optional)</span>"#),
            "{form}"
        );
        assert!(
            form.contains(r#"<button type="submit">Send</button>"#),
            "{form}"
        );
        for word in [
            "Write to the author",
            "reaches",
            "not shown",
            "Comment",
            "Post",
            "Publish",
        ] {
            assert!(!form.contains(word), "{word}");
        }
        let json: serde_json::Value = serde_json::from_str(&articles_json(&miscellany)).unwrap();
        assert_eq!(
            json["notes"],
            serde_json::json!([{ "id": "shown", "title": "A <shown> note", "url": "/miscellany/notes/shown/" }])
        );
        assert_eq!(json["diary"][0]["id"], "2026-10-10");
        assert_eq!(
            json["diary"][0]["url"],
            "/miscellany/diary/2026/10/#diary-2026-10-10"
        );
        assert!(json["reading"].as_array().is_some());
        assert_eq!(json["sketches"], serde_json::json!([]));
        assert!(!articles_json(&miscellany).contains("private-draft"));
    }

    fn miscellany(notes: &[&str], diary: &[&str]) -> Miscellany {
        Miscellany::from_sources(READING, notes, diary).unwrap()
    }

    fn passage_markers(html: &str) -> Vec<&str> {
        html.match_indices("data-passage=\"")
            .map(|(at, marker)| {
                let rest = &html[at + marker.len()..];
                &rest[..rest.find('"').unwrap()]
            })
            .collect()
    }

    #[test]
    fn one_article_keeps_one_identity_url_and_index_entry() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            &[MOON_EN, MOON_JA, ONLY_EN, ONLY_JA, MIXED],
            &[],
        ));
        let paths: Vec<_> = pages(&content, fixture_today())
            .into_iter()
            .map(|page| page.output_path)
            .collect();
        let note_pages: Vec<_> = paths
            .iter()
            .filter(|path| {
                path.starts_with("miscellany/notes/") && *path != "miscellany/notes/index.html"
            })
            .collect();
        assert_eq!(
            note_pages,
            [
                "miscellany/notes/mixed/index.html",
                "miscellany/notes/moon/index.html",
                "miscellany/notes/only-en/index.html",
                "miscellany/notes/only-ja/index.html",
            ]
        );
        let index = notes_page(content.miscellany()).into_string();
        assert_eq!(
            index.matches(r#"href="/miscellany/notes/moon/""#).count(),
            1
        );
        assert_eq!(index.matches("<li").count(), 4);
        // The index names each note once, by its one title; it offers no
        // language choice and never hides an entry.
        assert!(index.contains(
            r#"<li lang="ja"><h2><a href="/miscellany/notes/moon/">月について</a></h2></li>"#
        ));
        assert!(!index.contains("On &lt;the&gt; moon") && !index.contains("data-language"));
        for page in pages(&content, fixture_today()) {
            assert!(!page.html.contains("hreflang"), "{}", page.output_path);
        }
    }

    #[test]
    fn separately_authored_passages_render_in_display_order_with_their_languages() {
        let notes = miscellany(&[MOON_JA, MOON_EN], &[]);
        let note = notes.public_notes().next().unwrap();
        let languages = notes.display_languages();
        assert_eq!(languages, [DisplayLanguage::En, DisplayLanguage::Ja]);
        let html = note_detail(note, &languages).into_string();
        // English before Japanese, each marked with its language; the
        // passages have different lengths, titles, and sections.
        assert_eq!(passage_markers(&html), ["en", "ja"]);
        assert!(html.contains(r#"<div class="passage" data-passage="en" lang="en"><h2 class="passage-title">On &lt;the&gt; moon</h2><div class="prose miscellany-prose"><p>A longer English text"#));
        assert!(html.contains("<h3>A section only here</h3>"));
        assert!(html.contains(r#"<div class="passage longform-ja" data-passage="ja" lang="ja"><div class="prose miscellany-prose"><h2>見えるもの</h2>"#));
        // The article keeps its one title and heading.
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains(r#"<h1 lang="ja" data-pagefind-meta="title">月について</h1>"#));
        // Footnotes of the two passages never share an identifier.
        assert!(html.contains(r##"href="#en-1""##) && html.contains(r##"id="en-1""##));
        assert!(html.contains(r##"href="#ja-1""##) && html.contains(r##"id="ja-1""##));
        // A German phrase inside the Japanese passage is marked inline only.
        assert!(html.contains(r#"ここでは <span lang="de">Stimme</span> という語を借りる。"#));
        assert!(!html.contains("Deutsch") && !html.contains(r#"data-language="de""#));
        // Both languages are present, so no offered choice can empty the
        // article and it has no notice.
        assert!(!html.contains("data-language-notice"));
    }

    #[test]
    fn the_language_control_offers_only_languages_of_designated_passages() {
        let one_language = Miscellany::from_sources("items: []", &[ONLY_EN, MIXED], &[]).unwrap();
        assert_eq!(one_language.display_languages(), [DisplayLanguage::En]);
        let html = note_detail(
            one_language.public_notes().next().unwrap(),
            &one_language.display_languages(),
        )
        .into_string();
        assert!(!html.contains("data-language-filter") && !html.contains("data-language-notice"));

        let both = miscellany(&[ONLY_EN, ONLY_JA], &[]);
        let languages = both.display_languages();
        let note = both
            .public_notes()
            .find(|n| n.metadata().slug == "only-en")
            .unwrap();
        let html = note_detail(note, &languages).into_string();
        assert!(html.contains(r#"<div class="language-filter" role="group" aria-label="Language" hidden data-language-filter><button type="button" data-language="all" aria-pressed="true">All</button><button type="button" data-language="en" lang="en" aria-pressed="false">English</button><button type="button" data-language="ja" lang="ja" aria-pressed="false">日本語</button></div>"#));

        // A German-designated passage, added later, offers Deutsch.
        let german = miscellany(&[ONLY_EN, ONLY_EN_DE, ONLY_JA], &[]);
        let languages = german.display_languages();
        assert_eq!(languages, DisplayLanguage::ALL);
        let note = german
            .public_notes()
            .find(|n| n.metadata().slug == "only-en")
            .unwrap();
        let html = note_detail(note, &languages).into_string();
        assert!(html.contains(r#"<button type="button" data-language="de" lang="de" aria-pressed="false">Deutsch</button>"#));
        assert_eq!(passage_markers(&html), ["en", "de"]);
        assert!(html.contains(r#"<div class="passage" data-passage="de" lang="de">"#));
        // Under 日本語 this note would be empty, so it carries its notice.
        assert!(html.contains("Written in English and German. <button type=\"button\" data-language-reveal>Show the text</button>"));
    }

    #[test]
    fn single_language_articles_carry_a_hidden_notice_and_mixed_passages_none() {
        let notes = miscellany(&[ONLY_EN, ONLY_JA, MIXED], &[]);
        let languages = notes.display_languages();
        let find = |slug: &str| {
            notes
                .public_notes()
                .find(|n| n.metadata().slug == slug)
                .unwrap()
        };
        let ja = note_detail(find("only-ja"), &languages).into_string();
        assert!(ja.contains(r#"<p class="language-notice" data-language-notice data-pagefind-ignore="all" hidden>Written in Japanese. <button type="button" data-language-reveal>Read in Japanese</button></p>"#));
        let en = note_detail(find("only-en"), &languages).into_string();
        assert!(en.contains("Written in English. <button type=\"button\" data-language-reveal>Read in English</button>"));
        // A mixed passage is shown in every view: no notice, no language of
        // its own, and its inline marks kept intact.
        let mixed = note_detail(find("mixed"), &languages).into_string();
        assert!(mixed.contains(r#"<div class="passage" data-passage="mixed"><div class="prose miscellany-prose"><p>This sentence begins in English, そして日本語で続き、<span lang="de">und endet auf Deutsch</span>.</p>"#));
        assert!(!mixed.contains("data-language-notice"));
        assert!(!mixed.contains("translat"));
    }

    #[test]
    fn without_javascript_every_passage_is_visible_and_the_controls_are_hidden() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            &[MOON_JA, MOON_EN, ONLY_JA],
            &[DIARY_JA, DIARY_EN],
        ));
        for page in pages(&content, fixture_today())
            .iter()
            .filter(|page| page.output_path.starts_with("miscellany/"))
        {
            let html = &page.html;
            // Only the controls and the notices start hidden.
            let hidden = html.matches(" hidden").count();
            let controls = html.matches("data-language-filter").count()
                + html.matches("data-reading-control").count()
                + html.matches("data-language-notice").count()
                + html.matches("data-search-more hidden").count()
                // The message form, shown by the site's script.
                + html.matches("data-site-message data-category").count();
            assert_eq!(hidden, controls, "{}", page.output_path);
            assert!(
                !html.contains("data-passage=\"en\" hidden")
                    && !html.contains("aria-hidden=\"true\" data-passage")
            );
        }
    }

    #[test]
    fn diary_entries_keep_their_anchors_and_explicit_passages() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            &[],
            &[DIARY_JA, DIARY_OTHER_SAME_DAY, DIARY_EN],
        ));
        let page = pages(&content, fixture_today())
            .into_iter()
            .find(|page| page.output_path == "miscellany/diary/2026/10/index.html")
            .unwrap();
        let html = page.html;
        // Two entries on one date stay two entries; the English passage joins
        // only the entry it names. The second is marked as of the same day,
        // and the list of days names the day once.
        assert_eq!(html.matches("<article").count(), 2);
        let first = html
            .find(
                r#"<article class="diary-entry" data-reading-article="diary/2026-10-10" data-reading-seen><h2 class="diary-heading" id="diary-2026-10-10">"#,
            )
            .unwrap();
        let second = html
            .find(r#"<article class="diary-entry same-day" data-reading-article="diary/2026-10-10-evening" data-reading-seen><h2 class="diary-heading" id="diary-2026-10-10-evening">"#)
            .unwrap();
        assert_eq!(
            html.matches(r##"<a href="#diary-2026-10-10" aria-label="10 October">10</a>"##)
                .count(),
            1
        );
        assert!(!html.contains(r##"<a href="#diary-2026-10-10-evening" aria-label"##));
        assert_eq!(passage_markers(&html[first..second]), ["en", "ja"]);
        assert_eq!(passage_markers(&html[second..]), ["en"]);
        assert!(html[first..second].contains("A different, shorter English entry."));
        assert!(html.contains(r##"<a class="diary-date" href="#diary-2026-10-10" aria-label="Saturday 10 October 2026""##));
        // The evening entry is English only, so it can be emptied by 日本語.
        assert!(html[second..].contains("Written in English."));
        assert!(!html[first..second].contains("data-language-notice"));
    }

    #[test]
    fn reading_commentary_is_filtered_but_the_record_never_is() {
        let reading = miscellany(&[], &[]);
        let html = reading_page(&reading).into_string();
        assert!(html.contains("data-language-filter"));
        // A Japanese title with English commentary: the work keeps its
        // language; the commentary is an English passage.
        assert!(html.contains(r#"<cite lang="ja">架空の本</cite></h2><p class="reading-author" lang="ja">作者</p><div class="passages" data-passages><p class="reading-note" data-passage="en" lang="en">My English commentary on a Japanese book.</p><p class="language-notice" data-language-notice data-pagefind-ignore="all" hidden>Written in English."#));
        // Two commentaries, English first, never a notice.
        assert!(html.contains(r#"<p class="reading-note" data-passage="en" lang="en">An English comment, not a translation.</p><p class="reading-note" data-passage="ja" lang="ja">日本語の感想。</p></div>"#));
        // A work without commentary is a plain record.
        let plain = &html[html.find("No Commentary").unwrap()..];
        let plain = &plain[..plain.find("</li>").unwrap()];
        assert!(!plain.contains("passages") && !plain.contains("notice"));
        assert_eq!(html.matches("<li data-reading-article=").count(), 3);
    }

    #[test]
    fn the_editorial_note_appears_once_multilingual_text_is_published() {
        const NOTE: &str = super::MULTILINGUAL_NOTE;
        // Separate single-language entries are not multilingual articles.
        let separate = miscellany(&[ONLY_EN, ONLY_JA], &[]);
        let reading_only = Miscellany::from_sources(
            "items:\n  - title: X\n    note: English only.\n",
            &[ONLY_EN, ONLY_JA],
            &[],
        )
        .unwrap();
        assert!(
            !miscellany_overview(&reading_only, &[])
                .into_string()
                .contains(NOTE)
        );
        // The fixture reading list already has a work with two commentaries.
        assert!(
            miscellany_overview(&separate, &[])
                .into_string()
                .contains(NOTE)
        );
        let empty = Miscellany::default();
        assert!(
            !miscellany_overview(&empty, &[])
                .into_string()
                .contains(NOTE)
        );
        let html = miscellany_overview(&miscellany(&[MOON_JA, MOON_EN], &[]), &[]).into_string();
        assert_eq!(html.matches(NOTE).count(), 1);
        assert!(html.contains(&format!(
            r#"</h1></div><p class="miscellany-description">{NOTE}</p></header><form"#
        )));
    }
}

#[cfg(test)]
mod sketches_and_search_tests {
    use super::{fixture_today, pages, reading_anchors, sitemap};
    use crate::content::{Miscellany, ValidatedSiteContent};
    use crate::validate::SEARCHABLE_PAGES;
    use regex::Regex;

    // Made-up texts, parsed through the real validation and never written to
    // the content directory. Every draft carries a word that must appear on
    // no page and in no attribute of any page.
    const SKETCH_NEW: &str = "---\nslug: on-borrowed-words\ntitle: 借りた言葉について\nlang: ja\ndate: 2026-09-19\ndescription: 合成された説明文。\nmath: true\ndraft: false\n---\n\n## 一\n\n[Begriff]{lang=de} と [jugement]{lang=fr} を並べてみる。$x \\le y$。[^a]\n\n> 引用。\n\n[^a]: 注。\n";
    const SKETCH_OLD: &str = "---\nslug: an-older-piece\ntitle: An older piece\nlang: en\ndate: 2026-03-02\ndraft: false\n---\n\nPlain text.\n";
    const SKETCH_DRAFT: &str = "---\nslug: unpublished-sketch\ntitle: CANARY-SKETCH-TITLE\nlang: en\ndate: 2026-10-01\n---\n\nCANARY-SKETCH-BODY\n";
    const NOTE_DRAFT: &str = "---\nslug: unpublished-note\ntitle: CANARY-NOTE-TITLE\nlang: en\n---\n\nCANARY-NOTE-BODY\n";
    const NOTE: &str =
        "---\nslug: a-note\ntitle: A note\nlang: en\ndraft: false\n---\n\nNote text.\n";
    const DIARY: &str = "---\ndate: 2026-10-10\nlang: en\ndraft: false\n---\n\nDiary text.\n";
    const DIARY_DRAFT: &str = "---\ndate: 2026-11-01\nlang: en\n---\n\nCANARY-DIARY-BODY\n";
    const READING: &str = "items:\n  - title: 架空の本\n    lang: ja\n  - title: Another\n    author: Someone\n  - title: 架空の本\n    lang: ja\n";

    fn content() -> ValidatedSiteContent {
        ValidatedSiteContent::with_miscellany(
            Miscellany::from_all_sources(
                READING,
                &[NOTE, NOTE_DRAFT],
                &[DIARY, DIARY_DRAFT],
                &[SKETCH_OLD, SKETCH_DRAFT, SKETCH_NEW],
            )
            .unwrap(),
        )
    }

    fn page(content: &ValidatedSiteContent, output_path: &str) -> String {
        pages(content, fixture_today())
            .into_iter()
            .find(|page| page.output_path == output_path)
            .unwrap_or_else(|| panic!("missing {output_path}"))
            .html
    }

    #[test]
    fn sketches_are_listed_newest_first_with_their_dates_and_have_pages() {
        let content = content();
        let index = page(&content, "miscellany/sketches/index.html");
        assert!(index.contains("<h1>Sketches</h1>"));
        let newer = index
            .find(r#"<li lang="ja"><h2><a href="/miscellany/sketches/on-borrowed-words/">借りた言葉について</a></h2><p class="miscellany-date" lang="en"><time datetime="2026-09-19">19 September 2026</time></p><p>合成された説明文。</p></li>"#)
            .unwrap();
        let older = index
            .find(r#"<li lang="en"><h2><a href="/miscellany/sketches/an-older-piece/">An older piece</a></h2><p class="miscellany-date" lang="en"><time datetime="2026-03-02">2 March 2026</time></p></li>"#)
            .unwrap();
        assert!(newer < older);
        // The index adds no description of its own for the section.
        assert!(index.contains("<h1>Sketches</h1>"));
        assert!(index.contains("</nav></header>") && !index.contains("miscellany-description"));

        let html = page(&content, "miscellany/sketches/on-borrowed-words/index.html");
        assert!(html.contains(r#"<link rel="canonical" href="https://ryujin-hatakeyama.github.io/miscellany/sketches/on-borrowed-words/">"#));
        assert!(html.contains(r#"<meta property="og:type" content="article">"#));
        assert!(html.contains(r#"<article class="miscellany-note" data-pagefind-body data-pagefind-filter="section:Sketches" data-reading-article="sketches/on-borrowed-words"><header><h1 lang="ja" data-pagefind-meta="title">借りた言葉について</h1><p class="article-date" lang="en" data-pagefind-ignore><time datetime="2026-09-19" data-pagefind-meta="date[datetime]">19 September 2026</time></p><p class="summary" lang="ja">合成された説明文。</p></header>"#));
        // The same Markdown as Notes: headings, inline languages, math,
        // quotations, and footnotes.
        for fragment in [
            "<h2>一</h2>",
            r#"<span lang="de">Begriff</span>"#,
            r#"<span lang="fr">jugement</span>"#,
            r#"<span class="katex">"#,
            "<blockquote>",
            r#"class="footnote-definition""#,
        ] {
            assert!(html.contains(fragment), "{fragment}");
        }
        assert!(html.contains(r#"<a href="/miscellany/">Miscellany</a><span class="miscellany-trail-separator" aria-hidden="true">/</span><a href="/miscellany/sketches/">Sketches</a>"#));
        assert!(!html.contains(r#"<a href="/miscellany/sketches/" aria-current"#));
    }

    #[test]
    fn sketch_slugs_must_be_unique_and_valid() {
        let duplicate =
            Miscellany::from_all_sources("items: []", &[], &[], &[SKETCH_OLD, SKETCH_OLD]);
        assert!(format!("{:#}", duplicate.unwrap_err()).contains("sketch slug"));
        let undated = "---\nslug: undated\ntitle: x\nlang: en\ndraft: false\n---\n\nx\n";
        assert!(Miscellany::from_all_sources("items: []", &[], &[], &[undated]).is_err());
        let bad = "---\nslug: Bad_Slug\ntitle: x\nlang: en\ndate: 2026-01-01\n---\n\nx\n";
        assert!(Miscellany::from_all_sources("items: []", &[], &[], &[bad]).is_err());
        // A sketch and a note may share a slug: their addresses differ.
        let note = "---\nslug: an-older-piece\ntitle: x\nlang: en\n---\n\nx\n";
        assert!(Miscellany::from_all_sources("items: []", &[note], &[], &[SKETCH_OLD]).is_ok());
    }

    #[test]
    fn drafts_appear_on_no_page_and_only_listed_pages_are_marked_for_search() {
        let content = content();
        let all = pages(&content, fixture_today());
        let listed: Vec<_> = SEARCHABLE_PAGES
            .iter()
            .map(|pattern| Regex::new(pattern).unwrap())
            .collect();
        let mut marked = Vec::new();
        for page in &all {
            for canary in ["CANARY", "unpublished-", "2026/11"] {
                assert!(
                    !page.html.contains(canary),
                    "{} shows {canary}",
                    page.output_path
                );
            }
            let markers = page.html.matches("data-pagefind-body").count();
            if markers > 0 {
                assert_eq!(markers, 1, "{}", page.output_path);
                assert!(
                    listed
                        .iter()
                        .any(|pattern| pattern.is_match(&page.output_path)),
                    "{} is marked for search",
                    page.output_path
                );
                assert!(!page.html.contains("noindex"), "{}", page.output_path);
                marked.push(page.output_path.as_str());
            }
        }
        marked.sort_unstable();
        assert_eq!(
            marked,
            [
                "miscellany/diary/2026/10/index.html",
                "miscellany/notes/a-note/index.html",
                "miscellany/reading/index.html",
                "miscellany/sketches/an-older-piece/index.html",
                "miscellany/sketches/on-borrowed-words/index.html",
            ]
        );
        // Each text is on one marked page only: notes and sketches on their
        // own pages, diary entries on their month's, never on an index.
        for (text, page) in [
            ("Note text.", "miscellany/notes/a-note/index.html"),
            ("Diary text.", "miscellany/diary/2026/10/index.html"),
            (
                "Plain text.",
                "miscellany/sketches/an-older-piece/index.html",
            ),
        ] {
            let holders: Vec<_> = all
                .iter()
                .filter(|candidate| candidate.html.contains(text))
                .map(|candidate| candidate.output_path.as_str())
                .collect();
            assert_eq!(holders, [page], "{text}");
        }
    }

    #[test]
    fn the_search_page_is_neither_indexed_nor_in_the_sitemap() {
        let content = content();
        let all = pages(&content, fixture_today());
        let search = all
            .iter()
            .find(|page| page.output_path == "miscellany/search/index.html")
            .unwrap();
        assert!(
            search
                .html
                .contains(r#"<meta name="robots" content="noindex, nofollow">"#)
        );
        assert!(!search.html.contains("data-pagefind-body"));
        // A plain form: it works as a request for the page without the script.
        assert!(search.html.contains(r#"<form class="search-form" action="/miscellany/search/" method="get" role="search" aria-label="Search Miscellany" data-search-form>"#));
        for (value, label) in [
            ("", "All"),
            ("reading", "Reading"),
            ("notes", "Notes"),
            ("diary", "Diary"),
            ("sketches", "Sketches"),
        ] {
            assert!(
                search.html.contains(&format!(
                    r#"<input type="radio" name="section" value="{value}""#
                )),
                "{label}"
            );
            assert!(
                search.html.contains(&format!("<span>{label}</span>")),
                "{label}"
            );
        }
        assert!(search.html.contains("<noscript>"));
        let (map, _) = sitemap(&all);
        assert!(!map.contains("/miscellany/search/"));
        assert!(map.contains("/miscellany/sketches/on-borrowed-words/"));
        assert!(map.contains("/miscellany/diary/2026/10/"));
    }

    #[test]
    fn reading_items_have_stable_distinct_anchors() {
        let content = content();
        let reading = content.miscellany().reading();
        let anchors = reading_anchors(reading);
        assert_eq!(anchors.len(), 3);
        assert!(anchors.iter().all(|anchor| {
            Regex::new(r"^reading-[0-9a-f]{8}(-[0-9]+)?$")
                .unwrap()
                .is_match(anchor)
        }));
        // The same title and author twice: told apart by order.
        assert_eq!(anchors[2], format!("{}-2", anchors[0]));
        assert_ne!(anchors[0], anchors[1]);
        // Derived from the work only, so adding another item changes nothing.
        let more =
            Miscellany::from_all_sources(&format!("{READING}  - title: New\n"), &[], &[], &[])
                .unwrap();
        assert_eq!(&reading_anchors(more.reading())[..3], &anchors[..]);
        let html = page(&content, "miscellany/reading/index.html");
        assert!(html.contains(&format!(
            r#"<h2 class="reading-title" id="{}" data-reading-target><cite lang="ja">架空の本</cite></h2>"#,
            anchors[0]
        )));
        assert!(html.contains(r#"<ol class="reading-list" data-pagefind-body data-pagefind-filter="section:Reading" data-pagefind-meta="title:Reading">"#));
    }
}

#[cfg(test)]
mod timeline_tests {
    //! The one Updates timeline, from fictional update records and articles
    //! parsed through the real validation. Nothing here is written to the
    //! content directory.

    use super::pages;
    use crate::content::{Miscellany, ValidatedSiteContent};
    use crate::model::Update;
    use chrono::NaiveDate;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn today() -> NaiveDate {
        day(2026, 10, 21)
    }

    fn activity(id: &str, date: &str, status: &str, category: &str) -> Update {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","title":{{"en":"Attended event {id}"}},"summary":{{"en":"x"}},"date":"{date}",
            "eventStatus":"{status}","categories":["{category}"],
            "kind":{{"type":"participation"}},"related":{{}}}}"#
        ))
        .unwrap()
    }

    fn published(day: Option<&str>) -> String {
        day.map(|day| format!("published: {day}\n"))
            .unwrap_or_default()
    }

    fn note(slug: &str, title: &str, day: Option<&str>) -> String {
        format!(
            "---\nslug: {slug}\ntitle: {title}\nlang: en\n{}draft: false\n---\n\nText.\n",
            published(day)
        )
    }

    fn draft_note(slug: &str, title: &str, day: &str) -> String {
        format!(
            "---\nslug: {slug}\ntitle: {title}\nlang: en\npublished: {day}\n---\n\nPRIVATE-BODY\n"
        )
    }

    fn sketch(slug: &str, title: &str, date: &str, day: Option<&str>) -> String {
        format!(
            "---\nslug: {slug}\ntitle: {title}\nlang: ja\ndate: {date}\n{}draft: false\n---\n\n本文。\n",
            published(day)
        )
    }

    fn diary(date: &str, title: Option<&str>, day: Option<&str>) -> String {
        let title = title.map(|t| format!("title: {t}\n")).unwrap_or_default();
        format!(
            "---\ndate: {date}\nlang: en\n{title}{}draft: false\n---\n\nAn entry.\n",
            published(day)
        )
    }

    const READING: &str = "items:\n  - id: conservativity\n    title: Conservativity of Definitional Extension\n    published: 2026-10-12\n  - title: An item without a publication day\n";

    fn site(
        updates: Vec<Update>,
        reading: &str,
        notes: &[String],
        diary: &[String],
        sketches: &[String],
    ) -> ValidatedSiteContent {
        try_site(updates, reading, notes, diary, sketches).unwrap()
    }

    fn try_site(
        updates: Vec<Update>,
        reading: &str,
        notes: &[String],
        diary: &[String],
        sketches: &[String],
    ) -> anyhow::Result<ValidatedSiteContent> {
        fn refs(sources: &[String]) -> Vec<&str> {
            sources.iter().map(String::as_str).collect()
        }
        let miscellany =
            Miscellany::from_all_sources(reading, &refs(notes), &refs(diary), &refs(sketches))?;
        ValidatedSiteContent::with_updates_and_miscellany(updates, miscellany)
    }

    fn page(content: &ValidatedSiteContent, output_path: &str, on: NaiveDate) -> String {
        pages(content, on)
            .into_iter()
            .find(|page| page.output_path == output_path)
            .unwrap_or_else(|| panic!("missing {output_path}"))
            .html
    }

    /// The feed stays one of update records: articles are announced on the
    /// pages, never in it.
    fn feed(content: &ValidatedSiteContent) -> String {
        let feed = super::rss(content.updates());
        assert!(!feed.contains("/miscellany/") && !feed.contains("Published “"));
        feed
    }

    fn between<'a>(html: &'a str, start: &str, end: &str) -> &'a str {
        let Some(from) = html.find(start) else {
            return "";
        };
        let rest = &html[from..];
        &rest[..rest.find(end).unwrap()]
    }

    fn all_between<'a>(html: &'a str, open: &str, close: &str) -> Vec<&'a str> {
        html.split(open)
            .skip(1)
            .map(|part| &part[..part.find(close).unwrap()])
            .collect()
    }

    /// The entries of each view, as the text each shows: the home page's
    /// Recent Updates, the Updates page's, and the Miscellany overview's.
    struct Views {
        home: Vec<String>,
        updates: Vec<String>,
        miscellany: Vec<String>,
    }

    fn views(content: &ValidatedSiteContent, on: NaiveDate) -> Views {
        let home = page(content, "index.html", on);
        let updates = page(content, "updates/index.html", on);
        let miscellany = page(content, "miscellany/index.html", on);
        let updates_main = between(&updates, "<main", "</main>");
        // With upcoming or earlier plans present, completed entries have
        // their own section; otherwise the page is one list.
        let recent = if updates_main.contains(r#"aria-labelledby="recent-heading""#) {
            between(
                updates_main,
                r#"aria-labelledby="recent-heading""#,
                "</section>",
            )
        } else {
            updates_main
        };
        let owned = |items: Vec<&str>| items.into_iter().map(str::to_owned).collect();
        Views {
            home: owned(all_between(
                between(&home, r#"aria-labelledby="updates-heading""#, "</section>"),
                "<h3>",
                "</h3>",
            )),
            updates: owned(all_between(recent, "<h3>", "</h3>")),
            miscellany: owned(all_between(
                between(&miscellany, r#"<ol class="recent-updates">"#, "</ol>"),
                "<p>",
                "</p>",
            )),
        }
    }

    fn update_ids(html: &str) -> Vec<&str> {
        all_between(html, r#"<article id="update-"#, "\"")
    }

    fn assert_same_order(views: &Views) {
        assert!(!views.updates.is_empty());
        assert_eq!(
            views.home,
            views.updates[..views.updates.len().min(5)],
            "home"
        );
        assert_eq!(
            views.miscellany,
            views.updates[..views.updates.len().min(3)],
            "miscellany"
        );
    }

    const CONSERVATIVITY: &str = r#"Published “<a href="/miscellany/reading/#reading-conservativity">Conservativity of Definitional Extension</a>” in Reading."#;
    const MODAL: &str = r#"Published “<a href="/miscellany/notes/modal-logic/" lang="en">Some Notes on Modal Logic</a>” in Notes."#;

    #[test]
    fn only_update_records() {
        let content = site(
            vec![
                activity("older", "2026-09-01", "completed", "academia"),
                activity("newer", "2026-10-01", "completed", "research"),
                activity("future", "2026-11-01", "planned", "academia"),
            ],
            "items: []\n",
            &[note("no-date", "A note without a publication day", None)],
            &[],
            &[],
        );
        assert!(content.article_updates().is_empty());
        let views = views(&content, today());
        assert_eq!(
            views.updates,
            ["Attended event newer", "Attended event older"]
        );
        assert_same_order(&views);
        let miscellany = page(&content, "miscellany/index.html", today());
        assert!(miscellany.contains(
            r#"<time datetime="2026-10-01">01 Oct 2026</time><p>Attended event newer</p>"#
        ));
        // Planned events stay under Upcoming, never in the overview.
        assert!(!miscellany.contains("event future"));
    }

    #[test]
    fn only_published_articles_in_the_authors_words() {
        let content = site(
            Vec::new(),
            READING,
            &[note(
                "modal-logic",
                "Some Notes on Modal Logic",
                Some("2026-10-14"),
            )],
            &[diary("2026-10-02", None, Some("2026-10-05"))],
            &[sketch(
                "kaze",
                "風の<下書き> & 線",
                "2026-09-30",
                Some("2026-10-03"),
            )],
        );
        let views = views(&content, today());
        assert_eq!(
            views.updates,
            [
                MODAL,
                CONSERVATIVITY,
                r#"Published an entry for <a href="/miscellany/diary/2026/10/#diary-2026-10-02">2 October 2026</a> in Diary."#,
                r#"Published “<a href="/miscellany/sketches/kaze/" lang="ja">風の&lt;下書き&gt; &amp; 線</a>” in Sketches."#,
            ]
        );
        assert_same_order(&views);
        let updates = page(&content, "updates/index.html", today());
        assert_eq!(
            update_ids(&updates),
            [
                "published-notes-modal-logic",
                "published-reading-conservativity",
                "published-diary-2026-10-02",
                "published-sketches-kaze",
            ]
        );
        // Listed by the day of publication, labelled Writing, with no kind.
        assert!(updates.contains(r#"<article id="update-published-notes-modal-logic"><div class="update-meta"><time datetime="2026-10-14">14 Oct 2026</time><span class="category-label">Writing</span></div><div class="update-copy"><h3>Published"#));
        // The diary entry is listed when it was published, not on its date.
        assert!(updates.contains(r#"<time datetime="2026-10-05">05 Oct 2026</time>"#));
        assert!(!updates.contains(r#"datetime="2026-10-02""#));
        // The sketch likewise, not on its own date.
        assert!(!updates.contains(r#"datetime="2026-09-30""#));
    }

    #[test]
    fn records_and_articles_form_one_timeline_in_every_view() {
        let content = site(
            vec![
                activity("talk", "2026-10-13", "completed", "research"),
                activity("workshop", "2026-10-04", "completed", "academia"),
                activity("summer", "2026-08-01", "completed", "academia"),
                activity("visit", "2026-12-01", "planned", "academia"),
            ],
            READING,
            &[note(
                "modal-logic",
                "Some Notes on Modal Logic",
                Some("2026-10-14"),
            )],
            &[diary("2026-10-02", Some("Rain"), Some("2026-10-05"))],
            &[],
        );
        let views = views(&content, today());
        assert_eq!(views.updates.len(), 6);
        assert_eq!(
            update_ids(between(
                &page(&content, "updates/index.html", today()),
                r#"aria-labelledby="recent-heading""#,
                "</section>"
            )),
            [
                "published-notes-modal-logic",
                "talk",
                "published-reading-conservativity",
                "published-diary-2026-10-02",
                "workshop",
                "summer",
            ]
        );
        assert_same_order(&views);
        // Activities are kept beside the articles in the short previews.
        assert!(views.miscellany.iter().any(|entry| entry.contains("talk")));
        // The feed keeps to the update records, as before.
        let feed = feed(&content);
        assert_eq!(
            all_between(&feed, "<guid>", "</guid>"),
            [
                "https://ryujin-hatakeyama.github.io/updates/#update-visit",
                "https://ryujin-hatakeyama.github.io/updates/#update-talk",
                "https://ryujin-hatakeyama.github.io/updates/#update-workshop",
                "https://ryujin-hatakeyama.github.io/updates/#update-summer",
            ]
        );
    }

    #[test]
    fn with_no_articles_and_no_updates_the_overview_keeps_its_search() {
        let content = site(Vec::new(), "items: []\n", &[], &[], &[]);
        let miscellany = page(&content, "miscellany/index.html", today());
        assert!(
            miscellany.contains(r#"<form class="miscellany-search" action="/miscellany/search/""#)
        );
        assert!(!miscellany.contains("Recent Updates"));
        assert!(!miscellany.contains("recent-updates"));
        let updates = page(&content, "updates/index.html", today());
        assert!(updates.contains("No updates yet."));
    }

    #[test]
    fn entries_of_one_day_keep_one_order_in_every_view_and_build() {
        let content = site(
            vec![
                activity("b-meeting", "2026-10-10", "completed", "academia"),
                activity("z-seminar", "2026-10-10", "completed", "research"),
            ],
            "items: []\n",
            &[
                note("zeta", "Zeta", Some("2026-10-10")),
                note("alpha", "Alpha", Some("2026-10-10")),
            ],
            &[],
            &[sketch("mid", "Mid", "2026-10-10", Some("2026-10-10"))],
        );
        let page_ids = |on| {
            update_ids(&page(&content, "updates/index.html", on))
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        // Same day: by id.
        assert_eq!(
            page_ids(today()),
            [
                "b-meeting",
                "published-notes-alpha",
                "published-notes-zeta",
                "published-sketches-mid",
                "z-seminar",
            ]
        );
        assert_same_order(&views(&content, today()));
        // Building again gives the same pages, byte for byte.
        let first: Vec<_> = pages(&content, today())
            .into_iter()
            .map(|p| p.html)
            .collect();
        let second: Vec<_> = pages(&content, today())
            .into_iter()
            .map(|p| p.html)
            .collect();
        assert_eq!(first, second);
        assert_eq!(feed(&content), feed(&content));
    }

    #[test]
    fn an_article_published_with_an_earlier_day_takes_its_place_by_that_day() {
        let recent: Vec<_> = (1..=6)
            .map(|n| {
                activity(
                    &format!("recent-{n}"),
                    &format!("2026-10-{:02}", n + 10),
                    "completed",
                    "academia",
                )
            })
            .collect();
        let content = site(
            recent,
            "items: []\n",
            &[note("old-note", "Old note", Some("2025-01-15"))],
            &[],
            &[],
        );
        let views = views(&content, today());
        assert_eq!(views.updates.len(), 7);
        assert!(views.updates.last().unwrap().contains("Old note"));
        // Not pushed to the top of the previews for being new to the site.
        assert!(views.home.iter().all(|entry| !entry.contains("Old note")));
        assert!(
            views
                .miscellany
                .iter()
                .all(|entry| !entry.contains("Old note"))
        );
        assert_same_order(&views);
        let writing = page(&content, "updates/writing/index.html", today());
        assert!(writing.contains(r#"<time datetime="2025-01-15">15 Jan 2025</time>"#));
    }

    #[test]
    fn a_corrected_publication_day_moves_the_one_entry() {
        let records = || {
            vec![
                activity("first", "2026-10-01", "completed", "academia"),
                activity("second", "2026-10-08", "completed", "academia"),
            ]
        };
        let before = site(
            records(),
            "items: []\n",
            &[note("n", "N", Some("2026-10-09"))],
            &[],
            &[],
        );
        let after = site(
            records(),
            "items: []\n",
            &[note("n", "N", Some("2026-10-02"))],
            &[],
            &[],
        );
        let ids = |content: &ValidatedSiteContent| {
            update_ids(&page(content, "updates/index.html", today()))
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&before), ["published-notes-n", "second", "first"]);
        assert_eq!(ids(&after), ["second", "published-notes-n", "first"]);
        let html = page(&after, "updates/index.html", today());
        assert!(html.contains(r#"<time datetime="2026-10-02">02 Oct 2026</time>"#));
        assert!(!html.contains("2026-10-09"));
    }

    #[test]
    fn publishing_again_or_renaming_keeps_one_entry_under_one_id() {
        let source = note("stable", "First title", Some("2026-10-10"));
        // The same file published twice over is the same article.
        let again = site(Vec::new(), "items: []\n", &[source.clone()], &[], &[]);
        let html = page(&again, "updates/index.html", today());
        assert_eq!(update_ids(&html), ["published-notes-stable"]);
        assert_eq!(html.matches("First title").count(), 1);
        assert!(!feed(&again).contains("<item>"));
        // Two files claiming one note are refused rather than listed twice.
        assert!(
            try_site(
                Vec::new(),
                "items: []\n",
                &[source.clone(), source],
                &[],
                &[]
            )
            .is_err()
        );

        let renamed = site(
            Vec::new(),
            "items: []\n",
            &[note("stable", "Second title", Some("2026-10-10"))],
            &[],
            &[],
        );
        let html = page(&renamed, "updates/index.html", today());
        assert_eq!(update_ids(&html), ["published-notes-stable"]);
        assert!(html.contains(r#"Published “<a href="/miscellany/notes/stable/" lang="en">Second title</a>” in Notes."#));
        assert!(!html.contains("First title"));
        // A reading item is named by its id, so a corrected title keeps both
        // its anchor and its entry.
        let corrected = site(
            Vec::new(),
            &READING.replace("Conservativity of", "Conservativity in"),
            &[],
            &[],
            &[],
        );
        let reading = page(&corrected, "miscellany/reading/index.html", today());
        assert!(reading.contains(r#"id="reading-conservativity""#));
        assert_eq!(
            update_ids(&page(&corrected, "updates/index.html", today())),
            ["published-reading-conservativity"]
        );
    }

    #[test]
    fn drafts_undated_and_future_articles_are_not_announced() {
        let content = site(
            vec![activity("done", "2026-10-01", "completed", "academia")],
            "items: []\n",
            &[
                draft_note("private", "PRIVATE-TITLE", "2026-10-10"),
                note("undated", "UNDATED-TITLE", None),
                note("scheduled", "Scheduled", Some("2026-10-25")),
            ],
            &[diary("2026-10-10", Some("UNDATED-DIARY"), None)],
            &[],
        );
        for on in [today(), day(2026, 10, 24)] {
            for path in [
                "index.html",
                "updates/index.html",
                "updates/writing/index.html",
                "miscellany/index.html",
            ] {
                let html = page(&content, path, on);
                for canary in ["PRIVATE", "UNDATED", "Scheduled"] {
                    assert!(!html.contains(canary), "{path} on {on}: {canary}");
                }
            }
            assert!(!feed(&content).contains("Scheduled"));
        }
        // From its publication day on, in Tokyo, the scheduled note is listed.
        let on = day(2026, 10, 25);
        let views = views(&content, on);
        assert_eq!(
            views.updates[0],
            r#"Published “<a href="/miscellany/notes/scheduled/" lang="en">Scheduled</a>” in Notes."#
        );
        assert_same_order(&views);
        assert!(!feed(&content).contains("PRIVATE"));
    }

    #[test]
    fn articles_are_writing_and_records_keep_their_categories() {
        let content = site(
            vec![
                activity("talk", "2026-10-13", "completed", "research"),
                activity("workshop", "2026-10-04", "completed", "academia"),
            ],
            READING,
            &[],
            &[],
            &[],
        );
        let ids = |path| {
            update_ids(&page(&content, path, today()))
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            ids("updates/index.html"),
            ["talk", "published-reading-conservativity", "workshop"]
        );
        assert_eq!(ids("updates/research/index.html"), ["talk"]);
        assert_eq!(
            ids("updates/writing/index.html"),
            ["published-reading-conservativity"]
        );
        assert_eq!(ids("updates/academia/index.html"), ["workshop"]);
        // The filters themselves are unchanged.
        let all = page(&content, "updates/index.html", today());
        assert!(all.contains(r#"<nav class="update-filters" aria-label="Update categories"><a href="/updates/" aria-current="page">All</a><a href="/updates/research/">Research</a><a href="/updates/writing/">Writing</a></nav>"#));
    }

    #[test]
    fn every_article_entry_links_to_its_article() {
        let content = site(
            Vec::new(),
            READING,
            &[note(
                "modal-logic",
                "Some Notes on Modal Logic",
                Some("2026-10-14"),
            )],
            &[
                diary("2026-10-02", None, Some("2026-10-05")),
                "---\ndate: 2026-10-02\nslug: evening\nlang: en\npublished: 2026-10-06\ndraft: false\n---\n\nLater.\n"
                    .to_owned(),
            ],
            &[sketch("kaze", "風", "2026-09-30", Some("2026-10-03"))],
        );
        let all = pages(&content, today());
        assert_eq!(content.article_updates().len(), 5);
        for article in content.article_updates() {
            let (path, fragment) = article
                .path()
                .split_once('#')
                .unwrap_or((article.path(), ""));
            let target = all
                .iter()
                .find(|page| page.public_path == path)
                .unwrap_or_else(|| panic!("no page at {path}"));
            if !fragment.is_empty() {
                assert!(
                    target.html.contains(&format!(r#"id="{fragment}""#)),
                    "{}",
                    article.path()
                );
            }
        }
    }

    #[test]
    fn ids_and_publication_days_are_checked() {
        // An update record may not take an article's id.
        assert!(
            try_site(
                vec![activity(
                    "published-notes-n",
                    "2026-10-01",
                    "completed",
                    "academia"
                )],
                "items: []\n",
                &[note("n", "N", Some("2026-10-02"))],
                &[],
                &[],
            )
            .is_err()
        );
        // Not even a draft's, which would collide once it is published.
        assert!(
            try_site(
                vec![activity(
                    "published-notes-n",
                    "2026-10-01",
                    "completed",
                    "academia"
                )],
                "items: []\n",
                &[draft_note("n", "N", "2026-10-02")],
                &[],
                &[],
            )
            .is_err()
        );
        for reading in [
            // A reading item is announced only under a stable id.
            "items:\n  - title: T\n    published: 2026-10-01\n",
            "items:\n  - id: Not a slug\n    title: T\n",
            "items:\n  - id: same\n    title: A\n  - id: same\n    title: B\n",
            "items:\n  - id: t\n    title: T\n    published: 2026-02-30\n",
        ] {
            assert!(
                try_site(Vec::new(), reading, &[], &[], &[]).is_err(),
                "{reading}"
            );
        }
        // A companion passage cannot carry the article's publication day.
        let companion = "---\nof: n\nlang: ja\npublished: 2026-10-02\n---\n\n本文。\n".to_owned();
        assert!(
            try_site(
                Vec::new(),
                "items: []\n",
                &[note("n", "N", None), companion],
                &[],
                &[]
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod reading_state_tests {
    //! The markup the visitor's reading marks rely on (client/site.ts), from
    //! fictional articles that are never written to the content directory.

    use super::pages;
    use crate::content::{Miscellany, ValidatedSiteContent};
    use chrono::NaiveDate;

    const NOTE: &str = "---\nslug: modal-logic\ntitle: Some Notes on Modal Logic\nlang: en\npublished: 2026-10-14\ndraft: false\n---\n\nText.\n";
    const SKETCH: &str =
        "---\nslug: kaze\ntitle: 風\nlang: ja\ndate: 2026-10-01\ndraft: false\n---\n\n本文。\n";
    const DIARY: &str = "---\ndate: 2026-10-02\nlang: en\ndraft: false\n---\n\nAn entry.\n";
    const DIARY_TITLED: &str = "---\ndate: 2026-10-02\nslug: evening\nlang: en\ntitle: Evening\ndraft: false\n---\n\nLater.\n";
    const READING: &str = "items:\n  - id: conservativity\n    title: Conservativity of Definitional Extension\n  - title: Without an id\n";

    fn site() -> Vec<crate::render::GeneratedPage> {
        let miscellany =
            Miscellany::from_all_sources(READING, &[NOTE], &[DIARY, DIARY_TITLED], &[SKETCH])
                .unwrap();
        pages(
            &ValidatedSiteContent::with_miscellany(miscellany),
            NaiveDate::from_ymd_opt(2026, 10, 21).unwrap(),
        )
    }

    fn page<'a>(all: &'a [crate::render::GeneratedPage], path: &str) -> &'a str {
        &all.iter()
            .find(|page| page.output_path == path)
            .unwrap()
            .html
    }

    const CONTROL: &str = r#"<p class="reading-control" data-reading-control data-pagefind-ignore="all" hidden><button type="button" aria-pressed="false">Mark as read</button></p>"#;

    #[test]
    fn articles_carry_the_keys_their_links_lead_to() {
        let all = site();
        // A note's and a sketch's pages: the key from the address, and one
        // hidden control after the article, outside what the index reads.
        for (path, key) in [
            (
                "miscellany/notes/modal-logic/index.html",
                "notes/modal-logic",
            ),
            ("miscellany/sketches/kaze/index.html", "sketches/kaze"),
        ] {
            let html = page(&all, path);
            assert!(
                html.contains(&format!(r#" data-reading-article="{key}">"#)),
                "{path}"
            );
            assert_eq!(html.matches(CONTROL).count(), 1, "{path}");
            let article_end = html.find("</article>").unwrap();
            assert!(html.find(CONTROL).unwrap() > article_end, "{path}");
            // Opening the page is the visit: it is not one of those counted
            // by time on screen.
            assert!(!html.contains("data-reading-seen"), "{path}");
        }
        // The lists link to those addresses, and mark their links.
        assert!(page(&all, "miscellany/notes/index.html").contains(r#"<ol class="miscellany-list" data-reading-list><li lang="en"><h2><a href="/miscellany/notes/modal-logic/">"#));
        assert!(page(&all, "miscellany/sketches/index.html").contains(r#"<ol class="miscellany-list sketch-list" data-reading-list><li lang="ja"><h2><a href="/miscellany/sketches/kaze/">"#));
        // Diary entries share their month's page: each its own key, counted
        // as seen on screen, with its own control inside the entry.
        let month = page(&all, "miscellany/diary/2026/10/index.html");
        assert!(month.contains(r#"<article class="diary-entry" data-reading-article="diary/2026-10-02" data-reading-seen>"#));
        assert!(
            month.contains(r#"data-reading-article="diary/2026-10-02-evening" data-reading-seen>"#)
        );
        assert!(
            month.contains(
                r#"<span class="diary-title" lang="en" data-reading-target>Evening</span>"#
            )
        );
        assert_eq!(month.matches(CONTROL).count(), 2);
        // The month's row on the Diary page knows its entries, not itself.
        assert!(
            page(&all, "miscellany/diary/index.html")
                .contains(r#"data-reading-entries="diary/2026-10-02 diary/2026-10-02-evening""#)
        );
        // Reading records: by id when there is one, else by the hash anchor.
        let reading = page(&all, "miscellany/reading/index.html");
        assert!(reading.contains(r#"<li data-reading-article="reading/conservativity" data-reading-seen><h2 class="reading-title" id="reading-conservativity" data-reading-target>"#));
        assert_eq!(
            reading.matches("data-reading-article=\"reading/").count(),
            2
        );
        assert!(!reading.contains("data-reading-control"));
        // Recent Updates links to the article's own address, from which the
        // script derives the same key.
        assert!(
            page(&all, "miscellany/index.html")
                .contains(r#"Published “<a href="/miscellany/notes/modal-logic/" lang="en">"#)
        );
        // Section and overview pages are not articles.
        for path in [
            "miscellany/index.html",
            "miscellany/notes/index.html",
            "miscellany/diary/index.html",
        ] {
            assert!(!page(&all, path).contains("data-reading-article"), "{path}");
        }
    }

    #[test]
    fn the_privacy_page_says_where_the_record_is_kept() {
        let all = site();
        let privacy = page(&all, "privacy/index.html");
        for words in [
            "kept only in your browser’s storage on this device",
            "never sent to this site, to GoatCounter, or to anyone else",
            "Safari may also remove it",
        ] {
            assert!(privacy.contains(words), "{words}");
        }
    }
}

// ------------------------------------------------------------ messages to the author

/// Where the message form sends (the Writing Studio's intake, a Worker with
/// one door: it takes a message and gives nothing back). Messages are read
/// only in the Studio; nothing of them is shown on this site.
const MESSAGE_ENDPOINT: &str =
    "https://ryujin-site-comments.hatakeyama-ryujin-q7.workers.dev/messages";

/// One published article a message may be about.
struct MessageChoice {
    id: String,
    title: String,
    url: String,
}

/// The published articles of a category, as the form offers them and as
/// `/miscellany/articles.json` lists them: drafts and private writing are
/// never in them (only what this site publishes).
fn message_choices(miscellany: &Miscellany, category: &str) -> Vec<MessageChoice> {
    match category {
        "notes" => miscellany
            .public_notes()
            .map(|note| {
                let m = note.metadata();
                MessageChoice {
                    id: m.slug.clone(),
                    title: m.title.clone(),
                    url: note_path(&m.slug),
                }
            })
            .collect(),
        "sketches" => miscellany
            .public_sketches()
            .map(|sketch| {
                let m = sketch.metadata();
                MessageChoice {
                    id: m.slug.clone(),
                    title: m.title.clone(),
                    url: sketch_path(&m.slug),
                }
            })
            .collect(),
        "diary" => miscellany
            .public_diary()
            .map(|entry| {
                let m = entry.metadata();
                let date = m.date.format("%-d %B %Y").to_string();
                MessageChoice {
                    id: m.id(),
                    title: m
                        .title
                        .as_ref()
                        .map_or_else(|| date.clone(), |t| format!("{date}: {t}")),
                    url: format!("{}#{}", m.month_path(), m.anchor()),
                }
            })
            .collect(),
        "reading" => {
            let items = miscellany.reading();
            reading_anchors(items)
                .into_iter()
                .zip(items)
                .map(|(anchor, item)| MessageChoice {
                    title: item.work().title.clone(),
                    url: format!("/miscellany/reading/#{anchor}"),
                    id: anchor,
                })
                .collect()
        }
        _ => Vec::new(),
    }
}

/// `/miscellany/articles.json`: each category's published articles (id,
/// title, address), which the intake checks a message's articles against.
pub(crate) fn articles_json(miscellany: &Miscellany) -> String {
    let mut out = serde_json::Map::new();
    for category in ["reading", "notes", "diary", "sketches"] {
        let list: Vec<serde_json::Value> = message_choices(miscellany, category)
            .into_iter()
            .map(|c| serde_json::json!({ "id": c.id, "title": c.title, "url": c.url }))
            .collect();
        out.insert(category.to_owned(), serde_json::Value::Array(list));
    }
    serde_json::Value::Object(out).to_string()
}

/// What a category with nothing published says: the same for all four.
fn no_entries() -> Markup {
    html! { p.empty-state.miscellany-empty { "No entries yet." } }
}

/// The form at the end of a category's page, under "Messages welcome.": a
/// message, a name if one likes, and the articles it is about if there are
/// any to choose (else the category as a whole). Open from the start, and
/// quiet: no explanation, no panel; the words and the form say what it is.
/// A message reaches the author's Writing Studio only, never this site. Drawn hidden; the site's script shows it (client/site.ts), so a
/// page without scripts shows nothing that cannot work.
fn message_form(miscellany: &Miscellany, category: &str) -> Markup {
    let choices = message_choices(miscellany, category);
    html! {
        section.site-message aria-labelledby="site-message-heading" data-site-message data-category=(category) data-endpoint=(MESSAGE_ENDPOINT) hidden {
            h2 #site-message-heading { "Messages welcome." }
            form data-site-message-form novalidate {
                label for="site-message-body" { "Message" }
                textarea #site-message-body name="body" rows="3" maxlength="4000" required {}
                label for="site-message-name" { "Name " span.site-message-optional { "(optional)" } }
                input #site-message-name type="text" name="name" maxlength="80" autocomplete="name";
                @if !choices.is_empty() {
                    details.site-message-articles {
                        summary { "About particular articles " span.site-message-optional { "(optional)" } }
                        ul {
                            @for choice in &choices {
                                li {
                                    label {
                                        input type="checkbox" name="articles" value=(choice.id);
                                        " "
                                        span { (choice.title) }
                                    }
                                }
                            }
                        }
                    }
                }
                div.site-message-trap aria-hidden="true" {
                    label { "Leave this empty " input type="text" name="website" tabindex="-1" autocomplete="off"; }
                }
                p.site-message-actions {
                    button type="submit" { "Send" }
                    span.site-message-status role="status" aria-live="polite" {}
                }
            }
        }
    }
}
