# ProPresenter feature surface reconnaissance (read-only)

Scope: read-only static inspection of `/Applications/ProPresenter.app`. The application was not launched, user data directories were not accessed, and the application bundle was not modified. Commands were bounded and executed against shipped localized strings, interface builder archives, and default preference plists.

## Evidence and provenance

All raw outputs and checksum records are retained locally in `/Users/purvarajsinhgohil/Developer/PROPRESENTER/.local/research/r1/`.

### Inspected target files and SHA-256 digests
- `en.lproj/ProPresenter.strings` (UTF-8, 13,348 lines): `f5d1d31b7d956035587d316ecdeec714e72d70090c6cb7965c3c11f909fa7b41`  
  `/Applications/ProPresenter.app/Contents/Resources/Localization_Localization.bundle/Contents/Resources/en.lproj/ProPresenter.strings`
- `MainMenu.nib` (NIBArchive binary, 92,867 bytes): `870a876f0f3b30738ed092ebb6fe9af04b4b0cd9bbe0c0e505b5718cd585ef28`  
  `/Applications/ProPresenter.app/Contents/Resources/MainMenu.nib`
- `standard_defaults.plist` (Apple binary plist, 100 entries): `b5778eacdcc97c3177b571e8e1f7d3662d217edc7478563dce0180ba37625334`  
  `/Applications/ProPresenter.app/Contents/Resources/standard_defaults.plist`
- `RVTimerController.nib`: `74ed29c5e29637998765b3b224210a6fc680d51c875a085505afa551a4f1f093`  
  `/Applications/ProPresenter.app/Contents/Resources/RVTimerController.nib`
- `MessageFormTimerViewController.nib`: `bf39ca32ada37557cc4c7c2c6c89e071e7dddcadd43fee22a8e272e2edfe2d29`  
  `/Applications/ProPresenter.app/Contents/Resources/MessageFormTimerViewController.nib`
- `MessageActionViewController.nib`: `bb3a726604fde8d4dbea3a6cb5eb143074da649af97d232b79462046c660dd5d`  
  `/Applications/ProPresenter.app/Contents/Resources/MessageActionViewController.nib`
- `RVMessagesFormElementViewController.nib`: `b82b49fd636b09844efa9e4813ffde0cf6e96b98fa03bfbce57b1a62a4bc925b`  
  `/Applications/ProPresenter.app/Contents/Resources/RVMessagesFormElementViewController.nib`
- `ClearGroupConfigurationViewController.nib`: `501418a6b9d389830909838448a51e1295dc5eba139dc36344a205a7acc0d91d`  
  `/Applications/ProPresenter.app/Contents/Resources/ClearGroupConfigurationViewController.nib`
- `ClearGroupSidebarViewController.nib`: `57b25419a5fd8b560320495d66c0dafe569a41adbe09a36ca060ffb3ee40d2c0`  
  `/Applications/ProPresenter.app/Contents/Resources/ClearGroupSidebarViewController.nib`
- `KeyMappingsViewController.nib`: `70a93953cbf027b5a1d275b8480e9fdceb7a1be5fa52e1eec1bec3f03d72c3a3`  
  `/Applications/ProPresenter.app/Contents/Resources/KeyMappingsViewController.nib`
- `HotKeySelectionViewController.nib`: `57a777fe5fa12f4a4fc3fd3d9a8b834edac20a0bf6f62d2fdc5d3b9b7cf8e5df`  
  `/Applications/ProPresenter.app/Contents/Resources/HotKeySelectionViewController.nib`

### Commands and inspection tooling
- Local REA runtime NIB decoder (`.tools/rea-runtime/node_modules/rea-agents/dist/artifacts/apple/NibArchive.js`): decoded object hierarchies and key-equivalent mappings for `MainMenu.nib`, `KeyMappingsViewController.nib`, and related dialog nibs.
- `plutil -p /Applications/ProPresenter.app/Contents/Resources/standard_defaults.plist`
- `strings -n 4` on compiled NIB files.
- `shasum -a 256` on all inspected files, verified into `.local/research/r1/checksums.sha256`.

---

## 1. Timer types and options

- **Observed** (`ProPresenter.strings` keys `TIMERS_WINDOW_*`, `UNDO_TIMERS_*`, `ACTION_PALETTE_TIMER_*`):
  - Three distinct timer types exist:
    1. **Countdown**: counts down from a configured duration (`TIMERS_WINDOW_COUNTDOWN_DURATION_title` = "Duration:"). Shipped default preset: "Segment Countdown" (`TIMERS_WINDOW_DEFAULT_COUNTDOWN_TIMER_name`).
    2. **Countdown to Time**: counts down toward a specified time of day (`TIMERS_WINDOW_COUNTDOWN_TO_TIME_TIME_title` = "Time:"). Supports AM, PM, and 24-hour formats (`TIMERS_WINDOW_COUNTDOWN_TO_TIME_FORMAT_AM_title`, `PM`, `24_HOUR`). Shipped default preset: "PreShow Countdown" (`TIMERS_WINDOW_DEFAULT_COUNTDOWN_TO_TIME_TIMER_name`).
    3. **Elapsed Time**: counts up from a start time toward an optional end time (`TIMERS_WINDOW_ELAPSED_START_title`, `END_title`), or counts up without upper bound when configured with "No Limit" (`TIMERS_WINDOW_ELAPSED_NO_LIMIT_placeholder` = "No Limit"). Shipped default preset: "Game Timer" (`TIMERS_WINDOW_DEFAULT_ELAPSED_TIMER_name`).
  - **Overrun** (`UNDO_TIMERS_SET_OVERRUN_ENABLE` = "Allows Overrun", `MESSAGES_WINDOW_TIMER_OVERRUN_CHECKBOX_title`): when enabled, timers continue counting past zero into negative values (e.g. `-00:01:23`).
  - **Display formats & color triggers** (`EDITOR_PRESENTATION_TEXT_INSPECTOR_LINKED_TIMER_*`):
    - Configurable date and time formats including 24-hour time, "No Date", and "No Time".
    - "Color Triggers" (`EDITOR_PRESENTATION_TEXT_INSPECTOR_LINKED_TIMER_COLOR_TRIGGERS_label`): threshold timestamps where timer text changes display color (e.g. warning amber and expiration red).
  - **Cue & Macro control operations** (`ACTION_PALETTE_TIMER_*`):
    - Seven distinct timer actions: Start, Stop, Reset, Reset and Start, Stop and Reset, Increment, and Set Configuration.
  - **Playlist header integration** (`UNDO_PLAYLIST_ITEM_HEADER_SET_TIMER` = "Set Linked Timer"): playlist headers can bind to a timer to track live section pacing.
- **Inference**: Timers run as independent lightweight clocks within the runtime state engine. Presentation slides, stage displays, and messages subscribe to timer values as dynamic data links without mutating the underlying presentation schema.

---

## 2. Message features

- **Observed** (`ProPresenter.strings` keys `MESSAGES_WINDOW_*`, `UNDO_MESSAGES_*`, `standard_defaults.plist`):
  - **Token syntax**: `${TokenName}` string interpolation syntax (observed in `standard_defaults.plist` under `CCLIMessageFormat`, e.g. `${Title}`, `${Artist}`), alongside visual token chips (`MESSAGES_ADD_TOKEN_title` = "Add Token…").
  - **Token types** (`MESSAGES_WINDOW_TOKEN_*`):
    - Name, First Name, Last Name.
    - Message (arbitrary operator text entered via `MESSAGES_WINDOW_TEXT_PROMPT_TEXT_LABEL_title` = "Value:").
    - Room.
    - System Clock (with customizable Date/Time formats, 24-hour toggle, "No Date", "No Time").
    - Timers (linked Countdown, Countdown to Time, or Elapsed timers with interactive Start/Stop/Reset in message controller).
    - Custom Tokens (`MESSAGES_WINDOW_ADD_CUSTOM_TOKEN_BUTTON_tooltip` = "Add Custom Token").
  - **Display & dismissal behavior**:
    - Theme slide template: messages render styled text onto the audience output using an assigned theme slide (`MESSAGES_WINDOW_TEMPLATE_title` = "Theme:").
    - Dismissal modes (`MESSAGES_WINDOW_DISMISS_OPTION_*`):
      1. "Manually"
      2. "After Timer Expires" (single timer)
      3. "After All Timers Expire" (multiple timers)
      4. "After Time" (auto-dismisses after `%i seconds`)
    - Transitions: dedicated "Messages Transition" button (`MESSAGES_TRANSITION_BUTTON_tooltip`).
    - Web / Network notifications: "Allow Web Notifications" (`MESSAGES_WINDOW_ALLOW_WEB_NOTIFICATIONS_BUTTON_title`), exposing an endpoint for external devices to submit message requests requiring operator review ("Message Request").
- **Inference**: Messages form an isolated presentation layer sitting above slide text and media. They apply theme formatting to dynamic token strings and can be dismissed automatically by timer expiration without requiring an operator cue change.

---

## 3. Props

- **Observed** (`ProPresenter.strings` keys `EDITOR_PROP_*`, `PROPS_WINDOW_*`, `UNDO_PROPS_*`, `ACTION_TYPE_PROP*`):
  - **Definition**: Props are persistent overlay slide layers positioned above slide content and media layers (and beneath messages/masks). They remain active on screen across slide and presentation transitions.
  - **Configuration & inspector options** (`EDITOR_PROP_INSPECTOR_*`):
    - Name (`EDITOR_PROP_INSPECTOR_NAME_placeholder` = "Prop Name").
    - Background color (`EDITOR_PROP_INSPECTOR_BACKGROUND_title`).
    - Document size (`EDITOR_PROP_INSPECTOR_SIZE_title`, resizing applies to all props: `EDITOR_PROP_INSPECTOR_RESIZE_alert`).
    - Transition: per-prop transition and global document transition (`UNDO_DOCUMENT_PROPS_SET_GLOBAL_TRANSITION`).
    - Auto-Clear: configurable automatic clear enable and duration (`UNDO_PROPS_AUTOCLEAR`, `UNDO_PROPS_AUTOCLEARTIME`).
  - **Organization & modes**:
    - Prop Collections and Prop Groups (`PROPS_HEADER_COLLECTION_POPUP_title`, `UNDO_PROPS_GROUP_ADD`).
    - "Single Mode" (`UNDO_PROPS_GROUP_SINGLEMODE`): mutually exclusive triggering where activating one prop in the group automatically clears any previously active prop in that group.
  - **Show control integration**:
    - Interactive triggering and clearing (`PROPS_WINDOW_TABLE_ROW_CLICK_TO_TRIGGER_PROP_tooltip`, `PROPS_WINDOW_TABLE_ROW_CLICK_TO_CLEAR_PROP_tooltip`).
    - Cue action behavior: "Set Prop Action Behavior" (trigger or clear).
    - Clear Props layer action (`ACTION_TYPE_CLEAR_PROPS_title`, shortcut `F4`).
    - Audience Look toggle: props can be enabled or suppressed per output screen in Audience Looks (`AUDIENCE_LOOKS_PROPS_label`).
- **Inference**: Props are designed for persistent visual bugs, lower-third speaker identifiers, translation overlays, or emergency badges that must remain unchanged while presentation slides advance underneath.

---

## 4. Transition names/types and parameters offered

- **Observed** (`ProPresenter.strings` keys `TRANSITIONS_*`, `standard_defaults.plist`):
  - **37 distinct transition types cataloged**:
    - Amoeba, Color Burn, Color Push, Color Warp, Cross Hatch, Cube, Cut, Dispersion Blur, Dissolve (Default), Door, Door Wipe, Fade, Fade Black, Fade Bright, Fade Dark, Fade Gray, Fade White, Film Burn, Flip, Fly In, Iris, Kaleidoscope Wipe, Mosaic, Move In, Noisy Zoom, Push, Random Pixels, Random Squares, Random Squares Flicker, Reveal, Ripple, Square Wipe, Swap, Warp Fade, Wave Dissolve, Wipe, Zoom In.
  - **Parameters offered**:
    - **Duration**: slider in seconds (`TRANSITIONS_DURATION_label`, default `0.6s` in `standard_defaults.plist`, formatted as `%.1fs`).
    - **Direction**: 9 directional presets (`TRANSITIONS_DIRECTION_*`):
      1. From Top Left
      2. From Top
      3. From Top Right
      4. From Left
      5. Center
      6. From Right
      7. From Bottom Left
      8. From Bottom
      9. From Bottom Right
    - **Presets & Favorites**: "Save As…", "New Transition Favorite", Favorites, Recents, and All categories.
  - **Transition Scopes**:
    - Global default, Document level, Slide level, Media level (Background and Foreground), Props layer, Messages layer, Audio bin, and Audience Look level.
- **Inference**: The rendering engine relies on GPU shader transitions where Cut (instant) and Dissolve (cross-fade) represent the core pipeline, while spatial transitions (Wipe, Push, Fly In, Reveal) share uniform duration and 9-way directional vectors.

---

## 5. Stage display layout element types

- **Observed** (`ProPresenter.strings` keys `TEXT_LINK_*`, `FILL_LINK_*`, `EDITOR_STAGE_*`):
  - Stage layouts are created in the editor as canvas layouts containing dynamic linked elements.
  - **Text Link types** (`TEXT_LINK_*`):
    1. **Current Slide Text** (`TEXT_LINK_CURRENT_SLIDE_TEXT_title`)
    2. **Current Slide Notes** (`TEXT_LINK_CURRENT_SLIDE_NOTES_title`, with "Preserve Notes Format")
    3. **Current Slide Element Matching Name** (`TEXT_LINK_CURRENT_SLIDE_ELEMENT_MATCHING_NAME_title`)
    4. **Next Slide Text** (`TEXT_LINK_NEXT_SLIDE_TEXT_title`)
    5. **Next Slide Notes** (`TEXT_LINK_NEXT_SLIDE_NOTES_title`)
    6. **Next Slide Element Matching Name** (`TEXT_LINK_NEXT_SLIDE_ELEMENT_MATCHING_NAME_title`)
    7. **Timers** (Countdown, Elapsed, Countdown to Time)
    8. **System Clock** (with 12/24h and Date/Time format options)
    9. **Video Countdown** (with "Ignore Looping Videos" option)
    10. **Audio Countdown** (with "Ignore Looping Audio" option)
    11. **Stage Message** (with "Flash When Message Shown" and "Flash Color")
    12. **Auto Advance Time Remaining** (Presentation and Announcement)
    13. **Slide Count** (Current, Remaining, Total)
    14. **Group Name** (Current Slide, Next Slide, Next in Arrangement)
    15. **Slide Label** (Current Slide, Next Slide)
    16. **Presentation Name** (Global, Presentation, Announcement, Operator Notes)
    17. **Playlist Item** (Playlist Name, Current Item, Next Item, Current Header, Next Header, Show Arrangement)
    18. **Capture Status & Elapsed Time** (Inactive, Active, Caution, Error)
    19. **Playback Marker** (Marker Name, Marker Time)
    20. **Timecode**
    21. **CCLI**
    22. **Text Ticker** (RSS Feed, File)
  - **Fill Link types** (`FILL_LINK_*`):
    1. **Current Slide Image** (`FILL_LINK_CURRENT_SLIDE_IMAGE_title`)
    2. **Next Slide Image** (`FILL_LINK_NEXT_SLIDE_IMAGE_title`)
    3. **Screen Preview** (Live mirroring of an audience screen)
    4. **Chord Chart** (`FILL_LINK_CHORD_CHART_title`)
    5. **Group Color** / **Label Color**
    6. **Capture Status Color** / **Timecode Status Color** / **Playback Marker Color**
    7. **Planning Center Live** (fill link showing schedule/service items with Light/Dark themes and countdown modes: Full Item Length, End Item on Time, End Service on Time)
    8. **Slide Object** (graphic mirror of an element from the active slide)
- **Inference**: The stage display system does not use hardcoded templates; instead, stage layouts are editable presentations whose elements dynamically bind to live presenter state variables.

---

## 6. Cue/slide action types and macro action types

- **Observed** (`ProPresenter.strings` keys `ACTION_TYPE_*`, `ACTION_PALETTE_*`, `UNDO_DOCUMENT_CUE_ACTION_*`, `UNDO_MACRO_*`):
  - **Action Palette categories**:
    1. **Clear Actions**: Clear All, Clear Slide, Clear Media, Clear Props, Clear Messages, Clear Audio, Clear Video Input, Clear Announcements, Clear to Logo, Clear Group.
    2. **Media Actions**: Media (Foreground/Background image/video), Audio, Live Video (Video Input).
    3. **Interactive Actions**: Stage (Stage Layout assignment, Slide Destination: Stage vs Stage + Audience), Timer (Start, Stop, Reset, Reset & Start, Stop & Reset, Increment, Set Configuration), Message, Prop (Trigger, Clear).
    4. **Advanced Actions**: Audience Look, Communication (MIDI, DMX, TCP, UDP, Serial), Slide Destination, Macro, Media Playlist, Audio Playlist.
  - **Slide Destination Action** (`ACTION_PALETTE_STAGE_DESTINATION_*`): allows individual slide cues to route either to "Stage" only or "Stage + Audience".
  - **Macro concept and action types**:
    - A Macro is an editable named, color-coded sequence (`UNDO_MACRO_SET_NAME`, `UNDO_MACRO_SET_COLOR`) containing an ordered list of cue actions (`UNDO_MACRO_ACTIONS_ADD`).
    - Execution options: clicked directly in the Macros panel, triggered via slide cue ("Macro: %@"), triggered by hotkey or MIDI mapping, or executed on application launch ("Trigger on Startup": `UNDO_MACRO_TRIGGER_ON_STARTUP`).
    - Grouped into Macro Collections (`MACROS_HEADER_COLLECTION_POPUP_title`).
- **Inference**: A Macro is a reusable composite action container. The internal action payload of a macro is identical to the action array of a slide cue.

---

## 7. Default keyboard shortcuts for show control

- **Observed** (`MainMenu.nib` decoded via REA `NibArchive`, `KeyMappingsViewController.nib`, `ProPresenter.strings`):
  - **Show Control Navigation**:
    - `Next Slide`: `RightArrow` (also `Space` / `Return` in presenter context)
    - `Previous Slide`: `LeftArrow`
    - `Next Background`: `Command + RightArrow`
    - `Previous Background`: `Command + LeftArrow`
    - `New Slide`: `Command + Shift + N` / `Return` (in editor mode)
    - `Show Triggered Presentation`: `Command + l`
    - `Show Triggered Announcement`: `Command + Shift + L`
    - `Reveal in Library`: `Option + Command + l`
    - `Slide Hot Keys`: Per-slide single alphanumeric keys (`HotKeySelectionViewController.nib`)
  - **Clear Layer Shortcuts** (statically encoded in `MainMenu.nib`):
    - `Clear Slide`: `F2` (`\uf705`)
    - `Clear Media`: `F3` (`\uf706`)
    - `Clear Props`: `F4` (`\uf707`)
    - `Clear Audio`: `F5` (`\uf708`)
    - `Clear Messages`: `F6` (`\uf709`)
    - `Clear Announcements`: `F7` (`\uf70a`)
    - `Clear to Logo`: `F12` (`\uf70f`)
    - `Clear All`: `F1` (`\uf704` in key mappings; default show clear mapping)
  - **Workspace & Inspector Shortcuts** (`MainMenu.nib`):
    - `Show Mode`: `Control + s`
    - `Presentation Editor`: `Control + e`
    - `Reflow Editor`: `Control + r`
    - `Bibles`: `Control + b`
    - `Audio Bin`: `Control + a`
    - `Media Bin`: `Control + v`
    - `Timers`: `Control + c`
    - `Messages`: `Control + m`
    - `Props`: `Control + p`
    - `Macros`: `Control + Shift + M`
    - `Stage`: `Command + Shift + S`
    - `Key Mappings...`: `Option + k`
    - `Settings…`: `Command + ,`
- **Inference**: Show control uses unadorned function keys (F1–F7, F12) as panic clear buttons to guarantee instant access during live performance without multi-key chord latency.

---

## 8. Clear-layer groups

- **Observed** (`ProPresenter.strings` keys `CLEAR_GROUPS_*`, `ClearGroupConfigurationViewController.nib`, `ClearGroupSidebarViewController.nib`):
  - **Concept**: Clear Groups allow operators to configure custom multi-layer clear targets that clear a specific combination of active layers in a single cue or button press (`ACTION_TYPE_CLEAR_GROUP_title` = "Clear Group").
  - **Configurable layer targets** (`CLEAR_GROUPS_*_LAYER_title`):
    1. **Presentation** (Slide text and slide elements)
    2. **Presentation Media** (Background and Foreground media)
    3. **Props**
    4. **Messages**
    5. **Announcements**
    6. **Music** (Audio bin)
    7. **Audio Effects**
    8. **Video Input** (Live video input)
  - **Clear Group options**:
    - "Stop Timeline" (`CLEAR_GROUPS_STOP_TIMELINE_MENU_ITEM_title`)
    - "Clear Next" (`CLEAR_GROUPS_CLEAR_NEXT_SLIDE_MENU_ITEM_title`)
    - Custom icon selection, custom icon import, and icon tint color (`CLEAR_GROUPS_ICON_title`, `CLEAR_GROUPS_TINT_title`).
    - Preview bar visibility toggle: show or hide each clear group button in the main transport/preview area (`UNDO_CLEAR_GROUP_IS_HIDDEN_IN_PREVIEW`).
  - **Built-in clear layer buttons**: Clear All, Clear Slide, Clear Media, Clear Props, Clear Messages, Clear Audio, Clear Video Input, Clear Announcements, Clear to Logo.
- **Inference**: Clear Groups prevent catastrophic operator mistakes during complex transitions (such as clearing video and stage messages while keeping walk-in audio and lower-third props running).

---

## Recommended for Lumen 1.0 (≤ 12 bullets)

1. **Core Timers**: Implement Countdown (duration), Countdown to Time (wall clock), and Elapsed Time, with "Allows Overrun" counting negative past zero.
2. **Timer Color Triggers**: Support threshold-based warning text colors (e.g. amber at warning time, red at zero).
3. **Template Messages**: Render messages through theme slide templates with plain text and timer tokens, supporting manual and timer-based auto-dismissal.
4. **Props Layer**: Implement a persistent overlay layer rendered above slides and media with independent alpha, transition, and auto-clear timer.
5. **Prop Single Mode**: Provide mutually exclusive prop groups where triggering one prop automatically clears other active props in that group.
6. **Essential Transitions**: Prioritize Cut, Dissolve, Fade to Black, and directional Wipe/Push with duration control (0.1s to 10.0s).
7. **Stage Display Dynamic Links**: Support Current Slide Text/Image, Next Slide Text/Image, Stage Message (with flash condition), Timers, and System Clock.
8. **Cue Action Palette**: Enable attaching Clear (All, Slide, Media, Props, Audio), Timer (Start, Stop, Reset), and Prop actions to slide cues.
9. **Macros**: Provide named, color-coded composite action lists executable manually, via slide cue, or on startup.
10. **Clear Groups**: Allow user-defined clear buttons that clear customizable subsets of active layers (e.g., Slide + Media only).
11. **Standard Clear Shortcuts**: Bind function keys F1 (Clear All), F2 (Clear Slide), F3 (Clear Media), F4 (Clear Props), F5 (Clear Audio), and F6 (Clear Messages).
12. **Standard Show Navigation**: Bind Right Arrow / Space (Next Slide), Left Arrow (Previous Slide), and per-slide alphanumeric hotkeys.
