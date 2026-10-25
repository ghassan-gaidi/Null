# site/ — the Null landing page

The deployed page at https://ghassan-gaidi.github.io/Null/ (URL is
case-sensitive — the repo is `Null` with a capital N).

## Structure

```
site/
├── index.html          # the whole dossier; zero JavaScript
├── style.css           # hand-written; tokens at the top, per-section blocks below
├── favicon.svg         # ink square, yellow N
└── fonts/
    ├── anton.woff2               # Anton display face (latin), ~19 KB
    ├── jetbrains-mono-400.woff2  # JetBrains Mono 400 (latin), ~21 KB
    ├── jetbrains-mono-700.woff2  # JetBrains Mono 700 (latin), ~22 KB
    └── OFL.txt         # both faces under SIL OFL 1.1, full text
```

## Rules the page keeps

- **0 bytes of JavaScript**, **0 third-party requests** — fonts are
  self-hosted and referenced with relative URLs; everything else is
  anchor links into this repository.
- No animation/transitions: `:hover` + `:focus-visible` only.
- Copy is sourced, never invented: numbers trace to `README.md`,
  `docs/security-posture.md`, `docs/threat-model.md`, `SUMMARY.md`.
  (`cargo xtask doccheck` does not scan `site/` — audit copy changes
  against those sources by hand.)

## Fonts

Downloaded as latin woff2 subsets from Google Fonts (two JBM weights
are separate static files, not a variable slice):

```sh
UA="Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120 Safari/537.36"
curl -A "$UA" -o anton.woff2 "$(curl -A "$UA" 'https://fonts.googleapis.com/css2?family=Anton&display=swap' | awk '/\/\* latin \*\//{f=1} f && /url\(/{match($0,/https:[^)]*\.woff2/); print substr($0,RSTART,RLENGTH); f=0}')"
# repeat for family=JetBrains+Mono:wght@400 and :wght@700
```

## Deploy

GitHub Pages via `.github/workflows/pages.yml`: push to `main`
touching `site/**` or the workflow → artifact upload → deploy
(configure-pages has `enablement: true` so the workflow self-heals a
repo where Pages was never turned on). Pages is set to
**Source: GitHub Actions** (`build_type=workflow`).
