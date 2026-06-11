# requests-tui

A terminal client for [Bruno](https://www.usebruno.com/) collections. Browse your
`.bru` collections, switch environments, send HTTP/GraphQL requests, and inspect
responses — all from the terminal.

## Features

- **Bruno-native** — discovers collections by walking for `bruno.json`, and reads
  requests, environments, collection-level auth/headers/vars straight from your `.bru`
  files. No import step.
- **Environments & variables** — switch environments on the fly; `{{var}}` placeholders
  are expanded from collection vars, the selected environment, and session overrides.
- **Secrets** — Bruno-compatible `.env` support: values in a collection's `.env` file
  are referenced as `{{process.env.NAME}}` and are treated as secrets (masked, never
  written to disk). Any variable still missing at send time is prompted for.
- **Requests** — bearer auth, custom headers, path/query params, and `json` / `text` /
  `xml` / `graphql` bodies.
- **curl export** — turn the prepared request into a `curl` command and copy it.
- **Worktree-aware discovery** — linked git worktrees are skipped, and collection paths
  are shown relative to the scan root so same-named collections stay distinguishable.

## Installation

### Install script (recommended)

Downloads the right prebuilt binary for your platform into `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/allchanzi/requests-tui/main/scripts/install.sh | bash
```

Override the target directory or pin a version:

```bash
BINDIR=/usr/local/bin VERSION=v0.1.0 bash scripts/install.sh
```

Make sure the install directory is on your `PATH`.

### Prebuilt binary

Grab a tarball from the [latest release](https://github.com/allchanzi/requests-tui/releases/latest)
and extract the `requests-tui` binary somewhere on your `PATH`. Builds are published for:

| Platform | Asset |
|---|---|
| macOS (Apple Silicon) | `requests-tui-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `requests-tui-x86_64-apple-darwin.tar.gz` |
| Linux (arm64) | `requests-tui-aarch64-unknown-linux-gnu.tar.gz` |
| Linux (x86_64) | `requests-tui-x86_64-unknown-linux-gnu.tar.gz` |

### From source

Requires a [Rust](https://rustup.rs/) toolchain.

```bash
# straight from git
cargo install --git https://github.com/allchanzi/requests-tui --locked

# or from a local checkout
cargo install --path . --locked
```

## Usage

Run it against a directory that contains Bruno collections:

```bash
requests-tui [ROOT]
```

`ROOT` is the directory scanned for collections (any folder containing `bruno.json`).
It defaults to the current directory, so inside a project you can simply run:

```bash
requests-tui .
```

### Keybindings

| Key | Action |
|---|---|
| `tab` / `shift-tab` | Move focus between panes |
| `j` / `k`, `↑` / `↓` | Navigate within a pane |
| `enter` | Open collection / load request / activate environment |
| `esc` / `backspace` | Back to the collection list |
| `i` or `e` | Edit the focused request field |
| `esc` | Stop editing |
| `s` | Send the request |
| `c` | Generate a `curl` command (`y` to copy) |
| `h` | Toggle response headers (in the response pane) |
| `?` | Toggle help |
| `q` / `ctrl-c` | Quit |

## Secrets with `.env`

Keep secrets out of your `.bru` files using a collection-root `.env` (Bruno-compatible).
Add it to `.gitignore`:

```dotenv
# <collection>/.env
API_KEY=s3cr3t
TOKEN="multi\nline value"
```

Reference the values in any request, header, or body:

```
Authorization: Bearer {{process.env.API_KEY}}
```

These values are loaded from the `.env` next to the collection's `bruno.json`, treated
as secrets, and never persisted. If a referenced value is absent at send time, you'll be
prompted for it (the entry stays masked).

## Development

```bash
cargo build      # debug build
cargo test       # run the test suite
cargo run -- .   # run against the current directory
```

Releases are cut by pushing a version tag; a GitHub Actions workflow builds the binaries
for all targets and attaches them to the release:

```bash
# bump the version in Cargo.toml first
git tag v0.1.1
git push origin v0.1.1
```
