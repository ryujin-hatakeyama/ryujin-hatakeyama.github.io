//! Media addresses: Apple Music, Apple Podcasts, and Spotify, and only their
//! ordinary addresses for songs, albums, playlists, podcasts, and episodes.
//! Every address is taken apart and built again from its parts, and a
//! podcast's player address is made from those parts too, so nothing written
//! in the text reaches the page or a player unchecked.

use super::model::{EmbedSpec, MediaKind, MediaLink, MusicWork, PodcastWork, Provider};

const MAX_URL: usize = 600;

/// Parses a media address, or says it is not one.
pub fn parse_media_url(url: &str) -> Option<MediaLink> {
    if url.len() > MAX_URL || !url.is_ascii() {
        return None;
    }
    if let Some(rest) = url.strip_prefix("https://music.apple.com/") {
        apple_music(rest)
    } else if let Some(rest) = url.strip_prefix("https://podcasts.apple.com/") {
        apple_podcasts(rest)
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

/// An Apple address's storefront, its kind, its optional name, and its id,
/// with the one parameter that changes what is meant (`i`: an album's track,
/// a podcast's episode). The language of the page (`l`) and sharing and
/// tracking parameters are left out.
struct AppleParts<'a> {
    country: &'a str,
    kind: &'a str,
    slug: Option<&'a str>,
    id: &'a str,
    item: Option<&'a str>,
}

impl AppleParts<'_> {
    /// The address after the host, rebuilt from the parts.
    fn tail(&self) -> String {
        let Self {
            country, kind, id, ..
        } = self;
        let path = match self.slug {
            Some(slug) => format!("{country}/{kind}/{slug}/{id}"),
            None => format!("{country}/{kind}/{id}"),
        };
        let query = self.item.map(|i| format!("?i={i}")).unwrap_or_default();
        format!("{path}{query}")
    }
}

fn apple_parts(rest: &str) -> Option<AppleParts<'_>> {
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
    let mut item = None;
    for (key, value) in query {
        match key {
            "i" if digits(value) && item.is_none() => item = Some(value),
            "i" => return None,
            "l" | "ls" | "app" | "at" | "ct" | "itscg" | "itsct" | "uo" | "mt" | "pt" => {}
            _ => return None,
        }
    }
    Some(AppleParts {
        country,
        kind,
        slug,
        id,
        item,
    })
}

fn apple_music(rest: &str) -> Option<MediaLink> {
    let parts = apple_parts(rest)?;
    let (id, track) = (parts.id, parts.item);
    let work = match parts.kind {
        "album" if digits(id) => {
            if track.is_some() {
                MusicWork::Song
            } else {
                MusicWork::Album
            }
        }
        "song" if digits(id) && track.is_none() => MusicWork::Song,
        "playlist"
            if track.is_none()
                && id.strip_prefix("pl.").is_some_and(|p| {
                    !p.is_empty()
                        && p.len() <= 64
                        && p.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
                }) =>
        {
            MusicWork::Playlist
        }
        _ => return None,
    };
    Some(MediaLink {
        provider: Provider::AppleMusic,
        kind: MediaKind::Music(work),
        canonical: format!("https://music.apple.com/{}", parts.tail()),
        embed: None,
    })
}

fn apple_podcasts(rest: &str) -> Option<MediaLink> {
    let parts = apple_parts(rest)?;
    if parts.kind != "podcast" || !parts.id.strip_prefix("id").is_some_and(digits) {
        return None;
    }
    let tail = parts.tail();
    // Apple's own player: an episode's compact one, or the show's with its
    // list of episodes.
    let (work, height) = match parts.item {
        Some(_) => (PodcastWork::Episode, 175),
        None => (PodcastWork::Show, 450),
    };
    Some(MediaLink {
        provider: Provider::ApplePodcasts,
        kind: MediaKind::Podcast(work),
        canonical: format!("https://podcasts.apple.com/{tail}"),
        embed: Some(EmbedSpec {
            url: format!("https://embed.podcasts.apple.com/{tail}"),
            height,
        }),
    })
}

fn spotify(rest: &str) -> Option<MediaLink> {
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
    let kind_of = match *kind {
        "track" => MediaKind::Music(MusicWork::Song),
        "album" => MediaKind::Music(MusicWork::Album),
        "playlist" => MediaKind::Music(MusicWork::Playlist),
        "show" => MediaKind::Podcast(PodcastWork::Show),
        "episode" => MediaKind::Podcast(PodcastWork::Episode),
        _ => return None,
    };
    if id.len() != 22 || !id.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    // Spotify's own player: an episode's compact one, or the show's with its
    // episodes.
    let embed = match kind_of {
        MediaKind::Music(_) => None,
        MediaKind::Podcast(work) => Some(EmbedSpec {
            url: format!("https://open.spotify.com/embed/{kind}/{id}"),
            height: if work == PodcastWork::Episode {
                152
            } else {
                352
            },
        }),
    };
    Some(MediaLink {
        provider: Provider::Spotify,
        kind: kind_of,
        canonical: format!("https://open.spotify.com/{kind}/{id}"),
        embed,
    })
}

/// The links of a paragraph that is a media card: its whole text is one or
/// two media addresses, separated by spaces or a line break, at most one per
/// service and both of the same kind, on lines that start at the margin.
/// Anything else in the paragraph and it is an ordinary paragraph.
pub fn media_paragraph(source: &str) -> Option<Vec<MediaLink>> {
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
        .map(|word| parse_media_url(word))
        .collect::<Option<Vec<_>>>()?;
    // Two addresses are the same work on two services: one each, and the
    // same kind (a song with a song, an episode with an episode).
    if links.len() == 2
        && (links[0].provider == links[1].provider || links[0].kind != links[1].kind)
    {
        return None;
    }
    Some(links)
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

#[cfg(test)]
mod tests {
    use super::*;

    const SONG: MediaKind = MediaKind::Music(MusicWork::Song);
    const EPISODE: MediaKind = MediaKind::Podcast(PodcastWork::Episode);
    const SHOW: MediaKind = MediaKind::Podcast(PodcastWork::Show);

    #[test]
    fn reads_the_music_services_ordinary_addresses_and_rebuilds_them() {
        let song = parse_media_url(
            "https://music.apple.com/jp/album/symphony-no-9-in-d-minor-op-125/1440806071?i=1440806402&l=en-US&ls=1",
        )
        .unwrap();
        assert_eq!(song.provider, Provider::AppleMusic);
        assert_eq!(song.kind, SONG);
        assert_eq!(
            song.canonical,
            "https://music.apple.com/jp/album/symphony-no-9-in-d-minor-op-125/1440806071?i=1440806402"
        );
        // Music is never played on the page.
        assert_eq!(song.embed, None);
        let kind = |url: &str| parse_media_url(url).unwrap().kind;
        assert_eq!(
            kind("https://music.apple.com/us/album/1440806071"),
            MediaKind::Music(MusicWork::Album)
        );
        assert_eq!(kind("https://music.apple.com/gb/song/x/1440806402"), SONG);
        assert_eq!(
            kind("https://music.apple.com/jp/playlist/name/pl.u-8aAVZAoCrWZJ"),
            MediaKind::Music(MusicWork::Playlist)
        );
        assert_eq!(
            kind("https://music.apple.com/jp/playlist/name/pl.abc123"),
            MediaKind::Music(MusicWork::Playlist)
        );
        let track = parse_media_url(
            "https://open.spotify.com/intl-ja/track/4uLU6hMCjMI75M1A2tKUQC?si=abc123",
        )
        .unwrap();
        assert_eq!(
            track.canonical,
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC"
        );
        assert_eq!(track.embed, None);
        assert_eq!(
            kind("https://open.spotify.com/album/1ATL5GLyefJaxhQzSPVrLX"),
            MediaKind::Music(MusicWork::Album)
        );
    }

    #[test]
    fn reads_podcast_addresses_and_builds_their_players() {
        let episode = parse_media_url(
            "https://podcasts.apple.com/us/podcast/the-battle-over/id1200361736?i=1000797757135&uo=4&l=ja",
        )
        .unwrap();
        assert_eq!(episode.provider, Provider::ApplePodcasts);
        assert_eq!(episode.kind, EPISODE);
        assert_eq!(
            episode.canonical,
            "https://podcasts.apple.com/us/podcast/the-battle-over/id1200361736?i=1000797757135"
        );
        assert_eq!(
            episode.embed,
            Some(EmbedSpec {
                url: "https://embed.podcasts.apple.com/us/podcast/the-battle-over/id1200361736?i=1000797757135".into(),
                height: 175,
            })
        );
        let show = parse_media_url("https://podcasts.apple.com/jp/podcast/id1200361736").unwrap();
        assert_eq!(show.kind, SHOW);
        assert_eq!(
            show.embed.unwrap(),
            EmbedSpec {
                url: "https://embed.podcasts.apple.com/jp/podcast/id1200361736".into(),
                height: 450,
            }
        );
        let spotify = parse_media_url(
            "https://open.spotify.com/intl-ja/episode/512ojhOuo1ktJprKbVcKyQ?si=x&utm_source=copy-link",
        )
        .unwrap();
        assert_eq!(spotify.kind, EPISODE);
        assert_eq!(
            spotify.canonical,
            "https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ"
        );
        assert_eq!(
            spotify.embed.unwrap(),
            EmbedSpec {
                url: "https://open.spotify.com/embed/episode/512ojhOuo1ktJprKbVcKyQ".into(),
                height: 152,
            }
        );
        let show = parse_media_url("https://open.spotify.com/show/3IM0lmZxpFAY7CwMuv9H4g").unwrap();
        assert_eq!(show.kind, SHOW);
        assert_eq!(show.embed.unwrap().height, 352);
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
            "https://music.apple.com/jp/podcast/x/id1",
            "https://embed.music.apple.com/jp/album/x/1",
            "https://open.spotify.com/track/short",
            "https://open.spotify.com/embed/track/4uLU6hMCjMI75M1A2tKUQC",
            "https://open.spotify.com/artist/4uLU6hMCjMI75M1A2tKUQC",
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC?go=https://evil.example",
            "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC/../../x",
            "https://spotify.link/abc",
            "javascript:alert(1)",
            "https://music.apple.com/jp/album/x/１２３",
            // Podcasts.
            "http://podcasts.apple.com/us/podcast/x/id1",
            "https://podcasts.apple.com.evil.example/us/podcast/x/id1",
            "https://embed.podcasts.apple.com/us/podcast/x/id1",
            "https://podcasts.apple.com/us/podcast/x/1",
            "https://podcasts.apple.com/us/podcast/x/idabc",
            "https://podcasts.apple.com/us/album/x/id1",
            "https://podcasts.apple.com/us/channel/x/id1",
            "https://podcasts.apple.com/US/podcast/x/id1",
            "https://podcasts.apple.com/us/podcast/x/id1?i=2&i=3",
            "https://podcasts.apple.com/us/podcast/x/id1?i=ep",
            "https://podcasts.apple.com/us/podcast/x/id1?next=https://evil.example",
            "https://podcasts.apple.com/us/podcast/x/id1#t=60",
            "https://podcasts.apple.com/us/podcast/x\"onload=\"x/id1",
            "https://open.spotify.com/embed/episode/512ojhOuo1ktJprKbVcKyQ",
            "https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ?t=60",
            "https://open.spotify.com/show/short",
            "https://open.spotify.com/audiobook/512ojhOuo1ktJprKbVcKyQ",
        ] {
            assert!(parse_media_url(url).is_none(), "{url}");
        }
    }

    #[test]
    fn a_card_is_a_paragraph_of_addresses_of_one_kind() {
        let apple = "https://music.apple.com/jp/album/x/1?i=2";
        let spotify = "https://open.spotify.com/track/4uLU6hMCjMI75M1A2tKUQC";
        assert_eq!(media_paragraph(apple).unwrap().len(), 1);
        assert_eq!(
            media_paragraph(&format!("{apple}\n{spotify}"))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            media_paragraph(&format!("{spotify} {apple} "))
                .unwrap()
                .len(),
            2
        );
        assert!(media_paragraph(&format!("{spotify}\n  {apple}")).is_none());
        assert!(media_paragraph(&format!("{spotify}\u{3000}{apple}")).is_none());
        assert!(media_paragraph(&format!("Listen: {apple}")).is_none());
        assert!(media_paragraph(&format!("{apple}\n{apple}")).is_none());
        // A song is paired with a song, an album with an album.
        assert!(
            media_paragraph(&format!("https://music.apple.com/jp/album/x/1 {spotify}")).is_none()
        );
        assert!(media_paragraph("https://music.apple.com/jp/album/x/1 https://open.spotify.com/album/1ATL5GLyefJaxhQzSPVrLX").is_some());
        assert!(media_paragraph(&format!("{apple} {spotify} {apple}")).is_none());
        assert!(media_paragraph("").is_none());

        // Podcasts: an episode with an episode, a show with a show, never
        // with music, and one address per service.
        let apple_episode = "https://podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135";
        let spotify_episode = "https://open.spotify.com/episode/512ojhOuo1ktJprKbVcKyQ";
        let apple_show = "https://podcasts.apple.com/us/podcast/x/id1200361736";
        let spotify_show = "https://open.spotify.com/show/3IM0lmZxpFAY7CwMuv9H4g";
        assert!(media_paragraph(&format!("{apple_episode}\n{spotify_episode}")).is_some());
        assert!(media_paragraph(&format!("{spotify_show} {apple_show}")).is_some());
        assert!(media_paragraph(&format!("{apple_episode} {spotify_show}")).is_none());
        assert!(media_paragraph(&format!("{apple_show} {spotify}")).is_none());
        assert!(media_paragraph(&format!("{spotify_show} {spotify_episode}")).is_none());
        assert!(media_paragraph(&format!("{apple} {apple_episode}")).is_none());
    }

    #[test]
    fn artwork_only_from_the_services_image_servers() {
        assert!(artwork_allowed(
            "https://is1-ssl.mzstatic.com/image/thumb/Music/v4/a/b/c/source/300x300bb.jpg"
        ));
        assert!(artwork_allowed(
            "https://image-cdn-ak.spotifycdn.com/image/ab67656300005f1fc209144b2c67a524fcbaa70c"
        ));
        for artwork in [
            "http://is1-ssl.mzstatic.com/a.jpg",
            "https://evil.example/a.jpg",
            "https://is1-ssl.mzstatic.com.evil.example/a.jpg",
            "https://i.scdn.co/image/a\"onerror=\"x",
            "https://i.scdn.co/image/a?x=1",
            "javascript:alert(1)",
        ] {
            assert!(!artwork_allowed(artwork), "{artwork}");
        }
    }
}
