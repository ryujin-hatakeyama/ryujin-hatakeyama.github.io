# Update records

Each `*.json` file in this directory is a single update. The OCaml generator validates every record before Rust sees it. Drafts are validated but omitted from `target/generated/updates.json`.

This is a schema example only; it is not read by the build:

```json
{
  "id": "stable-lowercase-slug",
  "title": { "en": "Required English title", "ja": "Optional Japanese title" },
  "summary": { "en": "Required English summary", "ja": "Optional Japanese summary" },
  "date": "YYYY-MM-DD",
  "announced_on": "YYYY-MM-DD",
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

Allowed categories are exactly `research`, `academia`, and `writing`. They can overlap. Event kinds are `acceptance`, `presentation`, `publication`, `participation`, `visit`, `award`, `release`, and `other`.

Awards must state both a name and one of the typed outcomes `nominated`, `shortlisted`, or `won`:

```json
"kind": { "type": "award", "outcome": "shortlisted", "name": "Name supplied by the author" }
```

Do not put confidential drafts, unpublished manuscripts, credentials, or private correspondence in this directory, even with `"status": "draft"`; a public Git repository exposes source files.
