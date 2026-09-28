---
layout: default
title: Chronicle
---

# 📜 Chronicle

`Rust` · `git`

**Periodic activity digest for a tree of git repos.** Answers "what
happened while I wasn't looking" across every repo under a root directory
— commits since the last check, plus current uncommitted/untracked
status — without the noise a generic file-integrity tool would produce
pointed at the same tree (`.git` internals — objects, refs, the index —
change constantly with completely normal git activity).

## 🚀 What it does

```bash
chronicle                    # scan, report, remember where we left off
chronicle --show-unchanged   # also list repos with nothing to report
```

- Discovers every git repo under configured root(s) — stops descending the instant a `.git` is found, never looks for repos nested inside a repo
- First time it sees a repo: records its current state, reports "tracking started" — doesn't dump the entire pre-existing commit history
- Every run after that: lists commits since the last check (`git log --oneline old..new`), plus current working-tree status (modified/staged/untracked counts)
- Detects history rewrites (rebase/amend/force-push) as a distinct case rather than silently showing zero commits or crashing
- Quiet by default — a repo with nothing new doesn't print anything unless `--show-unchanged`

## 📓 Vault logging

Set `markdown_out_dir` in the config and every run appends a dated section
to `<dir>/YYYY-MM-DD.md` — multiple runs in a day accumulate under the
same file as separate `## HH:MM:SS run` sections, so pointing this at a
notes vault builds an actual running chronicle over time, not just a
one-off terminal report.

## ⚙️ Configuration

`~/.config/chronicle/config.toml`:

```toml
state_path = "~/.local/state/chronicle/state.json"
max_depth = 2
markdown_out_dir = "~/Vaults/darknotes/Chronicle"

[[root]]
path = "/home/raven/.sysops"
```

## 🧩 Layout

```
src/discover.rs   finds git repos under a root — hand-rolled, not walkdir,
                  since "stop the instant you find a .git" is simpler to
                  express directly than to bolt onto a general walker
src/git.rs        shells out to `git` itself for HEAD, log ranges, status
src/state.rs      persists last-seen HEAD per repo between runs
src/report.rs     terminal + markdown rendering, cybercore-themed colors
```

All discovery/git-parsing logic is unit-tested — `git.rs`'s tests actually
run real `git init`/`commit`/`status` in a scratch directory rather than
mocking git's output, and the full CLI was verified end-to-end (real
commits, an untracked file, first-run vs. subsequent-run behavior) before
being pointed at real project repos.

## 🗺 Known limitations

- No de-duplication if two configured roots overlap the same repo
- History-rewrite detection is binary (rewritten or not) — doesn't attempt to reconstruct what actually happened during the rewrite

## 📄 License

MIT


[Source on GitHub](https://github.com/cybercore-tech/chronicle)
