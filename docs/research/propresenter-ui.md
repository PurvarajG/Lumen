# ProPresenter operator UI observations

Observed 2026-10-16 through native macOS accessibility and screenshot tools, ProPresenter 21.4. Initial account activation sheet offered Skip for Now; that option opened the workbench without an account. No reference document, timer values, or media were edited. This is UI evidence; it does not establish implementation internals.

- Left sidebar separates Library and Playlist. A second pane lists the selected library/playlist contents and a Filter field. The presentation area offers Grid, Easy and Table view controls and a zoom slider.
- Right preview has a screen selector and separate Presentation, Announcements and Audio controls. Top toolbar exposes Audience and Stage output controls. Output timing/behavior was not exercised in this pass.
- Lower panels are selectable: Audio Bin, Stage, Timers, Messages, Props and Macros. A separate media bin has playlists, video inputs, search and grid/table controls.
- Timers show a name, current value, Start and Reset controls. Expanding a timer exposes Type, Duration and Allows Overrun. Another visible timer is labeled Countdown to 5:00 PM, supporting a separate countdown-to-time mode in the reference.
- Show, Edit, Reflow and Bible are distinct toolbar modes. Theme and Text controls act on selected slides.

The foundation implementation deliberately covers duration countdowns ending at zero. Reference overrun, countdown-to-time, production-layer controls, Bible content and advanced media bins are separately tracked parity work. UI control names alone are not evidence of underlying algorithms or file schemas.
