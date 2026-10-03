use crate::{edit::*, model::*};
use image::RgbaImage;
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[derive(Clone)]
pub struct Store {
    pub root: PathBuf,
}
impl Store {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        for part in ["captures", "edits", "exports", "trash"] {
            std::fs::create_dir_all(root.join(part)).map_err(err)?;
        }
        Ok(Self { root })
    }
    pub fn save_capture(
        &self,
        image: &RgbaImage,
        game: GameSnapshot,
        automatic: bool,
    ) -> Result<CaptureMeta, String> {
        if image.width() == 0 || image.height() == 0 {
            return Err(crate::i18n::text("Image vide.", "Empty image.").into());
        }
        let timestamp_ms = now_ms();
        let id = format!("{timestamp_ms}-{}", COUNTER.fetch_add(1, Ordering::Relaxed));
        let title = if game.level.is_empty() {
            crate::i18n::text("Capture libre", "Manual screenshot").into()
        } else {
            format!("{} · {:.1}%", game.level, game.percent)
        };
        let meta = CaptureMeta {
            id,
            title,
            game,
            timestamp_ms,
            width: image.width(),
            height: image.height(),
            automatic,
            favorite: false,
        };
        let path = self.image_path(&meta.id)?;
        save_png_atomic(&path, image)?;
        let thumb = image::imageops::thumbnail(image, 384, 216);
        save_png_atomic(&self.thumb_path(&meta.id)?, &thumb)?;
        self.update_meta(&meta)?;
        Ok(meta)
    }
    pub fn list(&self) -> Result<(Vec<CaptureMeta>, usize), String> {
        let mut items = Vec::new();
        let mut bad = 0;
        for entry in std::fs::read_dir(self.root.join("captures")).map_err(err)? {
            let path = entry.map_err(err)?.path();
            if path.extension().is_some_and(|e| e == "json") {
                match read_json::<CaptureMeta>(&path) {
                    Ok(m) if valid_id(&m.id) && self.image_path(&m.id)?.is_file() => items.push(m),
                    _ => bad += 1,
                }
            }
        }
        items.sort_by(|a, b| {
            b.timestamp_ms
                .cmp(&a.timestamp_ms)
                .then_with(|| b.id.cmp(&a.id))
        });
        Ok((items, bad))
    }
    pub fn image_path(&self, id: &str) -> Result<PathBuf, String> {
        checked(id)?;
        Ok(self.root.join("captures").join(format!("{id}.png")))
    }
    pub fn thumb_path(&self, id: &str) -> Result<PathBuf, String> {
        checked(id)?;
        Ok(self.root.join("captures").join(format!("{id}.thumb.png")))
    }
    pub fn update_meta(&self, meta: &CaptureMeta) -> Result<(), String> {
        checked(&meta.id)?;
        atomic_json(
            &self.root.join("captures").join(format!("{}.json", meta.id)),
            meta,
        )
    }
    pub fn load_meta(&self, id: &str) -> Result<CaptureMeta, String> {
        checked(id)?;
        let meta: CaptureMeta = read_json(&self.root.join("captures").join(format!("{id}.json")))?;
        if meta.id != id {
            return Err(crate::i18n::text(
                "Identifiant de métadonnée incohérent.",
                "Inconsistent metadata ID.",
            )
            .into());
        }
        Ok(meta)
    }
    pub fn save_document(&self, id: &str, doc: &Document) -> Result<(), String> {
        checked(id)?;
        atomic_json(&self.root.join("edits").join(format!("{id}.json")), doc)
    }
    pub fn load_document(&self, id: &str) -> Result<Document, String> {
        checked(id)?;
        let p = self.root.join("edits").join(format!("{id}.json"));
        if p.is_file() {
            read_json(&p)
        } else {
            Ok(Document::default())
        }
    }
    pub fn load_settings(&self) -> Result<Settings, String> {
        let p = self.root.join("settings.json");
        let mut settings = if p.is_file() {
            read_json(&p)?
        } else {
            Settings::default()
        };
        settings.normalize();
        Ok(settings)
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<(), String> {
        atomic_json(&self.root.join("settings.json"), settings)
    }
    pub fn trash(&self, id: &str) -> Result<(), String> {
        checked(id)?;
        let dest = self.root.join("trash").join(id);
        std::fs::create_dir_all(&dest).map_err(err)?;
        let source_meta = self.root.join("captures").join(format!("{id}.json"));
        // Move the metadata first so an interrupted move never exposes a broken gallery entry.
        for (src, name) in [
            (source_meta, "metadata.json"),
            (self.image_path(id)?, "original.png"),
            (self.thumb_path(id)?, "thumbnail.png"),
            (
                self.root.join("edits").join(format!("{id}.json")),
                "edits.json",
            ),
        ] {
            if src.exists() {
                std::fs::rename(src, dest.join(name)).map_err(err)?;
            }
        }
        Ok(())
    }
    pub fn export(&self, id: &str, doc: &Document) -> Result<PathBuf, String> {
        self.export_as(id, doc, ExportFormat::Png, 92)
    }
    pub fn export_as(
        &self,
        id: &str,
        doc: &Document,
        format: ExportFormat,
        quality: u8,
    ) -> Result<PathBuf, String> {
        let original = image::open(self.image_path(id)?).map_err(err)?.to_rgba8();
        let edited = render_edit(&original, doc)?;
        let extension = if format == ExportFormat::Png {
            "png"
        } else {
            "jpg"
        };
        let p = self.root.join("exports").join(format!(
            "{id}-edit-{}-{}.{extension}",
            now_ms(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        if format == ExportFormat::Png {
            save_png_atomic(&p, &edited)?;
        } else {
            use std::io::Write;
            let rgb = image::DynamicImage::ImageRgba8(edited).to_rgb8();
            let tmp = temp_path(&p);
            let mut file = std::fs::File::create(&tmp).map_err(err)?;
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality.clamp(40, 100))
                .encode_image(&rgb)
                .map_err(err)?;
            file.flush().map_err(err)?;
            file.sync_all().map_err(err)?;
            drop(file);
            std::fs::rename(&tmp, &p).map_err(err)?;
        }
        Ok(p)
    }
    pub fn rendered(&self, id: &str, doc: &Document) -> Result<image::RgbaImage, String> {
        render_edit(
            &image::open(self.image_path(id)?).map_err(err)?.to_rgba8(),
            doc,
        )
    }
    pub fn export_to(
        &self,
        id: &str,
        doc: &Document,
        path: &Path,
        quality: u8,
    ) -> Result<PathBuf, String> {
        let parent = path
            .parent()
            .ok_or(crate::i18n::text(
                "Dossier de destination invalide.",
                "Invalid destination folder.",
            ))?
            .canonicalize()
            .map_err(err)?;
        if !path.is_absolute()
            || ["captures", "trash"].into_iter().any(|folder| {
                self.root
                    .join(folder)
                    .canonicalize()
                    .is_ok_and(|protected| parent.starts_with(protected))
            })
        {
            return Err(crate::i18n::text(
                "Les originaux sont protégés. Choisis un autre dossier pour la copie modifiée.",
                "Originals are protected. Choose another folder for the edited copy.",
            )
            .into());
        }
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
            return Err(crate::i18n::text(
                "Choisis un fichier PNG ou JPEG.",
                "Choose a PNG or JPEG file.",
            )
            .into());
        }
        let image = self.rendered(id, doc)?;
        if extension == "png" {
            save_png_atomic(path, &image)?;
        } else {
            use std::io::Write;
            let temporary = temp_path(path);
            let mut file = std::fs::File::create(&temporary).map_err(err)?;
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut file, quality.clamp(40, 100))
                .encode_image(&image::DynamicImage::ImageRgba8(image).to_rgb8())
                .map_err(err)?;
            file.flush().map_err(err)?;
            file.sync_all().map_err(err)?;
            drop(file);
            std::fs::rename(temporary, path).map_err(err)?;
        }
        Ok(path.to_owned())
    }
}
static COUNTER: AtomicU64 = AtomicU64::new(0);
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.bytes().all(|b| b.is_ascii_digit() || b == b'-')
}
fn checked(id: &str) -> Result<(), String> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(crate::i18n::text("Identifiant de capture invalide.", "Invalid screenshot ID.").into())
    }
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn read_json<T: serde::de::DeserializeOwned>(p: &Path) -> Result<T, String> {
    serde_json::from_slice(&std::fs::read(p).map_err(err)?).map_err(err)
}
fn temp_path(p: &Path) -> PathBuf {
    p.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}
pub fn atomic_json<T: serde::Serialize>(p: &Path, value: &T) -> Result<(), String> {
    use std::io::Write;
    let bytes = serde_json::to_vec_pretty(value).map_err(err)?;
    let tmp = temp_path(p);
    let mut file = std::fs::File::create(&tmp).map_err(err)?;
    file.write_all(&bytes).map_err(err)?;
    file.sync_all().map_err(err)?;
    drop(file);
    std::fs::rename(&tmp, p).map_err(err)
}
fn save_png_atomic(p: &Path, image: &RgbaImage) -> Result<(), String> {
    let tmp = temp_path(p);
    image
        .save_with_format(&tmp, image::ImageFormat::Png)
        .map_err(err)?;
    std::fs::rename(tmp, p).map_err(err)
}
