use crate::{edit::*, model::*, store::Store};
use image::RgbaImage;
use std::sync::{
    Arc,
    mpsc::{self, Receiver, SyncSender},
};

pub enum Command {
    Capture(Arc<RgbaImage>, GameSnapshot, bool),
    Refresh,
    Thumbnail(String),
    Load(String),
    Update(CaptureMeta),
    Rename(String, String),
    ToggleFavorite(String),
    SaveDocument(String, Document),
    Export(String, Document),
    ExportAs(String, Document, ExportFormat, u8),
    ExportTo(String, Document, std::path::PathBuf, u8),
    CopyEdited(String, Document),
    Trash(String),
    Settings(Settings),
}
pub enum Event {
    Index(Vec<CaptureMeta>, usize),
    Saved(CaptureMeta, RgbaImage),
    Thumbnail(String, RgbaImage),
    Loaded(String, RgbaImage, Document),
    Updated(CaptureMeta),
    DocumentSaved(String, Document),
    Trashed(String),
    Exported(std::path::PathBuf),
    Notice(String),
    Failed(Failure, String),
}
#[derive(Debug, PartialEq)]
pub enum Failure {
    Thumbnail(String),
    Load(String),
    Capture,
    SaveDocument(String),
    Trash(String),
    Other,
}
pub fn start(store: Store) -> (SyncSender<Command>, Receiver<Event>) {
    let (tx, rx) = mpsc::sync_channel(16);
    let (out, events) = mpsc::channel();
    std::thread::Builder::new()
        .name("FRAME files".into())
        .spawn(move || {
            let mut auto_copy = store.load_settings().map(|s| s.auto_copy).unwrap_or(false);
            while let Ok(command) = rx.recv() {
                let mut after = None;
                let failure = match &command {
                    Command::Thumbnail(id) => Failure::Thumbnail(id.clone()),
                    Command::Load(id) => Failure::Load(id.clone()),
                    Command::Capture(..) => Failure::Capture,
                    Command::SaveDocument(id, _) => Failure::SaveDocument(id.clone()),
                    Command::Trash(id) => Failure::Trash(id.clone()),
                    _ => Failure::Other,
                };
                let result: Result<Event, String> = (|| {
                    Ok(match command {
                        Command::Capture(image, game, automatic) => {
                            let meta = store.save_capture(&image, game, automatic)?;
                            if auto_copy {
                                after = Some(match crate::clipboard::copy_image(&image) {
                                    Ok(()) => Event::Notice(
                                        crate::i18n::text(
                                            "Capture copiée dans le presse-papiers",
                                            "Screenshot copied to the clipboard",
                                        )
                                        .into(),
                                    ),
                                    Err(error) => Event::Failed(Failure::Other, error),
                                });
                            }
                            Event::Saved(meta, image::imageops::thumbnail(&*image, 384, 216))
                        }
                        Command::Refresh => {
                            let (items, bad) = store.list()?;
                            Event::Index(items, bad)
                        }
                        Command::Thumbnail(id) => {
                            let thumb = store.thumb_path(&id)?;
                            let image = image::open(thumb)
                                .or_else(|_| image::open(store.image_path(&id).unwrap_or_default()))
                                .map_err(|e| e.to_string())?;
                            Event::Thumbnail(id, image.thumbnail(384, 216).to_rgba8())
                        }
                        Command::Load(id) => {
                            let image = image::open(store.image_path(&id)?)
                                .map_err(|e| e.to_string())?
                                .to_rgba8();
                            let doc = store.load_document(&id)?;
                            Event::Loaded(id, image, doc)
                        }
                        Command::Update(meta) => {
                            store.update_meta(&meta)?;
                            Event::Updated(meta)
                        }
                        Command::Rename(id, title) => {
                            let mut meta = store.load_meta(&id)?;
                            meta.title = clean_title(&title).ok_or(crate::i18n::text(
                                "Le nom ne peut pas être vide.",
                                "The name cannot be empty.",
                            ))?;
                            store.update_meta(&meta)?;
                            Event::Updated(meta)
                        }
                        Command::ToggleFavorite(id) => {
                            let mut meta = store.load_meta(&id)?;
                            meta.favorite = !meta.favorite;
                            store.update_meta(&meta)?;
                            Event::Updated(meta)
                        }
                        Command::SaveDocument(id, doc) => {
                            store.save_document(&id, &doc)?;
                            Event::DocumentSaved(id, doc)
                        }
                        Command::Export(id, doc) => {
                            store.save_document(&id, &doc)?;
                            Event::Exported(store.export(&id, &doc)?)
                        }
                        Command::ExportAs(id, doc, format, quality) => {
                            store.save_document(&id, &doc)?;
                            Event::Exported(store.export_as(&id, &doc, format, quality)?)
                        }
                        Command::ExportTo(id, doc, path, quality) => {
                            Event::Exported(store.export_to(&id, &doc, &path, quality)?)
                        }
                        Command::CopyEdited(id, doc) => {
                            crate::clipboard::copy_image(&store.rendered(&id, &doc)?)?;
                            Event::Notice(
                                crate::i18n::text(
                                    "Image modifiée copiée dans le presse-papiers",
                                    "Edited image copied to the clipboard",
                                )
                                .into(),
                            )
                        }
                        Command::Trash(id) => {
                            store.trash(&id)?;
                            Event::Trashed(id)
                        }
                        Command::Settings(settings) => {
                            store.save_settings(&settings)?;
                            auto_copy = settings.auto_copy;
                            Event::Notice(
                                crate::i18n::text("Réglages enregistrés", "Settings saved").into(),
                            )
                        }
                    })
                })();
                if out
                    .send(result.unwrap_or_else(|e| Event::Failed(failure, e)))
                    .is_err()
                {
                    break;
                }
                if let Some(event) = after {
                    if out.send(event).is_err() {
                        break;
                    }
                }
            }
        })
        .expect("FRAME file worker could not start");
    let _ = tx.try_send(Command::Refresh);
    (tx, events)
}
