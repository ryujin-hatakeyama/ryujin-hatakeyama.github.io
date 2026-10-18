#!/bin/sh
# Builds the real site, with one made-up planned event added (16–18 October
# 2026), as of fixed Asia/Tokyo days around that event and long after it, and
# checks that each build succeeds and passes the built-site validation, that
# the Updates pages place the made-up record by date without changing it, and
# that pages unrelated to Updates, and the RSS feed, do not depend on the day.
# The published records' own statuses do not matter here: an event that has
# happened may be recorded as completed at any time. It also checks that the
# update generator still rejects structural errors. Run it after
# ./scripts/build-site.sh, which prepares the generated inputs; the generated
# updates are made again from content/updates when it ends.
set -eu

record=phase-fixture-planned
title="Planned attendance at the phase fixture"
base=target/update-phases
generated=target/generated/updates.json
fail() {
  echo "$1" >&2
  exit 1
}
generate() {
  dune exec --display=quiet ./ocaml/generate_updates.exe -- "$1" "$2"
}
restore() {
  generate content/updates "$generated"
}
fixture() {
  printf '{"id":"%s","title":{"en":"x"},"summary":{"en":"x"},"date":"%s",%s"event_status":"planned","categories":["academia"],"kind":{"type":"participation"},"status":"published"}\n' "$@"
}
rm -rf "$base"
mkdir -p "$base/sources"

# The published records, and the made-up planned event beside them.
cp content/updates/*.json "$base/sources/"
printf '{"id":"%s","title":{"en":"%s"},"summary":{"en":"x"},"date":"2026-10-16","end_date":"2026-10-18","event_status":"planned","categories":["academia"],"kind":{"type":"participation"},"status":"published"}\n' \
  "$record" "$title" > "$base/sources/zz-$record.json"
cp "$base/sources/zz-$record.json" "$base/fixture.json"
trap restore EXIT
generate "$base/sources" "$generated"

section_of() {
  # Prints the section heading id that precedes the record on a page.
  awk -v anchor="id=\"update-$record\"" '
    { line = $0
      while (match(line, /id="(upcoming|recent|past-plans)-heading"|id="update-[a-z0-9-]+"/)) {
        token = substr(line, RSTART, RLENGTH)
        if (token ~ /-heading"$/) heading = token
        if (token == anchor) { print heading; exit }
        line = substr(line, RSTART + RLENGTH)
      } }' "$1"
}

entry_of() {
  # Prints the record's entry on a page, from its anchor to its </article>.
  awk -v anchor="id=\"update-$record\"" '
    { text = text $0 "\n" }
    END { at = index(text, anchor); if (!at) exit
          rest = substr(text, at); print substr(rest, 1, index(rest, "</article>")) }' "$1"
}

expect_section() {
  day=$1 expected=$2
  actual=$(section_of "$base/$day/updates/index.html")
  [ "$actual" = "id=\"$expected-heading\"" ] ||
    fail "on $day the record is under ${actual:-no section}, expected $expected"
}

for day in 2026-10-15 2026-10-16 2026-10-18 2026-10-19 2099-12-31; do
  SITE_BUILD_DATE=$day cargo run --quiet --locked --release -p site-builder -- \
    build --output "$base/$day" > "$base/$day.log" 2>&1 || {
    cat "$base/$day.log" >&2
    fail "the build for $day failed"
  }
  cargo run --quiet --locked --release -p site-builder -- check-dist --output "$base/$day" > /dev/null
done

expect_section 2026-10-15 upcoming
expect_section 2026-10-16 upcoming
expect_section 2026-10-18 upcoming
expect_section 2026-10-19 past-plans
expect_section 2099-12-31 past-plans
ongoing='<span class="phase-label">Ongoing</span>'
grep -q "$ongoing" "$base/2026-10-16/index.html"
for day in 2026-10-16 2026-10-18; do
  entry_of "$base/$day/updates/index.html" | grep -q "$ongoing" || fail "not Ongoing on $day"
done
for day in 2026-10-15 2026-10-19; do
  if entry_of "$base/$day/updates/index.html" | grep -q 'phase-label'; then fail "Ongoing on $day"; fi
done
grep -q '<h2 id="past-plans-heading">Previously announced</h2>' "$base/2026-10-19/updates/index.html"
grep -q '<h2 id="past-plans-heading">Previously announced</h2>' "$base/2026-10-19/updates/academia/index.html"
entry_of "$base/2026-10-19/updates/index.html" | grep -q "$title" || fail "the past plan lost its wording"
if grep -q "$title" "$base/2026-10-19/index.html"; then fail "past plan on the homepage"; fi
grep -q "$title" "$base/2026-10-18/index.html"

# Only an ended plan produces the editorial warning, and it does not stop the build.
if grep -q "update \"$record\" is still planned" "$base/2026-10-18.log"; then fail "warning during the event"; fi
grep -q "update \"$record\" is still planned, but its event ended on 2026-10-18" "$base/2026-10-19.log"

# No build changes a record: the made-up one is as written, and the published
# ones are as committed. So is everything not derived from the day.
cmp -s "$base/fixture.json" "$base/sources/zz-$record.json" || fail "the made-up record changed"
git diff --quiet -- content/updates || fail "a build changed content/updates"
# Only the homepage and the Updates listing pages may differ between days.
for day in 2026-10-16 2026-10-18 2026-10-19 2099-12-31; do
  unexpected=$(diff -r -q "$base/2026-10-15" "$base/$day" |
    grep -v -E "^Files $base/2026-10-15/(index\.html|updates/(index\.html|[a-z]+/index\.html)) and " || true)
  [ -z "$unexpected" ] || fail "unexpected differences on $day: $unexpected"
done
cmp -s "$base/2026-10-15/rss.xml" "$base/2099-12-31/rss.xml"

# An invalid day is a configuration error, not a reason to guess one.
if SITE_BUILD_DATE=2026-10-32 cargo run --quiet --locked --release -p site-builder -- \
  build --output "$base/invalid" > /dev/null 2>&1; then
  fail "an invalid SITE_BUILD_DATE was accepted"
fi

# Structural errors in update sources still stop the pipeline.
expect_rejected() {
  label=$1 directory=$2
  if dune exec --display=quiet ./ocaml/generate_updates.exe -- "$directory" "$base/$label.json" \
    > /dev/null 2> "$base/$label.err"; then
    fail "the update generator accepted $label"
  fi
}
mkdir -p "$base/duplicate" "$base/bad-start" "$base/bad-end" "$base/reversed"
fixture same 2026-10-16 '' > "$base/duplicate/a.json"
fixture same 2026-10-17 '' > "$base/duplicate/b.json"
expect_rejected duplicate "$base/duplicate"
grep -q 'duplicate update id "same"' "$base/duplicate.err"
fixture bad 2026-02-30 '' > "$base/bad-start/a.json"
expect_rejected bad-start "$base/bad-start"
fixture bad 2026-10-16 '"end_date":"2026-10-1",' > "$base/bad-end/a.json"
expect_rejected bad-end "$base/bad-end"
fixture bad 2026-10-18 '"end_date":"2026-10-16",' > "$base/reversed/a.json"
expect_rejected reversed "$base/reversed"

rm -rf "$base"
echo "Update phase builds passed."
