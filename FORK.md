# lumiere

This repository is a fork of [jnsahaj/lumen](https://github.com/jnsahaj/lumen) for
reviewing agent-written changes: several repositories from one workspace folder, and
a review loop with a coding agent.

- `origin`: https://github.com/aduverger/lumiere.git
- `upstream`: https://github.com/jnsahaj/lumen.git

## What differs from upstream

- The binary is named `lumiere`, so it can be installed beside upstream `lumen`. The
  Cargo package name, the configuration file (`lumen.config.json`), its locations and
  the `LUMEN_*` environment variables are unchanged.

## Install

```bash
cargo install --path .
```

## Upstream updates

Use normal merges, so the fork's changes stay reviewable:

```bash
git fetch upstream main
git merge upstream/main
```

Resolve conflicts in fork-modified files normally. Do not rebase the fork onto
upstream and do not squash upstream history.

## Releases

Nothing is published from this fork under upstream's identity: no crates.io release
of `lumen`, no Homebrew tap update. `.github/workflows/release.yml` is upstream's
release pipeline. It only runs when started by hand and it needs secrets this fork does
not have. Do not run it.

GitHub Actions is switched off for this whole repository, because GitHub offers no
way to disable a single workflow that a fork has not registered yet. Adding CI later
means switching Actions back on:

```bash
gh api -X PUT repos/aduverger/lumiere/actions/permissions -F enabled=true
```

Before doing that, delete or rewrite `release.yml`.
