use egui::Key;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shortcut {
    Next,
    Previous,
    Trigger,
    Clear(u8),
    Blackout,
    Search,
    Undo,
    Redo,
    Save(bool),
    Open,
    New,
}

pub fn map(key: Key, command: bool, shift: bool, typing: bool) -> Option<Shortcut> {
    if typing {
        return None;
    }
    if command {
        return match key {
            Key::F => Some(Shortcut::Search),
            Key::Z => Some(if shift {
                Shortcut::Redo
            } else {
                Shortcut::Undo
            }),
            Key::S => Some(Shortcut::Save(shift)),
            Key::O => Some(Shortcut::Open),
            Key::N => Some(Shortcut::New),
            _ => None,
        };
    }
    match key {
        Key::Space | Key::ArrowRight | Key::PageDown => Some(Shortcut::Next),
        Key::ArrowLeft | Key::PageUp => Some(Shortcut::Previous),
        Key::Enter => Some(Shortcut::Trigger),
        Key::F1 => Some(Shortcut::Clear(1)),
        Key::F2 => Some(Shortcut::Clear(2)),
        Key::F3 => Some(Shortcut::Clear(3)),
        Key::F4 => Some(Shortcut::Clear(4)),
        Key::F5 => Some(Shortcut::Clear(5)),
        Key::F6 => Some(Shortcut::Clear(6)),
        Key::B => Some(Shortcut::Blackout),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mapping_matches_design_and_ignores_typing() {
        for key in [Key::Space, Key::ArrowRight, Key::PageDown] {
            assert_eq!(map(key, false, false, false), Some(Shortcut::Next));
        }
        for key in [Key::ArrowLeft, Key::PageUp] {
            assert_eq!(map(key, false, false, false), Some(Shortcut::Previous));
        }
        for (i, key) in [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                map(key, false, false, false),
                Some(Shortcut::Clear(i as u8 + 1))
            );
            assert_eq!(map(key, false, false, true), None);
        }
        assert_eq!(map(Key::Z, true, true, false), Some(Shortcut::Redo));
        assert_eq!(map(Key::Z, true, false, false), Some(Shortcut::Undo));
        assert_eq!(map(Key::F, true, false, false), Some(Shortcut::Search));
        assert_eq!(map(Key::B, true, false, false), None);
    }
}
