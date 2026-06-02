# Sunscreen — Distribution & Release Channels

A push of a `vX.Y.Z` tag drives `.github/workflows/release.yml`. The pipeline
produces the GitHub Release first (binaries + shell installer), then fans out
to three downstream channels in parallel: **Homebrew**, **Snap Store**, **APT**.

Channel publish jobs use `continue-on-error: true` — a single broken channel
will **not** block the release or the other channels. The `release-summary`
job aggregates per-channel status into the run's Job Summary tab.

## Install commands (post-release)

```bash
# Homebrew (macOS + Linux)
brew install Pantani/sunscreen/sunscreen

# Snap (Linux, classic confinement)
sudo snap install sunscreen-cli --classic

# APT (Linux, Debian/Ubuntu via Cloudsmith)
curl -1sLf 'https://dl.cloudsmith.io/public/pantani/sunscreen/setup.deb.sh' | sudo -E bash
sudo apt-get install sunscreen

# Shell installer (cargo-dist, any POSIX)
curl -fsSL https://github.com/Pantani/sunscreen/releases/latest/download/sunscreen-installer.sh | sh
```

## Pipeline shape

```
plan ──► build-local ──► build-global ──► publish ──┬── publish-homebrew
                                                    ├── publish-snap   (matrix: amd64, arm64)
                                                    ├── publish-apt    (matrix: amd64, arm64)
                                                    └── release-summary
```

| Job | Runs on | Hard-fails the workflow? |
|-----|---------|--------------------------|
| `plan` → `publish` | ubuntu/macos | yes |
| `publish-homebrew` | ubuntu | no (`continue-on-error`) |
| `publish-snap`     | ubuntu/ubuntu-24.04-arm | no |
| `publish-apt`      | ubuntu | no |
| `release-summary`  | ubuntu | only if `publish` itself failed |

## Required secrets

| Secret | Scope | Used by | Rotation |
|--------|-------|---------|----------|
| `GITHUB_TOKEN` | repo (auto-injected) | `publish`, `publish-apt` (release asset upload) | n/a |
| `HOMEBREW_TAP_TOKEN` | PAT, `contents:write` on `Pantani/homebrew-sunscreen` | `publish-homebrew` | rotate every 12 months or on team change |
| `SNAPCRAFT_STORE_CREDENTIALS` | output of `snapcraft export-login --snaps=sunscreen-cli --channels=stable` | `publish-snap` | rotate every 12 months; Snap Store macaroons expire |
| `CLOUDSMITH_API_KEY` | Cloudsmith API key with `Write` on `pantani/sunscreen` repo | `publish-apt` | rotate on team change |

All channel jobs guard the upload step with `if: env.<SECRET> != ''`, so a
missing secret degrades the channel into "build-only" without erroring.

### Generating the snapcraft credentials

```bash
# Run locally once; commit the exported macaroon to GitHub Secrets.
snapcraft login
snapcraft export-login --snaps=sunscreen-cli --channels=stable snap-creds.txt
# Paste the contents of snap-creds.txt into the SNAPCRAFT_STORE_CREDENTIALS secret.
rm snap-creds.txt
```

### Cloudsmith repo bootstrap

```
Organization: pantani
Repository:   sunscreen   (public, repo type: OSS / free tier)
```

The APT job pushes packages with `distro=any-distro` + `release=any-version`,
which makes the repo a single multi-distro pool. End users add it via the
auto-generated `setup.deb.sh` (see install commands above).

### Homebrew tap bootstrap

The tap repo `Pantani/homebrew-sunscreen` must exist and contain at minimum
an empty `Formula/` directory and a default branch. cargo-dist generates
`sunscreen.rb` inside the `dist-global` artifact during `build-global`; the
`publish-homebrew` job commits that file into the tap.

## Manual smoke after a release

```bash
# macOS (Homebrew)
brew uninstall sunscreen 2>/dev/null || true
brew install Pantani/sunscreen/sunscreen
sunscreen --version

# Linux (Snap)
sudo snap remove sunscreen-cli 2>/dev/null || true
sudo snap install sunscreen-cli --classic
sunscreen-cli --version

# Linux (APT)
sudo apt-get purge -y sunscreen 2>/dev/null || true
curl -1sLf 'https://dl.cloudsmith.io/public/pantani/sunscreen/setup.deb.sh' | sudo -E bash
sudo apt-get install -y sunscreen
sunscreen --version
```

## Re-running a failed channel

Trigger the workflow manually with the existing tag:

```bash
gh workflow run release.yml -f tag=v0.1.0
```

Each channel is idempotent:
- **Homebrew**: commits to the tap only if the formula content actually
  changed (`git diff --quiet` short-circuit).
- **Snap**: re-uploading the same revision is silently deduplicated by the
  Snap Store.
- **APT**: Cloudsmith is called with `republish: true`, which overwrites the
  existing version-arch tuple instead of erroring.

## Dry-run before tagging

```bash
# Local dry-run of the cargo-dist plan
cargo dist plan --tag v0.2.0-rc.1
```

Pre-release tags (`v0.2.0-rc.1`) are valid release triggers; cargo-deb maps
them to Debian `0.2.0~rc.1` (tilde) so `apt-cache policy` orders them below
the eventual stable.
