# A1: lumen-interchange (addendum to interchange.md)

Implement `docs/briefs/interchange.md` exactly, with these orchestrator changes:

- You work in a git worktree on your own branch. You may add the crate to the workspace (members are `crates/*`, so creating `crates/lumen-interchange/` is enough) and let Cargo.lock update. Commit on your branch with a conventional message instead of "no source commits".
- Read and follow `~/.claude/skills/ponytail/SKILL.md`: smallest complete implementation, no speculative API.
- lumen-core is being upgraded in parallel (schema v2 adds slide fields with defaults and inline markup where `*`, `_`, `[`, `\` become special). Build every slide through ONE private helper `fn text_slide(label: &str, text: &str, theme_id: Uuid) -> Slide` that calls `Slide::new(theme_id)` and sets `label`/`text`, so the parent can add markup escaping in one place at merge. Do not construct `Slide { .. }` literals. For `.txt` use `lumen_core::document_from_text`.
- Export takes text as stored; leave a `// parent: strip markup at merge` comment at the single place slide text is read for export.
- The real fixtures live in the main checkout, not your worktree: `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/Lumen-Research-Fixture.pro` and `Lumen-Unicode-Fixture.pro`. Read them by absolute path in an `#[ignore]` test or example; do not copy them into git.
- Report to `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/reports/a1-interchange.md`. Final reply: 10 lines max.
