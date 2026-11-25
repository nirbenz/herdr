# nirbenz herdr fork

Personal build of [herdr](https://github.com/herdrdev/herdr). Upstream does not
accept unsolicited pull requests (see `CONTRIBUTING.md`), so nothing here is
meant to go upstream.

The maintained branch is `custom/herdr`: the latest upstream release tag plus
the fork commits on top. `master` mirrors upstream and is never committed to.

## Sidebar tab tree

Lists tabs as tree rows under their space in the expanded sidebar, like tmux's
`choose-tree`.

```toml
[ui.sidebar.spaces]
tabs = "all" # off | active | all
```

- `off` is the default and leaves the sidebar unchanged.
- `active` lists the tabs of the focused space only.
- `all` lists the tabs of every space that has two or more tabs. A space with
  one tab gets no tab rows.

Each row shows the tab's rolled-up agent state, its number and its label. The
focused tab is bold.

Click a tab row to focus that tab, switching space (and machine) if needed.

Workspace navigation (`prefix+w`) visits tab rows too. It opens on the focused
tab, Up/Down walk spaces and tabs in sidebar order, and Enter focuses the
highlighted tab. The compact (collapsed) sidebar and the mobile layout keep
their flat space lists.

The change is client-side only; it uses the tab list and tab status the server
already sends. Most of it lives in `src/client/shell/sidebar_tabs.rs`, with
small hooks in the two sidebar renderers, mouse handling and workspace
navigation.

## Build and install

Needs the Rust toolchain from `rust-toolchain.toml` and Zig 0.16.0.

```sh
mise exec zig@0.16.0 -- cargo build --release --locked
install -m 755 target/release/herdr ~/code/bin/herdr
```

`~/code/bin` comes before Homebrew on `PATH`, so this shadows the Homebrew
`herdr`. Do not run `herdr update`; it installs the upstream release. Remove
`~/code/bin/herdr` to go back to Homebrew.

The server and client should run the same binary. After installing, restart
the server once with `herdr server stop` followed by `herdr`; the session shape
and agent sessions are restored.

## Syncing with upstream

Rebase the fork commits onto each new release tag:

```sh
git fetch upstream --tags
git rebase --onto vX.Y.Z vPREVIOUS custom/herdr
mise exec zig@0.16.0 cargo-nextest -- cargo nextest run --locked -E 'test(sidebar_tabs)'
mise exec zig@0.16.0 -- cargo clippy --all-targets --locked -- -D warnings
python3 scripts/config_reference_check.py
git push --force-with-lease origin custom/herdr
```

Conflicts, when they happen, are in the hook sites: `sidebar.rs` and
`endpoint_sidebar.rs` (row heights and the workspace render loop),
`workspace_navigation.rs` (target list), and `composition.rs` (render state).
