use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use crate::{CoreError, Project};

const MAX_PROJECT_BYTES: u64 = 16 * 1024 * 1024;

pub fn save_project(path: &Path, project: &Project) -> Result<(), CoreError> {
    project.validate()?;
    let bytes = serde_json::to_vec_pretty(project)?;
    if bytes.len() as u64 > MAX_PROJECT_BYTES {
        return Err(CoreError::ProjectTooLarge {
            actual: bytes.len() as u64,
            maximum: MAX_PROJECT_BYTES,
        });
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
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
    let project: Project = serde_json::from_slice(&bytes)?;
    project.validate()?;
    Ok(project)
}

fn io_error(operation: &str, source: std::io::Error) -> CoreError {
    CoreError::Io {
        operation: operation.into(),
        source,
    }
}
