//! Shows that move between Macs: gathering media next to the show file and relinking it later.

use crate::Project;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// What "Save with media" will do: files to copy `(source, destination)`, and paths that did not exist.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MediaPlan {
    pub copies: Vec<(PathBuf, PathBuf)>,
    pub missing: Vec<PathBuf>,
}

/// Points every existing file the show uses at a copy inside `media_dir` (one copy per source,
/// ` 2`, ` 3`… on a name clash) and returns the copies to make. Missing files keep their path.
pub fn plan_media_copies(
    project: &mut Project,
    media_dir: &Path,
    exists: impl Fn(&Path) -> bool,
) -> MediaPlan {
    let mut plan = MediaPlan::default();
    let mut target: HashMap<PathBuf, PathBuf> = HashMap::new();
    let mut taken: HashSet<PathBuf> = HashSet::new();
    for path in project.paths_mut() {
        if path.as_os_str().is_empty() {
            continue;
        }
        if let Some(copy) = target.get(path.as_path()) {
            *path = copy.clone();
        } else if exists(path) {
            let copy = unique_name(media_dir, path, &taken);
            taken.insert(copy.clone());
            target.insert(path.clone(), copy.clone());
            plan.copies.push((path.clone(), copy.clone()));
            *path = copy;
        } else if !plan.missing.contains(path) {
            plan.missing.push(path.clone());
        }
    }
    plan
}

fn unique_name(dir: &Path, source: &Path, taken: &HashSet<PathBuf>) -> PathBuf {
    let name = source.file_name().unwrap_or_default();
    let first = dir.join(name);
    if !taken.contains(&first) {
        return first;
    }
    let stem = source.file_stem().unwrap_or_default().to_string_lossy();
    let ext = source
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    (2..)
        .map(|n| dir.join(format!("{stem} {n}{ext}")))
        .find(|candidate| !taken.contains(candidate))
        .expect("an unused name exists")
}

/// The distinct non-empty paths the show uses that are not on disk.
pub fn missing_paths(project: &mut Project) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in project.paths_mut() {
        if !path.as_os_str().is_empty() && !path.exists() && seen.insert(path.clone()) {
            out.push(path.clone());
        }
    }
    out
}

const RELINK_DEPTH: usize = 4;
/// shortcut: stops indexing after this many entries so a huge folder cannot hang the worker;
/// raise it or report "stopped early" if someone relinks from a whole drive.
const RELINK_SCAN_LIMIT: usize = 200_000;

/// Files found under a folder, by file name, for relinking.
pub struct FileIndex(HashMap<std::ffi::OsString, PathBuf>);

/// Indexes the files under `root` (depth ≤ 4, shallowest and alphabetically first wins).
/// Touches the disk, so call it off the UI thread.
pub fn index_files(root: &Path) -> FileIndex {
    FileIndex(scan_files(root))
}

/// Points each missing path at the file with the same name in `index`. Returns how many paths
/// were relinked and how many are still missing.
pub fn relink(project: &mut Project, index: &FileIndex) -> (usize, usize) {
    let (mut found, mut missing) = (0, 0);
    for path in project.paths_mut() {
        if path.as_os_str().is_empty() || path.exists() {
            continue;
        }
        match path.file_name().and_then(|n| index.0.get(n)) {
            Some(file) => {
                *path = file.clone();
                found += 1;
            }
            None => missing += 1,
        }
    }
    (found, missing)
}

fn scan_files(root: &Path) -> HashMap<std::ffi::OsString, PathBuf> {
    let mut index = HashMap::new();
    let mut level = vec![root.to_path_buf()];
    let mut scanned = 0;
    for _ in 0..=RELINK_DEPTH {
        let mut next = Vec::new();
        for dir in level {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut entries: Vec<_> = entries.flatten().collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                scanned += 1;
                if scanned > RELINK_SCAN_LIMIT {
                    return index;
                }
                let Ok(kind) = entry.file_type() else {
                    continue;
                };
                if kind.is_dir() {
                    next.push(entry.path());
                } else if kind.is_file() {
                    index
                        .entry(entry.file_name())
                        .or_insert_with(|| entry.path());
                }
            }
        }
        level = next;
    }
    index
}

/// Rewrites every path under `folder` as relative to it, for a show file stored in `folder`.
pub(crate) fn make_relative(project: &mut Project, folder: &Path) {
    let canonical = folder.canonicalize().ok();
    for path in project.paths_mut() {
        let inside = path
            .strip_prefix(folder)
            .ok()
            .or_else(|| canonical.as_deref().and_then(|c| path.strip_prefix(c).ok()))
            .filter(|rel| !rel.as_os_str().is_empty());
        if let Some(relative) = inside {
            *path = relative.to_path_buf();
        }
    }
}

/// Resolves relative paths against the folder the show file was opened from.
pub(crate) fn make_absolute(project: &mut Project, folder: &Path) {
    let base = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    for path in project.paths_mut() {
        if path.is_relative() && !path.as_os_str().is_empty() {
            *path = base.join(&*path);
        }
    }
}
