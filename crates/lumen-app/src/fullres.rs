//! Full-resolution textures for the logos and overlay images painted on the outputs.
//! The 320x180 thumbnail stays the placeholder until the real decode arrives.

use egui::{Context, TextureHandle, TextureOptions};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
    thread,
};

type Decoded = (PathBuf, Result<egui::ColorImage, String>);

pub struct FullImages {
    ctx: Context,
    tx: Sender<Decoded>,
    rx: Receiver<Decoded>,
    /// A path stays here after a failed decode, so a bad file is not retried every frame.
    requested: HashSet<PathBuf>,
    textures: HashMap<PathBuf, TextureHandle>,
}

impl FullImages {
    pub fn new(ctx: Context) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            ctx,
            tx,
            rx,
            requested: HashSet::new(),
            textures: HashMap::new(),
        }
    }

    /// Decodes what `wanted` newly needs and drops what it no longer lists.
    pub fn sync(&mut self, wanted: &HashSet<PathBuf>) {
        while let Ok((path, result)) = self.rx.try_recv() {
            if let (Ok(image), true) = (result, wanted.contains(&path)) {
                let texture = self.ctx.load_texture(
                    format!("lumen-full-{}", path.display()),
                    image,
                    TextureOptions::LINEAR,
                );
                self.textures.insert(path, texture);
            }
        }
        self.requested.retain(|p| wanted.contains(p));
        self.textures.retain(|p, _| wanted.contains(p));
        for path in wanted {
            if self.requested.insert(path.clone()) {
                let (tx, ctx, path) = (self.tx.clone(), self.ctx.clone(), path.clone());
                thread::spawn(move || {
                    let result = crate::media::decode_image(&path);
                    let _ = tx.send((path, result));
                    ctx.request_repaint();
                });
            }
        }
    }

    /// The thumbnails with every ready full-size texture swapped in.
    pub fn over(
        &self,
        mut images: HashMap<PathBuf, TextureHandle>,
    ) -> HashMap<PathBuf, TextureHandle> {
        images.extend(self.textures.iter().map(|(p, t)| (p.clone(), t.clone())));
        images
    }
}
