# Spike: PowerPoint slideshow inside Lumen by window capture (H2.4)

Date 2026-10-20. Mac: Apple M5 Pro, macOS 26 (Darwin 25.6), PowerPoint 16.114. Displays: built-in 3024 × 1964 (120 Hz), two LG ULTRAFINE 3840 × 2160 (HiDPI, looks like 1920 × 1080, 60 Hz). Lumen audience fullscreen on LG (1), console on the built-in.

**Recommendation: GO.** Questions 1, 2, 3, 4 and 5 all hold on this Mac. Limits and untested parts are listed at the end; H2.5 must handle each of them.

Throwaway code and evidence are in `.local/qa/h2/` (not committed): `make_deck.py` + `anim-deck.pptx` (3 slides: click 1 fade-in text, click 2 spin emphasis, slide 2 has a Push transition, slide 3 plain), `sck/probe.swift` (ScreenCaptureKit probe), `next.scpt`, `restart-show.sh`, outputs in `sck/out*`.

## Method

- PowerPoint is driven only by AppleScript; the probe lists windows with `SCShareableContent`, then runs an `SCStream` with `SCContentFilter(desktopIndependentWindow:)`, BGRA, `minimumFrameInterval` 1/60, `queueDepth` 5, `showsCursor = false`, `ignoreShadowsSingleWindow = true`, optional `sourceRect` and output size.
- Every complete frame is hashed (sampled pixels) with a wall-clock time. `next.scpt` takes `perl Time::HiRes` timestamps just before and after `go to next slide`. Latency = first changed frame − call start; animation fps = changed frames / (last change − first change).
- CPU from `top -l 7 -s 1` on PowerPoint and the probe process.

## Answers

1. **Window-mode start without activating: yes.** `set show type of (slide show settings of p) to slide show type window` then `run slide show slide show settings of p` creates a window titled `PowerPoint Slide Show – [<deck name>]` (default 1512 × 949 pt at the top-left of the built-in display). With Lumen's console frontmost before the call, `System Events` still reported Lumen frontmost afterwards; the audience stayed fullscreen. `open -g file.pptx` loads a deck without bringing PowerPoint forward (frontmost app unchanged); plain `open` activates it. The window can be sized and placed by AppleScript: `set left position / top / width / height of slide show window 1`. Height is clamped to the display height minus 30 pt (1050 pt on an LG at 1080 pt); width is not clamped. The window always has a 32 pt title bar.
2. **Placement the operator never sees: yes.** A slide-show window moved onto LG (1) (behind the fullscreen Lumen audience window, which is its own Space: `kCGWindowIsOnscreen` false, `isOnScreen` false) is still captured at full rate (148 frames in 3 s) with the correct picture. A window partly off every screen (left 2600 or 4000, macOS pulls it back to 3032) also captures normally. **Not capturable:** app hidden with Cmd+H (`set visible of process … to false`) and window minimised (`set collapsed of slide show window 1 to true`): zero complete frames (status ≠ complete); frames resume right after unhide/restore (same window id).
3. **Builds: yes.** `go to next slide (slide show view of slide show window 1)` steps animation builds as well as slides: call 1 = the 800 ms fade entrance, call 2 = the 2 s spin emphasis, call 3 = slide 2 with the Push transition (738 ms), call 4 = slide 3, call 5 = the show ends. `go to previous slide` works the same way.
4. **Rate, latency, CPU:** window 1814 × 1050 pt (3628 × 2100 px, 2× backing).

   | Output size | frames/s overall | changed frames/s in animations | first change after call | PowerPoint CPU (peak) | probe CPU |
   |---|---|---|---|---|---|
   | 3628 × 2100 (window native) | 44–51 | 44–47 | 26–78 ms | 22.7 % | 1.6 % |
   | 3840 × 2160 (cropped, scaled by SCK) | 58 | 40 | 28–32 ms | 20.6 % | 2.2 % |
   | 1920 × 1080 (cropped, scaled by SCK) | 58 | 39 | 34–43 ms | 22.6 % | 2.0 % |

   Targets met (≥ 30 fps, ≤ 100 ms, total < 60 % of one core): PowerPoint ≤ 23 % + capture consumer ≈ 2 % + the Lumen debug build idling at about 10 %. **Not measured:** the cost of uploading each frame into an egui texture (30 MB per frame at panel size); H2.5 must measure it and may capture at 2560 × 1440 if needed. One extra changed frame arrives about 1 s after an animation ends (a settling re-render), harmless.
   Sharpness ceiling: on an LG the tallest window is 1050 pt, so the slide area is 1810 × 1018 pt = 3620 × 2036 px, a 6 % upscale to 3840 × 2160.
5. **Chrome: removable by `sourceRect`.** The captured window includes the 32 pt title bar with traffic lights, a window-shaped black edge of 1–2 px at the sides when the window is not exactly 16:9, and rounded bottom corners (about 10 px of dark leaks in at the bottom corners). `sourceRect` in window points works: `2,32,1810,1018` on the 1814 × 1050 window gave a clean 1920 × 1080 frame whose first row and first column were slide pixels. H2.5 should size the window so `(height − 32) × 16/9 ≤ width`, then crop the largest centred 16:9 box and inset the bottom by about 8 pt. The mouse cursor never appeared with `showsCursor = false` (and also not with true in my test, because PowerPoint hides it in show view); PowerPoint's own UI (ribbon, notes) is not part of the slide-show window.
6. **Permissions:** `CGPreflightScreenCaptureAccess()` returned true here and AppleScript to PowerPoint ran without a prompt (both already granted to this terminal host). **Not observed:** the first-run prompts and the denied state, because revoking Screen Recording or Automation would break the user's own tools. By design: with Screen Recording denied `SCShareableContent` throws and the preflight call returns false, so Lumen can check up front and fall back to the fullscreen "Play in PowerPoint" path.
7. **Live-show risks:**
   - The slide-show window id changes every time a show starts and PowerPoint reopens it at its default size and place: find it by owner `Microsoft PowerPoint` and title prefix `PowerPoint Slide Show`, and move/size it after every start.
   - `next` on the last slide ends the show (`slide index` becomes `missing value`) and the window disappears: Lumen must not forward Next past the last slide, and treat a vanished window as "show ended".
   - Cmd+H on PowerPoint or minimising its window freezes the picture (no frames): hold the last frame.
   - `open` without `-g` and `activate` bring PowerPoint to the front: never use them on the Lumen path.
   - The user's own PowerPoint documents may be open (one was during this spike): address the deck by name (`presentation "anim-deck.pptx"`), never `active presentation`, and never close or save others.
   - Not tested: Microsoft AutoUpdate or licence dialogs appearing mid-show, a deck with embedded video or audio, Morph (Push was used instead), the window changing Space when the user switches Spaces.

## Decision

**GO** for H2.5. Rule check: 1 yes, 2 yes (behind the fullscreen audience window, or partly off-screen), 3 yes, 4 yes (44 fps, ≤ 78 ms, CPU ≈ 25 %), 5 yes (`sourceRect`).
