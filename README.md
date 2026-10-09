# Ryujin Hatakeyama

This repository contains the source of [my personal website](https://ryujin-hatakeyama.github.io/).

The pages are generated in Rust with [Maud](https://maud.lambda.xyz/). A small OCaml program validates and orders the update records before the Rust generator reads them, while TypeScript handles the appearance and email controls in the browser. The result is a static site published through GitHub Pages.

## Build

The build requires Rust (see `rust-toolchain.toml`), Node.js 24, and OCaml with opam and Dune. Python 3 is used to serve the site locally.

For a fresh clone, create a project-local opam switch and install the dependencies:

```sh
opam switch create . --packages=ocaml-system
eval "$(opam env)"
opam install . --deps-only --with-test
npm ci
```

Build and check the site with:

```sh
opam exec -- npm run verify
```

To build without running all checks, or to preview an existing build:

```sh
opam exec -- npm run build
npm run preview
```

The generated files are written to `dist/`. Commits pushed to `main` are built and deployed by GitHub Actions.
