# R1: ProPresenter feature-surface research (read-only)

Goal: evidence for naming and parity decisions in Lumen's timers, messages, props, transitions, stage layouts, cue actions and keyboard shortcuts. Output: a new `docs/research/propresenter-feature-surface.md` in your worktree (commit it), raw outputs under `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/r1/`. Nothing else.

Rules (from AGENTS.md): inspect `/Applications/ProPresenter.app` read-only; never launch it, never open user data, never modify the bundle. Use the pinned REA CLI `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.tools/rea-runtime/node_modules/.bin/rea` (`rea capabilities`, `rea decode-interface-builder`, `rea analyze`, string/plist inspection commands — check `rea <cmd> --help`), plus `plutil -p`, `strings -n 4` on localized `.strings`/`.stringsdict`/nib resources. Bound every command (`head`, timeouts); do not decompile the whole binary. Record exact command, file path and SHA-256 of each inspected file. No proprietary executable, decompiled code or creative assets copied into the doc — short UI label lists are fine.

Answer, marking each item **Observed** (with source) or **Inference**:

1. Timer types and options (e.g. countdown, countdown-to-time, elapsed; overrun; formats).
2. Message features: token syntax/types, display behaviour options.
3. Props: what they are, options.
4. Transition names/types and parameters offered.
5. Stage display layout element types (current/next slide, notes, clock, timers, messages, video…).
6. Cue/slide action types and macro action types.
7. Default keyboard shortcuts for show control (next/previous, clear layers, blackout etc.) from the main menu nib or key-equivalent resources.
8. Clear-layer groups (e.g. clear slide, media, props, audio, messages, all).

End the doc with a "Recommended for Lumen 1.0" list (≤ 12 bullets) of the minimal subset worth matching. Final reply: 10 lines max.
