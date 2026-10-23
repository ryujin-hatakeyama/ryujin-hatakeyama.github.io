//! The card's static HTML, the same for music and podcasts: the work (cover,
//! title, a second line, a further line), then the services and "Open" for
//! the chosen one; a podcast also has the chosen service's own player below.
//!
//! Without the site's script each service is a plain link, "Open" opens the
//! first, and a podcast plays in the first service's player. With it
//! (client/media.ts) the services are a choice: "Open" follows it, and so
//! does the player, which is swapped for the one in its `<template>` (inert
//! until then), so a card never has two players.

use super::model::{MediaDetails, MediaLink, Provider};

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// An arrow out of the page, drawn in the text's colour beside "Open".
const OPEN_ICON: &str = "<svg class=\"media-open-icon\" viewBox=\"0 0 16 16\" width=\"14\" height=\"14\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M5 11 11 5M6.5 5H11v4.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg>";

/// What each service's own player is allowed, as its embed code asks: Apple
/// sandboxes its player itself; Spotify's needs none.
const fn frame_policy(provider: Provider) -> &'static str {
    match provider {
        Provider::ApplePodcasts => {
            "allow=\"autoplay; encrypted-media; clipboard-write\" sandbox=\"allow-forms allow-popups allow-same-origin allow-scripts allow-storage-access-by-user-activation allow-top-navigation-by-user-activation\""
        }
        Provider::Spotify | Provider::AppleMusic => {
            "allow=\"autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture\" allowfullscreen"
        }
    }
}

fn player(link: &MediaLink, title: &str) -> Option<String> {
    let embed = link.embed.as_ref()?;
    Some(format!(
        "<iframe src=\"{}\" title=\"{} player: {}\" height=\"{}\" loading=\"lazy\" {} data-service=\"{}\"></iframe>",
        escape(&embed.url),
        link.provider.name(),
        escape(title),
        embed.height,
        frame_policy(link.provider),
        link.provider.key()
    ))
}

/// The card for a media paragraph. The details are those of the first
/// address that has them; without any, the card says what kind of work it
/// is and where.
pub fn card_html(links: &[MediaLink], details: &[MediaDetails]) -> String {
    let found = links
        .iter()
        .find_map(|link| details.iter().find(|d| d.url == link.canonical));
    let first = &links[0];
    let mut html = format!(
        "<figure class=\"media-card\" data-media-card data-media-kind=\"{}\" data-work=\"{}\">\n<div class=\"media-card-body\">",
        first.kind.key(),
        first.kind.work_key()
    );
    match found.and_then(|d| d.artwork.as_deref()) {
        Some(artwork) => html.push_str(&format!(
            "<img class=\"media-art\" src=\"{}\" alt=\"\" width=\"72\" height=\"72\" loading=\"lazy\" decoding=\"async\" referrerpolicy=\"no-referrer\">",
            escape(artwork)
        )),
        None => html.push_str("<span class=\"media-art media-art-none\" aria-hidden=\"true\"></span>"),
    }
    html.push_str("<figcaption class=\"media-caption\">");
    let title = match found {
        Some(d) => {
            html.push_str(&format!(
                "<span class=\"media-title\">{}</span>",
                escape(&d.title)
            ));
            for (class, value) in [("media-byline", &d.byline), ("media-detail", &d.detail)] {
                if let Some(value) = value {
                    html.push_str(&format!("<span class=\"{class}\">{}</span>", escape(value)));
                }
            }
            d.title.clone()
        }
        None => {
            let phrase = format!("{} on {}", first.kind.phrase(), first.provider.name());
            html.push_str(&format!("<span class=\"media-title\">{phrase}</span>"));
            phrase
        }
    };
    html.push_str(
        "</figcaption></div>\n<div class=\"media-actions\" data-pagefind-ignore=\"all\">",
    );
    if links.len() > 1 {
        // A choice of services, the first written chosen; "Open" follows it.
        html.push_str("<span class=\"media-switch\">");
        for (n, link) in links.iter().enumerate() {
            html.push_str(&format!(
                "<a class=\"media-option\" href=\"{}\" data-service=\"{}\"{}>{}</a>",
                escape(&link.canonical),
                link.provider.key(),
                if n == 0 { " aria-current=\"true\"" } else { "" },
                link.provider.name()
            ));
        }
        html.push_str(&format!(
            "</span><a class=\"media-open\" href=\"{}\" target=\"_blank\" rel=\"noopener\">Open<span class=\"sr-only\" data-media-open-service> in {}</span>{OPEN_ICON}</a>",
            escape(&first.canonical),
            first.provider.name()
        ));
    } else {
        // One service: no choice to draw, only where "Open" goes.
        html.push_str(&format!(
            "<a class=\"media-open\" href=\"{}\" target=\"_blank\" rel=\"noopener\" data-service=\"{}\">Open in {}{OPEN_ICON}</a>",
            escape(&first.canonical),
            first.provider.key(),
            first.provider.name()
        ));
    }
    html.push_str("</div>\n");
    if let Some(frame) = player(first, &title) {
        html.push_str("<div class=\"media-player\" data-pagefind-ignore=\"all\">");
        html.push_str(&frame);
        for link in &links[1..] {
            if let Some(other) = player(link, &title) {
                html.push_str(&format!(
                    "<template data-service=\"{}\">{other}</template>",
                    link.provider.key()
                ));
            }
        }
        html.push_str("</div>\n");
    }
    html.push_str("</figure>\n");
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::media_paragraph;

    fn details(url: &str) -> MediaDetails {
        MediaDetails {
            url: url.into(),
            title: "<script>alert(1)</script> & “Ode”".into(),
            byline: Some("A \"quoted\" <b>artist</b>".into()),
            detail: None,
            artwork: None,
        }
    }

    #[test]
    fn card_html_matches_the_studio() {
        // The same literals are asserted by the Writing Studio's renderer
        // (studio/test/media.test.ts), so the preview and the page agree.
        let links = media_paragraph(
            "https://music.apple.com/jp/album/x/1440806071?i=1440806402 https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC",
        )
        .unwrap();
        assert_eq!(
            card_html(
                &links,
                &[details(
                    "https://music.apple.com/jp/album/x/1440806071?i=1440806402"
                )]
            ),
            "<figure class=\"media-card\" data-media-card data-media-kind=\"music\" data-work=\"song\">\n<div class=\"media-card-body\"><span class=\"media-art media-art-none\" aria-hidden=\"true\"></span><figcaption class=\"media-caption\"><span class=\"media-title\">&lt;script&gt;alert(1)&lt;/script&gt; &amp; “Ode”</span><span class=\"media-byline\">A &quot;quoted&quot; &lt;b&gt;artist&lt;/b&gt;</span></figcaption></div>\n<div class=\"media-actions\" data-pagefind-ignore=\"all\"><span class=\"media-switch\"><a class=\"media-option\" href=\"https://music.apple.com/jp/album/x/1440806071?i=1440806402\" data-service=\"apple-music\" aria-current=\"true\">Apple Music</a><a class=\"media-option\" href=\"https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC\" data-service=\"spotify\">Spotify</a></span><a class=\"media-open\" href=\"https://music.apple.com/jp/album/x/1440806071?i=1440806402\" target=\"_blank\" rel=\"noopener\">Open<span class=\"sr-only\" data-media-open-service> in Apple Music</span><svg class=\"media-open-icon\" viewBox=\"0 0 16 16\" width=\"14\" height=\"14\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M5 11 11 5M6.5 5H11v4.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg></a></div>\n</figure>\n"
        );
        let podcast = media_paragraph(
            "https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ\nhttps://podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135",
        )
        .unwrap();
        assert_eq!(
            card_html(&podcast, &[]),
            "<figure class=\"media-card\" data-media-card data-media-kind=\"podcast\" data-work=\"episode\">\n<div class=\"media-card-body\"><span class=\"media-art media-art-none\" aria-hidden=\"true\"></span><figcaption class=\"media-caption\"><span class=\"media-title\">An episode on Spotify</span></figcaption></div>\n<div class=\"media-actions\" data-pagefind-ignore=\"all\"><span class=\"media-switch\"><a class=\"media-option\" href=\"https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ\" data-service=\"spotify\" aria-current=\"true\">Spotify</a><a class=\"media-option\" href=\"https://podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135\" data-service=\"apple-podcasts\">Apple Podcasts</a></span><a class=\"media-open\" href=\"https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ\" target=\"_blank\" rel=\"noopener\">Open<span class=\"sr-only\" data-media-open-service> in Spotify</span><svg class=\"media-open-icon\" viewBox=\"0 0 16 16\" width=\"14\" height=\"14\" aria-hidden=\"true\" focusable=\"false\"><path d=\"M5 11 11 5M6.5 5H11v4.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.5\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/></svg></a></div>\n<div class=\"media-player\" data-pagefind-ignore=\"all\"><iframe src=\"https://open.spotify.com/embed/episode/512ojhOuo1ktJprKbVcKyQ\" title=\"Spotify player: An episode on Spotify\" height=\"152\" loading=\"lazy\" allow=\"autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture\" allowfullscreen data-service=\"spotify\"></iframe><template data-service=\"apple-podcasts\"><iframe src=\"https://embed.podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135\" title=\"Apple Podcasts player: An episode on Spotify\" height=\"175\" loading=\"lazy\" allow=\"autoplay; encrypted-media; clipboard-write\" sandbox=\"allow-forms allow-popups allow-same-origin allow-scripts allow-storage-access-by-user-activation allow-top-navigation-by-user-activation\" data-service=\"apple-podcasts\"></iframe></template></div>\n</figure>\n"
        );
    }

    #[test]
    fn escapes_what_it_shows_and_plays_no_music() {
        let links = media_paragraph("https://music.apple.com/jp/album/x/1?i=2 https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC").unwrap();
        let html = card_html(
            &links,
            &[details("https://music.apple.com/jp/album/x/1?i=2")],
        );
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<iframe"));
        assert!(!html.contains("Play here"));
        assert!(!html.contains('↗'));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; “Ode”"));
        // One service: no choice, and "Open" says where.
        let bare = card_html(&links[1..], &[]);
        assert!(bare.contains("A song on Spotify"));
        assert!(!bare.contains("media-switch"));
        assert!(bare.contains(">Open in Spotify<svg"));
        assert!(bare.contains(r#"href="https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC""#));
    }

    #[test]
    fn a_podcast_has_one_live_player_and_the_others_inert() {
        let show = media_paragraph(
            "https://podcasts.apple.com/jp/podcast/x/id1200361736 https://open.spotify.com/show/3IM0lmZxpFAY7CwMuv9H4g",
        )
        .unwrap();
        let html = card_html(&show, &[]);
        assert_eq!(html.matches("<iframe").count(), 2);
        assert_eq!(html.matches("<template").count(), 1);
        let live = &html[..html.find("<template").unwrap()];
        assert_eq!(live.matches("<iframe").count(), 1);
        assert!(
            live.contains("src=\"https://embed.podcasts.apple.com/jp/podcast/x/id1200361736\"")
        );
        assert!(live.contains("height=\"450\""));
        assert!(html.contains("height=\"352\""));
        assert!(!html.contains("autoplay=1"));
        let single = card_html(&show[1..], &[]);
        assert!(!single.contains("<template"));
        assert!(single.contains(">Open in Spotify<svg"));
    }
}
