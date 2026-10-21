# Update records

Each `*.json` file in this directory is a single update. The OCaml generator validates every record before Rust sees it. Drafts are validated but omitted from `target/generated/updates.json`.

This is a schema example only; it is not read by the build:

```json
{
  "id": "stable-lowercase-slug",
  "title": { "en": "Required English title", "ja": "Optional Japanese title" },
  "title_link": { "text": "Event name as it appears in the title", "url": "https://example.org/event", "lang": "ja" },
  "summary": { "en": "Required English summary", "ja": "Optional Japanese summary" },
  "date": "YYYY-MM-DD",
  "end_date": "YYYY-MM-DD",
  "event_status": "completed",
  "categories": ["research", "writing"],
  "kind": { "type": "presentation" },
  "links": [
    { "label": "Slides", "url": "https://example.org/slides", "type": "slides" }
  ],
  "related": { "publications": [], "projects": [], "writings": [] },
  "detail": false,
  "status": "draft"
}
```

`date` is the day the activity begins and the optional `end_date` its last day. These are the only dates a record carries, and every view shows and orders records by these dates.

Updates also lists published Miscellany articles that state the day they were published (`content/miscellany/README.md`). They are not records in this directory and are never written here: the Rust build derives them from the articles. On every page, records and articles form one list (the RSS feed lists records only), most recent first, by a record's final day or an article's publication day, then by a record's first day, then by id, so entries of one day always appear in the same order. Article entries are in the Writing category and use ids beginning `published-`, which records may not use.

Event names follow their organizers: use an official English name only when one can be verified, and otherwise keep the original name. The optional `title_link` links the event's name, and only that, to the event's primary official resource, such as its official page or official flyer: `text` must occur exactly once in `title.en`, `url` must use HTTP(S), and `lang` (`en` or `ja`) marks the language of the linked text when it differs from English. Omit it when the event itself has no official resource; never point it at a broader organization's homepage. `links` then holds only additional, distinct resources: a link to the title link's destination (a trailing slash makes no difference) fails validation. A record with a detail page offers it as a separate "Details" link. RSS and page metadata keep the plain title.

The English summary may contain inline links written as `[text](https://…)`; only HTTP(S) URLs form links, and RSS and meta descriptions show the link text alone. Any other text, including brackets that do not form such a link, is shown verbatim.

While a record is a draft awaiting confirmation, `date` may be omitted rather than guessed; a published record must have it.

`event_status` is required and is either `completed` or `planned`. It is the record's editorial status: whether the recorded activity has taken place (`completed`) or is still intended (`planned`). It is unrelated to `status`, which only controls whether the record is a draft. A `planned` record should be worded as an intention ("I plan to attend …"). Only the author changes it: the build never converts a planned record, because an event having begun or ended does not mean that you took part.

Where a record appears depends on that status and on the build day, a calendar day in Asia/Tokyo; nothing derived from the day is stored in the record. A single-day event ends on its `date`.

| `event_status` | Build day | Shown under |
| --- | --- | --- |
| `planned` | before `date` | Upcoming |
| `planned` | from `date` to the last day | Upcoming, marked "Ongoing" |
| `planned` | after the last day | "Previously announced" on the Updates pages, and a build warning |
| `completed` | any | Recent Updates |

Upcoming is ordered soonest start first; Recent Updates and Previously announced most recently ended first (then latest start, then `id`). The homepage previews Upcoming and Recent Updates, each with its own limit, and leaves plans that have ended to the Updates pages. "Ongoing" describes the event's dates, not your attendance. A plan that has ended stays published with its wording, anchor, and RSS item unchanged, and the build succeeds with a warning that names it; then either set it to `draft` or, if you attended, reword it in the past tense and set `event_status` to `completed`. See `docs/update-dates.md` for the date rules and why pages change only when the site is rebuilt.

Allowed categories are exactly `research`, `academia`, and `writing`; `academia` is displayed as "Activities" and keeps its `/updates/academia/` route. Classify each record by its actual context:

- `research`: substantive research developments, such as a paper's acceptance or publication, a research presentation, or a research software release.
- `academia` (Activities): workshop attendance, educational visits, training sessions, and similar participation reports, including a poster presented as part of a training event.
- `writing`: writing-related updates.

Categories are shown as plain metadata. Keep the nature of an achievement explicit in the title or summary and in the typed `kind`: an accepted paper (`acceptance`) is distinct from a published one (`publication`), and an award nomination (`outcome: "nominated"`, displayed as "Award nomination") from an award won. They can overlap. Event kinds are `acceptance`, `presentation`, `publication`, `participation`, `visit`, `award`, `release`, and `other`.

Awards must state both a name and one of the typed outcomes `nominated`, `shortlisted`, or `won`:

```json
"kind": { "type": "award", "outcome": "shortlisted", "name": "Name supplied by the author" }
```

Do not put confidential drafts, unpublished manuscripts, credentials, or private correspondence in this directory, even with `"status": "draft"`; a public Git repository exposes source files.
