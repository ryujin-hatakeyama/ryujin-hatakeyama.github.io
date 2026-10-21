# Update dates and the build day

## Two kinds of state

An update record carries one statement by its author, `event_status`:

- `planned`: the activity is intended;
- `completed`: the activity took place.

Only the author changes it. The build never converts a planned record, because an event having begun or ended does not mean that the author took part.

From that status, the record's `date` and `end_date`, and the build day, the site builder derives a display phase (`UpdatePhase` in `site-builder/src/model.rs`, computed by `Update::phase_on`):

| `event_status` | Build day | Phase | Shown under |
| --- | --- | --- | --- |
| `planned` | before `date` | Upcoming | Upcoming |
| `planned` | `date` to the last day, inclusive | Ongoing | Upcoming, with an "Ongoing" label |
| `planned` | after the last day | Past plan | "Previously announced", on the Updates pages only |
| `completed` | any | Completed | Recent Updates |

The last day is `end_date`, or `date` for a single-day event. The phase is never written to the JSON sources or to the generated update data.

"Ongoing" describes the event's dates, not attendance. A past plan is a plan that the author has not yet resolved: it is not presented as completed, and it is not removed. It keeps its wording, its `#update-…` anchor on the Updates pages, and its RSS item and guid. The homepage previews only Upcoming and Recent Updates, so a past plan leaves the homepage on the day after the event; it stays on `/updates/` and its category pages until the author either records what happened (`completed`, past-tense wording) or sets the record to `draft`.

## The build day

The build day is a calendar day in Asia/Tokyo, because the events are dated in Japan:

- `SITE_BUILD_DATE=YYYY-MM-DD`, when set, is used as is (an invalid value fails the build);
- otherwise the system clock is read once and converted at a fixed UTC+09:00 (`site-builder/src/site_date.rs`). Japan has not observed daylight saving time since 1952, so this matches the time zone database without depending on it, on the machine's local time zone, or on the UTC date. Between 15:00 and 24:00 UTC the UTC date is a day behind Tokyo; the Tokyo date is the one used.

The build prints the day it used. `scripts/verify.sh` sets `SITE_BUILD_DATE` once, so that the two builds it compares for determinism describe the same day even across midnight.

The day affects only the homepage and the Updates listing pages (`/updates/` and the category pages). Every other route, the RSS feed, and the sitemap are independent of it, except that a Miscellany article whose `published` day is still to come is left out of Updates and the Miscellany overview until then (`content/miscellany/README.md`); `scripts/test-update-phases.sh` checks this by building the real site on several days.

## What stops the build

Structural problems still fail the build: invalid JSON, schema violations, impossible dates, an end date before the start, duplicate update ids (in both the OCaml generator and the Rust loader), and broken internal links or anchors.

A planned update whose event has ended is an editorial matter, not a structural one. The build succeeds and prints a warning naming the record, its last day, and the build day; on GitHub Actions the warning is also emitted as a `::warning` annotation, which appears on the run's summary page. It never blocks publishing unrelated changes.

## Pages change only when the site is rebuilt

The site is static. The GitHub Pages workflow (`.github/workflows/deploy.yml`) runs only on a push to `main` and on manual dispatch, so the published pages reflect the day of the latest deployment, not the current day. For example, if the last deployment ran on 15 October, `/updates/` keeps showing 数学基礎論若手の会2026 as upcoming, without the Ongoing label, until the next push or manual run; a build on or after 19 October moves it under Previously announced and prints the warning.

To bring the phases up to date without a content change, run the workflow manually (Actions → Deploy to GitHub Pages → Run workflow, or `gh workflow run deploy.yml`).

## Option: a daily build (not enabled)

A scheduled run would keep the phases current without anyone pushing. It is not enabled; whether to build on a schedule is a separate decision about how the site is run. If adopted, the smallest change is a `schedule` trigger on the existing workflow:

```yaml
on:
  push:
    branches: [main]
  workflow_dispatch:
  schedule:
    # 00:17 in Tokyo; avoid the top of the hour, when GitHub delays scheduled runs.
    - cron: '17 0 * * *'
      timezone: Asia/Tokyo
```

Benefits:

- Upcoming, Ongoing, and Previously announced follow the calendar within about a day.
- A plan that has ended produces its warning annotation daily, so it is noticed even when no one pushes.
- The full verification (tests, link and anchor checks, public-artifact audit) also runs daily, catching breakage from toolchain or action updates early.

Costs and limits (from GitHub's documentation on the `schedule` event, checked 2026-10-14):

- Actions minutes are free for standard GitHub-hosted runners in public repositories. A run here takes a few minutes, mostly toolchain setup; each also creates a GitHub Pages deployment, even on days when nothing changed.
- Scheduled runs use the latest commit on the default branch and can be delayed under load, especially at the start of each hour; if the load is high enough, queued runs can be dropped. A phase can therefore change hours late, or a day late.
- Without the `timezone` key, cron times are UTC.
- In a public repository, scheduled workflows are disabled automatically after 60 days without repository activity, and must then be re-enabled from the Actions tab.
- Notifications for scheduled runs go to the user who last modified the cron line in the workflow file.

Failure behavior:

- If a scheduled build fails, the deploy job does not run, and the previous deployment stays online; the job's failure is shown in the Actions tab and notified as above.
- A plan that has ended does not fail the build; it appears as a warning annotation on the run, which GitHub does not notify on its own. If a notification is wanted for that case, add a step that fails or opens an issue on these warnings, keeping it separate from the deploy so that publication is not blocked.
- The workflow's `pages` concurrency group (`cancel-in-progress: false`) already serializes a scheduled run with a push-triggered one.

Alternatives: keep deploying on push and run the workflow manually around events (the current practice), or schedule runs only on the days an event starts and ends. A client-side script could switch the labels at view time, but it would show different content to visitors and crawlers and depend on the visitor's clock, so it is not recommended.
