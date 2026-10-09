# Writings

Published English Markdown files in this directory generate `/writings/<slug>/` routes. Drafts and Japanese-language source files are validated but are not published. MDX file extensions are accepted for backward-compatible plain Markdown/HTML content; JSX components and JavaScript expressions are intentionally unsupported now that Astro is not part of the build.

Required YAML front matter:

```yaml
---
slug: stable-slug
title: Authored title
description: Concise description
date: 2026-01-01
lang: en
kind: essay
math: false
draft: true
---
```

Allowed kinds are `essay`, `note`, `literary`, `prose`, and `fragment`. Set `math: true` to render inline and display TeX with KaTeX during the Rust build. Optional fields are `translationKey`, `publication`, and `externalUrl`.
