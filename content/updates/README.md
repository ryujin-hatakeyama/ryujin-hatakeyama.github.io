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
  "announced_on": "YYYY-MM-DD",
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

`date` is the day the event begins and the optional `end_date` its last day. `announced_on` is the day the update is published on the site and determines its position in Updates and RSS; set it to the actual publication date rather than backdating it.

Event names follow their organizers: use an official English name only when one can be verified, and otherwise keep the original name. The optional `title_link` links the event's name, and only that, to its official page: `text` must occur exactly once in `title.en`, `url` must use HTTP(S), and `lang` (`en` or `ja`) marks the language of the linked text when it differs from English. Omit it when there is no official page for the event itself; never point it at a broader organization's homepage or a program PDF. A record link with the same URL is not shown again beside the title, and a record with a detail page then offers it as a separate "Details" link. RSS and page metadata keep the plain title.

The English summary may contain inline links written as `[text](https://…)`; only HTTP(S) URLs form links, and RSS and meta descriptions show the link text alone. Any other text, including brackets that do not form such a link, is shown verbatim.

While a record is a draft awaiting confirmation, `date` and `announced_on` may be omitted rather than guessed; a published record must have both.

`event_status` is required and is either `completed` or `planned`. It records whether the activity had taken place when the update was announced; it is unrelated to `status`, which only controls whether the record is a draft. A `planned` record must be announced before `date` and should be worded as an intention ("I plan to attend …"); a `completed` record must not be announced before the event ends. Planned records are listed under "Upcoming", soonest event first, and completed records under "Recent Updates", newest announcement first; the homepage previews each section with its own limit. Planned records are never converted automatically. Once a planned event has begun, the Rust build stops with an editorial-review error rather than publish a past event as upcoming; then either set the record to `draft` or, if you attended, reword it and set `event_status` to `completed`.

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
