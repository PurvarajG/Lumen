# Lumen design system (operator console)

Mode: **Operate** (impeccable). Taste dials: DESIGN_VARIANCE 3, MOTION_INTENSITY 2, VISUAL_DENSITY 6. Design read: native macOS live-presentation console for sabha AV volunteers in a dim room, calm dark language, standard egui controls, one saffron signature.

## World (type, palette, density, one signature move)

- **Type:** Poppins for all English/Latin text, UI and slides (bundled, OFL; Regular, Medium, SemiBold, Bold + Italic). ChitraUni for all Gujarati, UI and slides, loaded at runtime from the user's installed fonts (`~/Library/Fonts`, `/Library/Fonts`; commercial © Summit Information Technologies, never bundled or committed); bundled Noto Sans Gujarati (OFL) is the fallback when ChitraUni is not installed, and the app shows a one-line notice in Settings saying which Gujarati face is active. Gujarati is always shaped through lumen-text, including in UI lists. Scale (logical pt): 11 caption, 13 body, 15 section heading (semibold), 20 panel title, 28 live clock. Tabular numerals for clocks/timers. Never below 11.
- **Palette (tokens in `style.rs`, the only place colours are defined):**
  - `bg` #121316 window ground, `panel` #1A1C20, `raised` #23262B, `line` #2E3238 (1 px separators only)
  - `text` #ECE8E1 warm white, `text_dim` #A39E96 (≥4.5:1 on panel), `text_faint` #77726B (captions only, ≥3:1)
  - `saffron` #F2A33A signature / primary action / selection, `saffron_ink` #1A1206 text on saffron
  - `live` #E5484D LIVE state only, `live_ink` #FFFFFF
  - `ok` #46A758 running timer/connected, `warn` #F5B83D timer warning, `info` #5B9BD5 neutral notices
- **Density:** 8 pt grid; control height 28 pt (primary transport 40 pt); panel padding 12; group gap 8; section gap 20.
- **Signature move — the saffron "up next" rail:** the next cue to go live carries a 3 pt saffron rail and a soft saffron glow in the cue grid and in the running order, so the operator always sees what Next will send. LIVE is a red pill + red outline only on the cue currently on air. Nothing else uses saffron fill except the primary "Go live" action.

## Layout (Present mode, default)

Header row: project name · Prepare / Present segmented switch · output status pills (Audience, Stage, Remote) · Save state.

Three regions:
1. **Left rail (300 pt, resizable 240–520):** Search field on top (books + library + playlist, results grouped), then Running order (playlist), Library and Books (downloaded anirdesh books) tabs. The running order takes the rail's full remaining height (12 rows at 1500×949); Macros sit in a collapsible header at the bottom, collapsed in Present and open in Prepare. "New document" is a Prepare-only action.
2. **Centre:** large Preview of the selected/on-deck cue with its label, then the cue grid (16:9 thumbnails, width-aware columns, labels below, LIVE / on-deck states).
3. **Right column (360 pt):** Live output monitor (what the audience sees, with a red LIVE caption), transport (Previous · Next / Go live: cue · Clear · Blackout · Transition picker), a now-playing strip, then tabbed tools: Timers · Messages · Props · Audio · Stage.

**Now-playing strip** (directly under the transport row, only while an audio cue or an unmuted video with an audio track is playing; one row per source, audio first): a 28 pt pause/resume icon button, the file name (truncated), the clock `1:23 / -2:41` (tabular figures; just `1:23` when the length is unknown) and a 28 pt stop icon button, then a 4 pt progress bar (`ok` green while playing, `text_faint` while paused). The audio row adds a volume slider bound to the live sound; the video row has none (video volume is not adjustable live, and there is no seeking). Stop fades audio over 300 ms and clears a video like Clear > Media. Pausing a video freezes picture, sound and clock; resuming continues from the same frame.

Prepare mode swaps the right column's tools for the slide inspector (text, box, style, objects, background, audio, transition, actions) and enables editing affordances in the grid; transport stays visible but secondary.

## States (every surface)

Hover (raised + 1 step lighter), pressed, focused (2 pt saffron focus ring, keyboard visible), disabled (text_faint, no hover), loading (inline spinner + verb, never a blocking modal; a media cue without a thumbnail shows "Loading preview…" and the file name on a raised panel in the running order, the cue grid and the Preview, never black), empty (one sentence + the one action that fixes it, e.g. "No books downloaded yet — Download books"), error (inline, names problem + recovery, `live`-red icon only for on-air problems, `warn` otherwise; a media cue that cannot be previewed shows a `warn` icon with "Can't preview <file>" or "File not found: <file>", the reason in the tooltip, and is retried after 10 s unless the file is missing).

## Controls and copy

Standard egui widgets only: buttons, tabs/segmented, text fields, combo boxes, sliders, checkboxes, tables/lists. Icons from one embedded line-icon set (Lucide/Phosphor via a font, OFL/MIT) at one stroke weight — no emoji or Unicode glyphs as icons. Labels name the action ("Go live", "Clear slide", "Download Vachanamrut"). Errors: "Couldn't open kirtan.mp3 — file was moved. Choose file…".

## Keyboard (Present)

The selected cue is the armed cue; going live arms the one after it (Present only), so the selection always shows what Next sends and the Preview reads "Up next". Space / → / Page Down / Return: send the armed cue (button: "Next", or "Go live: <cue>" after you pick another) · ← / Page Up: previous, then arm the cue after it · F1 clear all · F2 clear slide · F3 clear media · F4 clear props · F5 clear audio · F6 clear messages · B blackout · Cmd+F search · Cmd+Z / Cmd+Shift+Z undo/redo. Shortcuts never fire while a text field has focus.

## Motion

Only output transitions (cut, dissolve, fade through black, push) and a 120 ms hover/selection ease. No UI animation delays a cue.

## Refuse

Cards nested in cards, eyebrow labels, gradient text, glass, coloured side borders > 1 pt (the saffron rail is the single exception and is the signature), emoji icons, grey-on-grey low contrast, modals for non-destructive tasks.
