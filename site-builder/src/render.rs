use std::fmt::Write as _;

use chrono::NaiveDate;
use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::content::{
    Miscellany, ValidatedDiaryEntry, ValidatedNote, ValidatedSiteContent, ValidatedWriting,
    public_english_writings,
};
use crate::model::{
    Category, EventStatus, Language, Link, Project, Publication, ReadingItem, SummarySegment,
    Update, summary_plain_text, summary_segments,
};

const SITE_ORIGIN: &str = "https://ryujin-hatakeyama.github.io";
const SITE_NAME: &str = "Ryujin Hatakeyama";
/// The date of the last substantive content revision, shown in the footer.
/// It is editorial metadata maintained by hand: change it deliberately with
/// each such revision. It is never taken from the clock, Git, or deployment,
/// so builds stay reproducible.
const LAST_UPDATED: NaiveDate = match NaiveDate::from_ymd_opt(2026, 10, 10) {
    Some(date) => date,
    None => panic!("LAST_UPDATED must be a real date"),
};
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

pub(crate) fn pages(content: &ValidatedSiteContent) -> Vec<GeneratedPage> {
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
            home(content.updates()),
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
            updates_page(content.updates(), None),
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
                description: "Reading, study notes, and occasional diary entries by Ryujin Hatakeyama.",
                current_path: "/miscellany/",
                noindex: false,
                article: false,
            },
            miscellany_overview(),
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
            reading_page(content.miscellany().reading()),
        ),
        page(
            "miscellany/notes/index.html",
            "/miscellany/notes/",
            true,
            PageMetadata {
                title: "Notes",
                description: "Study notes and guides by Ryujin Hatakeyama.",
                current_path: "/miscellany/notes/",
                noindex: false,
                article: false,
            },
            notes_page(content.miscellany()),
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
            diary_page(content.miscellany()),
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
        let category_updates: Vec<_> = content
            .updates()
            .iter()
            .filter(|update| update.categories.contains(&category))
            .collect();
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
            updates_page_refs(&category_updates, Some(category)),
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
            note_detail(note),
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
                (nav_link("/cv/", "CV", current_path.starts_with("/cv/")))
                (nav_link("/updates/", "Updates", current_path.starts_with("/updates/")))
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
            // The feed stays discoverable through the <head> link.
            p.footer-updated {
                "Last updated "
                time datetime=(LAST_UPDATED.format("%Y-%m-%d")) { (LAST_UPDATED.format("%-d %B %Y")) }
            }
            p.footer-privacy {
                "Aggregate, cookie-free visit statistics are collected with "
                a href="https://www.goatcounter.com/" { "GoatCounter" }
                "."
            }
        }
    }
}

/// The homepage previews each section separately, so a busy Upcoming section
/// can never crowd completed updates out of Recent Updates.
const HOME_UPCOMING_LIMIT: usize = 3;
const HOME_RECENT_LIMIT: usize = 5;

fn home(updates: &[Update]) -> Markup {
    let refs: Vec<_> = updates.iter().collect();
    let (mut upcoming, mut recent) = split_upcoming(&refs);
    upcoming.truncate(HOME_UPCOMING_LIMIT);
    recent.truncate(HOME_RECENT_LIMIT);
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
                p.home-bio {
                    "Hello! I'm Ryujin, a second-year master's student in the "
                    a href="https://www.is.tohoku.ac.jp/en/laboratory/list_dept/" { "Department of Computer and Mathematical Sciences" }
                    " at "
                    a href="https://www.is.tohoku.ac.jp/en/" { "Tohoku University's Graduate School of Information Sciences" }
                    ". I work in the "
                    a href="https://www.is.tohoku.ac.jp/en/laboratory/list_dept/a11.html" { "Foundations of Software Science" }
                    " group under the supervision of "
                    a href="https://www.kb.ecei.tohoku.ac.jp/~sumii/" { "Professor Eijiro Sumii" }
                    "."
                }
                p.home-bio {
                    "My research is in programming language theory. I'm currently working with "
                    a href="https://okmij.org/ftp/" { "Oleg Kiselyov" }
                    " on compositional descriptions of probabilistic models and staged inference code generation."
                }
                p.home-bio { "My broader interests lie in modal and categorical logic, and in the conditions of intelligibility of formal reasoning." }
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
                (upcoming_list(&upcoming, true))
            }
        }
        @if !recent.is_empty() || upcoming.is_empty() {
            section.home-news aria-labelledby="updates-heading" {
                header.home-news-heading {
                    h2 id="updates-heading" { "Recent Updates" }
                }
                (update_list(&recent, true))
            }
        }
        // One link to the complete records, after both previews, instead of a
        // "read more" link on every entry.
        p.home-all-updates { a href="/updates/" { "All updates" } }
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
            nav.jump-links aria-label="Research sections" {
                a href="#publications" { "Publications" }
                @if !projects.is_empty() { a href="#projects" { "Projects" } }
                a href="#presentations" { "Talks & Presentations" }
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

fn updates_page(updates: &[Update], active: Option<Category>) -> Markup {
    let refs: Vec<_> = updates.iter().collect();
    updates_page_refs(&refs, active)
}

fn updates_page_refs(updates: &[&Update], active: Option<Category>) -> Markup {
    let (upcoming, recent) = split_upcoming(updates);
    html! {
        (page_intro(active.map_or("Updates", Category::label)))
        (update_filters(active))
        @if !upcoming.is_empty() {
            section.update-section.upcoming-updates aria-labelledby="upcoming-heading" {
                h2 id="upcoming-heading" { "Upcoming" }
                (upcoming_list(&upcoming, false))
            }
        }
        @if !recent.is_empty() {
            section.update-section aria-labelledby="recent-heading" {
                h2 id="recent-heading" { "Recent Updates" }
                (update_list(&recent, false))
            }
        } @else if upcoming.is_empty() {
            (update_list(&recent, false))
        }
    }
}

/// Separates planned events (Upcoming), soonest first, from completed ones
/// (Recent Updates), most recently held first: by the event's final day, then
/// its first day, then id. Both orders use when the activity takes place. The split depends only on each record's stated
/// status, never on the build date; a planned record whose event has begun
/// stops the build for editorial review instead.
fn split_upcoming<'a>(updates: &[&'a Update]) -> (Vec<&'a Update>, Vec<&'a Update>) {
    let (mut upcoming, mut recent): (Vec<&Update>, Vec<&Update>) = updates
        .iter()
        .copied()
        .partition(|update| update.event_status == EventStatus::Planned);
    upcoming.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.id.cmp(&right.id))
    });
    recent.sort_by(|left, right| Update::most_recent_first(left, right));
    (upcoming, recent)
}

/// Planned events lead with their event date.
fn upcoming_list(updates: &[&Update], compact: bool) -> Markup {
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
                            span.kind-label { (update.kind.label()) }
                            span.category-label { (category_labels(update)) }
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

fn category_labels(update: &Update) -> String {
    update
        .categories
        .iter()
        .map(|category| category.label())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn page_intro(title: &str) -> Markup {
    html! { header.page-intro { h1 { (title) } } }
}

fn update_filters(active: Option<Category>) -> Markup {
    html! {
        nav.update-filters aria-label="Update categories" {
            (filter_link("/updates/", "All", active.is_none()))
            @for category in Category::ALL {
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

fn update_list(updates: &[&Update], compact: bool) -> Markup {
    if updates.is_empty() {
        return html! { p.empty-state { "No updates yet." } };
    }
    let class = if compact {
        "update-list compact"
    } else {
        "update-list"
    };
    html! {
        ol class=(class) {
            @for update in updates {
                li {
                    article id=[(!compact).then(|| format!("update-{}", update.id))] {
                        div.update-meta {
                            (event_dates(update))
                            span.kind-label { (update.kind.label()) }
                            // Categories are plain, muted metadata rather than
                            // links; the category filters provide navigation.
                            span.category-label { (category_labels(update)) }
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
                div.detail-meta { dt { "Categories" } dd { (category_labels(update)) } }
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

/// The three Miscellany sections, with their functional labels.
const MISCELLANY_SECTIONS: [(&str, &str, &str); 3] = [
    ("/miscellany/reading/", "Reading", "Books and papers"),
    ("/miscellany/notes/", "Notes", "Study notes and guides"),
    ("/miscellany/diary/", "Diary", "Occasional entries"),
];

fn note_path(slug: &str) -> String {
    format!("/miscellany/notes/{slug}/")
}

fn miscellany_overview() -> Markup {
    html! {
        header.plain-page-header { h1 { "Miscellany" } }
        div.miscellany-overview {
            @for (href, label, description) in MISCELLANY_SECTIONS {
                section.content-section {
                    h2 { a href=(href) { (label) } }
                    p { (description) }
                }
            }
        }
    }
}

/// Text links back to the overview and between the three sections, shown
/// above the heading of every page below `/miscellany/`.
fn miscellany_nav(current_path: &str) -> Markup {
    html! {
        nav.jump-links.miscellany-nav aria-label="Miscellany sections" {
            (nav_link("/miscellany/", "Overview", current_path == "/miscellany/"))
            @for (href, label, _) in MISCELLANY_SECTIONS {
                (nav_link(href, label, current_path == href))
            }
        }
    }
}

fn miscellany_section_header(path: &str, title: &str, description: &str) -> Markup {
    html! {
        header.plain-page-header.miscellany-header {
            (miscellany_nav(path))
            h1 { (title) }
            p { (description) }
        }
    }
}

/// The reading record in the author's order. Titles and names are shown as
/// written, marked with their language when it is not English.
fn reading_page(items: &[ReadingItem]) -> Markup {
    let (path, title, description) = MISCELLANY_SECTIONS[0];
    html! {
        (miscellany_section_header(path, title, description))
        @if items.is_empty() {
            p.empty-state { "No entries yet." }
        } @else {
            ol.reading-list {
                @for item in items {
                    li {
                        p.reading-title {
                            cite lang=[item.lang.as_ref().map(|tag| tag.as_str())] {
                                @if let Some(url) = &item.url {
                                    a href=(url) { (&item.title) }
                                } @else {
                                    (&item.title)
                                }
                            }
                        }
                        @if let Some(author) = &item.author {
                            p.reading-author lang=[item.lang.as_ref().map(|tag| tag.as_str())] { (author) }
                        }
                        @if let Some(note) = &item.note {
                            p.reading-note lang=[item.note_lang.as_ref().map(|tag| tag.as_str())] { (note) }
                        }
                    }
                }
            }
        }
    }
}

fn notes_page(miscellany: &Miscellany) -> Markup {
    let (path, title, description) = MISCELLANY_SECTIONS[1];
    let notes: Vec<_> = miscellany.public_notes().collect();
    html! {
        (miscellany_section_header(path, title, description))
        @if notes.is_empty() {
            p.empty-state { "No notes yet." }
        } @else {
            ol.miscellany-list {
                @for note in notes {
                    @let metadata = note.metadata();
                    li lang=(metadata.lang.code()) {
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

fn note_detail(note: &ValidatedNote) -> Markup {
    let metadata = note.metadata();
    let class = if metadata.lang == Language::Ja {
        "miscellany-note longform-ja"
    } else {
        "miscellany-note"
    };
    html! {
        div.miscellany-header { (miscellany_nav(&note_path(&metadata.slug))) }
        article class=(class) lang=(metadata.lang.code()) {
            header {
                h1 { (&metadata.title) }
                @if let Some(description) = &metadata.description {
                    p.summary { (description) }
                }
            }
            div.prose.miscellany-prose { (PreEscaped(note.rendered_body())) }
        }
    }
}

/// Diary entries newest first, each shown in full under its date. The date
/// links to the entry's own anchor; a title, when given, follows it.
fn diary_page(miscellany: &Miscellany) -> Markup {
    let (path, title, description) = MISCELLANY_SECTIONS[2];
    let entries: Vec<_> = miscellany.public_diary().collect();
    html! {
        (miscellany_section_header(path, title, description))
        @if entries.is_empty() {
            p.empty-state { "No entries yet." }
        } @else {
            div.diary {
                @for entry in entries { (diary_entry(entry)) }
            }
        }
    }
}

fn diary_entry(entry: &ValidatedDiaryEntry) -> Markup {
    let metadata = entry.metadata();
    let anchor = metadata.anchor();
    let lang = metadata.lang.code();
    let class = if metadata.lang == Language::Ja {
        "diary-entry longform-ja"
    } else {
        "diary-entry"
    };
    html! {
        article class=(class) id=(anchor) {
            h2 {
                a.diary-date href=(format!("#{anchor}")) {
                    time datetime=(metadata.date.format("%Y-%m-%d")) { (metadata.date.format("%-d %B %Y")) }
                }
                @if let Some(title) = &metadata.title {
                    span.diary-title lang=(lang) { (title) }
                }
            }
            div.prose.miscellany-prose lang=(lang) { (PreEscaped(entry.rendered_body())) }
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

#[cfg(test)]
mod tests {
    use super::{
        classification, cv, escape_xml, filterable_publications, footer, format_date,
        format_date_range, header, home, publication_list, research, rss, update_detail,
        update_list, update_summary, updates_page_refs,
    };
    use crate::model::{Category, EventStatus, Publication};
    use chrono::NaiveDate;

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
        let html = home(&updates).into_string();

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
        let html = home(&updates).into_string();
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
        let html = updates_page_refs(&refs, None).into_string();

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
        let without_plans = updates_page_refs(&completed, None).into_string();
        assert!(!without_plans.contains("Upcoming"));
        assert!(without_plans.contains(r#"<h2 id="recent-heading">Recent Updates</h2>"#));
        assert_eq!(without_plans.matches(r#"<article id="update-"#).count(), 7);

        let without_records = updates_page_refs(&[], None).into_string();
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
        let html = updates_page_refs(&[&ranged, &talk], Some(Category::Research)).into_string();
        assert!(html.contains("<h1>Research</h1>"));
        assert!(html.contains(r#"<a href="/updates/research/" aria-current="page">Research</a>"#));
        assert!(html.contains(r#"<h2 id="upcoming-heading">Upcoming</h2>"#));
        assert!(html.contains(r#"<h2 id="recent-heading">Recent Updates</h2>"#));
        assert!(html.contains(
            r#"<div class="update-meta"><time datetime="2026-10-16">16–18 Oct 2026</time>"#
        ));

        // A filter that leaves only planned records shows no empty Recent Updates.
        let only_planned = updates_page_refs(&[&ranged], Some(Category::Research)).into_string();
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

        let full = updates_page_refs(&refs, None).into_string();
        assert_eq!(ids_in_order(&full), expected);
        let home_html = home(&updates).into_string();
        let (start, end) = section_bounds(&home_html, "updates-heading");
        assert_eq!(ids_in_order(&home_html[start..end]), expected[..5]);
        let research: Vec<_> = refs
            .iter()
            .copied()
            .filter(|update| update.categories.contains(&Category::Research))
            .collect();
        let category = updates_page_refs(&research, Some(Category::Research)).into_string();
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
        for (update, title, text, url) in [
            (
                &aie,
                "Presented at Effective Presentation & Communication in English",
                "Effective Presentation & Communication in English",
                AIE_PDF,
            ),
            (
                &llal,
                "Attendance at LLAL@GSIS (XIII)",
                "LLAL@GSIS (XIII)",
                LLAL_PAGE,
            ),
        ] {
            assert_eq!(update.title.en, title);
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

        let aie_heading = format!(
            r#"<h3>Presented at <a href="{AIE_PDF}">Effective Presentation &amp; Communication in English</a></h3>"#
        );
        let llal_heading =
            format!(r#"<h3>Attendance at <a href="{LLAL_PAGE}">LLAL@GSIS (XIII)</a></h3>"#);
        let updates = [aie, llal];
        let refs: Vec<_> = updates.iter().collect();
        let views = [
            ("home", home(&updates).into_string()),
            ("updates", updates_page_refs(&refs, None).into_string()),
            (
                "activities",
                updates_page_refs(&refs, Some(Category::Academia)).into_string(),
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
        assert!(feed.contains(
            "<item><title>Presented at Effective Presentation &amp; Communication in English</title>"
        ));
        assert!(feed.contains("<title>Attendance at LLAL@GSIS (XIII)</title>"));
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

    const LINKED_EVENTS: [(&str, &str); 6] = [
        (
            "2026-10-aie-english-training-poster.json",
            r#"<h3>Presented at <a href="https://www.aie.tohoku.ac.jp/data/news/20261009_english.pdf">Effective Presentation &amp; Communication in English</a></h3>"#,
        ),
        (
            "2026-03-llal-gsis-13.json",
            r#"<h3>Attendance at <a href="https://sites.google.com/view/llal-at-gsis/meetings/llalgsis-13">LLAL@GSIS (XIII)</a></h3>"#,
        ),
        (
            "2026-10-graham-priest-welcome.json",
            r#"<h3>Planned attendance at the <a href="https://sites.google.com/view/welcome-graham/">workshop welcoming Graham Priest to Sendai</a></h3>"#,
        ),
        (
            "2026-10-wakate-no-kai.json",
            r#"<h3>Planned attendance at <a href="https://sites.google.com/view/wakatenokai2026/" lang="ja">数学基礎論若手の会2026</a></h3>"#,
        ),
        (
            "2026-09-ppl-summer-school.json",
            r#"<h3>Attendance at <a href="https://jssst-ppl.org/wiki/ss2026">PPL Summer School 2026</a></h3>"#,
        ),
        (
            "2026-09-jssst-2026-presentation.json",
            r#"<h3>Oral presentation at <a href="https://jssst2026.wordpress.com/">the 43rd JSSST Annual Conference</a></h3>"#,
        ),
    ];

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
        for page in super::pages(&content) {
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
        let html = home(&[]).into_string();
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains(r#"<h1 class="person-name" id="home-name"><span class="name-latin" lang="en">Ryujin Hatakeyama</span> <span class="name-japanese" lang="ja">畠山竜迅</span></h1>"#));
        assert_eq!(html.matches("畠山竜迅").count(), 1);
        // The pronunciation follows the heading unchanged.
        assert!(html.contains(r#"</h1><p class="name-pronunciation"><span lang="ja">はたけやま りゅうじん</span><span aria-hidden="true"> · </span><span class="ipa">/hatakejama ɾʲɯːdʑiɴ/</span></p>"#));
        assert!(html.contains(r#"<section class="home-intro" aria-labelledby="home-name">"#));
    }

    #[test]
    fn footer_discloses_cookie_free_aggregate_statistics() {
        let html = footer().into_string();
        assert!(html.contains(r#"<p class="footer-privacy">Aggregate, cookie-free visit statistics are collected with <a href="https://www.goatcounter.com/">GoatCounter</a>.</p>"#));
    }

    #[test]
    fn cv_is_a_primary_navigation_item() {
        let html = header("/cv/").into_string();
        let order: Vec<_> = [
            r#"href="/">Home<"#,
            r#"href="/research/">Research<"#,
            r#"href="/cv/""#,
            r#"href="/updates/">Updates<"#,
        ]
        .iter()
        .map(|needle| html.find(needle).unwrap())
        .collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(html.contains(r#"<a href="/cv/" aria-current="page">CV</a>"#));
        assert_eq!(html.matches(r#"aria-current="page""#).count(), 1);
        assert!(
            header("/research/")
                .into_string()
                .contains(r#"<a href="/cv/">CV</a>"#)
        );
    }

    #[test]
    fn research_page_has_no_redundant_cv_section() {
        let html = research(&[], &[], &[]).into_string();
        assert!(!html.contains(r##"href="#cv""##));
        assert!(!html.contains(r#"id="cv""#));
        assert!(!html.contains(r#"href="/cv/""#));
    }

    #[test]
    fn homepage_has_one_all_updates_link_after_both_previews() {
        let html = home(&mixed_updates()).into_string();
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
        let pages = [
            home(&updates).into_string(),
            updates_page_refs(&refs, None).into_string(),
            updates_page_refs(&research, Some(Category::Research)).into_string()
                + &updates_page_refs(&academia, Some(Category::Academia)).into_string(),
        ];
        for page in &pages {
            for (file, heading) in LINKED_EVENTS {
                assert_eq!(page.matches(heading).count(), 1, "{file}");
            }
            assert_no_nested_anchors(page);
            assert!(
                !page.contains("サマースクール")
                    && !page.contains("日本ソフトウェア科学会第43回大会")
            );
        }
        // Only the event name is linked: the activity phrase stays text.
        for (_, heading) in LINKED_EVENTS {
            let before_link = &heading[..heading.find("<a ").unwrap()];
            assert!(!before_link.contains("href"));
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
        assert!(feed.contains("<item><title>Planned attendance at 数学基礎論若手の会2026</title>"));
        assert!(feed.contains("<title>Attendance at PPL Summer School 2026</title>"));
        assert!(feed.contains(
            "<title>Planned attendance at the workshop welcoming Graham Priest to Sendai</title>"
        ));
        assert!(feed.contains(
            "<description>Attended PPL Summer School 2026, a summer school on quantum program compilation.</description>"
        ));
        assert!(
            feed.contains("<title>Oral presentation at the 43rd JSSST Annual Conference</title>")
        );
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
        assert!(html.contains("<h3>AIE study visit to Seattle and Boise</h3>"));
        let summary = &html[html.find("<p>Selected").unwrap()..];
        let summary = &summary[..summary.find("</p>").unwrap()];
        assert_eq!(summary.matches("<a ").count(), 6);
        assert!(!summary.contains("target="));
    }

    #[test]
    fn footer_shows_the_editorial_last_updated_date_and_no_rss_link() {
        let html = footer().into_string();
        assert!(html.contains(r#"<p class="footer-name">© 2026 Ryujin Hatakeyama</p>"#));
        assert!(html.contains(
            r#"<p class="footer-updated">Last updated <time datetime="2026-10-10">10 October 2026</time></p>"#
        ));
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
        diary_page, header, miscellany_overview, note_detail, notes_page, pages, reading_page,
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
        pages(content)
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

    #[test]
    fn miscellany_is_the_fifth_primary_section_and_current_below_its_route() {
        for path in [
            "/miscellany/",
            "/miscellany/reading/",
            "/miscellany/notes/",
            "/miscellany/notes/staged-interpreters/",
            "/miscellany/diary/",
        ] {
            let html = header(path).into_string();
            let order: Vec<_> = ["Home<", "Research<", "CV<", "Updates<", "Miscellany<"]
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
    fn overview_has_one_heading_and_three_plain_sections() {
        let html = miscellany_overview().into_string();
        assert_eq!(html.matches("<h1").count(), 1);
        assert!(html.contains("<h1>Miscellany</h1>"));
        let sections = [
            r#"<h2><a href="/miscellany/reading/">Reading</a></h2><p>Books and papers</p>"#,
            r#"<h2><a href="/miscellany/notes/">Notes</a></h2><p>Study notes and guides</p>"#,
            r#"<h2><a href="/miscellany/diary/">Diary</a></h2><p>Occasional entries</p>"#,
        ];
        let positions: Vec<_> = sections.iter().map(|s| html.find(s).unwrap()).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(html.matches("<section").count(), 3);
        for decoration in [
            "card",
            "badge",
            "tab",
            "<svg",
            "<img",
            "No entries",
            "role=",
        ] {
            assert!(!html.contains(decoration), "{decoration}");
        }
    }

    #[test]
    fn empty_sections_are_quiet_and_contain_no_entries() {
        let content = ValidatedSiteContent::with_miscellany(Miscellany::default());
        for (path, title, empty) in [
            (
                "miscellany/reading/index.html",
                "Reading",
                "No entries yet.",
            ),
            ("miscellany/notes/index.html", "Notes", "No notes yet."),
            ("miscellany/diary/index.html", "Diary", "No entries yet."),
        ] {
            let html = page_html(&content, path);
            let main = main_of(&html);
            assert!(main.contains(&format!("<h1>{title}</h1>")), "{path}");
            assert!(main.contains(&format!(r#"<p class="empty-state">{empty}</p>"#)));
            assert!(!main.contains("<article") && !main.contains("<li") && !main.contains("<h2"));
            assert!(main.contains(r#"<nav class="jump-links miscellany-nav" aria-label="Miscellany sections"><a href="/miscellany/">Overview</a>"#));
            assert_eq!(main.matches(r#"aria-current="page""#).count(), 1);
        }
        assert!(
            !pages(&content)
                .iter()
                .any(|page| page.output_path.starts_with("miscellany/notes/")
                    && page.output_path != "miscellany/notes/index.html")
        );
    }

    #[test]
    fn reading_keeps_the_authors_order_original_titles_and_languages() {
        let reading = "items:\n  - author: 夏目漱石\n    title: こころ\n    lang: ja\n    note: Read in the original.\n  - author: Saul A. Kripke\n    title: Naming and Necessity\n    url: https://example.org/naming\n  - title: 竹取物語\n    lang: ja\n    note: 二度目。\n    noteLang: ja\n  - author: A <b> & \"C\"\n    title: Title <script>alert(1)</script>\n";
        let html = reading_page(miscellany(reading, &[], &[]).reading()).into_string();
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
        assert!(html.contains(r#"<p class="reading-title"><cite lang="ja">こころ</cite></p><p class="reading-author" lang="ja">夏目漱石</p><p class="reading-note">Read in the original.</p>"#));
        assert!(html.contains(r#"<cite><a href="https://example.org/naming">Naming and Necessity</a></cite></p><p class="reading-author">Saul A. Kripke</p>"#));
        // An anonymous work has no author line; a Japanese note is marked.
        assert!(html.contains(
            r#"<cite lang="ja">竹取物語</cite></p><p class="reading-note" lang="ja">二度目。</p>"#
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

        let paths: Vec<_> = pages(&content)
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
        assert!(main.contains(r#"<article class="miscellany-note" lang="en"><header><h1>Staged &lt;interpreters&gt; &amp; you</h1><p class="summary">How a staged interpreter becomes a compiler.</p></header>"#));
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
        assert!(main.contains(
            r#"<nav class="jump-links miscellany-nav" aria-label="Miscellany sections">"#
        ));
        // The navigation stays outside the language-marked article.
        assert!(main.find("miscellany-nav").unwrap() < main.find("<article").unwrap());
    }

    #[test]
    fn japanese_notes_are_marked_as_japanese() {
        let notes = miscellany("items: []", &[NOTE_JA], &[]);
        let note = notes.public_notes().next().unwrap();
        let html = note_detail(note).into_string();
        assert!(html.contains(r#"<article class="miscellany-note longform-ja" lang="ja"><header><h1>クリプキ意味論の覚え書き</h1></header>"#));
        assert!(html.contains("<h2>可能世界</h2>"));
        assert!(!html.contains("summary"));
        assert!(
            notes_page(&notes)
                .into_string()
                .contains(r#"<li lang="ja">"#)
        );
    }

    #[test]
    fn diary_shows_entries_newest_first_in_full_with_stable_anchors() {
        let entries = miscellany("items: []", &[], &[DIARY_OLD, DIARY_DRAFT, DIARY_NEW]);
        let html = diary_page(&entries).into_string();
        let newest = html
            .find(r##"<article class="diary-entry longform-ja" id="diary-2026-10-10"><h2><a class="diary-date" href="#diary-2026-10-10"><time datetime="2026-10-10">10 October 2026</time></a><span class="diary-title" lang="ja">仙台にて</span></h2><div class="prose miscellany-prose" lang="ja"><p>今日は図書館で一日を過ごした。</p>"##)
            .unwrap();
        let older = html
            .find(r##"<article class="diary-entry" id="diary-2026-03-01"><h2><a class="diary-date" href="#diary-2026-03-01"><time datetime="2026-03-01">1 March 2026</time></a></h2><div class="prose miscellany-prose" lang="en"><p>First paragraph.</p>
<p>Second paragraph.</p>"##)
            .unwrap();
        assert!(newest < older);
        assert!(!html.contains("Not public") && !html.contains("2026-10-11"));
        assert_eq!(html.matches("<article").count(), 2);
        assert!(!html.contains("Read more") && !html.contains("<nav class=\"pagination"));
    }

    #[test]
    fn existing_routes_and_feed_are_unchanged_by_miscellany() {
        let content = ValidatedSiteContent::with_miscellany(miscellany(
            "items:\n  - title: Fixture work\n",
            &[NOTE_EN],
            &[DIARY_OLD],
        ));
        let paths: Vec<_> = pages(&content)
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
