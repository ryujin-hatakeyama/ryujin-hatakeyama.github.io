//! Media cards: a paragraph that holds nothing but one or two addresses of
//! the same work (an Apple Music or Spotify song, album, or playlist; an
//! Apple Podcasts or Spotify podcast or episode) becomes a card with the
//! work's title, a second line, and artwork, and a link that opens the
//! chosen service. Music is never played on the page; a podcast has its
//! service's own player.
//!
//! - `model`: the types;
//! - `url`: which addresses are recognised, and how each is rebuilt;
//! - `render`: the card's static HTML.
//!
//! The details come from the file's front matter, written by the Writing
//! Studio when it publishes (`music:` and `podcasts:`, checked here); the
//! build itself never asks any service anything.

mod model;
mod render;
mod url;

use anyhow::{Result, bail, ensure};

pub use model::{
    EmbedSpec, MediaDetails, MediaKind, MediaLink, MusicMetadata, MusicWork, PodcastMetadata,
    PodcastWork, Provider,
};
pub use render::card_html;
pub use url::{artwork_allowed, media_paragraph, parse_media_url};

fn plain_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
}

/// Checks one entry: a canonical address of the section's kind, listed once,
/// with a title, plain lines, and artwork only from the services' image
/// servers.
fn check(
    section: &str,
    n: usize,
    entry: &MediaDetails,
    earlier: &[MediaDetails],
    is_kind: fn(MediaKind) -> bool,
    byline: &str,
) -> Result<()> {
    let n = n + 1;
    let Some(link) = parse_media_url(&entry.url).filter(|link| is_kind(link.kind)) else {
        bail!(
            "{section} entry {n}: {:?} is not a {section} address",
            entry.url
        );
    };
    ensure!(
        link.canonical == entry.url,
        "{section} entry {n}: write the address as {}",
        link.canonical
    );
    ensure!(
        earlier.iter().all(|other| other.url != entry.url),
        "{section} entry {n}: {} is listed twice",
        entry.url
    );
    ensure!(
        plain_text(&entry.title, 300),
        "{section} entry {n}: the title must be plain text of at most 300 characters"
    );
    for (field, value) in [(byline, &entry.byline), ("detail", &entry.detail)] {
        if let Some(value) = value {
            ensure!(
                plain_text(value, 300),
                "{section} entry {n}: the {field} must be plain text of at most 300 characters"
            );
        }
    }
    if let Some(artwork) = &entry.artwork {
        ensure!(
            artwork_allowed(artwork),
            "{section} entry {n}: artwork must be on the services' own image servers"
        );
    }
    Ok(())
}

/// Checks a file's `music:` and `podcasts:` entries, and gives them as one
/// list of details for its cards.
pub fn details(music: &[MusicMetadata], podcasts: &[PodcastMetadata]) -> Result<Vec<MediaDetails>> {
    let mut all: Vec<MediaDetails> = Vec::with_capacity(music.len() + podcasts.len());
    for (n, entry) in music.iter().enumerate() {
        let entry = MediaDetails::from(entry);
        check(
            "music",
            n,
            &entry,
            &all,
            |k| matches!(k, MediaKind::Music(_)),
            "artist",
        )?;
        all.push(entry);
    }
    for (n, entry) in podcasts.iter().enumerate() {
        let entry = MediaDetails::from(entry);
        check(
            "podcast",
            n,
            &entry,
            &all,
            |k| matches!(k, MediaKind::Podcast(_)),
            "show",
        )?;
        all.push(entry);
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn music(url: &str, artwork: Option<&str>) -> MusicMetadata {
        MusicMetadata {
            url: url.into(),
            title: "Symphony No. 9".into(),
            artist: Some("Wiener Philharmoniker & Karl Böhm".into()),
            detail: None,
            artwork: artwork.map(Into::into),
        }
    }

    #[test]
    fn checks_the_front_matter_entries() {
        let good = music(
            "https://music.apple.com/jp/album/x/1?i=2",
            Some("https://is1-ssl.mzstatic.com/image/thumb/Music/v4/a/b/c/source/300x300bb.jpg"),
        );
        assert!(details(&[good.clone()], &[]).is_ok());
        assert!(details(&[good.clone(), good.clone()], &[]).is_err());
        assert!(
            details(
                &[music("https://music.apple.com/jp/album/x/1?i=2&l=en", None)],
                &[]
            )
            .is_err()
        );
        assert!(details(&[music(&good.url, Some("https://evil.example/a.jpg"))], &[]).is_err());
        let mut bad = good.clone();
        bad.title = "line\nbreak".into();
        assert!(details(&[bad], &[]).is_err());

        let episode = PodcastMetadata {
            url: "https://podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135".into(),
            title: "The Battle Over Black Representation in the South".into(),
            show: Some("The Daily".into()),
            detail: Some("23 Oct 2026 · 30 min".into()),
            artwork: None,
        };
        let all = details(&[good.clone()], &[episode.clone()]).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].byline.as_deref(), Some("The Daily"));
        // Each section holds only its own kind of address.
        assert!(details(&[music(&episode.url, None)], &[]).is_err());
        assert!(
            details(
                &[],
                &[PodcastMetadata {
                    url: good.url.clone(),
                    ..episode.clone()
                }]
            )
            .is_err()
        );
        assert!(details(&[], &[episode.clone(), episode]).is_err());
    }
}
