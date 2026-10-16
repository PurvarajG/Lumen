# Native workbench design

The user requested the gpt-taste skill for the final UI pass. Its editorial hierarchy, contrast, typography, balanced preview imagery and hover feedback are adapted to a Rust egui desktop workbench. AIDA marketing sections, GSAP scroll effects, marquees and stock photography do not serve cue operation and are not part of this native application.

The seeded design exercise selected an editorial split and Satoshi. The shipped font is Geist because the downloaded npm `geist` 1.7.2 package supplies an explicit SIL Open Font License for redistribution. Its regular TTF is embedded; egui's original fallback fonts remain available. The font license is retained beside the asset and copied into the macOS bundle's Resources directory.

- Fixed dark graphite palette, warm white text, pale green selection/Go Live, coral live state.
- Project identity and file/output controls occupy two clear header rows.
- Running order, document library and macros share the left rail.
- A large selected-cue preview leads the central workspace. The thumbnail grid uses consistent 16:9 geometry and width-aware columns, including one column in narrow space.
- Audience preview and transport remain visible above the inspector. Edit slide and Stage tools switch the lower panel so controls do not compete for height.
- Selected previews and editable thumbnails can differ from the detached live snapshot. Live borders identify the cue that was triggered; they do not imply that subsequent edits are already on air.
- Buttons have explicit hover/active contrast and native egui feedback. No scroll animation delays cue controls.

The desktop minimum is 1100 by 700 logical points. Large documents and inspector content scroll. Media previews for an untriggered, different external image/video do not yet have a separate thumbnail decoder; live media is shown when its background matches the selected cue. This is a documented limitation, not an asset-import failure.
