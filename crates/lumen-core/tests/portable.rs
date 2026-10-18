use lumen_core::{
    AudioCue, Background, CueAction, MediaFit, Project, Slide, SlideObject, load_project,
    portable::{index_files, missing_paths, plan_media_copies, relink},
    save_project,
};
use std::path::{Path, PathBuf};

/// One of each of the seven places a show stores a file path, all under `dir`.
fn show_with_paths(dir: &Path) -> Project {
    let mut project = Project::demo();
    let theme = project.themes[0].id;
    let file = |name: &str| dir.join(name);
    let mut first = Slide::new(theme);
    first.background = Background::Video {
        path: file("clip.mp4"),
        looped: false,
        muted: false,
        fit: MediaFit::default(),
    };
    first.audio = Some(AudioCue {
        path: file("song.mp3"),
        volume: 1.0,
    });
    first.actions = vec![CueAction::PlayDeck {
        path: file("deck.pdf"),
    }];
    first.objects = vec![SlideObject::Image {
        id: uuid::Uuid::new_v4(),
        rect: Default::default(),
        path: file("logo.png"),
    }];
    let mut second = Slide::new(theme);
    second.background = Background::Image {
        path: file("photo.png"),
        fit: MediaFit::default(),
    };
    let mut third = Slide::new(theme);
    third.background = Background::Pdf {
        path: file("deck.pdf"),
        page: 1,
    };
    let doc = &mut project.documents[0];
    doc.source = Some(file("deck.pptx"));
    doc.slides = vec![first, second, third];
    project.playlist.clear();
    project
}

#[test]
fn paths_mut_visits_every_path_field_once() {
    let mut project = show_with_paths(Path::new("/x"));
    assert_eq!(project.paths_mut().len(), 7);
}

#[test]
fn paths_inside_the_show_folder_are_saved_relative_and_load_absolute() {
    let dir = tempfile::tempdir().unwrap();
    let outside = PathBuf::from("/elsewhere/outside.png");
    let mut project = show_with_paths(dir.path());
    project.documents[0].slides[1].background = Background::Image {
        path: outside.clone(),
        fit: MediaFit::default(),
    };
    let file = dir.path().join("show.lumen.json");
    save_project(&file, &project).unwrap();

    let json = std::fs::read_to_string(&file).unwrap();
    assert!(json.contains("\"clip.mp4\"") && json.contains("\"/elsewhere/outside.png\""));
    assert!(!json.contains(dir.path().to_str().unwrap()));

    let loaded = load_project(&file).unwrap();
    let base = dir.path().canonicalize().unwrap();
    let Background::Video { path, .. } = &loaded.documents[0].slides[0].background else {
        panic!()
    };
    assert_eq!(path, &base.join("clip.mp4"));
    let Background::Image { path, .. } = &loaded.documents[0].slides[1].background else {
        panic!()
    };
    assert_eq!(path, &outside);
}

#[test]
fn a_moved_show_folder_still_resolves() {
    let dir = tempfile::tempdir().unwrap();
    let from = dir.path().join("a");
    std::fs::create_dir(&from).unwrap();
    std::fs::write(from.join("clip.mp4"), b"v").unwrap();
    save_project(&from.join("s.lumen.json"), &show_with_paths(&from)).unwrap();

    let to = dir.path().join("b");
    std::fs::rename(&from, &to).unwrap();
    let mut loaded = load_project(&to.join("s.lumen.json")).unwrap();
    let Background::Video { path, .. } = &loaded.documents[0].slides[0].background else {
        panic!()
    };
    assert!(path.exists(), "{path:?}");
    assert_eq!(missing_paths(&mut loaded).len(), 5); // deck.pdf is used twice
}

#[test]
fn old_shows_with_absolute_paths_load_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let project = show_with_paths(Path::new("/old/mac"));
    let file = dir.path().join("old.json");
    save_project(&file, &project).unwrap();
    assert_eq!(load_project(&file).unwrap(), project);
}

#[test]
fn media_collection_copies_once_names_clashes_and_lists_missing() {
    let src = tempfile::tempdir().unwrap();
    let (a, b) = (src.path().join("a"), src.path().join("b"));
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("logo.png"), b"1").unwrap();
    std::fs::write(b.join("logo.png"), b"2").unwrap();
    let mut project = show_with_paths(&a);
    let theme = project.themes[0].id;
    // a second, different file with the same name, the same file again, and a missing one
    let mut extra = Slide::new(theme);
    extra.objects = vec![
        SlideObject::Image {
            id: uuid::Uuid::new_v4(),
            rect: Default::default(),
            path: b.join("logo.png"),
        },
        SlideObject::Image {
            id: uuid::Uuid::new_v4(),
            rect: Default::default(),
            path: a.join("logo.png"),
        },
    ];
    project.documents[0].slides.push(extra);

    let media = Path::new("/out/Media");
    let plan = plan_media_copies(&mut project, media, |p| p.ends_with("logo.png"));
    assert_eq!(plan.missing.len(), 5);
    let SlideObject::Image { path, .. } = &project.documents[0].slides[3].objects[1] else {
        panic!()
    };
    assert_eq!(path, &media.join("logo.png"));
}

#[test]
fn relink_finds_missing_files_by_name_within_four_levels() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = show_with_paths(Path::new("/gone"));
    let deep = dir.path().join("1/2/3/4");
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("clip.mp4"), b"v").unwrap();
    let too_deep = dir.path().join("1/2/3/4/5");
    std::fs::create_dir_all(&too_deep).unwrap();
    std::fs::write(too_deep.join("song.mp3"), b"a").unwrap();
    std::fs::write(dir.path().join("logo.png"), b"l").unwrap();

    let (found, missing) = relink(&mut project, &index_files(dir.path()));
    assert_eq!((found, missing), (2, 5));
    let Background::Video { path, .. } = &project.documents[0].slides[0].background else {
        panic!()
    };
    assert_eq!(path, &deep.join("clip.mp4"));
}
