use lumen_core::Project;
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

pub struct Recovered {
    pub path: PathBuf,
    pub project: Project,
    pub time: String,
}

pub fn should_offer(recovery: Option<SystemTime>, saved: Option<SystemTime>) -> bool {
    recovery.is_some_and(|time| saved.is_none_or(|saved| time > saved))
}

pub fn recovery_path(project: Option<&Path>) -> Option<PathBuf> {
    let settings = crate::settings::settings_path()?;
    let mut hash = 0xcbf29ce484222325_u64;
    // Stable FNV-1a keeps recovery paths unchanged across launches and toolchains.
    let path = project.map(|p| p.to_string_lossy()).unwrap_or_default();
    for byte in path.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let stem = project
        .and_then(|p| p.file_stem())
        .and_then(|s| s.to_str())
        .unwrap_or("untitled");
    Some(
        settings
            .parent()?
            .join("Recovery")
            .join(format!("{stem}-{hash:016x}.json")),
    )
}

pub fn check(project: Option<&Path>) -> Result<Option<Recovered>, String> {
    let Some(path) = recovery_path(project) else {
        return Ok(None);
    };
    let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    let saved = project.and_then(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok());
    if !should_offer(modified, saved) {
        return Ok(None);
    }
    let value = lumen_core::load_project(&path)
        .map_err(|e| format!("Couldn't read recovery {}: {e}", path.display()))?;
    let time: chrono::DateTime<chrono::Local> = modified.unwrap_or(SystemTime::UNIX_EPOCH).into();
    Ok(Some(Recovered {
        path,
        project: value,
        time: time.format("%d %b %H:%M:%S").to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};
    #[test]
    fn only_strictly_newer_recovery_is_offered() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(10);
        assert!(should_offer(Some(t), None));
        assert!(!should_offer(None, Some(t)));
        assert!(!should_offer(Some(t), Some(t)));
        assert!(should_offer(Some(t + Duration::from_secs(1)), Some(t)));
        assert!(!should_offer(Some(t), Some(t + Duration::from_secs(1))));
    }
    #[test]
    fn path_hash_is_stable_and_distinguishes_same_stem() {
        let a = recovery_path(Some(std::path::Path::new("/a/show.json"))).unwrap();
        assert_eq!(
            a,
            recovery_path(Some(std::path::Path::new("/a/show.json"))).unwrap()
        );
        assert_ne!(
            a,
            recovery_path(Some(std::path::Path::new("/b/show.json"))).unwrap()
        );
        assert!(
            recovery_path(None)
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("untitled-")
        );
    }
}
