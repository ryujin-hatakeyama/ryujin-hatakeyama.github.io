# Media cards

A paragraph of Miscellany text that holds only one or two addresses of the
same work becomes a media card (`site-builder/src/media/`; how to write one:
`content/miscellany/README.md`):

| Kind | Services | Addresses |
|---|---|---|
| Music | Apple Music, Spotify | song (track), album, playlist |
| Podcast | Apple Podcasts, Spotify | show, episode |

Two addresses must name the same kind of work on two services (a song with a
song, an episode with an episode); anything else is an ordinary paragraph.

## What the page holds

- **Static HTML, one card for both kinds.** The build writes the card: cover
  (72×72, `loading="lazy"`, its size reserved), title, a second line
  (performers, or an episode's show), a further line, and `data-media-kind`
  saying which kind it is.
- **Music opens the service; nothing plays on the page.** With two services
  the card has a choice of service and one action, **Open**, for the chosen
  one (a link, in a new tab). With one service there is no choice to draw:
  the action says where (**Open in Spotify**). The arrow beside it is an
  inline SVG in the text's colour, hidden from assistive technology; the
  word stays.
- **A podcast has its service's own player** (`embed.podcasts.apple.com`,
  `open.spotify.com/embed`), an `iframe` with `loading="lazy"` and no
  autoplay, at the service's size (Apple: an episode 175px, a show 450px;
  Spotify: an episode 152px, a show 352px), with the permissions each
  service's embed code asks for (Apple sandboxes its player; Spotify's needs
  none). Each other service's player is in a `<template>`, inert, so a card
  never loads two. Apple asks for at least 290px: a card on a phone gives its
  player that from a 360px-wide screen (on a 320px screen it has about 275px).
- **The script** (`client/media.ts`, loaded by `client/site.ts` only on a
  page with a card that offers two services) makes the services one choice
  (radio buttons, arrow keys): choosing changes where **Open** goes, and for
  a podcast swaps the player for the chosen service's (removing the other,
  and its sound). A modified click on a service opens its page, as a link.
  Without the script each service is a plain link, **Open** opens the first,
  and a podcast plays in the first service's player.
- **Addresses rebuilt, never copied.** Each address is taken apart and built
  again from its parts (service, storefront, kind, id, track or episode);
  player addresses are made from those parts in Rust. The script trusts the
  build's markup and parses no address. Titles and lines are escaped text;
  covers only from the services' own image servers (`is1–5-ssl.mzstatic.com`,
  `i.scdn.co`, `mosaic.scdn.co`, `image-cdn-ak/fa.spotifycdn.com`), linked,
  never copied into the repository.
- **Colours.** The card follows the site's theme (paper, ink, rules) in Light
  and Dark. The services' colours appear only as a small mark beside their
  names; the chosen one is also told by its raised pill and weight.
- **Search.** Titles and lines are indexed; the controls and players are not.

## Where the details come from

The Writing Studio finds them when the text is reviewed for publishing
(`studio/src/media/lookup.ts`) and writes them into the published file's
front matter: `music:` (with `artist`) and `podcasts:` (with `show`). The
build reads only that and checks each entry against its section's kind: a
service being down never stops it, and the page never asks the services
anything.

| Service | Source | Key needed | Gives |
|---|---|---|---|
| Apple Music | iTunes Lookup API | none | title, performers, album, length, release date, cover |
| Apple Podcasts | iTunes Lookup API | none | a show's name, publisher, cover; an episode's title, show, date, length (among the show's latest 100) |
| Spotify | oEmbed | none | title, cover (no performers; nothing for a show) |

## Rollback

Revert the commit. Published files with `podcasts:` front matter would then
fail the build (front matter fields are checked strictly), so remove those
entries too; `music:` entries are read by the earlier version as well.
