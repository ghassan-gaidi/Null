# Null Website — Design Spec

Date: 2026-10-25 · Status: approved (full-page mock verdict, in-chat) · Input: `README.md`, `SUMMARY.md`, `docs/`

Single landing page for the Null project, published on GitHub Pages. The page
carries the whole argument — what Null is, who it defeats, where it stands, what
is proven, how to run it — without shipping a single claim the repository does
not already make.

## 1. Purpose & success criteria

Audience: privacy-motivated developers, auditors, journalists, potential ops users.

A visitor to the page can, in under a minute and without leaving it:

1. State what makes Null different (serverless P2P, deniability, ongoing PQ rekeying, honest proof labeling).
2. See the honest proof status (proven / tested / named-limitation), not silent overclaim.
3. Find the threat model, audit brief, and full postures (linked to the repo, not paraphrased into decoys).
4. Run the quickstart (single command; loopback demo works with no daemons).

Non-goals: a docs hub, a marketing funnel, analytics of any kind.

## 2. Locked decisions

| Decision | Choice | Rationale / notes |
|---|---|---|
| Visual direction | **D · "Redacted"** (industrial brutalism) | Censors' bars, dossier file-tags, stamps, hazard stripes; deniability as a visual device. User selected from 4 mock directions. |
| Page structure | **Full dossier** (single long page) | User selected from 3 skeletons. Docs stay in repo, linked. |
| Display typeface | **Anton** | User selected from 3 real-font mock options. |
| Mono typeface | **JetBrains Mono** | Default in approved mock; IBM Plex Mono is a one-line swap (§ open items). |
| Stack | Pure static HTML + hand-written CSS, **zero JavaScript** | On-brand: zero third-party requests; content works without JS everywhere. |
| Fonts delivery | Self-hosted latin-subset woff2, OFL, `font-display: swap` | No CDN anywhere. |
| Repository layout | `site/` directory on `main` | `site/index.html`, `site/style.css`, `site/fonts/`, `site/favicon.svg`. |
| Hosting | GitHub Pages via Actions workflow, default project URL | `https://ghassan-gaidi.github.io/null/` — resolved from origin remote; custom domain not in scope. |
| Color system | Paper `#ebe7de`, ink `#141414`, yellow `#f2c744`, red `#d0342c` | As in approved mock. See tokens §4. |

## 3. Page outline (section → copy source)

In order. Every figure/word traced to repository text at build time.

| § | Section | Content | Source |
|---|---|---|---|
| nav | Sticky nav | `NULL // ZERO TELEMETRY` + anchors + `GitHub ↗` | brand |
| 1 | Hero | Headline `Private conversation, verifiable.`; sub; redacted bars; stamp `NO TELEMETRY`; CTAs → ledger / quickstart; 4-cell evidence strip: handshake `proven 5/5 · tamarin · CI`, ratchet `ECDH ∥ chain ∥ ML-KEM rekey`, transport `tor · i2p · nym · snowflake`, endpoint `ram-only · 3-pass wipe`; hazard stripe | mission (README), `docs/security-posture.md` |
| 2 | The Enemy | 8 adversary cells (Name / threat line / `⇒` defense line): observer, active attacker, quantum-HNDL, local forensics, endpoint malware (user), censor, coercion, supply chain. Defense lines use exact defenses from README/`docs/security-posture.md` | threat model |
| 3 | Against the 2026 state of the art | Comparison table (8 rows: architecture, E2E default, identity, post-quantum, deniable mode, metadata/cover, circumvention, source) × Null / Signal / iMessage / Telegram; Null column highlighted; failure marks in red; footnotes carry README caveats (ranking disclaimer, iCloud/ADP, Telegram cloud choice) | README §sota |
| 4 | Honesty ledger | 11 rows with status pills: PROVEN (handshake secrecy, initiator KCI, verified-pinned agreement), IN PROGRESS (ratchet FS/PQ-PCS), TESTED (deniability tripwire, downgrade matrix, TreeKEM chaos, decoder fuzz), NOT RUN (unsafe audit w/o miri, constant-time dudect, third-party audit). Legend explains the three tiers | `docs/security-posture.md` §2 |
| 5 | Run it | Terminal block: `cargo build --locked --bin null`; loopback TUI demo; two-process chat; Tor hint (`--control-port 9051`, `NULL_LIVE_TRANSPORT=1`); `--verified` hint | README quickstart |
| 6 | Dossier shelf | 17 design docs + `SECURITY.md` as linked mono list → absolute repo URLs; note "the site publishes nothing the repo doesn't prove" | README §design docs |
| 7 | Footer | Closing line `Zero telemetry. Zero servers. Zero "take our word for it."` (red bar on third clause); 3 columns: project / auditors / this page (`0 third-party requests · no cookies · no analytics`, license); hazard stripe | SECURITY.md, license |

## 4. Visual system

### Color tokens

| Token | Value | Use | Contrast |
|---|---|---|---|
| `--paper` | `#ebe7de` | page bg | ink 14.9:1 |
| `--ink` | `#141414` | text, borders, blocks | — |
| `--yellow` | `#f2c744` | nav band, primary CTA, hazard, Null column wash | ink 9.9:1 |
| `--red` | `#d0342c` | stamps, failure marks, NOT RUN pills (white text) | on paper 4.6:1 — large/bold text only |
| `--red-alt` | `#8f2c25` | small red text (footnote marks, hovered doc links) | on paper ~7:1 |
| `--ink-dim` | `#55524a` | secondary copy on paper | ~5.5:1 |
| `--band` | `#e2dccd` | section header band | as paper |
| `--ok` | `#1d7a2d` | green proof/victory marks | ~4.7:1 bold mono only |

### Type

- Display: Anton (`font-family: 'AntonF'`, fallback `Impact, system-ui, sans-serif`), uppercase, `letter-spacing ≈ .01em`.
  - Hero H1: `clamp(48px, 7.4vw, 82px)`, line-height .97; section H2: `clamp(24px, 3vw, 34px)`; footer closer: `clamp(24px, 3.6vw, 40px)`.
- Mono: JetBrains Mono (fallback `ui-monospace, Menlo, Consolas, monospace`), base 12.5–13.5px, `line-height 1.5–2.05`; uppercase labels 9.5–11px, `letter-spacing .13–.2em`.
- Body paragraphs use mono at ≥12.5px on `--paper`; contrast everywhere ≥ 4.5:1.

### Components (from approved mock)

redaction bars (`inline-block` black, height 1em) · rotated stamps (double 3px border, `rotate(-9deg)`) · file tag (top-right, `FILE: … / DISPOSITION: …`) · evidence grid (4 cells, 2px internal rules) · adversary cells (2px rules, `⇒` mono defense lines) · SOTA table (ink header row, Null column `rgba(242,199,68,.67)`, red `.dud` marks) · ledger rows (property / mono evidence / right-aligned status pill) · pills (2px border: green PROVEN, yellow TESTED, dashed red IN PROGRESS, solid red NOT RUN) · terminal (`#141414`, yellow title bar, `$` in yellow, commands plain, `#` comments italic) · hazard stripe (`repeating-linear-gradient(45deg, yellow 0 14px, ink 14px 28px)`) · hard shadows (`5px 5px 0 #141414` on primary CTA / terminal) · footer black panel with Anton closer.

### Motion policy

**Zero animation.** No transitions, no blinking, no parallax. `prefers-reduced-motion` trivially satisfied. Interactions are :hover (color/border) and focus outlines only.

## 5. Technical architecture

### 5.1 Files

```
site/
  index.html        — entire page, semantic HTML5
  style.css         — hand-written, ~600 lines, custom properties at top
  fonts/
    anton.woff2               ~ 31 KB (latin subset)
    jetbrains-mono.woff2      ~ 31 KB (latin subset)
    OFL.txt                   — both faces are OFL; license text carried here
  favicon.svg       — square "N." ink/yellow mark, inline SVG
site/README.md      — manual serve/deploy notes (disable-JS not needed; how fonts were produced)
.github/workflows/pages.yml — build+deploy
```

`@font-face` uses **relative URLs** (`fonts/anton.woff2`) — the site serves under the `/null/` base path; nothing may assume root.

### 5.2 Workflow (`.github/workflows/pages.yml`)

- Trigger: `push` to `main` with `paths: ["site/**", ".github/workflows/pages.yml"]`; `workflow_dispatch`.
- Steps: checkout → `actions/configure-pages@v5` → `actions/upload-pages-artifact@v3` with `path: site` → `actions/deploy-pages@v4`, `id: deployment`, `environment: github-pages`.
- Permissions: `pages: write; id-token: write; contents: read`; `concurrency: { group: "pages", cancel-in-progress: false }`.
- No build step. Nothing is generated; the artifact *is* the directory.

### 5.3 Meta & assets

`<title>Null — post-quantum, metadata-resistant terminal messenger</title>`; description ≤160ch from the mission sentence; `canonical` = default Pages URL; OG metadata textual only (`og:title`, `og:description`, `og:type=website`, `og:url` — no `og:image` in v1; a generated PNG is an optional follow-up); `twitter:card=summary`; `<meta name="color-scheme" content="light">`; favicon inline SVG; `lang=en`.

### 5.4 Accessibility contract

- Landmarks: `header, nav, main, section[aria-labelledby], footer`; skip link to `#main`.
- One `<h1>` (hero); sections use `h2`; cells use `h3`.
- SOTA table: real `<table>`, `th scope` on both axes, `<tfoot>`-like footnotes as `<figcaption>`-equivalent prose below.
- Ledger pills never rely on color alone — text label inside every pill.
- External links: `rel="noopener"` + `target="_blank"` only in footer/shelf; in-page anchors are plain `#`.
- Decorative redaction bars/stamps: `aria-hidden="true"` where SVG/emoji; bars are actual `span`s with role decoration semantics; all load-bearing copy never hidden.
- Visible focus: `:focus-visible { outline: 3px solid var(--ink); outline-offset: 2px; }`.
- Nav collapses without JS; every action anchor works with keyboard.

### 5.5 Performance budget

| Item | Budget | Expect |
|---|---|---|
| HTML | ≤ 60 KB | single page, ~35 KB |
| CSS | ≤ 15 KB | one file |
| Fonts | ≤ 75 KB total (2 woff2) | 62 KB |
| JS | 0 bytes | — |
| Third-party requests | **0** | fonts self-hosted, links static |
| Console errors | 0 | — |

## 6. Responsive contract

- Breakpoints: default = approved mock (≥ 900px).
- ≤ 900px: enemy grid 2-col (recompute 2px right rules on 2n), evidence grid 2×2, SOTA font 9.6px.
- ≤ 720px: ledger drops the evidence column (3→2 cols), docs shelf 1 column, hero CTAs wrap, nav wraps to 2 rows (brand row + links row).
- SOTA table sits in `overflow-x: auto` wrapper at any width (no sticky needed for 5 columns × 8 rows at ≥ 720px).

## 7. Verification plan (evidence before "done")

1. `python3 -m http.server` in `site/`; open via OpenCode browser at 1180 / 768 / 375 widths; screenshot each; no horizontal scroll, no overlaps.
2. Network panel: exactly the local font/CSS/HTML requests — 0 third-party, 0 console errors, fonts loaded (`document.fonts.check`).
3. Link audit (throwaway script run from `/tmp`, **not** committed): every `#anchor` target exists in `index.html`; every doc-shelf URL matches `docs/…` filenames in `origin/main`.
4. HTML sanity: balanced tags + one H1 (manual or `tidy` if present).
5. CI: workflow file lints (`actionlint` if available else visual check), pages deployment thumbs-up in Actions tab after first push.
6. Copy audit: every number on the page (5/5, ≤50 msgs/7d, 2048B, 3-pass, MIT OR Apache-2.0, license files present) matches `null-core` constants / README at time of writing. Note: `cargo xtask doccheck` does **not** guard the site — this checklist is the stand-in, which is why the footer stamps the build provenance.

## 8. Out of scope (YAGNI)

Rendered doc pages (§8 dossier+ variant — rejected: sync debt), blog/news, i18n, dark mode (paper is the design), JS features, sitemap.xml beyond default, search, comments, generated OG image (optional follow-up), custom domain, GitHub Pages custom domain DNS, screenshots of the TUI.

## 9. Risks & mitigations

| Risk | Mitigation |
|---|---|
| Content/weathering: numbers rot as code evolves | Every section links its source doc; footer carries a provenance stamp (`built from` commit/date at release); ledger is a mirror of the repo's own posture table, not a paraphrase. |
| doccheck doesn't guard `site/` | Verification plan §7.6; long-term option: add `site/index.html` strings to `doccheck` constants list (out of scope v1, noted for repo maintainers). |
| Redaction bars could hide load-bearing info from skimmers | Bars are decor; all load-bearing copy is plain text next to them; stamps/bars never overlap text (z-index safe). |
| `/null/` base path breaks asset URLs | Relative URLs only; verified in §7. |
| Anton (all-caps display) hurts readability of long copy | Display face used for headings/closers only; body copy is mono at readable size. |

## 10. Open items (criticized defaults from brainstorm, confirmable at review)

- **JetBrains Mono locked** as the mono voice; IBM Plex Mono remains a one-line swap.
- URL: default GitHub Pages (`ghassan-gaidi.github.io/null`). Custom domain changes §5.2/§5.3 only.
- Optional follow-ups recorded, not built: OG image, `doccheck` coverage of site copy, README badge linking to the site.

## 11. Acceptance

Process gate: this spec goes to the human reviewer next; approval here does not authorize implementation. The next artifact is the implementation plan (written task-by-task), which itself gets approved before any site file is created. Implementation tasks then finish with evidence per §7.
