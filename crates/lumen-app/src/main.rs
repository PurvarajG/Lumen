mod add;
mod app;
mod audio;
mod books;
mod brand;
mod deck;
mod displays;
mod dock;
mod file;
mod fullres;
mod inspector;
#[cfg(target_os = "macos")]
mod macos_quit;
mod media;
mod panels;
mod pdf;
mod quit;
mod recovery;
mod remote;
mod render;
mod settings;
mod shortcuts;
mod style;
mod text;
mod thumbnails;
mod tools;
mod transport;

#[cfg(target_os = "macos")]
use std::env;
use std::{path::PathBuf, process::ExitCode};

use app::LumenApp;
use lumen_core::{Project, load_project, save_project};

enum Mode {
    /// Path, project and a notice for the operator (empty when there is none).
    Gui(Option<PathBuf>, Project, String),
    Smoke(Option<PathBuf>, Project),
}

/// The most recent show, so a relaunch picks up where the last service left off. A show that
/// has gone missing is skipped quietly; one that fails to load is reported and the demo opens.
fn reopen_last() -> (Option<PathBuf>, Project, String) {
    let last = settings::settings_path()
        .map(|p| settings::Settings::load(&p).settings.recent_projects)
        .and_then(|recent| recent.into_iter().find(|path| path.is_file()));
    match last {
        Some(path) => match load_project(&path) {
            Ok(project) => (Some(path), project, String::new()),
            Err(e) => (
                None,
                Project::demo(),
                format!("Couldn't reopen {}: {e}", path.display()),
            ),
        },
        None => (None, Project::demo(), String::new()),
    }
}

fn parse() -> Result<Mode, String> {
    let mut args = std::env::args_os().skip(1);
    let mut project = None;
    let mut smoke = false;
    let mut export = None;
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help") | Some("-h") => return Err("HELP".into()),
            Some("--project") => {
                let Some(path) = args.next() else {
                    return Err("--project requires PATH".into());
                };
                project = Some(PathBuf::from(path));
            }
            Some("--smoke") | Some("--smoke-test") => smoke = true,
            Some("--export-demo") => {
                let Some(path) = args.next() else {
                    return Err("--export-demo requires PATH".into());
                };
                export = Some(PathBuf::from(path));
            }
            Some(other) => return Err(format!("unknown option: {other}")),
            None => return Err("unknown option: non-UTF-8 argument".into()),
        }
    }
    if let Some(path) = export {
        if project.is_some() || smoke {
            return Err("--export-demo cannot be combined with --project or --smoke-test".into());
        }
        save_project(&path, &Project::demo()).map_err(|e| format!("could not export demo: {e}"))?;
        return Err("EXPORTED".into());
    }
    let loaded = if let Some(path) = &project {
        if !path.is_file() {
            return Err(format!("project file does not exist: {}", path.display()));
        }
        Some(
            load_project(path)
                .map_err(|e| format!("could not load project {}: {e}", path.display()))?,
        )
    } else {
        None
    };
    Ok(match (loaded, smoke) {
        (loaded, true) => Mode::Smoke(project, loaded.unwrap_or_else(Project::demo)),
        (Some(loaded), false) => Mode::Gui(project, loaded, String::new()),
        (None, false) => {
            let (path, loaded, notice) = reopen_last();
            Mode::Gui(path, loaded, notice)
        }
    })
}

/// The app icon for the Dock and window, so it shows however the app is launched (the bundle's
/// `AppIcon.icns` only applies to a packaged app).
fn app_icon() -> eframe::egui::IconData {
    let png = include_bytes!("../assets/AppIcon-512.png");
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("embedded app icon is a PNG")
        .to_rgba8();
    eframe::egui::IconData {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    }
}

fn main() -> ExitCode {
    discover_homebrew_tools();
    let mode = match parse() {
        Ok(mode) => mode,
        Err(e) if e == "HELP" => {
            println!(
                "Lumen Presenter\n\nUsage: lumen [--project PATH] [--smoke-test]\n       lumen --export-demo PATH\n\nOptions:\n  --project PATH    Open a Lumen JSON show\n  --export-demo PATH  Write the original demonstration show\n  --smoke-test      Open audience and stage output viewports\n  --help            Show this help"
            );
            return ExitCode::SUCCESS;
        }
        Err(e) if e == "EXPORTED" => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lumen: {e}\nTry --help for usage.");
            return ExitCode::FAILURE;
        }
    };
    let (path, project, smoke, notice) = match mode {
        Mode::Gui(p, project, notice) => (p, project, false, notice),
        Mode::Smoke(p, project) => (p, project, true, String::new()),
    };
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("Lumen Presenter")
            .with_inner_size([1500.0, 982.0])
            .with_min_inner_size([1100.0, 700.0])
            .with_icon(app_icon()),
        ..Default::default()
    };
    match eframe::run_native(
        "Lumen Presenter",
        native,
        Box::new(move |cc| {
            let mut app = LumenApp::new(cc, project, path, smoke);
            app.error = notice;
            Ok(Box::new(app))
        }),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("lumen: could not start native window: {e}");
            ExitCode::FAILURE
        }
    }
}

fn discover_homebrew_tools() {
    #[cfg(target_os = "macos")]
    {
        let mut paths = vec![
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
        ];
        if let Some(existing) = env::var_os("PATH") {
            paths.extend(env::split_paths(&existing));
        }
        if let Ok(path) = env::join_paths(paths) {
            // This only updates the child application's environment, before it starts worker threads.
            unsafe {
                env::set_var("PATH", path);
            }
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod backend_tests {
    #[test]
    fn metal_backend_is_compiled_for_native_macos_windows() {
        assert!(
            wgpu::Instance::enabled_backend_features().contains(wgpu::Backends::METAL),
            "the native macOS application requires wgpu's Metal backend"
        );
    }
}
