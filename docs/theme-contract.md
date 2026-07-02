# The marea theme contract

marea-ui components are styled exclusively by `marea.css` (shipped with the
crate as an asset) — plain CSS written against **CSS custom properties** the
app defines. The app's token file is the entire reskin surface: identical
components render as Mediterranea-parchment, Ascend-teal or Roommates-sage
purely by swapping tokens.

## How an app wires it

1. Define every token below in its root `tailwind.css` (or any stylesheet)
   under `:root`, plus optional `[data-theme="dark"]` overrides.
   `assets/tokens.reference.css` in this repo is a copy-paste starting point.
2. Mount the framework sheet once, before its own stylesheet:

   ```rust
   document::Stylesheet { href: marea_ui::MAREA_CSS }
   document::Stylesheet { href: asset!("/assets/tailwind.css") }
   ```

3. App screens keep using Tailwind (or anything else). Framework components
   never use Tailwind utility classes, so no cross-crate `@source` scanning
   is needed and there is no compiled-CSS drift to manage.

## Required tokens

| Token | Meaning |
|---|---|
| `--c-bg` | page background |
| `--c-surface`, `--c-surface-2`, `--c-surface-hover`, `--c-surface-active` | card / raised surfaces + interaction states |
| `--c-border`, `--c-border-2` | hairlines / stronger lines |
| `--c-text`, `--c-text-2`, `--c-text-3`, `--c-text-4` | ink → soft → muted → faint |
| `--c-heading` | headings (often = `--c-text`) |
| `--c-link` | inline links |
| `--c-primary`, `--c-primary-h`, `--c-primary-deep`, `--c-primary-muted`, `--c-primary-text` | primary accent, hover, pressed/deep, tinted fill, text-on-primary |
| `--c-accent`, `--c-accent-h`, `--c-accent-text` | secondary accent |
| `--c-success`, `--c-success-bg` | positive status |
| `--c-warning`, `--c-warning-bg` | caution status |
| `--c-danger`, `--c-danger-h`, `--c-danger-bg`, `--c-danger-text` | destructive / errors |
| `--c-overlay` | modal scrim |
| `--c-scrollbar-thumb`, `--c-scrollbar-track` | scrollbars |
| `--shadow-sm`, `--shadow-md`, `--shadow-lg` | elevation |
| `--navbar-h` | bottom-nav height (content padding uses it) |
| `--font-sans`, `--font-display` | UI + display faces (app loads the font files/links) |
| `--radius-sm/md/lg/xl` | corner radii (6/10/14/18px reference) |

Dark mode: the framework toggles `data-theme="dark"` on `<html>`
(`use_theme_mode`); the app's `[data-theme="dark"]` block re-declares
whichever tokens change. A light-only app simply omits the block.

App-specific tokens (Mediterranea's `--c-protein`, Ascend's grid colors …)
live in the app's file under an app prefix and are never referenced by
framework CSS.

## Rules for framework components (enforced in review)

- Semantic classes only (`btn-primary`, `m-card`, `bottom-nav__tab`, …),
  defined in `marea.css`; **no Tailwind utilities** in marea-ui rsx.
- Every color in `marea.css` is a `var(--c-*)` reference — a literal hex in
  that file is a bug.
- SVG icons use `currentColor` (never interpolated hex attributes), so
  active/inactive states are pure CSS.
- Components accept a `class` prop appended after their base classes, so apps
  can layer their own utilities on top.
