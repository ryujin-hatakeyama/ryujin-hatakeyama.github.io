# Music cards

A paragraph of Miscellany text that holds only an Apple Music address, a
Spotify address, or one of each, becomes a music card
(`site-builder/src/music.rs`; how to write one: `content/miscellany/README.md`).

## What the page holds

- **Static HTML only.** The card is written by the build: cover (72×72,
  `loading="lazy"`, its size reserved), title, performers, and one link per
  service. No player, script, or request to either service is part of the page.
- **Players on request.** With the site's script (`client/site.ts`,
  `setUpMusicCards`, about 3 KB before compression), each service link
  becomes a toggle: the chosen service's own player (an `iframe` of
  `embed.music.apple.com` or `open.spotify.com/embed`) is made only when
  pressed, never starts by itself, and replaces the other in that card.
  The links keep their size and place when the script starts, so nothing
  moves. Without the script each service is a plain link.
- **Addresses rebuilt, never copied.** Each address is taken apart and built
  again from its parts (service, storefront, kind, id, track); anything else
  is not a card. Player addresses are made from those parts, and the
  script accepts only those two services' player addresses. Titles and
  performers are escaped text; covers only from the services' own image
  servers (`is1–5-ssl.mzstatic.com`, `i.scdn.co`, `mosaic.scdn.co`,
  `image-cdn-ak/fa.spotifycdn.com`). Images are linked, never copied into the
  repository.
- **Colours.** The card follows the site's theme (paper, ink, rules) in Light
  and Dark. The services' colours appear only as a small mark on each
  choice; the chosen one is also told by its outline and weight.
- **Search.** Titles and performers are indexed; the service links are not.

## Where the details come from

The Writing Studio finds them when the text is reviewed for publishing
(`studio/src/music/lookup.ts`) and writes them into the published file's
front matter (`music:`). The build reads only that: a service being down
never stops it, and the page never asks the services anything.

| Service | Source | Key needed | Gives |
|---|---|---|---|
| Apple Music | iTunes Lookup API | none | title, performers, album, length, release date, cover |
| Spotify | oEmbed | none | title, cover (no performers) |

## Rollback

Revert the commit. Published files that have `music:` front matter would then
fail the build (front matter fields are checked strictly), so remove those
entries too, or revert only `client/site.ts` and the CSS to keep cards as
links.
