//! Music cards: a paragraph that holds nothing but one Apple Music link, one
//! Spotify link, or one of each (for the same recording) becomes a card with
//! the work's title, its performers, and its artwork, and a choice of the
//! two services' own players. The players are not part of the page: the
//! site's script makes one only when a reader asks for it, so a page with
//! many cards loads no player at all until then, and without the script
//! each service is a plain link.
//!
//! Only these two services are recognised, and only their ordinary addresses
//! for songs, albums, and playlists; every address is taken apart and built
//! again from its parts, so nothing written in the text reaches a player
//! address or the page unchecked. The title, performers, and artwork come
//! from the file's front matter (`music:`), written by the Writing Studio
//! when it publishes; the build itself never asks either service anything.

use anyhow::{Result, bail, ensure};
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    AppleMusic,
    Spotify,
}

impl Service {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AppleMusic => "Apple Music",
            Self::Spotify => "Spotify",
        }
    }
    const fn key(self) -> &'static str {
        match self {
            Self::AppleMusic => "apple-music",
            Self::Spotify => "spotify",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Work {
    Song,
    Album,
    Playlist,
}

/// One link to a song, album, or playlist on one service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MusicLink {
    pub service: Service,
    pub work: Work,
    /// The address rebuilt from its parts, without tracking parameters: the
    /// key of its front matter entry, and where the card links.
    pub canonical: String,
    /// The service's own player for it.
    pub embed: String,
    /// The player's height in CSS pixels, as each service sizes it.
    pub height: u16,
}

const MAX_URL: usize = 600;

/// Parses an Apple Music or Spotify address, or says it is not one.
pub fn parse_music_url(url: &str) -> Option<MusicLink> {
    if url.len() > MAX_URL || !url.is_ascii() {
        return None;
    }
    if let Some(rest) = url.strip_prefix("https://music.apple.com/") {
        apple(rest)
    } else if let Some(rest) = url.strip_prefix("https://open.spotify.com/") {
        spotify(rest)
    } else {
        None
    }
}

fn split_query(rest: &str) -> Option<(&str, Vec<(&str, &str)>)> {
    if rest.contains('#') {
        return None;
    }
    let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
    let mut pairs = Vec::new();
    if !query.is_empty() {
        for pair in query.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            let plain = |s: &str| {
                s.len() <= 128
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b))
            };
            if key.is_empty() || !plain(key) || !plain(value) {
                return None;
            }
            pairs.push((key, value));
        }
    }
    Some((path, pairs))
}

fn digits(s: &str) -> bool {
    !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit())
}

fn apple(rest: &str) -> Option<MusicLink> {
    let (path, query) = split_query(rest)?;
    let parts: Vec<&str> = path.split('/').collect();
    let (country, kind, slug, id) = match parts.as_slice() {
        [country, kind, id] => (*country, *kind, None, *id),
        [country, kind, slug, id] => (*country, *kind, Some(*slug), *id),
        _ => return None,
    };
    if country.len() != 2 || !country.bytes().all(|b| b.is_ascii_lowercase()) {
        return None;
    }
    if let Some(slug) = slug {
        let ok = !slug.is_empty()
            && slug.len() <= 300
            && slug
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~%".contains(&b));
        if !ok {
            return None;
        }
    }
    // Only an album's track (`i`) changes what is meant; the language of the
    // page (`l`) and sharing and tracking parameters are left out.
    let mut track = None;
    for (key, value) in query {
        match key {
            "i" if digits(value) && track.is_none() => track = Some(value),
            "i" => return None,
            "l" | "ls" | "app" | "at" | "ct" | "itscg" | "itsct" | "uo" | "mt" | "pt" => {}
            _ => return None,
        }
    }
    let work = match kind {
        "album" if digits(id) => {
            if track.is_some() {
                Work::Song
            } else {
                Work::Album
            }
        }
        "song" if digits(id) && track.is_none() => Work::Song,
        "playlist"
            if track.is_none()
                && id.strip_prefix("pl.").is_some_and(|p| {
                    !p.is_empty()
                        && p.len() <= 64
                        && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                }) =>
        {
            Work::Playlist
        }
        _ => return None,
    };
    let tail = match slug {
        Some(slug) => format!("{country}/{kind}/{slug}/{id}"),
        None => format!("{country}/{kind}/{id}"),
    };
    let query = track.map(|i| format!("?i={i}")).unwrap_or_default();
    Some(MusicLink {
        service: Service::AppleMusic,
        work,
        canonical: format!("https://music.apple.com/{tail}{query}"),
        embed: format!("https://embed.music.apple.com/{tail}{query}"),
        height: if work == Work::Song { 175 } else { 450 },
    })
}

fn spotify(rest: &str) -> Option<MusicLink> {
    let (path, query) = split_query(rest)?;
    for (key, _) in query {
        if !matches!(key, "si" | "pi" | "nd" | "context" | "dl_branch") && !key.starts_with("utm_")
        {
            return None;
        }
    }
    let mut parts: Vec<&str> = path.split('/').collect();
    if parts.first().is_some_and(|p| p.starts_with("intl-")) {
        let locale = &parts[0]["intl-".len()..];
        if locale.is_empty()
            || locale.len() > 5
            || !locale.bytes().all(|b| b.is_ascii_alphabetic() || b == b'-')
        {
            return None;
        }
        parts.remove(0);
    }
    let [kind, id] = parts.as_slice() else {
        return None;
    };
    let work = match *kind {
        "track" => Work::Song,
        "album" => Work::Album,
        "playlist" => Work::Playlist,
        _ => return None,
    };
    if id.len() != 22 || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(MusicLink {
        service: Service::Spotify,
        work,
        canonical: format!("https://open.spotify.com/{kind}/{id}"),
        embed: format!("https://open.spotify.com/embed/{kind}/{id}"),
        height: if work == Work::Song { 152 } else { 352 },
    })
}

/// The links of a paragraph that is a music card: its whole text is one or
/// two music addresses, separated by spaces or a line break, at most one per
/// service, on lines that start at the margin. Anything else in the
/// paragraph and it is an ordinary paragraph.
pub fn music_paragraph(source: &str) -> Option<Vec<MusicLink>> {
    // Every line starts at the margin (the Writing Studio reads the text by
    // its lines, and must find the same cards).
    if source.lines().any(|line| line.starts_with([' ', '\t'])) {
        return None;
    }
    let words: Vec<&str> = source.split_ascii_whitespace().collect();
    if words.is_empty() || words.len() > 2 {
        return None;
    }
    let links = words
        .iter()
        .map(|word| parse_music_url(word))
        .collect::<Option<Vec<_>>>()?;
    // Two addresses are the same work on two services: one each, and the
    // same kind (a song with a song, an album with an album).
    if links.len() == 2 && (links[0].service == links[1].service || links[0].work != links[1].work)
    {
        return None;
    }
    Some(links)
}

/// What the Writing Studio found out about one address when it published:
/// a front matter entry under `music:`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MusicMetadata {
    /// The address in its canonical form ([`MusicLink::canonical`]).
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    /// A further line: for a song, its album or work.
    pub detail: Option<String>,
    /// The cover, on the service's own image server.
    pub artwork: Option<String>,
}

const ARTWORK_HOSTS: &[&str] = &[
    "is1-ssl.mzstatic.com",
    "is2-ssl.mzstatic.com",
    "is3-ssl.mzstatic.com",
    "is4-ssl.mzstatic.com",
    "is5-ssl.mzstatic.com",
    "i.scdn.co",
    "mosaic.scdn.co",
    "image-cdn-ak.spotifycdn.com",
    "image-cdn-fa.spotifycdn.com",
];

/// Whether an artwork address is on one of the services' image servers.
pub fn artwork_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else {
        return false;
    };
    let Some((host, path)) = rest.split_once('/') else {
        return false;
    };
    url.len() <= MAX_URL
        && ARTWORK_HOSTS.contains(&host)
        && !path.is_empty()
        && path
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._~%/".contains(&b))
}

fn plain_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}

/// Checks a file's `music:` entries: each names a canonical music address
/// once, with a title, and artwork only from the services' image servers.
pub fn validate_metadata(entries: &[MusicMetadata]) -> Result<()> {
    for (n, entry) in entries.iter().enumerate() {
        let Some(link) = parse_music_url(&entry.url) else {
            bail!(
                "music entry {}: {:?} is not an Apple Music or Spotify address",
                n + 1,
                entry.url
            );
        };
        ensure!(
            link.canonical == entry.url,
            "music entry {}: write the address as {}",
            n + 1,
            link.canonical
        );
        ensure!(
            entries[..n].iter().all(|other| other.url != entry.url),
            "music entry {}: {} is listed twice",
            n + 1,
            entry.url
        );
        ensure!(
            plain_text(&entry.title, 300),
            "music entry {}: the title must be plain text of at most 300 characters",
            n + 1
        );
        if let Some(artist) = &entry.artist {
            ensure!(
                plain_text(artist, 300),
                "music entry {}: the artist must be plain text of at most 300 characters",
                n + 1
            );
        }
        if let Some(detail) = &entry.detail {
            ensure!(
                plain_text(detail, 300),
                "music entry {}: the detail must be plain text of at most 300 characters",
                n + 1
            );
        }
        if let Some(artwork) = &entry.artwork {
            ensure!(
                artwork_allowed(artwork),
                "music entry {}: artwork must be on the services' own image servers",
                n + 1
            );
        }
    }
    Ok(())
}

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

const fn work_phrase(work: Work) -> &'static str {
    match work {
        Work::Song => "A song",
        Work::Album => "An album",
        Work::Playlist => "A playlist",
    }
}

const fn work_key(work: Work) -> &'static str {
    match work {
        Work::Song => "song",
        Work::Album => "album",
        Work::Playlist => "playlist",
    }
}

/// The card for a music paragraph: the work (cover, title, performers, a
/// further line), then one row of controls: the services (the first
/// written is chosen), "Play here", and a link to the chosen service.
///
/// Without the site's script every control is a link to a service, and
/// "Play here" (which only the script can do) keeps its place unseen, so
/// nothing moves when the script starts. The details are those of the first
/// address that has them; without any, the card says what kind of work it
/// is and where.
pub fn card_html(links: &[MusicLink], metadata: &[MusicMetadata]) -> String {
    let found = links
        .iter()
        .find_map(|link| metadata.iter().find(|m| m.url == link.canonical));
    let first = &links[0];
    let mut html = format!(
        "<figure class=\"music-card\" data-music-card data-work=\"{}\">\n<div class=\"music-card-body\">",
        work_key(first.work)
    );
    match found.and_then(|m| m.artwork.as_deref()) {
        Some(artwork) => html.push_str(&format!(
            "<img class=\"music-art\" src=\"{}\" alt=\"\" width=\"72\" height=\"72\" loading=\"lazy\" decoding=\"async\" referrerpolicy=\"no-referrer\">",
            escape(artwork)
        )),
        None => html.push_str("<span class=\"music-art music-art-none\" aria-hidden=\"true\"></span>"),
    }
    html.push_str("<figcaption class=\"music-caption\">");
    match found {
        Some(m) => {
            html.push_str(&format!(
                "<span class=\"music-title\">{}</span>",
                escape(&m.title)
            ));
            for (class, value) in [("music-artist", &m.artist), ("music-detail", &m.detail)] {
                if let Some(value) = value {
                    html.push_str(&format!("<span class=\"{class}\">{}</span>", escape(value)));
                }
            }
        }
        None => html.push_str(&format!(
            "<span class=\"music-title\">{} on {}</span>",
            work_phrase(first.work),
            first.service.name()
        )),
    }
    html.push_str(&format!(
        "</figcaption></div>\n<div class=\"music-actions\" data-pagefind-ignore=\"all\"><span class=\"music-switch\" data-count=\"{}\">",
        links.len()
    ));
    for (n, link) in links.iter().enumerate() {
        html.push_str(&format!(
            "<a class=\"music-option\" href=\"{}\" data-service=\"{}\" data-embed=\"{}\" data-height=\"{}\"{}>{}</a>",
            escape(&link.canonical),
            link.service.key(),
            escape(&link.embed),
            link.height,
            if n == 0 { " aria-current=\"true\"" } else { "" },
            link.service.name()
        ));
    }
    html.push_str(&format!(
        "</span><button class=\"music-play\" type=\"button\" disabled><span class=\"music-play-icon\" aria-hidden=\"true\"></span><span class=\"music-play-label\">Play here</span></button><a class=\"music-open\" href=\"{}\" target=\"_blank\" rel=\"noopener\" aria-label=\"Open in {}\"><span aria-hidden=\"true\">↗</span></a></div>\n</figure>\n",
        escape(&first.canonical),
        first.service.name()
    ));
    html
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_services_ordinary_addresses_and_rebuilds_them() {
        let song = parse_music_url(
            "https://music.apple.com/jp/album/symphony-no-9-in-d-minor-op-125/1440806071?i=1440806402&l=en-US&ls=1",
        )
        .unwrap();
        assert_eq!(song.service, Service::AppleMusic);
        assert_eq!(song.work, Work::Song);
        assert_eq!(
            song.canonical,
            "https://music.apple.com/jp/album/symphony-no-9-in-d-minor-op-125/1440806071?i=1440806402"
        );
        assert_eq!(
            song.embed,
            "https://embed.music.apple.com/jp/album/symphony-no-9-in-d-minor-op-125/1440806071?i=1440806402"
        );
        assert_eq!(
            parse_music_url("https://music.apple.com/us/album/1440806071")
                .unwrap()
                .work,
            Work::Album
        );
        assert_eq!(
            parse_music_url("https://music.apple.com/gb/song/x/1440806402")
                .unwrap()
                .work,
            Work::Song
        );
        assert_eq!(
            parse_music_url("https://music.apple.com/jp/playlist/name/pl.u-8aAVZAoCrWZJ")
                .unwrap()
                .work,
            Work::Playlist
        );
        assert_eq!(
            parse_music_url("https://music.apple.com/jp/playlist/name/pl.abc123")
                .unwrap()
                .work,
            Work::Playlist
        );
        let track = parse_music_url(
            "https://open.spotify.com/intl-ja/track/4uLU6hMCjMI75M1A2tKUQC?si=abc123",
        )
        .unwrap();
        assert_eq!(
            track.canonical,
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"
        );
        assert_eq!(
            track.embed,
            "https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC"
        );
        assert_eq!(
            parse_music_url("https://open.spotify.com/album/1ATL5GLyefJaxhQzSPVrLX")
                .unwrap()
                .work,
            Work::Album
        );
    }

    #[test]
    fn refuses_anything_else() {
        for url in [
            "http://music.apple.com/jp/album/x/1",
            "https://music.apple.com.evil.example/jp/album/x/1",
            "https://music.apple.com@evil.example/jp/album/x/1",
            "https://evil.example/https://music.apple.com/jp/album/x/1",
            "https://music.apple.com/jp/album/x/1?i=2&i=3",
            "https://music.apple.com/jp/album/x/1?redirect=https://evil.example",
            "https://music.apple.com/jp/album/x/1#t=1",
            "https://music.apple.com/jp/album/x\"onload=\"x/1",
            "https://music.apple.com/jp/album/<script>/1",
            "https://music.apple.com/JP/album/x/1",
            "https://music.apple.com/jp/artist/x/1",
            "https://music.apple.com/jp/album/x/abc",
            "https://music.apple.com/jp/song/x/1?i=2",
            "https://embed.music.apple.com/jp/album/x/1",
            "https://open.spotify.com/track/short",
            "https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC",
            "https://open.spotify.com/artist/4uLU6hMCjMI75M1A2tKUQC",
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?go=https://evil.example",
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC/../../x",
            "https://spotify.link/abc",
            "javascript:alert(1)",
            "https://music.apple.com/jp/album/x/１２３",
        ] {
            assert!(parse_music_url(url).is_none(), "{url}");
        }
    }

    #[test]
    fn a_card_is_a_paragraph_of_addresses_only() {
        let apple = "https://music.apple.com/jp/album/x/1?i=2";
        let spotify = "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC";
        assert_eq!(music_paragraph(apple).unwrap().len(), 1);
        assert_eq!(
            music_paragraph(&format!("{apple}\n{spotify}"))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            music_paragraph(&format!("{spotify} {apple} "))
                .unwrap()
                .len(),
            2
        );
        assert!(music_paragraph(&format!("{spotify}\n  {apple}")).is_none());
        assert!(music_paragraph(&format!("{spotify}\u{3000}{apple}")).is_none());
        assert!(music_paragraph(&format!("Listen: {apple}")).is_none());
        assert!(music_paragraph(&format!("{apple}\n{apple}")).is_none());
        // A song is paired with a song, an album with an album.
        assert!(
            music_paragraph(&format!("https://music.apple.com/jp/album/x/1 {spotify}")).is_none()
        );
        assert!(music_paragraph("https://music.apple.com/jp/album/x/1 https://open.spotify.com/album/1ATL5GLyefJaxhQzSPVrLX").is_some());
        assert!(music_paragraph(&format!("{apple} {spotify} {apple}")).is_none());
        assert!(music_paragraph("").is_none());
    }

    #[test]
    fn checks_the_front_matter_entries() {
        let entry = |url: &str, artwork: Option<&str>| MusicMetadata {
            url: url.into(),
            title: "Symphony No. 9".into(),
            artist: Some("Wiener Philharmoniker & Karl Böhm".into()),
            detail: None,
            artwork: artwork.map(Into::into),
        };
        let good = entry(
            "https://music.apple.com/jp/album/x/1?i=2",
            Some("https://is1-ssl.mzstatic.com/image/thumb/Music/v4/a/b/c/source/300x300bb.jpg"),
        );
        assert!(validate_metadata(&[good.clone()]).is_ok());
        assert!(validate_metadata(&[good.clone(), good.clone()]).is_err());
        assert!(
            validate_metadata(&[entry("https://music.apple.com/jp/album/x/1?i=2&l=en", None)])
                .is_err()
        );
        for artwork in [
            "http://is1-ssl.mzstatic.com/a.jpg",
            "https://evil.example/a.jpg",
            "https://is1-ssl.mzstatic.com.evil.example/a.jpg",
            "https://i.scdn.co/image/a\"onerror=\"x",
            "https://i.scdn.co/image/a?x=1",
            "javascript:alert(1)",
        ] {
            assert!(
                validate_metadata(&[entry(&good.url, Some(artwork))]).is_err(),
                "{artwork}"
            );
        }
        let mut bad = good.clone();
        bad.title = "line\nbreak".into();
        assert!(validate_metadata(&[bad]).is_err());
    }

    #[test]
    fn card_html_matches_the_studio() {
        // The same literal is asserted by the Writing Studio's renderer
        // (studio/test/music.test.ts), so the preview and the page agree.
        let links = music_paragraph(
            "https://music.apple.com/jp/album/x/1440806071?i=1440806402 https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC",
        )
        .unwrap();
        let metadata = [MusicMetadata {
            url: "https://music.apple.com/jp/album/x/1440806071?i=1440806402".into(),
            title: "<script>alert(1)</script> & “Ode”".into(),
            artist: Some("A \"quoted\" <b>artist</b>".into()),
            detail: None,
            artwork: None,
        }];
        assert_eq!(
            card_html(&links, &metadata),
            "<figure class=\"music-card\" data-music-card data-work=\"song\">\n<div class=\"music-card-body\"><span class=\"music-art music-art-none\" aria-hidden=\"true\"></span><figcaption class=\"music-caption\"><span class=\"music-title\">&lt;script&gt;alert(1)&lt;/script&gt; &amp; “Ode”</span><span class=\"music-artist\">A &quot;quoted&quot; &lt;b&gt;artist&lt;/b&gt;</span></figcaption></div>\n<div class=\"music-actions\" data-pagefind-ignore=\"all\"><span class=\"music-switch\" data-count=\"2\"><a class=\"music-option\" href=\"https://music.apple.com/jp/album/x/1440806071?i=1440806402\" data-service=\"apple-music\" data-embed=\"https://embed.music.apple.com/jp/album/x/1440806071?i=1440806402\" data-height=\"175\" aria-current=\"true\">Apple Music</a><a class=\"music-option\" href=\"https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC\" data-service=\"spotify\" data-embed=\"https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC\" data-height=\"152\">Spotify</a></span><button class=\"music-play\" type=\"button\" disabled><span class=\"music-play-icon\" aria-hidden=\"true\"></span><span class=\"music-play-label\">Play here</span></button><a class=\"music-open\" href=\"https://music.apple.com/jp/album/x/1440806071?i=1440806402\" target=\"_blank\" rel=\"noopener\" aria-label=\"Open in Apple Music\"><span aria-hidden=\"true\">↗</span></a></div>\n</figure>\n"
        );
    }

    #[test]
    fn escapes_what_it_shows_and_builds_no_player() {
        let links = music_paragraph("https://music.apple.com/jp/album/x/1?i=2 https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC").unwrap();
        let metadata = [MusicMetadata {
            url: "https://music.apple.com/jp/album/x/1?i=2".into(),
            title: "<script>alert(1)</script> & “Ode”".into(),
            artist: Some("A \"quoted\" <b>artist</b>".into()),
            detail: None,
            artwork: None,
        }];
        let html = card_html(&links, &metadata);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("<b>"));
        assert!(!html.contains("<iframe"));
        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt; &amp; “Ode”"));
        assert!(html.contains(r#"data-service="apple-music""#));
        // "Play here" is the script's alone: without it, it does nothing.
        assert!(html.contains(r#"<button class="music-play" type="button" disabled>"#));
        assert!(html.contains(
            r#"data-embed="https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC""#
        ));
        // Without details: what it is, and where.
        let bare = card_html(&links[1..], &[]);
        assert!(bare.contains("A song on Spotify"));
        assert!(bare.contains(r#"href="https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC""#));
    }
}
