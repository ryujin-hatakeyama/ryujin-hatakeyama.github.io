//! What a media card is about, as types only: which service, what kind of
//! work, its address, and, for a podcast, the service's own player.

use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    AppleMusic,
    ApplePodcasts,
    Spotify,
}

impl Provider {
    pub const fn name(self) -> &'static str {
        match self {
            Self::AppleMusic => "Apple Music",
            Self::ApplePodcasts => "Apple Podcasts",
            Self::Spotify => "Spotify",
        }
    }
    pub const fn key(self) -> &'static str {
        match self {
            Self::AppleMusic => "apple-music",
            Self::ApplePodcasts => "apple-podcasts",
            Self::Spotify => "spotify",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MusicWork {
    Song,
    Album,
    Playlist,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PodcastWork {
    Show,
    Episode,
}

/// What an address names. Two addresses make one card only when they name
/// the same kind (a song with a song, an episode with an episode).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaKind {
    Music(MusicWork),
    Podcast(PodcastWork),
}

impl MediaKind {
    pub const fn key(self) -> &'static str {
        match self {
            Self::Music(_) => "music",
            Self::Podcast(_) => "podcast",
        }
    }
    pub const fn work_key(self) -> &'static str {
        match self {
            Self::Music(MusicWork::Song) => "song",
            Self::Music(MusicWork::Album) => "album",
            Self::Music(MusicWork::Playlist) => "playlist",
            Self::Podcast(PodcastWork::Show) => "show",
            Self::Podcast(PodcastWork::Episode) => "episode",
        }
    }
    /// What the card says without details: "A song", "An episode".
    pub const fn phrase(self) -> &'static str {
        match self {
            Self::Music(MusicWork::Song) => "A song",
            Self::Music(MusicWork::Album) => "An album",
            Self::Music(MusicWork::Playlist) => "A playlist",
            Self::Podcast(PodcastWork::Show) => "A podcast",
            Self::Podcast(PodcastWork::Episode) => "An episode",
        }
    }
}

/// A service's own player, at an address rebuilt from the parsed parts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmbedSpec {
    pub url: String,
    /// The player's height in CSS pixels, as the service sizes it.
    pub height: u16,
}

/// One address on one service.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaLink {
    pub provider: Provider,
    pub kind: MediaKind,
    /// The address rebuilt from its parts, without tracking parameters: the
    /// key of its front matter entry, and where the card links.
    pub canonical: String,
    /// A podcast's player; music is never played on the page.
    pub embed: Option<EmbedSpec>,
}

/// What a card shows about its work, from either section of the front
/// matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaDetails {
    pub url: String,
    pub title: String,
    /// The second line: a song's performers, an episode's show.
    pub byline: Option<String>,
    /// A further line: a song's album, an episode's date.
    pub detail: Option<String>,
    /// The cover, on the service's own image server.
    pub artwork: Option<String>,
}

/// A front matter entry under `music:`, written by the Writing Studio when
/// it publishes.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MusicMetadata {
    /// The address in its canonical form ([`MediaLink::canonical`]).
    pub url: String,
    pub title: String,
    pub artist: Option<String>,
    pub detail: Option<String>,
    pub artwork: Option<String>,
}

/// A front matter entry under `podcasts:`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PodcastMetadata {
    pub url: String,
    pub title: String,
    /// For an episode, the show it belongs to.
    pub show: Option<String>,
    pub detail: Option<String>,
    pub artwork: Option<String>,
}

impl From<&MusicMetadata> for MediaDetails {
    fn from(m: &MusicMetadata) -> Self {
        Self {
            url: m.url.clone(),
            title: m.title.clone(),
            byline: m.artist.clone(),
            detail: m.detail.clone(),
            artwork: m.artwork.clone(),
        }
    }
}

impl From<&PodcastMetadata> for MediaDetails {
    fn from(p: &PodcastMetadata) -> Self {
        Self {
            url: p.url.clone(),
            title: p.title.clone(),
            byline: p.show.clone(),
            detail: p.detail.clone(),
            artwork: p.artwork.clone(),
        }
    }
}
