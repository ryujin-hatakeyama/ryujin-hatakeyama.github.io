use std::fmt::Write as _;

use chrono::NaiveDate;
use maud::{DOCTYPE, Markup, PreEscaped, html};

use crate::content::{ValidatedSiteContent, ValidatedWriting, public_english_writings};
use crate::model::{Category, EventStatus, Language, Link, Project, Publication, Update};

const SITE_ORIGIN: &str = "https://ryujin-hatakeyama.github.io";
const SITE_NAME: &str = "Ryujin Hatakeyama";
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
        let path = format!("/updates/item/{}/", update.id);
        pages.push(page_owned(
            format!("updates/item/{}/index.html", update.id),
            path.clone(),
            true,
            &update.title.en,
            &update.summary.en,
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

fn header(current_path: &str) -> Markup {
    html! {
        header.site-header {
            a.wordmark href="/" aria-label="Ryujin Hatakeyama home" { "Ryujin Hatakeyama" }
            nav.primary-nav aria-label="Primary navigation" {
                (nav_link("/", "Home", current_path == "/"))
                (nav_link("/research/", "Research", current_path.starts_with("/research/")))
                (nav_link("/updates/", "Updates", current_path.starts_with("/updates/")))
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
            p.footer-meta { a href="/rss.xml" { "RSS" } }
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
                h1 id="home-name" { "Ryujin Hatakeyama" }
                p.name-japanese lang="ja" { "畠山竜迅" }
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
                    a href="/updates/" { "All updates" }
                }
                (upcoming_list(&upcoming, true))
            }
        }
        @if !recent.is_empty() || upcoming.is_empty() {
            section.home-news aria-labelledby="updates-heading" {
                header.home-news-heading {
                    h2 id="updates-heading" { "Recent Updates" }
                    @if upcoming.is_empty() { a href="/updates/" { "All updates" } }
                }
                (update_list(&recent, true))
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
                a href="#cv" { "CV" }
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
            section id="cv" class="content-section" {
                h2 { "CV" }
                p { a href="/cv/" { "CV" } }
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
/// (Recent Updates), which keep their newest-first announcement order. The
/// split depends only on each record's stated status, never on the build date;
/// a planned record whose event has begun stops the build for editorial review
/// instead.
fn split_upcoming<'a>(updates: &[&'a Update]) -> (Vec<&'a Update>, Vec<&'a Update>) {
    let (mut upcoming, recent): (Vec<&Update>, Vec<&Update>) = updates
        .iter()
        .copied()
        .partition(|update| update.event_status == EventStatus::Planned);
    upcoming.sort_by(|left, right| {
        left.date
            .cmp(&right.date)
            .then_with(|| left.id.cmp(&right.id))
    });
    (upcoming, recent)
}

/// Planned events lead with their event date; the announcement date is
/// secondary because it does not say when the event takes place.
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
                            h3 {
                                @if update.detail {
                                    a href=(format!("/updates/item/{}/", update.id)) { (&update.title.en) }
                                } @else { (&update.title.en) }
                            }
                            @if !compact {
                                p { (&update.summary.en) }
                                p.event-line {
                                    "Announced "
                                    time datetime=(update.announced_on.format("%Y-%m-%d")) { (format_date(update.announced_on)) }
                                }
                                @if !update.links.is_empty() { (record_links(&update.links, true)) }
                            }
                        }
                    }
                }
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
                            time datetime=(update.announced_on.format("%Y-%m-%d")) { (format_date(update.announced_on)) }
                            span.kind-label { (update.kind.label()) }
                            // Categories are plain, muted metadata rather than
                            // links; the category filters provide navigation.
                            span.category-label { (category_labels(update)) }
                        }
                        div.update-copy {
                            h3 {
                                @if update.detail {
                                    a href=(format!("/updates/item/{}/", update.id)) { (&update.title.en) }
                                } @else { (&update.title.en) }
                            }
                            @if !compact {
                                p { (&update.summary.en) }
                                p.event-line { (event_line(update)) }
                                @if !update.links.is_empty() { (record_links(&update.links, true)) }
                            }
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
                h1 { (&update.title.en) }
                p.summary { (&update.summary.en) }
            }
            dl {
                div.detail-meta { dt { "Event date" } dd { (event_dates(update)) } }
                @if update.event_status == EventStatus::Planned {
                    div.detail-meta { dt { "Status" } dd { "Planned" } }
                }
                div.detail-meta { dt { "Announced" } dd { time datetime=(update.announced_on.format("%Y-%m-%d")) { (format_date(update.announced_on)) } } }
                div.detail-meta { dt { "Categories" } dd { (category_labels(update)) } }
            }
            @if let Some(body) = &update.body { div.prose { p { (&body.en) } } }
            @if !update.links.is_empty() { (record_links(&update.links, true)) }
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

fn event_line(update: &Update) -> Markup {
    html! {
        "Event: " (event_dates(update))
        @if update.event_status == EventStatus::Planned { " · Planned" }
    }
}

fn record_links(links: &[Link], include_type: bool) -> Markup {
    html! {
        p.record-links {
            @for link in links {
                a href=(&link.url) {
                    (&link.label)
                    @if include_type { span.sr-only { " (" (link.link_type.label()) ")" } }
                }
            }
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
            "<item><title>{}</title><description>{}</description><link>{link}</link><guid>{link}</guid><pubDate>{}</pubDate>{categories}</item>",
            escape_xml(&update.title.en),
            escape_xml(&update.summary.en),
            update
                .announced_on
                .format("%a, %d %b %Y 00:00:00 +0000"),
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
        classification, cv, escape_xml, filterable_publications, format_date, format_date_range,
        header, home, publication_list, rss, update_detail, update_list, updates_page_refs,
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
                    "announcedOn":"2026-10-09","eventStatus":"completed","categories":["academia"],
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
            "announcedOn":"2026-10-09","eventStatus":"completed","categories":["academia"],
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

    fn test_update(id: &str, date: &str, announced: &str, status: &str) -> crate::model::Update {
        test_update_in(id, date, "", announced, status, "academia")
    }

    fn test_update_in(
        id: &str,
        date: &str,
        end: &str,
        announced: &str,
        status: &str,
        category: &str,
    ) -> crate::model::Update {
        serde_json::from_str(&format!(
            r#"{{"id":"{id}","title":{{"en":"title-{id}"}},"summary":{{"en":"x"}},"date":"{date}"{end},
            "announcedOn":"{announced}","eventStatus":"{status}","categories":["{category}"],
            "kind":{{"type":"participation"}},"related":{{}}}}"#
        ))
        .unwrap()
    }

    /// Records in the loader's newest-announcement-first order, with more
    /// planned records than the homepage shows and the planned ones announced
    /// last, so that limiting the combined list before separating it would
    /// leave no completed record in the preview.
    fn mixed_updates() -> Vec<crate::model::Update> {
        let mut updates: Vec<_> = (1..=4)
            .map(|n| {
                test_update(
                    &format!("planned-{n}"),
                    &format!("2026-11-{:02}", 10 - n),
                    "2026-10-09",
                    "planned",
                )
            })
            .collect();
        updates.extend((1..=7).map(|n| {
            test_update(
                &format!("completed-{n}"),
                &format!("2026-09-{:02}", 20 - n),
                &format!("2026-10-{:02}", 9 - n),
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

        // Each section has its own limit, applied after the split; Upcoming
        // is ordered by event date, Recent Updates by announcement.
        assert_eq!(upcoming.matches("<li>").count(), 3);
        assert_eq!(recent.matches("<li>").count(), 5);
        let at = |section: &str, id: &str| section.find(format!(">title-{id}<").as_str());
        assert!(at(upcoming, "planned-4").unwrap() < at(upcoming, "planned-3").unwrap());
        assert!(at(upcoming, "planned-3").unwrap() < at(upcoming, "planned-2").unwrap());
        assert!(at(upcoming, "planned-1").is_none());
        assert!(at(recent, "completed-1").unwrap() < at(recent, "completed-5").unwrap());
        assert!(at(recent, "completed-6").is_none());
        assert!(!upcoming.contains("completed-") && !recent.contains("planned-"));
        // Planned events lead with their event date, not the announcement.
        assert!(upcoming.contains(r#"<time datetime="2026-11-06">06 Nov 2026</time>"#));

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
        // Upcoming is soonest event first; the announcement date is secondary.
        assert!(
            html.find(r#"id="update-planned-4""#).unwrap()
                < html.find(r#"id="update-planned-1""#).unwrap()
        );
        assert!(html.contains(r#"Announced <time datetime="2026-10-09">09 Oct 2026</time>"#));
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
            "2026-10-09",
            "planned",
            "research",
        );
        let talk = test_update_in(
            "talk",
            "2026-09-10",
            "",
            "2026-10-09",
            "completed",
            "research",
        );
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

    /// The October 9 record is titled with the event's official name, verbatim
    /// from the AIE program, not with a description of the poster session.
    #[test]
    fn english_training_event_name_is_preserved_verbatim() {
        const NAME: &str = "Effective Presentation & Communication in English";
        let record = update_source("2026-10-aie-english-training-poster.json");
        assert_eq!(record["title"]["en"], NAME);

        let mut update = test_update(
            "aie-english-training-poster-2026",
            "2026-10-09",
            "2026-10-09",
            "completed",
        );
        update.title.en = NAME.to_owned();
        let updates = [update];
        let escaped = "Effective Presentation &amp; Communication in English";
        assert!(
            home(&updates)
                .into_string()
                .contains(&format!("<h3>{escaped}</h3>"))
        );
        let refs: Vec<_> = updates.iter().collect();
        assert!(
            updates_page_refs(&refs, None)
                .into_string()
                .contains(&format!("<h3>{escaped}</h3>"))
        );
        assert!(rss(&updates).contains(&format!("<item><title>{escaped}</title>")));
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
}
