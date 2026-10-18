use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{CoreError, Project, SCHEMA_VERSION};

const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;

fn show_folder(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Saves atomically. Paths inside the show file's folder are written relative to it so the
/// folder can move; in memory every path stays absolute (see `load_project`).
pub fn save_project(path: &Path, project: &Project) -> Result<(), CoreError> {
    project.validate()?;
    let mut stored = project.clone();
    crate::portable::make_relative(&mut stored, show_folder(path));
    let bytes = serde_json::to_vec_pretty(&stored)?;
    if bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(CoreError::ProjectTooLarge {
            actual: bytes.len() as u64,
            maximum: MAX_PROJECT_BYTES,
        });
    }
    let parent = show_folder(path);
    let mut tempfile = tempfile::NamedTempFile::new_in(parent)
        .map_err(|source| io_error("creating temporary project file", source))?;
    tempfile
        .write_all(&bytes)
        .map_err(|source| io_error("writing temporary project file", source))?;
    tempfile
        .flush()
        .map_err(|source| io_error("flushing temporary project file", source))?;
    tempfile
        .as_file()
        .sync_all()
        .map_err(|source| io_error("syncing temporary project file", source))?;
    tempfile
        .persist(path)
        .map_err(|error| io_error("atomically replacing project file", error.error))?;
    Ok(())
}

/// Loads a show; relative paths resolve against the show file's folder, absolute ones are kept.
pub fn load_project(path: &Path) -> Result<Project, CoreError> {
    let file = File::open(path).map_err(|source| io_error("opening project file", source))?;
    let size = file
        .metadata()
        .map_err(|source| io_error("reading project file metadata", source))?
        .len();
    if size > MAX_PROJECT_BYTES {
        return Err(CoreError::ProjectTooLarge {
            actual: size,
            maximum: MAX_PROJECT_BYTES,
        });
    }
    let mut bytes = Vec::with_capacity(size as usize);
    file.take(MAX_PROJECT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|source| io_error("reading project file", source))?;
    if bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(CoreError::ProjectTooLarge {
            actual: bytes.len() as u64,
            maximum: MAX_PROJECT_BYTES,
        });
    }
    let mut project: Project = serde_json::from_slice(&bytes)?;
    migrate(&mut project)?;
    project.validate()?;
    crate::portable::make_absolute(&mut project, show_folder(path));
    Ok(project)
}

/// Migrate a project from v1 to v2 in memory.
/// v1 projects get new fields as defaults and schema_version becomes 2.
/// Versions > 2 or 0 error.
fn migrate(project: &mut Project) -> Result<(), CoreError> {
    match project.schema_version {
        1 => {
            // Migrate v1 → v2: new fields already have #[serde(default)]
            // so they deserialized to defaults. Just bump the version.
            project.schema_version = SCHEMA_VERSION;
            // Seed a default countdown timer if none exist
            if project.timers.is_empty() {
                project.timers.push(crate::TimerDef {
                    id: uuid::Uuid::new_v4(),
                    name: "Countdown".into(),
                    kind: crate::TimerKind::Countdown {
                        duration_secs: 300.0,
                    },
                    allow_overrun: false,
                    warn_at_secs: None,
                });
            }
            Ok(())
        }
        v if v == SCHEMA_VERSION => Ok(()),
        other => Err(CoreError::UnsupportedSchema {
            found: other,
            expected: SCHEMA_VERSION,
        }),
    }
}

fn io_error(operation: &str, source: std::io::Error) -> CoreError {
    CoreError::Io {
        operation: operation.into(),
        source,
    }
}
