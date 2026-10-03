use crate::{
    editor_ui::Editor,
    icons::{self, Icon},
    model::*,
    native,
    store::Store,
    worker::{Command, Event, Failure},
};
use geode_egui::egui::{self, Color32, RichText, TextureHandle, Vec2};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::mpsc::{Receiver, SyncSender},
};

pub const CYAN: Color32 = Color32::from_rgb(99, 235, 221);
pub const MUTED: Color32 = Color32::from_rgb(142, 157, 175);
pub const PANEL: Color32 = Color32::from_rgb(20, 29, 41);
pub const BG: Color32 = Color32::from_rgb(12, 19, 28);
#[derive(Clone, Copy, PartialEq)]
enum Page {
    Gallery,
    Favorites,
    Auto,
    Settings,
}
pub struct Studio {
    store: Store,
    tx: SyncSender<Command>,
    events: Receiver<Event>,
    pub settings: Settings,
    open: bool,
    page: Page,
    items: Vec<CaptureMeta>,
    textures: HashMap<String, TextureHandle>,
    texture_order: VecDeque<String>,
    pending_thumbs: HashSet<String>,
    failed_thumbs: HashSet<String>,
    search: String,
    notice: Option<(String, f64, bool)>,
    preview: Option<(CaptureMeta, TextureHandle, f64)>,
    flash_at: Option<f64>,
    editor: Option<Editor>,
    loading: Option<String>,
    delete_id: Option<String>,
    rename_id: Option<String>,
    rename_text: String,
    rename_focus: bool,
    logo: Option<TextureHandle>,
    styled: bool,
    gallery_page: usize,
    settings_tab: u8,
    settings_dirty: bool,
    shortcut_listening: Option<u8>,
    shortcut_error: Option<String>,
    pending_documents: HashMap<String, crate::edit::Document>,
    inflight_documents: HashMap<String, crate::edit::Document>,
    failed_documents: HashSet<String>,
    pending_trash: HashSet<String>,
    inflight_trash: HashSet<String>,
    save_as_document: Option<(String, crate::edit::Document, u8)>,
    pending_save_as: Option<Command>,
}
impl Studio {
    pub fn new(
        store: Store,
        tx: SyncSender<Command>,
        events: Receiver<Event>,
        settings: Settings,
    ) -> Self {
        Self {
            store,
            tx,
            events,
            settings,
            open: false,
            page: Page::Gallery,
            items: Vec::new(),
            textures: HashMap::new(),
            texture_order: VecDeque::new(),
            pending_thumbs: HashSet::new(),
            failed_thumbs: HashSet::new(),
            search: String::new(),
            notice: None,
            preview: None,
            flash_at: None,
            editor: None,
            loading: None,
            delete_id: None,
            rename_id: None,
            rename_text: String::new(),
            rename_focus: false,
            logo: None,
            styled: false,
            gallery_page: 0,
            settings_tab: 0,
            settings_dirty: false,
            shortcut_listening: None,
            shortcut_error: None,
            pending_documents: HashMap::new(),
            inflight_documents: HashMap::new(),
            failed_documents: HashSet::new(),
            pending_trash: HashSet::new(),
            inflight_trash: HashSet::new(),
            save_as_document: None,
            pending_save_as: None,
        }
    }
    pub fn send(&mut self, command: Command) -> bool {
        if let Command::Trash(id) = command {
            if self.editor.as_ref().is_some_and(|editor| editor.id == id) {
                self.save_editor();
                self.editor = None;
            }
            if self.loading.as_deref() == Some(&id) {
                self.loading = None;
            }
            self.pending_trash.insert(id);
            return true;
        }
        if let Command::Load(id) = &command {
            if self.pending_trash.contains(id) {
                return false;
            }
        }
        if self.tx.try_send(command).is_err() {
            self.alert(
                crate::i18n::text(
                    "FRAME est occupé, réessaie dans un instant.",
                    "FRAME is busy. Try again in a moment.",
                ),
                true,
            );
            false
        } else {
            true
        }
    }
    pub fn alert(&mut self, text: impl Into<String>, error: bool) {
        self.notice = Some((text.into(), native::seconds(), error));
    }
    fn texture(ctx: &egui::Context, id: &str, image: &image::RgbaImage) -> TextureHandle {
        ctx.load_texture(
            id,
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        )
    }
    fn put_texture(&mut self, ctx: &egui::Context, id: String, image: &image::RgbaImage) {
        self.pending_thumbs.remove(&id);
        if !self.textures.contains_key(&id) {
            self.texture_order.push_back(id.clone());
        }
        self.textures
            .insert(id.clone(), Self::texture(ctx, &id, image));
        while self.texture_order.len() > 80 {
            if let Some(old) = self.texture_order.pop_front() {
                self.textures.remove(&old);
            }
        }
    }
    fn events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Index(items, bad) => {
                    self.pending_thumbs.clear();
                    self.failed_thumbs.clear();
                    self.items = items;
                    if bad > 0 {
                        self.alert(crate::translated_format!("{bad} fichier(s) de métadonnées ignoré(s). Les autres captures sont disponibles.", "Skipped {bad} unreadable metadata file(s). Other captures remain available."),true);
                    }
                }
                Event::Saved(meta, image) => {
                    native::finish_capture();
                    self.put_texture(ctx, meta.id.clone(), &image);
                    let tex = Self::texture(ctx, "frame-preview", &image);
                    let now = native::seconds();
                    self.preview = Some((meta.clone(), tex, now));
                    self.flash_at = Some(now);
                    if self.settings.sound {
                        native::play_shutter();
                    }
                    self.items.insert(0, meta);
                    self.alert(
                        crate::i18n::text("Capture enregistrée", "Screenshot saved"),
                        false,
                    );
                }
                Event::Thumbnail(id, image) => self.put_texture(ctx, id, &image),
                Event::Loaded(id, image, doc) => {
                    if !self.pending_trash.contains(&id) && self.loading.as_deref() == Some(&id) {
                        self.loading = None;
                        let meta = self.items.iter().find(|m| m.id == id).cloned();
                        let doc = self.pending_documents.get(&id).cloned().unwrap_or(doc);
                        self.editor = Some(Editor::new(
                            ctx,
                            id,
                            image,
                            doc,
                            meta.map(|m| m.title).unwrap_or_default(),
                        ));
                        if let Some(editor) = &mut self.editor {
                            editor.apply_defaults(&self.settings);
                        }
                    }
                }
                Event::Updated(meta) => {
                    if let Some(editor) = &mut self.editor {
                        if editor.id == meta.id {
                            editor.set_title(meta.title.clone());
                        }
                    }
                    if let Some(m) = self.items.iter_mut().find(|m| m.id == meta.id) {
                        *m = meta;
                    }
                }
                Event::Trashed(id) => {
                    self.pending_trash.remove(&id);
                    self.inflight_trash.remove(&id);
                    if self.loading.as_deref() == Some(&id) {
                        self.loading = None;
                    }
                    if self.editor.as_ref().is_some_and(|editor| editor.id == id) {
                        self.editor = None;
                    }
                    self.items.retain(|m| m.id != id);
                    self.textures.remove(&id);
                    self.pending_documents.remove(&id);
                    self.inflight_documents.remove(&id);
                    self.failed_documents.remove(&id);
                    self.alert(
                        crate::i18n::text(
                            "Capture déplacée dans la corbeille",
                            "Screenshot moved to trash",
                        ),
                        false,
                    );
                }
                Event::Exported(path) => {
                    self.alert(
                        crate::translated_format!(
                            "Image exportée : {}",
                            "Image exported: {}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        false,
                    );
                    if self.settings.open_exports {
                        if self.settings.external_app.trim().is_empty() {
                            native::open_path(path.parent().unwrap_or(&self.store.root));
                        } else if let Err(error) =
                            native::open_in_app(&self.settings.external_app, &path)
                        {
                            self.alert(error, true);
                        }
                    }
                }
                Event::Notice(s) => self.alert(s, false),
                Event::DocumentSaved(id, doc) => {
                    self.inflight_documents.remove(&id);
                    if self.pending_documents.get(&id) == Some(&doc) {
                        self.pending_documents.remove(&id);
                    }
                    self.alert(
                        crate::i18n::text("Modifications enregistrées", "Changes saved"),
                        false,
                    );
                }
                Event::Failed(failure, e) => {
                    match failure {
                        Failure::Thumbnail(id) => {
                            self.pending_thumbs.remove(&id);
                            self.failed_thumbs.insert(id);
                        }
                        Failure::Load(id) => {
                            if self.loading.as_deref() == Some(&id) {
                                self.loading = None;
                            }
                        }
                        Failure::Capture => native::finish_capture(),
                        Failure::SaveDocument(id) => {
                            self.inflight_documents.remove(&id);
                            self.failed_documents.insert(id);
                        }
                        Failure::Trash(id) => {
                            self.pending_trash.remove(&id);
                            self.inflight_trash.remove(&id);
                        }
                        Failure::Other => {}
                    }
                    self.alert(e, true);
                }
            }
        }
        if let Some(error) = native::take_error() {
            self.alert(error, true);
        }
    }
    fn style(&mut self, ctx: &egui::Context) {
        let mut style = egui::Style::default();
        style.visuals = egui::Visuals::dark();
        style.visuals.window_fill = BG;
        style.visuals.panel_fill = BG;
        style.visuals.override_text_color = Some(Color32::from_rgb(234, 241, 249));
        style.visuals.selection.bg_fill = CYAN;
        style.visuals.selection.stroke = egui::Stroke::new(1.0, BG);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 40, 55);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(40, 60, 76);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(40, 86, 90);
        style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
        style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
        style.visuals.window_corner_radius = egui::CornerRadius::same(18);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(12.0, 9.0);
        style.spacing.interact_size = egui::vec2(36.0, 34.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(15.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(27.0));
        ctx.set_style_of(egui::Theme::Dark, style);
        ctx.set_theme(egui::Theme::Dark);
        ctx.set_zoom_factor(self.settings.ui_scale);
        let logo = image::load_from_memory(include_bytes!("../assets/camera.png"))
            .unwrap()
            .to_rgba8();
        self.logo = Some(Self::texture(ctx, "frame-logo", &logo));
        self.styled = true;
    }
    pub fn draw(&mut self, ctx: &egui::Context) {
        crate::i18n::set_language(self.settings.language);
        if !self.styled {
            self.style(ctx);
        }
        self.events(ctx);
        if native::take_toggle() {
            if self.open {
                self.save_editor();
                self.commit_rename();
                self.delete_id = None;
            }
            self.open = !self.open;
        }
        if let Some(key) = native::take_recorded_key() {
            self.accept_shortcut(&key);
        }
        if let Some(Some(path)) = native::take_chosen_app() {
            self.settings.external_app = path;
            self.persist_settings();
        }
        if let Some(path) = native::take_export_path() {
            if let (Some(path), Some((id, doc, quality))) = (path, self.save_as_document.take()) {
                self.pending_save_as = Some(Command::ExportTo(id, doc, path, quality));
            }
        }
        if self.open && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.shortcut_listening.take().is_some() {
                self.shortcut_error = None;
            } else if self.editor.as_ref().is_some_and(Editor::handles_escape) {
            }
            // The editor cancels its active gesture.
            else if self.rename_id.is_some() {
                self.rename_id = None;
            } else if self.delete_id.is_some() {
                self.delete_id = None;
            } else if self.editor.is_some() {
                self.save_editor();
                self.editor = None;
            } else {
                self.open = false;
            }
        }
        geode_egui::set_input_mode(if self.open {
            geode_egui::InputMode::Blocking
        } else {
            geode_egui::InputMode::Passthrough
        });
        if self.open {
            let mut root_ui = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("frame-fullscreen"),
                egui::UiBuilder::new().max_rect(ctx.content_rect()),
            );
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(BG).inner_margin(22))
                .show(&mut root_ui, |ui| self.window(ui, ctx));
        }
        if self.open {
            self.dialogs(ctx);
        }
        self.flush_documents();
        if let Some(command) = self.pending_save_as.take() {
            match self.tx.try_send(command) {
                Ok(()) => {},
                Err(std::sync::mpsc::TrySendError::Full(command)) => self.pending_save_as = Some(command),
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => self.alert(crate::i18n::text("Le service de fichiers ne répond plus. Relance GD pour enregistrer cette copie.", "The file service stopped responding. Restart GD to save this copy."),true),
            }
        }
        self.flush_settings();
        native::keyboard_context(
            self.open && ctx.text_edit_focused(),
            self.open && self.shortcut_listening.is_some(),
        );
        self.effects(ctx);
        geode_egui::set_input_mode(if self.open {
            geode_egui::InputMode::Blocking
        } else {
            geode_egui::InputMode::Passthrough
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(50));
    }
    fn window(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            if let Some(logo) = &self.logo {
                ui.image((logo.id(), egui::vec2(46.0, 46.0)));
            }
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("FRAME")
                        .size(28.0)
                        .strong()
                        .extra_letter_spacing(3.0),
                );
                ui.label(
                    RichText::new(crate::i18n::text(
                        "Ton jeu. Tes moments.",
                        "Your game. Your moments.",
                    ))
                    .color(MUTED)
                    .size(12.0),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(crate::i18n::text("Fermer ×", "Close ×"))
                    .clicked()
                {
                    self.save_editor();
                    self.open = false;
                }
                if ui
                    .button(crate::i18n::text("Raccourcis", "Shortcuts"))
                    .on_hover_text(crate::i18n::text(
                        "Changer les touches du mod",
                        "Change the mod's keyboard shortcuts",
                    ))
                    .clicked()
                {
                    self.save_editor();
                    self.editor = None;
                    self.page = Page::Settings;
                    self.settings_tab = 3;
                }
                if ui
                    .button(crate::i18n::text("Réglages", "Settings"))
                    .clicked()
                {
                    self.save_editor();
                    self.editor = None;
                    self.page = Page::Settings;
                }
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new(crate::translated_format!(
                                "Capturer  {}",
                                "Capture  {}",
                                crate::i18n::shortcut_label(&self.settings.capture_key)
                            ))
                            .color(BG)
                            .strong(),
                        )
                        .fill(CYAN),
                    )
                    .clicked()
                {
                    native::request_capture();
                }
            });
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);
        if self.editor.is_some() {
            let mut editor = self.editor.take().unwrap();
            let result = ui
                .allocate_ui_with_layout(
                    egui::vec2(
                        ui.available_width(),
                        (ui.available_height() - 46.0).max(180.0),
                    ),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| editor.draw(ui, ctx),
                )
                .inner;
            if result.save {
                self.queue_document(editor.id.clone(), editor.history.current.clone());
            }
            if result.export {
                self.send(Command::ExportAs(
                    editor.id.clone(),
                    editor.history.current.clone(),
                    editor.export_format,
                    editor.jpeg_quality,
                ));
            }
            if result.copy {
                self.send(Command::CopyEdited(
                    editor.id.clone(),
                    editor.history.current.clone(),
                ));
            }
            if result.save_as {
                if self.save_as_document.is_none()
                    && self.pending_save_as.is_none()
                    && native::choose_export(editor.export_format, &editor.id)
                {
                    self.save_as_document = Some((
                        editor.id.clone(),
                        editor.history.current.clone(),
                        editor.jpeg_quality,
                    ));
                } else {
                    self.alert(
                        crate::i18n::text(
                            "Un enregistrement est déjà en cours.",
                            "A save is already in progress.",
                        ),
                        true,
                    );
                }
            }
            if result.close {
                self.queue_document(editor.id.clone(), editor.history.current.clone());
            } else {
                self.editor = Some(editor);
            }
        } else {
            let height = (ui.available_height() - 30.0).max(220.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(152.0, height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| self.sidebar(ui),
                    );
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_min_width((ui.available_width() - 6.0).max(190.0));
                        match self.page {
                            Page::Gallery | Page::Favorites => self.gallery(ui, ctx, height),
                            Page::Auto => self.auto_settings(ui, height),
                            Page::Settings => self.settings_ui(ui, height),
                        }
                    });
                },
            );
        }
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("LOCAL").color(CYAN).size(11.0));
            if !self.pending_documents.is_empty() {
                ui.label(RichText::new(if self.failed_documents.is_empty(){crate::i18n::text("Enregistrement des modifications…", "Saving changes…")}else{crate::i18n::text("Échec d’enregistrement · Modifications conservées en mémoire · Enregistrer pour réessayer", "Save failed · Changes kept in memory · Click Save to retry")}).color(if self.failed_documents.is_empty(){CYAN}else{Color32::from_rgb(255,149,149)}).size(12.0));
            } else if let Some((text, _, error)) = &self.notice {
                ui.label(
                    RichText::new(text)
                        .color(if *error {
                            Color32::from_rgb(255, 149, 149)
                        } else {
                            MUTED
                        })
                        .size(12.0),
                );
            } else {
                ui.label(
                    RichText::new(crate::i18n::text("PNG originaux conservés · Échap pour revenir", "Original PNGs preserved · Escape to go back"))
                        .color(MUTED)
                        .size(12.0),
                );
            }
        });
    }
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(crate::i18n::text("BIBLIOTHÈQUE", "LIBRARY"))
                .size(10.0)
                .color(MUTED),
        );
        ui.add_space(8.0);
        for (page, label) in [
            (Page::Gallery, crate::i18n::text("Galerie", "Gallery")),
            (
                Page::Favorites,
                crate::i18n::text("☆  Favoris", "☆  Favorites"),
            ),
            (Page::Auto, "Auto-capture"),
            (
                Page::Settings,
                crate::i18n::text("⚙  Réglages", "⚙  Settings"),
            ),
        ] {
            let selected = self.page == page;
            let button = egui::Button::new(RichText::new(label).color(if selected {
                CYAN
            } else {
                Color32::WHITE
            }))
            .fill(if selected {
                Color32::from_rgb(24, 55, 61)
            } else {
                BG
            });
            if ui.add_sized([148.0, 42.0], button).clicked() {
                self.page = page;
                self.gallery_page = 0;
            }
        }
        ui.add_space(22.0);
        ui.separator();
        ui.add_space(12.0);
        ui.label(
            RichText::new(crate::translated_format!(
                "{} moments",
                "{} moments",
                self.items.len()
            ))
            .strong()
            .size(21.0),
        );
        ui.label(
            RichText::new(crate::translated_format!(
                "{} favoris",
                "{} favorites",
                self.items.iter().filter(|m| m.favorite).count()
            ))
            .color(MUTED)
            .size(12.0),
        );
        ui.add_space(18.0);
        if ui
            .button(crate::i18n::text("Ouvrir le dossier", "Open folder"))
            .clicked()
        {
            native::open_path(&self.store.root);
        }
        if ui
            .button(crate::i18n::text("Actualiser", "Refresh"))
            .clicked()
        {
            self.send(Command::Refresh);
        }
        ui.add_space(18.0);
        egui::Frame::new()
            .fill(PANEL)
            .corner_radius(12)
            .inner_margin(12)
            .show(ui, |ui| {
                ui.label(
                    RichText::new(if self.settings.auto_enabled {
                        crate::i18n::text("AUTO ACTIVÉ", "AUTO ENABLED")
                    } else {
                        crate::i18n::text("AUTO EN PAUSE", "AUTO PAUSED")
                    })
                    .color(if self.settings.auto_enabled {
                        CYAN
                    } else {
                        MUTED
                    })
                    .size(10.0),
                );
                ui.label(
                    RichText::new(crate::translated_format!(
                        "À partir de {:.0}%",
                        "From {:.0}%",
                        self.settings.minimum_percent
                    ))
                    .size(13.0),
                );
                ui.label(
                    RichText::new(crate::i18n::text("À la mort du joueur", "On player death"))
                        .color(MUTED)
                        .size(11.0),
                );
            });
    }
    fn gallery(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, height: f32) {
        ui.heading(if self.page == Page::Favorites {
            crate::i18n::text("Tes favoris", "Your favorites")
        } else {
            crate::i18n::text("Tes captures", "Your screenshots")
        });
        ui.label(
            RichText::new(crate::i18n::text(
                "Ouvre un moment pour le zoomer, l’annoter et l’exporter.",
                "Open a moment to zoom, annotate and export it.",
            ))
            .color(MUTED),
        );
        ui.add_space(10.0);
        if ui
            .add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(crate::i18n::text(
                        "Rechercher un niveau ou une capture…",
                        "Search for a level or screenshot…",
                    ))
                    .desired_width(f32::INFINITY),
            )
            .changed()
        {
            self.gallery_page = 0;
        }
        ui.add_space(8.0);
        let query = self.search.to_lowercase();
        let mut filtered: Vec<CaptureMeta> = self
            .items
            .iter()
            .filter(|m| {
                !self.pending_trash.contains(&m.id)
                    && (self.page != Page::Favorites || m.favorite)
                    && (query.is_empty()
                        || m.title.to_lowercase().contains(&query)
                        || m.game.level.to_lowercase().contains(&query))
            })
            .cloned()
            .collect();
        match self.settings.gallery_sort {
            GallerySort::Newest => filtered.sort_by(|a, b| {
                b.timestamp_ms
                    .cmp(&a.timestamp_ms)
                    .then_with(|| b.id.cmp(&a.id))
            }),
            GallerySort::Oldest => filtered.sort_by(|a, b| {
                a.timestamp_ms
                    .cmp(&b.timestamp_ms)
                    .then_with(|| a.id.cmp(&b.id))
            }),
            GallerySort::Title => filtered.sort_by(|a, b| {
                a.title
                    .to_lowercase()
                    .cmp(&b.title.to_lowercase())
                    .then_with(|| b.timestamp_ms.cmp(&a.timestamp_ms))
            }),
        }
        if filtered.is_empty() {
            ui.add_space(35.0);
            ui.vertical_centered(|ui| {
                if let Some(logo) = &self.logo {
                    ui.image((logo.id(), egui::vec2(80.0, 80.0)));
                }
                ui.add_space(12.0);
                ui.label(
                    RichText::new(if self.search.is_empty() {
                        crate::i18n::text(
                            "Le prochain moment t’attend.",
                            "Your next moment is waiting.",
                        )
                    } else {
                        crate::i18n::text(
                            "Aucune capture ne correspond.",
                            "No matching screenshots.",
                        )
                    })
                    .size(21.0)
                    .strong(),
                );
                ui.label(
                    RichText::new(crate::translated_format!(
                        "Appuie sur {} dans le jeu pour commencer.",
                        "Press {} in the game to get started.",
                        crate::i18n::shortcut_label(&self.settings.capture_key)
                    ))
                    .color(MUTED),
                );
                if ui
                    .button(crate::i18n::text(
                        "Prendre une capture",
                        "Take a screenshot",
                    ))
                    .clicked()
                {
                    native::request_capture();
                }
            });
            return;
        }
        let pages = filtered.len().div_ceil(48);
        self.gallery_page = self.gallery_page.min(pages - 1);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(crate::translated_format!(
                    "{} captures",
                    "{} screenshots",
                    filtered.len()
                ))
                .small()
                .color(MUTED),
            );
            let old = self.settings.gallery_sort;
            egui::ComboBox::from_id_salt("gallery-sort")
                .selected_text(match old {
                    GallerySort::Newest => crate::i18n::text("Récentes", "Newest"),
                    GallerySort::Oldest => crate::i18n::text("Anciennes", "Oldest"),
                    GallerySort::Title => crate::i18n::text("Nom A–Z", "Name A–Z"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.settings.gallery_sort,
                        GallerySort::Newest,
                        crate::i18n::text("Récentes", "Newest"),
                    );
                    ui.selectable_value(
                        &mut self.settings.gallery_sort,
                        GallerySort::Oldest,
                        crate::i18n::text("Anciennes", "Oldest"),
                    );
                    ui.selectable_value(
                        &mut self.settings.gallery_sort,
                        GallerySort::Title,
                        crate::i18n::text("Nom A–Z", "Name A–Z"),
                    );
                });
            if old != self.settings.gallery_sort {
                self.gallery_page = 0;
                self.persist_settings();
            }
            if pages > 1 {
                if ui.button("‹").clicked() {
                    self.gallery_page = self.gallery_page.saturating_sub(1);
                }
                ui.label(format!("{}/{}", self.gallery_page + 1, pages));
                if ui.button("›").clicked() {
                    self.gallery_page = (self.gallery_page + 1).min(pages - 1);
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Slider::new(&mut self.settings.gallery_tile_size, 180.0..=360.0)
                            .text(crate::i18n::text("Vignettes", "Thumbnails"))
                            .show_value(false),
                    )
                    .changed()
                {
                    self.persist_settings();
                }
            });
        });
        let available = (ui.available_width() - 22.0).max(170.0);
        let columns = ((available / self.settings.gallery_tile_size).floor() as usize).clamp(1, 8);
        let width = (available - 12.0 * (columns - 1) as f32) / columns as f32;
        egui::ScrollArea::vertical()
            .max_height((height - 160.0).max(120.0))
            .show(ui, |ui| {
                egui::Grid::new("frame-grid")
                    .num_columns(columns)
                    .spacing([12.0, 14.0])
                    .show(ui, |ui| {
                        for (n, meta) in filtered
                            .iter()
                            .skip(self.gallery_page * 48)
                            .take(48)
                            .enumerate()
                        {
                            if !self.failed_thumbs.contains(&meta.id)
                                && !self.textures.contains_key(&meta.id)
                                && !self.pending_thumbs.contains(&meta.id)
                            {
                                if self
                                    .tx
                                    .try_send(Command::Thumbnail(meta.id.clone()))
                                    .is_ok()
                                {
                                    self.pending_thumbs.insert(meta.id.clone());
                                }
                            }
                            egui::Frame::new()
                                .fill(PANEL)
                                .corner_radius(12)
                                .inner_margin(9)
                                .stroke(egui::Stroke::new(1.0, Color32::from_rgb(36, 49, 65)))
                                .show(ui, |ui| {
                                    ui.with_layout(
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width((width - 18.0).max(100.0));
                                            let (rect, response) = ui.allocate_exact_size(
                                                egui::vec2(
                                                    (width - 18.0).max(100.0),
                                                    (width - 18.0) * 9.0 / 16.0,
                                                ),
                                                egui::Sense::click(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                8,
                                                Color32::from_rgb(6, 12, 19),
                                            );
                                            if let Some(tex) = self.textures.get(&meta.id) {
                                                let ratio = tex.size_vec2();
                                                let scale = (rect.width() / ratio.x)
                                                    .min(rect.height() / ratio.y);
                                                let image_rect = egui::Rect::from_center_size(
                                                    rect.center(),
                                                    ratio * scale,
                                                );
                                                ui.painter().image(
                                                    tex.id(),
                                                    image_rect,
                                                    egui::Rect::from_min_max(
                                                        egui::Pos2::ZERO,
                                                        egui::pos2(1.0, 1.0),
                                                    ),
                                                    Color32::WHITE,
                                                );
                                            } else {
                                                ui.painter().text(
                                                    rect.center(),
                                                    egui::Align2::CENTER_CENTER,
                                                    if self.failed_thumbs.contains(&meta.id) {
                                                        crate::i18n::text(
                                                            "Image indisponible",
                                                            "Image unavailable",
                                                        )
                                                    } else {
                                                        crate::i18n::text("Chargement…", "Loading…")
                                                    },
                                                    egui::FontId::proportional(12.0),
                                                    MUTED,
                                                );
                                            }
                                            if response.hovered() {
                                                ui.painter().rect_stroke(
                                                    rect,
                                                    8,
                                                    egui::Stroke::new(1.5, CYAN),
                                                    egui::StrokeKind::Inside,
                                                );
                                            }
                                            if response.clicked() {
                                                if self.send(Command::Load(meta.id.clone())) {
                                                    self.loading = Some(meta.id.clone());
                                                }
                                            }
                                            if self.rename_id.as_deref() == Some(meta.id.as_str()) {
                                                let mut output = egui::TextEdit::singleline(
                                                    &mut self.rename_text,
                                                )
                                                .id_salt((&meta.id, "rename"))
                                                .char_limit(120)
                                                .desired_width(f32::INFINITY)
                                                .show(ui);
                                                if self.rename_focus {
                                                    output.response.request_focus();
                                                    output.state.cursor.set_char_range(Some(
                                                        egui::text::CCursorRange::two(
                                                            egui::text::CCursor::new(0),
                                                            egui::text::CCursor::new(
                                                                self.rename_text.chars().count(),
                                                            ),
                                                        ),
                                                    ));
                                                    output.state.store(ctx, output.response.id);
                                                    self.rename_focus = false;
                                                }
                                                let enter =
                                                    ctx.input(|i| i.key_pressed(egui::Key::Enter));
                                                if enter
                                                    && (output.response.has_focus()
                                                        || output.response.lost_focus())
                                                {
                                                    self.commit_rename();
                                                }
                                            } else {
                                                if ui
                                                    .add(
                                                        egui::Label::new(
                                                            RichText::new(&meta.title)
                                                                .strong()
                                                                .size(14.0),
                                                        )
                                                        .truncate()
                                                        .sense(egui::Sense::click()),
                                                    )
                                                    .on_hover_text(crate::i18n::text(
                                                        "Double-clique pour renommer",
                                                        "Double-click to rename",
                                                    ))
                                                    .double_clicked()
                                                {
                                                    self.begin_rename(meta);
                                                }
                                            }
                                            if self.settings.show_capture_details {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} × {} · {}",
                                                        meta.width,
                                                        meta.height,
                                                        if meta.automatic {
                                                            "AUTO"
                                                        } else {
                                                            crate::i18n::text("MANUEL", "MANUAL")
                                                        }
                                                    ))
                                                    .size(11.0)
                                                    .color(MUTED),
                                                );
                                            }
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 5.0;
                                                if icons::button(
                                                    ui,
                                                    Icon::Star,
                                                    crate::i18n::text("Favori", "Favorite"),
                                                    meta.favorite,
                                                )
                                                .clicked()
                                                {
                                                    if self.send(Command::ToggleFavorite(
                                                        meta.id.clone(),
                                                    )) {
                                                        if let Some(m) = self
                                                            .items
                                                            .iter_mut()
                                                            .find(|m| m.id == meta.id)
                                                        {
                                                            m.favorite = !m.favorite;
                                                        }
                                                    }
                                                }
                                                if self.rename_id.as_deref()
                                                    == Some(meta.id.as_str())
                                                {
                                                    if icons::button(
                                                        ui,
                                                        Icon::Check,
                                                        crate::i18n::text(
                                                            "Valider le nom · Entrée",
                                                            "Confirm name · Enter",
                                                        ),
                                                        false,
                                                    )
                                                    .clicked()
                                                    {
                                                        self.commit_rename();
                                                    }
                                                    if icons::button(
                                                        ui,
                                                        Icon::Close,
                                                        crate::i18n::text(
                                                            "Annuler le renommage · Échap",
                                                            "Cancel rename · Escape",
                                                        ),
                                                        false,
                                                    )
                                                    .clicked()
                                                    {
                                                        self.rename_id = None;
                                                    }
                                                } else if icons::button(
                                                    ui,
                                                    Icon::Pencil,
                                                    crate::i18n::text("Renommer", "Rename"),
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    self.begin_rename(meta);
                                                }
                                                if icons::button(
                                                    ui,
                                                    Icon::Trash,
                                                    crate::i18n::text(
                                                        "Déplacer dans la corbeille",
                                                        "Move to trash",
                                                    ),
                                                    false,
                                                )
                                                .clicked()
                                                {
                                                    if self.settings.confirm_trash {
                                                        self.delete_id = Some(meta.id.clone());
                                                    } else {
                                                        self.send(Command::Trash(meta.id.clone()));
                                                    }
                                                }
                                            });
                                        },
                                    );
                                });
                            if (n + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });
            });
        if self.loading.is_some() {
            ui.label(
                RichText::new(crate::i18n::text(
                    "Ouverture de l’éditeur…",
                    "Opening the editor…",
                ))
                .color(CYAN),
            );
        }
        let _ = ctx;
    }
    fn auto_settings(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading(crate::i18n::text(
            "Ne rate plus ce moment.",
            "Never miss that moment.",
        ));
        ui.label(
            RichText::new(crate::i18n::text(
                "Une capture automatique quand tu meurs au bon pourcentage.",
                "An automatic screenshot when you die at your chosen percentage.",
            ))
            .color(MUTED),
        );
        egui::ScrollArea::vertical().max_height(height-60.0).show(ui,|ui|{
            ui.add_space(16.0);let mut changed=ui.checkbox(&mut self.settings.auto_enabled,crate::i18n::text("Activer les captures à la mort", "Enable screenshots on death")).changed();
            ui.add_space(12.0);changed|=ui.checkbox(&mut self.settings.range_mode,crate::i18n::text("Limiter à une plage de pourcentages", "Limit to a percentage range")).changed();
            changed|=ui.add(egui::Slider::new(&mut self.settings.minimum_percent,0.0..=100.0).text(crate::i18n::text("Pourcentage minimum", "Minimum percentage")).suffix(" %").step_by(1.0)).changed();
            if self.settings.range_mode { self.settings.maximum_percent = self.settings.maximum_percent.max(self.settings.minimum_percent); }
            if self.settings.range_mode{changed|=ui.add(egui::Slider::new(&mut self.settings.maximum_percent,self.settings.minimum_percent..=100.0).text(crate::i18n::text("Pourcentage maximum", "Maximum percentage")).suffix(" %").step_by(1.0)).changed();}
            changed|=ui.add(egui::Slider::new(&mut self.settings.cooldown_seconds,0.0..=30.0).text(crate::i18n::text("Délai entre captures", "Time between screenshots")).suffix(" s")).changed();
            changed|=ui.checkbox(&mut self.settings.include_practice,crate::i18n::text("Inclure le mode entraînement", "Include practice mode")).changed();
            ui.add_space(18.0);egui::Frame::new().fill(PANEL).corner_radius(12).inner_margin(18).show(ui,|ui|{
                ui.label(RichText::new(crate::i18n::text("UNE MORT, UNE IMAGE", "ONE DEATH, ONE IMAGE")).color(CYAN).strong());
                ui.label(crate::i18n::text("Le mode dual ne crée pas de doublons. Une seule capture est prise après confirmation de la mort, au prochain rendu du jeu.", "Dual mode does not create duplicates. One screenshot is taken after death is confirmed, on the next game render."));
                ui.label(RichText::new(crate::i18n::text("Aucune image n’est capturée en continu pendant la partie. Les effets de mort peuvent apparaître sur la capture.", "No images are captured continuously during gameplay. Death effects may appear in the screenshot.")).color(MUTED).small());
            });
            if changed{self.persist_settings();}
        });
    }
    fn settings_ui(&mut self, ui: &mut egui::Ui, height: f32) {
        ui.heading(crate::i18n::text("À ta façon.", "Make it yours."));
        ui.label(
            RichText::new(crate::i18n::text(
                "Chaque détail se règle ici. Les changements sont enregistrés automatiquement.",
                "Adjust every detail here. Changes are saved automatically.",
            ))
            .color(MUTED),
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in [
                (
                    0,
                    crate::i18n::text("Capture et aperçu", "Capture and preview"),
                ),
                (1, crate::i18n::text("Éditeur", "Editor")),
                (
                    2,
                    crate::i18n::text("Galerie et export", "Gallery and export"),
                ),
                (3, crate::i18n::text("Raccourcis", "Shortcuts")),
                (
                    4,
                    crate::i18n::text("Interface et fichiers", "Interface and files"),
                ),
            ] {
                if ui
                    .selectable_value(&mut self.settings_tab, tab, label)
                    .clicked()
                {
                    self.shortcut_listening = None;
                }
            }
        });
        ui.separator();
        let mut changed = false;
        egui::ScrollArea::vertical().id_salt("settings-content").max_height((height-130.0).max(100.0)).show(ui,|ui| {
            ui.add_space(14.0);
            match self.settings_tab {
                0=>{
                    ui.label(RichText::new(crate::i18n::text("EFFETS DE CAPTURE", "CAPTURE EFFECTS")).color(CYAN).small().strong());
                    changed|=ui.checkbox(&mut self.settings.flash,crate::i18n::text("Petit flash blanc", "Small white flash")).changed();
                    ui.add_enabled_ui(self.settings.flash,|ui| {
                        changed|=ui.add(egui::Slider::new(&mut self.settings.flash_strength,0.0..=0.5).text(crate::i18n::text("Intensité", "Intensity"))).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.settings.flash_seconds,0.05..=1.0).text(crate::i18n::text("Durée du flash", "Flash duration")).suffix(" s")).changed();
                    });
                    changed|=ui.checkbox(&mut self.settings.sound,crate::i18n::text("Son d’obturateur original", "Original shutter sound")).changed();
                    changed|=ui.checkbox(&mut self.settings.auto_copy,crate::i18n::text("Copier chaque capture dans le presse-papiers", "Copy every screenshot to the clipboard")).changed();
                    ui.label(RichText::new(crate::i18n::text("Captures manuelles et automatiques. L’option est désactivée au départ.", "Manual and automatic screenshots. Disabled by default.")).small().color(MUTED));
                    if ui.button(crate::i18n::text("Écouter le son", "Preview sound")).clicked() { native::play_shutter(); }
                    ui.add_space(20.0);ui.label(RichText::new(crate::i18n::text("APERÇU", "PREVIEW")).color(CYAN).small().strong());
                    changed|=ui.checkbox(&mut self.settings.show_preview,crate::i18n::text("Afficher l’aperçu après une capture", "Show a preview after capture")).changed();
                    ui.add_enabled_ui(self.settings.show_preview,|ui| {
                        changed|=ui.add(egui::Slider::new(&mut self.settings.preview_seconds,0.5..=10.0).text(crate::i18n::text("Durée", "Duration")).suffix(" s")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.settings.preview_width,140.0..=420.0).text(crate::i18n::text("Largeur", "Width")).suffix(" px")).changed();
                        changed|=ui.add(egui::Slider::new(&mut self.settings.preview_border,0.0..=6.0).text(crate::i18n::text("Contour blanc", "White border")).suffix(" px")).changed();
                        ui.horizontal(|ui| { ui.label("Position");
                            egui::ComboBox::from_id_salt("preview-corner").selected_text(match self.settings.preview_corner {PreviewCorner::TopLeft=>crate::i18n::text("Haut gauche", "Top left"),PreviewCorner::TopRight=>crate::i18n::text("Haut droite", "Top right"),PreviewCorner::BottomLeft=>crate::i18n::text("Bas gauche", "Bottom left"),PreviewCorner::BottomRight=>crate::i18n::text("Bas droite", "Bottom right")}).show_ui(ui,|ui| {
                                for (value,label) in [(PreviewCorner::TopLeft,crate::i18n::text("Haut gauche", "Top left")),(PreviewCorner::TopRight,crate::i18n::text("Haut droite", "Top right")),(PreviewCorner::BottomLeft,crate::i18n::text("Bas gauche", "Bottom left")),(PreviewCorner::BottomRight,crate::i18n::text("Bas droite", "Bottom right"))] { changed|=ui.selectable_value(&mut self.settings.preview_corner,value,label).changed(); }
                            });
                        });
                    });
                }
                1=>{
                    ui.label(RichText::new(crate::i18n::text("VALEURS À L’OUVERTURE D’UNE CAPTURE", "DEFAULTS WHEN OPENING A SCREENSHOT")).color(CYAN).small().strong());
                    ui.horizontal(|ui| { ui.label(crate::i18n::text("Couleur des annotations", "Annotation color"));changed|=ui.color_edit_button_srgb(&mut self.settings.editor_color).changed(); });
                    changed|=ui.add(egui::Slider::new(&mut self.settings.editor_width,1.0..=64.0).text(crate::i18n::text("Épaisseur par défaut", "Default stroke width")).suffix(" px")).changed();
                    changed|=ui.add(egui::Slider::new(&mut self.settings.editor_text_size,8.0..=256.0).text(crate::i18n::text("Taille du texte", "Text size")).suffix(" px")).changed();
                    changed|=ui.checkbox(&mut self.settings.editor_grid,crate::i18n::text("Afficher la grille des tiers par défaut", "Show the rule of thirds grid by default")).changed();
                    ui.add_space(18.0);
                    egui::Frame::new().fill(PANEL).corner_radius(12).inner_margin(18).show(ui,|ui| {
                        ui.label(RichText::new(crate::i18n::text("TON ÉDITEUR", "YOUR EDITOR")).strong().color(CYAN));
                        ui.label(crate::i18n::text("Sélectionne, déplace et redimensionne tes annotations. Le panneau de propriétés modifie leur couleur, épaisseur, remplissage ou texte.", "Select, move and resize your annotations. The properties panel adjusts color, stroke width, fill and text."));
                        ui.label(crate::i18n::text("Les calques se dupliquent et changent d’ordre. Le recadrage propose 1:1, 16:9, 4:3 et 9:16, avec des dimensions précises.", "Duplicate and reorder layers. Crop to 1:1, 16:9, 4:3 or 9:16, or enter exact dimensions."));
                        ui.label(RichText::new(crate::i18n::text("Ctrl+Z / Ctrl+Y : annuler / rétablir · Ctrl+D : dupliquer · Suppr : retirer\nFlèches : déplacer d’un pixel · Maj+flèches : dix pixels", "Ctrl+Z / Ctrl+Y: undo / redo · Ctrl+D: duplicate · Delete: remove\nArrow keys: move one pixel · Shift+arrows: ten pixels")).small().color(MUTED));
                    });
                }
                2=>{
                    ui.label(RichText::new("EXPORT").color(CYAN).small().strong());
                    ui.horizontal(|ui| { ui.label(crate::i18n::text("Format par défaut", "Default format"));changed|=ui.selectable_value(&mut self.settings.export_format,ExportFormat::Png,"PNG").changed();changed|=ui.selectable_value(&mut self.settings.export_format,ExportFormat::Jpeg,"JPEG").changed(); });
                    ui.label(RichText::new(crate::i18n::text("PNG conserve tous les détails. JPEG produit un fichier plus léger.", "PNG preserves all details. JPEG creates a smaller file.")).small().color(MUTED));
                    changed|=ui.add(egui::Slider::new(&mut self.settings.jpeg_quality,40..=100).text(crate::i18n::text("Qualité JPEG", "JPEG quality"))).changed();
                    changed|=ui.checkbox(&mut self.settings.open_exports,crate::i18n::text("Ouvrir automatiquement après un export", "Open automatically after export")).changed();
                    ui.add_space(12.0);ui.label(crate::i18n::text("Application externe (facultative)", "External application (optional)"));
                    changed|=ui.add(egui::TextEdit::singleline(&mut self.settings.external_app).desired_width(480.0).hint_text(crate::i18n::text("Chemin complet de ton éditeur .exe", "Full path to your editor's .exe"))).changed();
                    ui.horizontal(|ui| {if ui.button(crate::i18n::text("Choisir une application…", "Choose an application…")).clicked(){native::choose_app();}if ui.button(crate::i18n::text("Utiliser le dossier", "Use the folder")).clicked(){self.settings.external_app.clear();changed=true;}});
                    ui.label(RichText::new(crate::i18n::text("Sans application, FRAME ouvre le dossier. Avec une application, il lui transmet la copie exportée.", "Without an application, FRAME opens the folder. With one selected, it opens the exported copy in that application.")).small().color(MUTED));
                    ui.add_space(20.0);ui.label(RichText::new(crate::i18n::text("GALERIE", "GALLERY")).color(CYAN).small().strong());
                    changed|=ui.add(egui::Slider::new(&mut self.settings.gallery_tile_size,180.0..=360.0).text(crate::i18n::text("Taille des vignettes", "Thumbnail size"))).changed();
                    changed|=ui.checkbox(&mut self.settings.show_capture_details,crate::i18n::text("Afficher dimensions et type de capture", "Show dimensions and capture type")).changed();
                    changed|=ui.checkbox(&mut self.settings.confirm_trash,crate::i18n::text("Confirmer le déplacement dans la corbeille", "Confirm moving screenshots to trash")).changed();
                    ui.label(RichText::new(crate::i18n::text("Crayon ou double-clic sur le titre : renommer. Entrée valide, Échap annule.\nLes originaux restent protégés ; les fichiers retirés restent dans la corbeille locale.", "Pencil or double-click the title to rename. Enter confirms, Escape cancels.\nOriginals stay protected; removed files remain in the local trash folder.")).color(MUTED));
                }
                3=>{
                    ui.label(RichText::new(crate::i18n::text("RACCOURCIS", "SHORTCUTS")).color(CYAN).small().strong());
                    ui.label(crate::i18n::text("Clique sur Changer, puis presse la touche ou la combinaison que tu veux.", "Click Change, then press your preferred key or combination."));ui.add_space(18.0);
                    for (index,label,key) in [(0,crate::i18n::text("Prendre une capture", "Take a screenshot"),self.settings.capture_key.clone()),(1,crate::i18n::text("Ouvrir / fermer FRAME", "Open / close FRAME"),self.settings.studio_key.clone())] {
                        egui::Frame::new().fill(PANEL).corner_radius(10).inner_margin(16).show(ui,|ui| {
                            ui.horizontal(|ui| {ui.set_min_width(500.0);ui.label(RichText::new(label).strong());ui.add_space(20.0);ui.label(RichText::new(crate::i18n::shortcut_label(&key)).monospace().size(20.0).color(CYAN));
                                if ui.button(if self.shortcut_listening==Some(index){crate::i18n::text("En attente…", "Listening…")}else{crate::i18n::text("Changer…", "Change…")}).clicked(){self.shortcut_listening=Some(index);self.shortcut_error=None;}
                            });
                            if self.shortcut_listening==Some(index){ui.label(RichText::new(crate::i18n::text("Presse une touche · Ctrl / Maj / Alt possibles · Échap annule", "Press a key · Ctrl / Shift / Alt supported · Escape cancels")).color(CYAN));if ui.small_button(crate::i18n::text("Annuler", "Cancel")).clicked(){self.shortcut_listening=None;}}
                        });ui.add_space(12.0);
                    }
                    if let Some(error)=&self.shortcut_error {ui.colored_label(Color32::from_rgb(255,149,149),error);}
                    ui.label(RichText::new(crate::i18n::text("Lettres, chiffres, F1–F12, Espace, flèches et touches de navigation. Les deux actions doivent avoir des combinaisons différentes.", "Letters, digits, F1–F12, Space, arrows and navigation keys. The two actions must use different combinations.")).color(MUTED));
                    ui.label(RichText::new(crate::i18n::text("Les raccourcis sont suspendus quand tu écris dans un champ. Choisis des touches libres dans tes autres mods et dans le jeu.", "Shortcuts are suspended while typing in a text field. Choose keys that are free in the game and your other mods.")).small().color(MUTED));
                    if ui.button(crate::i18n::text("Rétablir F8 / F6", "Restore F8 / F6")).clicked(){self.settings.capture_key="F8".into();self.settings.studio_key="F6".into();self.shortcut_listening=None;self.shortcut_error=None;changed=true;}
                    ui.add_space(24.0);ui.label(RichText::new(crate::i18n::text("DANS L’ÉDITEUR", "IN THE EDITOR")).color(CYAN).strong());
                    ui.label(crate::i18n::text("Ctrl+Z : annuler · Ctrl+Y : rétablir · Ctrl+D : dupliquer\nSuppr : supprimer · Flèches : déplacer · Maj+flèches : déplacer de 10 px\nDouble-clic : modifier le texte · Maj pendant un tracé : forme carrée\nEntrée : valider le texte · Maj+Entrée : nouvelle ligne · Échap : annuler le geste", "Ctrl+Z: undo · Ctrl+Y: redo · Ctrl+D: duplicate\nDelete: remove · Arrows: move · Shift+arrows: move by 10 px\nDouble-click: edit text · Shift while drawing: square shape\nEnter: confirm text · Shift+Enter: new line · Escape: cancel the gesture"));
                }
                _=>{
                    ui.horizontal(|ui|{ui.label("Langue / Language");changed|=ui.selectable_value(&mut self.settings.language,crate::i18n::Language::French,"Français").changed();changed|=ui.selectable_value(&mut self.settings.language,crate::i18n::Language::English,"English").changed();});
                    ui.add_space(16.0);
                    changed|=ui.add(egui::Slider::new(&mut self.settings.ui_scale,0.7..=1.5).text(crate::i18n::text("Taille de l’interface", "Interface scale"))).changed();
                    ui.add_space(20.0);ui.label(RichText::new(crate::i18n::text("FICHIERS LOCAUX", "LOCAL FILES")).color(CYAN).small().strong());
                    ui.label(RichText::new(self.store.root.display().to_string()).small().color(MUTED));
                    ui.horizontal(|ui| {
                        if ui.button(crate::i18n::text("Captures", "Screenshots")).clicked() { native::open_path(&self.store.root.join("captures")); }
                        if ui.button("Exports").clicked() { native::open_path(&self.store.root.join("exports")); }
                        if ui.button(crate::i18n::text("Corbeille", "Trash")).clicked() { native::open_path(&self.store.root.join("trash")); }
                    });
                    ui.add_space(20.0);
                    if ui.button(crate::i18n::text("Rétablir les réglages par défaut", "Restore default settings")).clicked() { self.settings=Settings::default();changed=true; }
                }
            }
        });
        if changed {
            self.persist_settings();
            ui.ctx().set_zoom_factor(self.settings.ui_scale);
        }
    }
    fn persist_settings(&mut self) {
        self.settings.normalize();
        native::set_settings(self.settings.clone());
        self.settings_dirty = true;
    }
    fn accept_shortcut(&mut self, key: &str) {
        let Some(index) = self.shortcut_listening else {
            return;
        };
        let Some(shortcut) = Shortcut::parse(key) else {
            self.shortcut_error=Some(crate::i18n::text("Cette touche ne peut pas être utilisée. Essaie une lettre, F1–F12 ou une combinaison.", "This key is not supported. Try a letter, F1–F12 or a combination.").into());
            return;
        };
        let key = shortcut.label();
        let other = if index == 0 {
            &self.settings.studio_key
        } else {
            &self.settings.capture_key
        };
        if &key == other {
            self.shortcut_error=Some(crate::i18n::text("Cette combinaison est déjà utilisée par l’autre action. Choisis-en une différente.", "The other action already uses this combination. Choose a different one.").into());
            return;
        }
        if index == 0 {
            self.settings.capture_key = key;
        } else {
            self.settings.studio_key = key;
        }
        self.shortcut_listening = None;
        self.shortcut_error = None;
        self.persist_settings();
    }
    fn flush_settings(&mut self) {
        if self.settings_dirty
            && self
                .tx
                .try_send(Command::Settings(self.settings.clone()))
                .is_ok()
        {
            self.settings_dirty = false;
        }
    }
    fn save_editor(&mut self) {
        if let Some(editor) = &mut self.editor {
            editor.settle();
            let id = editor.id.clone();
            let doc = editor.history.current.clone();
            self.queue_document(id, doc);
        }
    }
    fn queue_document(&mut self, id: String, doc: crate::edit::Document) {
        self.failed_documents.remove(&id);
        self.pending_documents.insert(id, doc);
    }
    fn flush_documents(&mut self) {
        for (id, doc) in &self.pending_documents {
            if !self.inflight_documents.contains_key(id) && !self.failed_documents.contains(id) {
                if self
                    .tx
                    .try_send(Command::SaveDocument(id.clone(), doc.clone()))
                    .is_err()
                {
                    break;
                }
                self.inflight_documents.insert(id.clone(), doc.clone());
            }
        }
        let ready: Vec<String> = self
            .pending_trash
            .iter()
            .filter(|id| {
                !self.pending_documents.contains_key(*id) && !self.inflight_trash.contains(*id)
            })
            .cloned()
            .collect();
        for id in ready {
            if self.tx.try_send(Command::Trash(id.clone())).is_err() {
                break;
            }
            self.inflight_trash.insert(id);
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.delete_id.clone() {
            egui::Window::new(crate::i18n::text("Déplacer dans la corbeille ?", "Move to trash?")).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,Vec2::ZERO).show(ctx,|ui|{
            ui.label(crate::i18n::text("L’original et ses annotations restent récupérables dans le dossier Corbeille.", "The original and its annotations can still be recovered from the Trash folder."));ui.horizontal(|ui|{if ui.button(crate::i18n::text("Annuler", "Cancel")).clicked(){self.delete_id=None;}if ui.button(crate::i18n::text("Déplacer", "Move")).clicked(){self.send(Command::Trash(id));self.delete_id=None;}});
        });
        }
    }
    fn begin_rename(&mut self, meta: &CaptureMeta) {
        if self.rename_id.is_some() {
            self.commit_rename();
            if self.rename_id.is_some() {
                return;
            }
        }
        self.rename_id = Some(meta.id.clone());
        self.rename_text = meta.title.clone();
        self.rename_focus = true;
    }
    fn commit_rename(&mut self) {
        let Some(id) = self.rename_id.clone() else {
            return;
        };
        let Some(title) = clean_title(&self.rename_text) else {
            self.alert(
                crate::i18n::text(
                    "Le nom doit contenir au moins un caractère.",
                    "The name must contain at least one character.",
                ),
                true,
            );
            return;
        };
        if let Some(meta) = self.items.iter().find(|m| m.id == id).cloned() {
            if title == meta.title {
                self.rename_id = None;
                return;
            }
            if self.send(Command::Rename(id.clone(), title.clone())) {
                if let Some(m) = self.items.iter_mut().find(|m| m.id == id) {
                    m.title = title;
                }
                self.rename_id = None;
            }
        } else {
            self.rename_id = None;
        }
    }
    fn effects(&mut self, ctx: &egui::Context) {
        let now = native::seconds();
        if let Some(at) = self.flash_at {
            let dt = now - at;
            if dt < self.settings.flash_seconds as f64 && self.settings.flash {
                let alpha = ((1.0 - dt / self.settings.flash_seconds as f64)
                    * self.settings.flash_strength as f64
                    * 255.0) as u8;
                ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("frame-flash"),
                ))
                .rect_filled(
                    ctx.content_rect(),
                    0,
                    Color32::from_white_alpha(alpha),
                );
            } else if dt >= self.settings.flash_seconds as f64 {
                self.flash_at = None;
            }
        }
        if let Some((meta, tex, at)) = &self.preview {
            let dt = now - *at;
            if self.settings.show_preview && dt < self.settings.preview_seconds as f64 {
                let enter = (dt / 0.22).clamp(0.0, 1.0) as f32;
                let offset = 30.0 * (1.0 - enter).powi(3);
                egui::Area::new(egui::Id::new("frame-preview"))
                    .interactable(self.open)
                    .order(egui::Order::Foreground)
                    .anchor(
                        match self.settings.preview_corner {
                            PreviewCorner::TopLeft => egui::Align2::LEFT_TOP,
                            PreviewCorner::TopRight => egui::Align2::RIGHT_TOP,
                            PreviewCorner::BottomLeft => egui::Align2::LEFT_BOTTOM,
                            PreviewCorner::BottomRight => egui::Align2::RIGHT_BOTTOM,
                        },
                        egui::vec2(
                            if matches!(
                                self.settings.preview_corner,
                                PreviewCorner::TopLeft | PreviewCorner::BottomLeft
                            ) {
                                22.0 - offset
                            } else {
                                -22.0 + offset
                            },
                            if matches!(
                                self.settings.preview_corner,
                                PreviewCorner::TopLeft | PreviewCorner::TopRight
                            ) {
                                22.0
                            } else {
                                -22.0
                            },
                        ),
                    )
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(BG)
                            .stroke(egui::Stroke::new(
                                self.settings.preview_border,
                                Color32::WHITE,
                            ))
                            .corner_radius(10)
                            .inner_margin(6)
                            .show(ui, |ui| {
                                let response = ui.add(
                                    egui::Image::new((
                                        tex.id(),
                                        egui::vec2(
                                            self.settings.preview_width,
                                            self.settings.preview_width * tex.size_vec2().y
                                                / tex.size_vec2().x,
                                        ),
                                    ))
                                    .sense(egui::Sense::click()),
                                );
                                ui.label(
                                    RichText::new(crate::i18n::text(
                                        "CAPTURE ENREGISTRÉE",
                                        "SCREENSHOT SAVED",
                                    ))
                                    .color(CYAN)
                                    .size(10.0)
                                    .strong(),
                                );
                                ui.label(RichText::new(&meta.title).size(11.0));
                                if response.clicked() {
                                    self.open = true;
                                    self.page = Page::Gallery;
                                }
                            });
                    });
            } else {
                self.preview = None;
            }
        }
        if !self.open {
            if let Some((text, at, error)) = &self.notice {
                if *error && now - *at < 6.0 {
                    egui::Area::new(egui::Id::new("frame-error"))
                        .order(egui::Order::Foreground)
                        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 20.0))
                        .show(ctx, |ui| {
                            egui::Frame::new()
                                .fill(BG)
                                .corner_radius(9)
                                .inner_margin(12)
                                .show(ui, |ui| {
                                    ui.label(
                                        RichText::new(text).color(Color32::from_rgb(255, 149, 149)),
                                    );
                                });
                        });
                }
            }
        }
    }
}

#[cfg(test)]
mod interaction_tests {
    use super::*;
    use std::sync::mpsc;
    #[test]
    fn trash_waits_for_saved_edits_and_blocks_stale_loads_until_acknowledged() {
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "frame-trash-race-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(8);
        let (out, events) = mpsc::channel();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        let doc = crate::edit::Document::default();
        studio.queue_document("123-1".into(), doc.clone());
        studio.loading = Some("123-1".into());
        studio.send(Command::Trash("123-1".into()));
        studio.flush_documents();
        assert!(matches!(
            commands.try_recv().unwrap(),
            Command::SaveDocument(..)
        ));
        assert!(commands.try_recv().is_err());
        out.send(Event::Loaded(
            "123-1".into(),
            image::RgbaImage::new(100, 100),
            doc.clone(),
        ))
        .unwrap();
        out.send(Event::DocumentSaved("123-1".into(), doc)).unwrap();
        studio.events(&ctx);
        assert!(
            studio.editor.is_none(),
            "A stale load must not open a capture being trashed"
        );
        studio.flush_documents();
        assert!(matches!(commands.try_recv().unwrap(), Command::Trash(..)));
        assert!(!studio.send(Command::Load("123-1".into())));
        studio.flush_documents();
        assert!(
            commands.try_recv().is_err(),
            "Only one trash command should be in flight"
        );
        out.send(Event::Trashed("123-1".into())).unwrap();
        studio.events(&ctx);
        assert!(!studio.pending_trash.contains("123-1"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn passive_overlay_never_takes_game_input_even_with_previous_ui_focus() {
        use geode_egui::{InputMode, captures_input};
        for wanted in [true, false] {
            assert!(!captures_input(InputMode::Passthrough, true, wanted));
            assert!(!captures_input(InputMode::Blocking, false, wanted));
        }
        assert!(captures_input(InputMode::Blocking, true, false));
        assert!(captures_input(InputMode::Default, true, true));
    }
    #[test]
    fn closing_editor_with_a_full_queue_retries_the_latest_document_afterwards() {
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "frame-document-retry-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(1);
        let (_out, events) = mpsc::channel();
        tx.try_send(Command::Refresh).unwrap();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        let doc = crate::edit::Document {
            crop: None,
            annotations: vec![crate::edit::Annotation::Text {
                at: crate::edit::Point { x: 20.0, y: 30.0 },
                text: "Dernière modification".into(),
                color: [255; 4],
                size: 24.0,
            }],
        };
        studio.editor = Some(Editor::new(
            &ctx,
            "123-1".into(),
            image::RgbaImage::new(100, 100),
            doc.clone(),
            "Capture".into(),
        ));
        studio.save_editor();
        studio.editor = None;
        let _ = commands.try_recv().unwrap();
        ctx.begin_pass(egui::RawInput::default());
        studio.draw(&ctx);
        let _ = ctx.end_pass();
        match commands
            .try_recv()
            .expect("La sauvegarde doit être réessayée après fermeture")
        {
            Command::SaveDocument(id, saved) => {
                assert_eq!(id, "123-1");
                assert_eq!(saved, doc);
            }
            _ => panic!("Document attendu"),
        };
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn reopening_before_save_finishes_uses_the_latest_pending_document() {
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "frame-document-reopen-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(1);
        let (out, events) = mpsc::channel();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        let first = crate::edit::Document {
            crop: None,
            annotations: vec![crate::edit::Annotation::Text {
                at: crate::edit::Point { x: 20.0, y: 30.0 },
                text: "Première version".into(),
                color: [255; 4],
                size: 24.0,
            }],
        };
        studio.queue_document("123-1".into(), first.clone());
        studio.flush_documents();
        let _ = commands.try_recv().unwrap();
        let mut latest = first.clone();
        if let crate::edit::Annotation::Text { text, .. } = &mut latest.annotations[0] {
            *text = "Dernière version".into();
        }
        studio.queue_document("123-1".into(), latest.clone());
        studio.loading = Some("123-1".into());
        out.send(Event::Loaded(
            "123-1".into(),
            image::RgbaImage::new(100, 100),
            first.clone(),
        ))
        .unwrap();
        out.send(Event::DocumentSaved("123-1".into(), first))
            .unwrap();
        studio.events(&ctx);
        assert_eq!(studio.editor.as_ref().unwrap().history.current, latest);
        studio.flush_documents();
        match commands.try_recv().unwrap() {
            Command::SaveDocument(_, doc) => assert_eq!(doc, latest),
            _ => panic!("La dernière version doit suivre l’ancienne"),
        };
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn shortcut_conflict_keeps_existing_keys_and_a_valid_chord_is_persisted() {
        let root = std::env::temp_dir().join(format!(
            "frame-shortcut-test-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(8);
        let (_out, events) = mpsc::channel();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        studio.shortcut_listening = Some(0);
        studio.accept_shortcut("F6");
        assert_eq!(studio.settings.capture_key, "F8");
        assert!(studio.shortcut_error.is_some());
        assert!(commands.try_recv().is_err());
        studio.accept_shortcut("shift+ctrl+p");
        assert_eq!(studio.settings.capture_key, "Ctrl+Maj+P");
        assert!(studio.shortcut_listening.is_none());
        studio.flush_settings();
        match commands.try_recv().unwrap() {
            Command::Settings(settings) => assert_eq!(settings.capture_key, "Ctrl+Maj+P"),
            _ => panic!("Enregistrement attendu"),
        };
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn settings_retry_after_a_busy_worker_and_save_the_latest_value() {
        let root = std::env::temp_dir().join(format!(
            "frame-settings-retry-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(1);
        let (_out, events) = mpsc::channel();
        tx.try_send(Command::Refresh).unwrap();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        studio.settings.preview_width = 350.0;
        studio.persist_settings();
        studio.flush_settings();
        assert!(studio.settings_dirty);
        studio.settings.preview_width = 390.0;
        studio.persist_settings();
        let _ = commands.try_recv().unwrap();
        studio.flush_settings();
        assert!(!studio.settings_dirty);
        match commands.try_recv().unwrap() {
            Command::Settings(s) => assert_eq!(s.preview_width, 390.0),
            _ => panic!("Les derniers réglages doivent être envoyés"),
        };
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn inline_rename_selects_the_old_title_and_enter_queues_the_new_title() {
        let ctx = egui::Context::default();
        let root = std::env::temp_dir().join(format!(
            "frame-rename-test-{}-{}",
            std::process::id(),
            crate::store::now_ms()
        ));
        let store = Store::new(root.clone()).unwrap();
        let (tx, commands) = mpsc::sync_channel(16);
        let (_out, events) = mpsc::channel();
        let mut studio = Studio::new(store, tx, events, Settings::default());
        let meta = CaptureMeta {
            id: "123-1".into(),
            title: "Ancien nom".into(),
            game: GameSnapshot::default(),
            timestamp_ms: 123,
            width: 1000,
            height: 800,
            automatic: false,
            favorite: false,
        };
        studio.items.push(meta.clone());
        studio.failed_thumbs.insert(meta.id.clone());
        studio.begin_rename(&meta);
        let draw = |studio: &mut Studio, events: Vec<egui::Event>| {
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1024.0, 768.0),
                )),
                events,
                ..Default::default()
            });
            let mut ui = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("rename-test"),
                egui::UiBuilder::new(),
            );
            studio.gallery(&mut ui, &ctx, 700.0);
            let _ = ctx.end_pass();
        };
        draw(&mut studio, vec![]);
        draw(
            &mut studio,
            vec![egui::Event::Text("Mon meilleur moment".into())],
        );
        assert_eq!(studio.rename_text, "Mon meilleur moment");
        draw(
            &mut studio,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: Some(egui::Key::Enter),
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert!(studio.rename_id.is_none());
        match commands.try_recv().unwrap() {
            Command::Rename(id, title) => {
                assert_eq!(id, meta.id);
                assert_eq!(title, "Mon meilleur moment");
            }
            _ => panic!("Une commande de renommage est attendue"),
        };
        assert_eq!(studio.items[0].title, "Mon meilleur moment");
        std::fs::remove_dir_all(root).unwrap();
    }
}
