# Sunscreen documentation site

This is the source for <https://Pantani.github.io/sunscreen/>, built with [mdBook](https://rust-lang.github.io/mdBook/).

## Local preview

```bash
# Install the toolchain (one-time)
cargo install mdbook mdbook-admonish mdbook-mermaid mdbook-linkcheck

# Serve at http://localhost:3000 with live reload
mdbook serve docs/site --open
```

## Build

```bash
mdbook build docs/site
# Output: docs/site/book/html/
```

## Adding a page

1. Create the `.md` file under `docs/site/src/<track>/`.
2. Add an entry in `docs/site/src/SUMMARY.md` under the matching section.
3. `mdbook serve` will pick it up immediately.

## Conventions

- One topic per page.
- `Learn` pages define every term on first use; link to `concepts/` for depth.
- `Reference` pages are scannable: synopsis → flags table → examples → exit codes.
- Code blocks must be copy-paste runnable against the current `main` branch.
- Diagrams use Mermaid. Embed with a ` ```mermaid ` fenced block.

## Deploy

Pushes to `main` that touch `docs/site/**` trigger `.github/workflows/docs.yml`, which publishes to the `gh-pages` branch.
