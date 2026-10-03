use crate::{
    edit::*,
    icons::{self, Icon},
    model::{ExportFormat, Settings},
    ui::{BG, CYAN, MUTED},
};
use geode_egui::egui::{self, Color32, Pos2, Rect, TextureHandle, Vec2};

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Pan,
    Brush,
    Arrow,
    Frame,
    Ellipse,
    Text,
    Crop,
}
struct TextSession {
    index: Option<usize>,
    at: Point,
    text: String,
    color: Color32,
    size: f32,
    focus: bool,
}
pub struct Editor {
    pub id: String,
    pub history: History,
    title: String,
    texture: TextureHandle,
    dimensions: [u32; 2],
    tool: Tool,
    zoom: f32,
    pan: Vec2,
    color: Color32,
    width: f32,
    filled: bool,
    view_scale: f32,
    text_session: Option<TextSession>,
    text_size: f32,
    stroke: Vec<Point>,
    start: Option<Point>,
    end: Option<Point>,
    selected: Option<usize>,
    drag_original: Option<Annotation>,
    resize_corner: Option<usize>,
    property_before: Option<Document>,
    pub export_format: ExportFormat,
    pub jpeg_quality: u8,
    grid: bool,
    show_original: bool,
    #[cfg(test)]
    last_image_rect: Rect,
    #[cfg(test)]
    last_text_rect: Rect,
    #[cfg(test)]
    last_undo_rect: Rect,
}
#[derive(Default)]
pub struct EditorResult {
    pub save: bool,
    pub export: bool,
    pub copy: bool,
    pub save_as: bool,
    pub close: bool,
}
impl Editor {
    pub fn new(
        ctx: &egui::Context,
        id: String,
        image: image::RgbaImage,
        doc: Document,
        title: String,
    ) -> Self {
        let dimensions = [image.width(), image.height()];
        let texture = ctx.load_texture(
            format!("editor-{id}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [dimensions[0] as usize, dimensions[1] as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        let mut history = History::default();
        history.current = doc;
        Self {
            id,
            history,
            title,
            texture,
            dimensions,
            tool: Tool::Select,
            zoom: 1.0,
            pan: Vec2::ZERO,
            color: Color32::WHITE,
            width: 6.0,
            filled: false,
            view_scale: 1.0,
            text_session: None,
            text_size: 28.0,
            stroke: Vec::new(),
            start: None,
            end: None,
            selected: None,
            drag_original: None,
            resize_corner: None,
            property_before: None,
            export_format: ExportFormat::Png,
            jpeg_quality: 92,
            grid: false,
            show_original: false,
            #[cfg(test)]
            last_image_rect: Rect::NOTHING,
            #[cfg(test)]
            last_text_rect: Rect::NOTHING,
            #[cfg(test)]
            last_undo_rect: Rect::NOTHING,
        }
    }
    pub fn apply_defaults(&mut self, settings: &Settings) {
        self.color = Color32::from_rgb(
            settings.editor_color[0],
            settings.editor_color[1],
            settings.editor_color[2],
        );
        self.width = settings.editor_width;
        self.text_size = settings.editor_text_size;
        self.grid = settings.editor_grid;
        self.export_format = settings.export_format;
        self.jpeg_quality = settings.jpeg_quality;
    }
    pub fn set_title(&mut self, title: String) {
        self.title = title;
    }
    pub fn settle(&mut self) {
        self.finish_text(true);
        if let Some(before) = self.property_before.take() {
            self.history.checkpoint(before);
        }
        if let (Some(original), Some(index)) = (
            self.drag_original.take(),
            self.selected
                .filter(|i| *i < self.history.current.annotations.len()),
        ) {
            let mut before = self.history.current.clone();
            before.annotations[index] = original;
            self.history.checkpoint(before);
        }
        self.start = None;
        self.end = None;
        self.stroke.clear();
        self.resize_corner = None;
    }
    pub fn handles_escape(&self) -> bool {
        self.text_session.is_some() || self.drag_original.is_some() || self.start.is_some()
    }
    fn begin_text(&mut self, at: Point, index: Option<usize>) {
        let (text, color, size, at) =
            match index.and_then(|i| self.history.current.annotations.get(i)) {
                Some(Annotation::Text {
                    at,
                    text,
                    color,
                    size,
                }) => (
                    text.clone(),
                    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
                    *size,
                    *at,
                ),
                _ => (String::new(), self.color, self.text_size, at),
            };
        self.text_session = Some(TextSession {
            index,
            at,
            text,
            color,
            size,
            focus: true,
        });
    }
    fn finish_text(&mut self, commit: bool) -> bool {
        let Some(session) = self.text_session.take() else {
            return false;
        };
        if !commit {
            return false;
        }
        let mut doc = self.history.current.clone();
        if session.text.trim().is_empty() {
            if let Some(index) = session.index.filter(|i| *i < doc.annotations.len()) {
                doc.annotations.remove(index);
                self.history.commit(doc);
                self.selected = None;
                return true;
            }
            return false;
        }
        let annotation = Annotation::Text {
            at: session.at,
            text: session.text.chars().take(1024).collect(),
            color: session.color.to_array(),
            size: session.size,
        };
        if let Some(index) = session.index.filter(|i| *i < doc.annotations.len()) {
            doc.annotations[index] = annotation;
            self.selected = Some(index);
        } else {
            doc.annotations.push(annotation);
            self.selected = Some(doc.annotations.len() - 1);
        }
        if doc == self.history.current {
            return false;
        }
        self.history.commit(doc);
        true
    }
    pub fn draw(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) -> EditorResult {
        let mut result = EditorResult::default();
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) && self.handles_escape() {
            self.finish_text(false);
            if let (Some(original), Some(index)) = (self.drag_original.take(), self.selected) {
                if let Some(a) = self.history.current.annotations.get_mut(index) {
                    *a = original;
                }
            }
            self.start = None;
            self.end = None;
            self.stroke.clear();
            self.resize_corner = None;
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
            });
        }
        let mut zoom_action = None;
        ui.horizontal(|ui| {
            if ui
                .button(crate::i18n::text("‹ Galerie", "‹ Gallery"))
                .clicked()
            {
                result.close = true;
            }
            ui.label(egui::RichText::new(&self.title).strong().size(18.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(if self.export_format == ExportFormat::Png {
                                crate::i18n::text("Exporter PNG", "Export PNG")
                            } else {
                                crate::i18n::text("Exporter JPEG", "Export JPEG")
                            })
                            .strong()
                            .color(BG),
                        )
                        .fill(CYAN),
                    )
                    .clicked()
                {
                    result.export = true;
                }
                if ui
                    .button(crate::i18n::text("Enregistrer", "Save"))
                    .clicked()
                {
                    result.save = true;
                }
                result.save_as = ui
                    .button(crate::i18n::text("Enregistrer sous…", "Save as…"))
                    .on_hover_text(crate::i18n::text(
                        "Choisir où enregistrer une copie PNG ou JPEG modifiée",
                        "Choose where to save an edited PNG or JPEG copy",
                    ))
                    .clicked();
                result.copy = ui
                    .button(crate::i18n::text("Copier", "Copy"))
                    .on_hover_text(crate::i18n::text(
                        "Copier l’image modifiée dans le presse-papiers",
                        "Copy the edited image to the clipboard",
                    ))
                    .clicked();
            });
        });
        ui.horizontal_wrapped(|ui| {
            for (tool, label) in [
                (
                    Tool::Select,
                    crate::i18n::text("Sélection / déplacer", "Select / move"),
                ),
                (Tool::Pan, crate::i18n::text("Main · vue", "Hand · view")),
                (Tool::Brush, crate::i18n::text("Dessiner", "Draw")),
                (Tool::Arrow, crate::i18n::text("Flèche", "Arrow")),
                (Tool::Frame, crate::i18n::text("Cadre", "Frame")),
                (Tool::Ellipse, crate::i18n::text("Cercle", "Ellipse")),
                (Tool::Text, crate::i18n::text("Texte", "Text")),
                (Tool::Crop, crate::i18n::text("Recadrer", "Crop")),
            ] {
                if ui.selectable_label(self.tool == tool, label).clicked() {
                    result.save |= self.finish_text(true);
                    self.finish_properties(&mut result);
                    self.tool = if self.tool == tool {
                        Tool::Select
                    } else {
                        tool
                    };
                    self.start = None;
                    self.end = None;
                    self.stroke.clear();
                }
            }
            ui.separator();
            let undo = ui
                .add_enabled(
                    self.history.can_undo() || self.text_session.is_some(),
                    egui::Button::new(crate::i18n::text("Annuler", "Undo")),
                )
                .on_hover_text(crate::i18n::text("Annuler · Ctrl+Z", "Undo · Ctrl+Z"));
            #[cfg(test)]
            {
                self.last_undo_rect = undo.rect;
            }
            if undo.clicked() {
                self.finish_text(true);
                self.finish_properties(&mut result);
                self.history.undo();
                self.selected = None;
                result.save = true;
            }
            if ui
                .add_enabled(
                    self.history.can_redo() && self.text_session.is_none(),
                    egui::Button::new(crate::i18n::text("Rétablir", "Redo")),
                )
                .on_hover_text(crate::i18n::text("Rétablir · Ctrl+Y", "Redo · Ctrl+Y"))
                .clicked()
            {
                self.finish_properties(&mut result);
                self.history.redo();
                self.selected = None;
                result.save = true;
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(crate::i18n::text("Couleur", "Color"));
            let mut rgb = [self.color.r(), self.color.g(), self.color.b()];
            if ui.color_edit_button_srgb(&mut rgb).changed() {
                self.color = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
            }
            match self.tool {
                Tool::Text => {
                    ui.add(
                        egui::Slider::new(&mut self.text_size, 8.0..=256.0)
                            .text(crate::i18n::text("Taille", "Size")),
                    );
                    ui.label(
                        egui::RichText::new(crate::i18n::text(
                            "Clique dans l’image et écris directement.",
                            "Click the image and type directly.",
                        ))
                        .color(MUTED),
                    );
                }
                Tool::Brush | Tool::Arrow => {
                    ui.add(
                        egui::Slider::new(&mut self.width, 1.0..=64.0)
                            .text(crate::i18n::text("Épaisseur", "Stroke width")),
                    );
                }
                Tool::Frame | Tool::Ellipse => {
                    ui.add(
                        egui::Slider::new(&mut self.width, 1.0..=64.0)
                            .text(crate::i18n::text("Épaisseur", "Stroke width")),
                    );
                    ui.checkbox(&mut self.filled, crate::i18n::text("Remplir", "Fill"));
                    ui.label(
                        egui::RichText::new(crate::i18n::text(
                            "Maintiens et glisse · Maj : carré / cercle parfait",
                            "Hold and drag · Shift: perfect square / circle",
                        ))
                        .color(MUTED),
                    );
                }
                Tool::Crop => {
                    if ui
                        .button(crate::i18n::text("Retirer le recadrage", "Remove crop"))
                        .clicked()
                    {
                        self.finish_properties(&mut result);
                        let mut doc = self.history.current.clone();
                        doc.crop = None;
                        self.history.commit(doc);
                        result.save = true;
                    }
                    ui.label(
                        egui::RichText::new(crate::i18n::text(
                            "Glisse pour choisir la zone à exporter.",
                            "Drag to select the area to export.",
                        ))
                        .color(MUTED),
                    );
                }
                Tool::Select => {
                    ui.label(
                        egui::RichText::new(crate::i18n::text(
                            "Glisse un élément · Coins : redimensionner · Double-clic : texte",
                            "Drag an element · Corners: resize · Double-click: text",
                        ))
                        .color(MUTED),
                    );
                }
                Tool::Pan => {
                    ui.label(
                        egui::RichText::new(crate::i18n::text(
                            "Glisse pour déplacer la vue · Molette pour zoomer",
                            "Drag to pan the view · Scroll to zoom",
                        ))
                        .color(MUTED),
                    );
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(crate::i18n::text("Ajuster", "Fit")).clicked() {
                    zoom_action = Some(0);
                }
                if ui
                    .button("100 %")
                    .on_hover_text(crate::i18n::text(
                        "Un pixel de l’image par point de l’interface",
                        "One image pixel per interface point",
                    ))
                    .clicked()
                {
                    zoom_action = Some(1);
                }
                if ui
                    .button("+")
                    .on_hover_text(crate::i18n::text("Zoom avant", "Zoom in"))
                    .clicked()
                {
                    zoom_action = Some(2);
                }
                ui.label(format!("{:.0} %", self.view_scale * 100.0));
                if ui
                    .button("−")
                    .on_hover_text(crate::i18n::text("Zoom arrière", "Zoom out"))
                    .clicked()
                {
                    zoom_action = Some(3);
                }
            });
        });
        let full_size = egui::vec2(
            ui.available_width(),
            (ui.available_height() - 60.0).max(100.0),
        );
        let panel_width = (full_size.x * 0.27).clamp(225.0, 275.0);
        let canvas_size = egui::vec2((full_size.x - panel_width - 12.0).max(120.0), full_size.y);
        let (canvas, response) = ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());
        let side_rect = Rect::from_min_size(
            canvas.right_top() + egui::vec2(12.0, 0.0),
            egui::vec2(panel_width, canvas.height()),
        );
        ui.expand_to_include_rect(side_rect);
        let mut side = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(side_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        side.set_clip_rect(side_rect.intersect(ui.clip_rect()));
        self.inspector(&mut side, &mut result);
        let painter = ui.painter().with_clip_rect(canvas);
        painter.rect_filled(canvas, 10, Color32::from_rgb(6, 11, 18));
        let dims = egui::vec2(self.dimensions[0] as f32, self.dimensions[1] as f32);
        let fit = ((canvas.width() - 30.0) / dims.x)
            .min((canvas.height() - 30.0) / dims.y)
            .max(0.001);
        match zoom_action {
            Some(0) => {
                self.zoom = 1.0;
                self.pan = Vec2::ZERO;
            }
            Some(1) => {
                self.zoom = 1.0 / fit;
                self.pan = Vec2::ZERO;
            }
            Some(2) => self.zoom = (self.zoom * 1.25).min(16.0 / fit),
            Some(3) => self.zoom = (self.zoom / 1.25).max(0.1),
            _ => {}
        }
        let mut scale = fit * self.zoom;
        let mut image_rect = Rect::from_center_size(canvas.center() + self.pan, dims * scale);
        if response.hovered() {
            let scroll = ctx.input(|i| i.smooth_scroll_delta.y);
            if scroll.abs() > 0.0 {
                let old_scale = scale;
                self.zoom = (self.zoom * (scroll.clamp(-100.0, 100.0) * 0.006).exp())
                    .clamp(0.1, 16.0 / fit);
                scale = fit * self.zoom;
                if let Some(pointer) = ctx.input(|i| i.pointer.hover_pos()) {
                    let origin = pointer - (pointer - image_rect.min) * (scale / old_scale);
                    self.pan = origin - canvas.center() + dims * scale * 0.5;
                }
                image_rect = Rect::from_center_size(canvas.center() + self.pan, dims * scale);
            }
        }
        self.view_scale = scale;
        #[cfg(test)]
        {
            self.last_image_rect = image_rect;
        }
        let middle = ctx.input(|i| i.pointer.button_down(egui::PointerButton::Middle));
        if response.dragged() && (self.tool == Tool::Pan || middle) {
            self.pan += ctx.input(|i| i.pointer.delta());
            image_rect = Rect::from_center_size(canvas.center() + self.pan, dims * scale);
        }
        painter.image(
            self.texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        painter.rect_stroke(
            image_rect,
            0,
            egui::Stroke::new(1.0, Color32::from_rgb(70, 89, 107)),
            egui::StrokeKind::Outside,
        );
        let release = ctx.input(|i| {
            i.events.iter().rev().find_map(|event| match event {
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers,
                } => Some((*pos, *modifiers)),
                _ => None,
            })
        });
        let pointer = release
            .map(|(pos, _)| pos)
            .or_else(|| ctx.input(|i| i.pointer.interact_pos()));
        let square = release.map_or_else(
            || ctx.input(|i| i.modifiers.shift),
            |(_, modifiers)| modifiers.shift,
        );
        let point = pointer.map(|p| Point {
            x: ((p.x - image_rect.min.x) / scale).clamp(
                0.0,
                if self.tool == Tool::Crop {
                    dims.x
                } else {
                    dims.x - 1.0
                },
            ),
            y: ((p.y - image_rect.min.y) / scale).clamp(
                0.0,
                if self.tool == Tool::Crop {
                    dims.y
                } else {
                    dims.y - 1.0
                },
            ),
        });
        if self.tool == Tool::Select
            && !middle
            && !self.show_original
            && self.text_session.is_none()
        {
            if response.clicked() && pointer.is_some_and(|p| image_rect.contains(p)) {
                self.finish_properties(&mut result);
                self.selected = point.and_then(|p| self.history.current.pick(p, 7.0 / scale));
                if response.double_clicked()
                    && let Some(index) = self.selected
                    && matches!(
                        self.history.current.annotations[index],
                        Annotation::Text { .. }
                    )
                {
                    self.begin_text(point.unwrap(), Some(index));
                }
            }
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.finish_properties(&mut result);
                if let Some(origin) = ctx
                    .input(|i| i.pointer.press_origin())
                    .filter(|p| canvas.contains(*p))
                {
                    let at = Point {
                        x: (origin.x - image_rect.min.x) / scale,
                        y: (origin.y - image_rect.min.y) / scale,
                    };
                    self.resize_corner = self
                        .selected
                        .and_then(|index| self.history.current.annotations.get(index))
                        .filter(|a| {
                            !matches!(a, Annotation::Text { .. } | Annotation::Arrow { .. })
                        })
                        .and_then(Annotation::bounds)
                        .and_then(|[min, max]| {
                            let r = Rect::from_min_max(
                                image_rect.min + egui::vec2(min.x, min.y) * scale,
                                image_rect.min + egui::vec2(max.x, max.y) * scale,
                            )
                            .expand(5.0);
                            [
                                r.left_top(),
                                r.right_top(),
                                r.left_bottom(),
                                r.right_bottom(),
                            ]
                            .iter()
                            .position(|p| p.distance(origin) <= 10.0)
                        });
                    if self.resize_corner.is_none() {
                        self.selected = image_rect
                            .contains(origin)
                            .then(|| self.history.current.pick(at, 7.0 / scale))
                            .flatten();
                    }
                    self.start = Some(at);
                    self.drag_original = self
                        .selected
                        .and_then(|index| self.history.current.annotations.get(index).cloned());
                }
            }
            let motion_point = if self.resize_corner.is_some() {
                pointer.map(|p| Point {
                    x: (p.x - image_rect.min.x) / scale,
                    y: (p.y - image_rect.min.y) / scale,
                })
            } else {
                point
            };
            if let (Some(original), Some(from), Some(to), Some(index)) =
                (&self.drag_original, self.start, motion_point, self.selected)
            {
                if response.dragged_by(egui::PointerButton::Primary)
                    || response.drag_stopped_by(egui::PointerButton::Primary)
                {
                    let moved = if let Some(corner) = self.resize_corner {
                        original.resized_corner(
                            corner,
                            to.x - from.x,
                            to.y - from.y,
                            square,
                            self.dimensions,
                        )
                    } else {
                        original.translated_clamped(to.x - from.x, to.y - from.y, self.dimensions)
                    };
                    if response.drag_stopped_by(egui::PointerButton::Primary) {
                        let mut before = self.history.current.clone();
                        before.annotations[index] = original.clone();
                        self.history.current.annotations[index] = moved;
                        self.history.checkpoint(before);
                        result.save = true;
                    } else {
                        self.history.current.annotations[index] = moved;
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                self.drag_original = None;
                self.start = None;
                self.end = None;
                self.resize_corner = None;
            }
        }
        if !matches!(self.tool, Tool::Pan | Tool::Select | Tool::Text)
            && !middle
            && !self.show_original
            && self.text_session.is_none()
        {
            let press_origin = ctx.input(|i| i.pointer.press_origin());
            if response.drag_started_by(egui::PointerButton::Primary)
                && press_origin.is_some_and(|p| image_rect.contains(p))
            {
                self.finish_properties(&mut result);
                let origin = press_origin.map(|p| Point {
                    x: ((p.x - image_rect.min.x) / scale).clamp(
                        0.0,
                        if self.tool == Tool::Crop {
                            dims.x
                        } else {
                            dims.x - 1.0
                        },
                    ),
                    y: ((p.y - image_rect.min.y) / scale).clamp(
                        0.0,
                        if self.tool == Tool::Crop {
                            dims.y
                        } else {
                            dims.y - 1.0
                        },
                    ),
                });
                self.start = origin;
                self.end = origin;
                self.stroke.clear();
                if let Some(p) = origin.filter(|_| self.tool == Tool::Brush) {
                    self.stroke.push(p);
                }
            }
            if (response.dragged_by(egui::PointerButton::Primary)
                || response.drag_stopped_by(egui::PointerButton::Primary))
                && self.start.is_some()
            {
                self.end = point.map(|p| {
                    if matches!(self.tool, Tool::Frame | Tool::Ellipse) && square {
                        square_endpoint(self.start.unwrap(), p, self.dimensions)
                    } else {
                        p
                    }
                });
                if self.tool == Tool::Brush {
                    if let Some(p) = point {
                        if self
                            .stroke
                            .last()
                            .is_none_or(|last| (last.x - p.x).hypot(last.y - p.y) > 0.6)
                            && self.stroke.len() < 100_000
                        {
                            self.stroke.push(p);
                        }
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) && self.start.is_some() {
                let from = self.start.take().unwrap();
                let to = self.end.take().unwrap_or(from);
                let mut doc = self.history.current.clone();
                match self.tool {
                    Tool::Brush => {
                        doc.annotations.push(Annotation::Stroke {
                            points: std::mem::take(&mut self.stroke),
                            color: self.color.to_array(),
                            width: self.width,
                        });
                    }
                    Tool::Arrow => doc.annotations.push(Annotation::Arrow {
                        from,
                        to,
                        color: self.color.to_array(),
                        width: self.width,
                    }),
                    Tool::Crop => {
                        doc.crop = crop_from_drag(from, to, self.dimensions);
                    }
                    Tool::Frame | Tool::Ellipse => {
                        if shape_bounds(from, to).is_some() {
                            doc.annotations.push(if self.tool == Tool::Frame {
                                Annotation::Frame {
                                    from,
                                    to,
                                    color: self.color.to_array(),
                                    width: self.width,
                                    filled: self.filled,
                                }
                            } else {
                                Annotation::Ellipse {
                                    from,
                                    to,
                                    color: self.color.to_array(),
                                    width: self.width,
                                    filled: self.filled,
                                }
                            });
                        }
                    }
                    _ => {}
                }
                if doc.crop != self.history.current.crop
                    || doc.annotations.len() != self.history.current.annotations.len()
                {
                    let created = doc.annotations.len() > self.history.current.annotations.len();
                    self.history.commit(doc);
                    if created {
                        self.selected = Some(self.history.current.annotations.len() - 1);
                    }
                    result.save = true;
                }
                self.stroke.clear();
            }
            if response.clicked() && pointer.is_some_and(|p| image_rect.contains(p)) {
                if let Some(at) = point {
                    self.finish_properties(&mut result);
                    let mut doc = self.history.current.clone();
                    if self.tool == Tool::Brush {
                        doc.annotations.push(Annotation::Stroke {
                            points: vec![at],
                            color: self.color.to_array(),
                            width: self.width,
                        });
                        self.history.commit(doc);
                        self.selected = Some(self.history.current.annotations.len() - 1);
                        result.save = true;
                    }
                }
            }
        }
        if self.tool == Tool::Text
            && self.text_session.is_none()
            && response.clicked()
            && pointer.is_some_and(|p| image_rect.contains(p))
            && !self.show_original
        {
            self.finish_properties(&mut result);
            let index = point
                .and_then(|p| self.history.current.pick(p, 7.0 / scale))
                .filter(|i| {
                    matches!(
                        self.history.current.annotations[*i],
                        Annotation::Text { .. }
                    )
                });
            self.begin_text(point.unwrap(), index);
        }
        let mapped = |p: Point| image_rect.min + egui::vec2(p.x, p.y) * scale;
        if self.grid {
            for fraction in [1.0 / 3.0, 2.0 / 3.0] {
                painter.line_segment(
                    [
                        image_rect.min + egui::vec2(image_rect.width() * fraction, 0.0),
                        image_rect.min
                            + egui::vec2(image_rect.width() * fraction, image_rect.height()),
                    ],
                    egui::Stroke::new(1.0, Color32::from_white_alpha(90)),
                );
                painter.line_segment(
                    [
                        image_rect.min + egui::vec2(0.0, image_rect.height() * fraction),
                        image_rect.min
                            + egui::vec2(image_rect.width(), image_rect.height() * fraction),
                    ],
                    egui::Stroke::new(1.0, Color32::from_white_alpha(90)),
                );
            }
        }
        if !self.show_original {
            for (index, annotation) in self.history.current.annotations.iter().enumerate() {
                if self
                    .text_session
                    .as_ref()
                    .is_some_and(|s| s.index == Some(index))
                {
                    continue;
                }
                paint_annotation(&painter, annotation, &mapped, scale);
            }
            if let Some([min, max]) = self
                .selected
                .and_then(|index| self.history.current.annotations.get(index))
                .and_then(Annotation::bounds)
            {
                let r = Rect::from_min_max(mapped(min), mapped(max)).expand(5.0);
                painter.rect_stroke(
                    r,
                    2,
                    egui::Stroke::new(1.5, CYAN),
                    egui::StrokeKind::Outside,
                );
                if self
                    .selected
                    .and_then(|i| self.history.current.annotations.get(i))
                    .is_some_and(|a| {
                        !matches!(a, Annotation::Text { .. } | Annotation::Arrow { .. })
                    })
                {
                    for p in [
                        r.left_top(),
                        r.right_top(),
                        r.left_bottom(),
                        r.right_bottom(),
                    ] {
                        painter.rect_filled(
                            Rect::from_center_size(p, egui::vec2(9.0, 9.0)),
                            2,
                            CYAN,
                        );
                    }
                }
            }
        }
        if self.tool == Tool::Select
            && response.hovered()
            && self.text_session.is_none()
            && !self.show_original
        {
            let hovered = ctx.input(|i| i.pointer.hover_pos());
            let corner = self
                .selected
                .and_then(|index| self.history.current.annotations.get(index))
                .filter(|a| !matches!(a, Annotation::Text { .. } | Annotation::Arrow { .. }))
                .and_then(Annotation::bounds)
                .and_then(|[min, max]| {
                    let r = Rect::from_min_max(mapped(min), mapped(max)).expand(5.0);
                    [
                        r.left_top(),
                        r.right_top(),
                        r.left_bottom(),
                        r.right_bottom(),
                    ]
                    .iter()
                    .position(|p| hovered.is_some_and(|h| h.distance(*p) <= 10.0))
                });
            if let Some(corner) = corner {
                ctx.set_cursor_icon(if corner == 0 || corner == 3 {
                    egui::CursorIcon::ResizeNwSe
                } else {
                    egui::CursorIcon::ResizeNeSw
                });
            } else if point.is_some_and(|p| self.history.current.pick(p, 7.0 / scale).is_some()) {
                ctx.set_cursor_icon(egui::CursorIcon::Grab);
            }
        } else if self.tool == Tool::Text && response.hovered() {
            ctx.set_cursor_icon(egui::CursorIcon::Text);
        }
        if self.drag_original.is_some() {
            ctx.set_cursor_icon(if self.resize_corner.is_some() {
                egui::CursorIcon::ResizeNwSe
            } else {
                egui::CursorIcon::Grabbing
            });
        }
        if let Some(session) = &mut self.text_session {
            let pos = mapped(session.at);
            let mut done = false;
            let enter = ctx.input_mut(|i| {
                let pressed=i.events.iter().any(|e|matches!(e,egui::Event::Key{key:egui::Key::Enter,pressed:true,modifiers,..} if !modifiers.shift&&!modifiers.ctrl&&!modifiers.alt));
                if pressed {i.events.retain(|e|!matches!(e,egui::Event::Key{key:egui::Key::Enter,pressed:true,modifiers,..} if !modifiers.shift&&!modifiers.ctrl&&!modifiers.alt));}
                pressed
            });
            egui::Area::new(egui::Id::new(("frame-inline-text", &self.id)))
                .order(egui::Order::Foreground)
                .fixed_pos(pos - egui::vec2(4.0, 4.0))
                .show(ctx, |ui| {
                    ui.set_clip_rect(canvas.intersect(ui.clip_rect()));
                    egui::Frame::new()
                        .fill(Color32::from_black_alpha(205))
                        .stroke(egui::Stroke::new(1.5, CYAN))
                        .inner_margin(4)
                        .corner_radius(3)
                        .show(ui, |ui| {
                            let width = (image_rect.right() - pos.x - 12.0).clamp(60.0, 500.0);
                            let font = egui::FontId::proportional((session.size * scale).max(10.0));
                            let color = session.color;
                            let mut layouter =
                                |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap_width: f32| {
                                    ui.painter().layout(
                                        text.as_str().to_owned(),
                                        font.clone(),
                                        color,
                                        f32::INFINITY,
                                    )
                                };
                            let response = ui.add(
                                egui::TextEdit::multiline(&mut session.text)
                                    .margin(egui::Vec2::ZERO)
                                    .frame(egui::Frame::NONE)
                                    .id_salt("direct-text-field")
                                    .font(egui::FontId::proportional(
                                        (session.size * scale).max(10.0),
                                    ))
                                    .text_color(session.color)
                                    .desired_width(width)
                                    .desired_rows(2)
                                    .layouter(&mut layouter)
                                    .char_limit(1024)
                                    .hint_text(crate::i18n::text("Écris ici…", "Type here…")),
                            );
                            if session.focus {
                                response.request_focus();
                                session.focus = false;
                            }
                            done = enter
                                || (response.lost_focus() && !ctx.input(|i| i.pointer.any_down()));
                        });
                });
            if done {
                result.save |= self.finish_text(true);
            }
        }
        if !self.stroke.is_empty() {
            paint_annotation(
                &painter,
                &Annotation::Stroke {
                    points: self.stroke.clone(),
                    color: self.color.to_array(),
                    width: self.width,
                },
                &mapped,
                scale,
            );
        }
        if let (Some(from), Some(to)) = (self.start, self.end) {
            if matches!(self.tool, Tool::Frame | Tool::Ellipse) {
                let annotation = if self.tool == Tool::Frame {
                    Annotation::Frame {
                        from,
                        to,
                        color: self.color.to_array(),
                        width: self.width,
                        filled: self.filled,
                    }
                } else {
                    Annotation::Ellipse {
                        from,
                        to,
                        color: self.color.to_array(),
                        width: self.width,
                        filled: self.filled,
                    }
                };
                paint_annotation(&painter, &annotation, &mapped, scale);
                if let Some([min, max]) = shape_bounds(from, to) {
                    painter.text(
                        mapped(max) + egui::vec2(10.0, 10.0),
                        egui::Align2::LEFT_TOP,
                        format!("{:.0} × {:.0} px", max.x - min.x, max.y - min.y),
                        egui::FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                }
            }
            if self.tool == Tool::Arrow {
                paint_annotation(
                    &painter,
                    &Annotation::Arrow {
                        from,
                        to,
                        color: self.color.to_array(),
                        width: self.width,
                    },
                    &mapped,
                    scale,
                );
            }
            if self.tool == Tool::Crop {
                painter.rect_stroke(
                    Rect::from_two_pos(mapped(from), mapped(to)),
                    0,
                    egui::Stroke::new(2.0, CYAN),
                    egui::StrokeKind::Inside,
                );
            }
        }
        if let Some(c) = self.history.current.crop.filter(|_| !self.show_original) {
            let r = Rect::from_min_size(
                mapped(Point {
                    x: c.x as f32,
                    y: c.y as f32,
                }),
                egui::vec2(c.width as f32, c.height as f32) * scale,
            );
            for shade in [
                Rect::from_min_max(image_rect.min, egui::pos2(image_rect.max.x, r.min.y)),
                Rect::from_min_max(egui::pos2(image_rect.min.x, r.max.y), image_rect.max),
                Rect::from_min_max(
                    egui::pos2(image_rect.min.x, r.min.y),
                    egui::pos2(r.min.x, r.max.y),
                ),
                Rect::from_min_max(
                    egui::pos2(r.max.x, r.min.y),
                    egui::pos2(image_rect.max.x, r.max.y),
                ),
            ] {
                painter.rect_filled(shade, 0, Color32::from_black_alpha(150));
            }
            painter.rect_stroke(r, 0, egui::Stroke::new(2.0, CYAN), egui::StrokeKind::Inside);
        }
        let control_key = |key| {
            ctx.input(|i|i.events.iter().any(|e|matches!(e,egui::Event::Key{key:k,pressed:true,modifiers,..} if *k==key&&(modifiers.ctrl||modifiers.command))))
        };
        if !ctx.input(|i| i.pointer.any_down())
            && (!ctx.text_edit_focused() || result.export || result.close || result.save)
        {
            if let Some(before) = self.property_before.take() {
                self.history.checkpoint(before);
                result.save = true;
            }
        }
        let document_keys = !ctx.text_edit_focused()
            && self.text_session.is_none()
            && self.drag_original.is_none()
            && !ctx.input(|i| i.pointer.any_down());
        if document_keys && self.selected.is_some() {
            if ctx.input(|i| i.key_pressed(egui::Key::Delete)) {
                self.remove_selected(&mut result);
            }
            if control_key(egui::Key::D) {
                self.duplicate_selected(&mut result);
            }
            let step = if ctx.input(|i| i.modifiers.shift) {
                10.0
            } else {
                1.0
            };
            let dx = ctx.input(|i| {
                (i.key_pressed(egui::Key::ArrowRight) as i32
                    - i.key_pressed(egui::Key::ArrowLeft) as i32) as f32
            }) * step;
            let dy = ctx.input(|i| {
                (i.key_pressed(egui::Key::ArrowDown) as i32
                    - i.key_pressed(egui::Key::ArrowUp) as i32) as f32
            }) * step;
            if (dx != 0.0 || dy != 0.0)
                && let Some(index) = self
                    .selected
                    .filter(|i| *i < self.history.current.annotations.len())
            {
                self.finish_properties(&mut result);
                let mut doc = self.history.current.clone();
                doc.annotations[index] =
                    doc.annotations[index].translated_clamped(dx, dy, self.dimensions);
                self.history.commit(doc);
                result.save = true;
            }
        }
        if document_keys && control_key(egui::Key::Z) {
            self.finish_properties(&mut result);
            self.history.undo();
            self.selected = None;
            result.save = true;
        }
        if document_keys && control_key(egui::Key::Y) {
            self.finish_properties(&mut result);
            self.history.redo();
            self.selected = None;
            result.save = true;
        }
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{} × {} px",
                    self.dimensions[0], self.dimensions[1]
                ))
                .small()
                .color(MUTED),
            );
            ui.label(
                egui::RichText::new(crate::i18n::text(
                    "Original protégé · Les exports sont des copies",
                    "Original protected · Exports are copies",
                ))
                .small()
                .color(CYAN),
            );
        });
        if result.save || result.export || result.close || result.copy || result.save_as {
            result.save |= self.finish_text(true);
        }
        result
    }
    fn finish_properties(&mut self, result: &mut EditorResult) {
        if let Some(before) = self.property_before.take() {
            self.history.checkpoint(before);
            result.save = true;
        }
    }
    fn remove_selected(&mut self, result: &mut EditorResult) {
        self.finish_properties(result);
        if let Some(index) = self
            .selected
            .take()
            .filter(|i| *i < self.history.current.annotations.len())
        {
            let mut doc = self.history.current.clone();
            doc.annotations.remove(index);
            self.history.commit(doc);
            result.save = true;
        }
    }
    fn duplicate_selected(&mut self, result: &mut EditorResult) {
        self.finish_properties(result);
        if let Some(index) = self
            .selected
            .filter(|i| *i < self.history.current.annotations.len())
        {
            let mut doc = self.history.current.clone();
            let copy = doc.annotations[index].translated_clamped(20.0, 20.0, self.dimensions);
            doc.annotations.push(copy);
            self.selected = Some(doc.annotations.len() - 1);
            self.history.commit(doc);
            result.save = true;
        }
    }
    fn inspector(&mut self, ui: &mut egui::Ui, result: &mut EditorResult) {
        if let Some(session) = &mut self.text_session {
            let mut done = false;
            let mut cancel = false;
            egui::Frame::new().fill(crate::ui::PANEL).corner_radius(10).inner_margin(12).show(ui,|ui| {
                ui.label(egui::RichText::new(crate::i18n::text("TEXTE SUR L’IMAGE", "TEXT ON THE IMAGE")).strong().color(CYAN));
                ui.add_space(10.0);ui.label(crate::i18n::text("Écris directement dans le cadre sur la capture.", "Type directly in the box on the screenshot."));
                ui.label(egui::RichText::new(crate::i18n::text("Entrée : valider\nMaj+Entrée : nouvelle ligne\nÉchap : annuler", "Enter: confirm\nShift+Enter: new line\nEscape: cancel")).color(MUTED));
                ui.add_space(16.0);ui.add(egui::Slider::new(&mut session.size,8.0..=256.0).text(crate::i18n::text("Taille", "Size")));
                let mut rgb=[session.color.r(),session.color.g(),session.color.b()];
                if ui.color_edit_button_srgb(&mut rgb).changed(){session.color=Color32::from_rgb(rgb[0],rgb[1],rgb[2]);}
                ui.add_space(16.0);done=ui.button(crate::i18n::text("Valider le texte", "Confirm text")).clicked();cancel=ui.button(crate::i18n::text("Annuler la saisie", "Cancel typing")).clicked();
                ui.add_space(16.0);ui.label(egui::RichText::new(crate::i18n::text("Après validation, glisse le texte pour le déplacer. Double-clique dessus pour le corriger.", "After confirming, drag the text to move it. Double-click to edit it again.")).small().color(MUTED));
            });
            if done || cancel {
                result.save |= self.finish_text(done);
            }
            return;
        }
        self.selected = self
            .selected
            .filter(|i| *i < self.history.current.annotations.len());
        egui::Frame::new().fill(crate::ui::PANEL).corner_radius(10).inner_margin(12).show(ui,|ui| {
            ui.set_width((ui.available_width()-2.0).max(150.0));
            egui::ScrollArea::vertical().id_salt("editor-inspector").max_height(ui.available_height()-24.0).show(ui,|ui| {
                ui.label(egui::RichText::new(crate::i18n::text("PROPRIÉTÉS", "PROPERTIES")).strong().color(CYAN).small());
                if let Some(index)=self.selected {
                    let before=self.history.current.clone();
                    let mut annotation=before.annotations[index].clone();
                    ui.label(egui::RichText::new(format!("{} · {}",index+1,annotation.name())).strong().size(18.0));
                    ui.horizontal(|ui| {
                        if icons::button(ui,Icon::Copy,crate::i18n::text("Dupliquer · Ctrl+D", "Duplicate · Ctrl+D"),false).clicked() { self.duplicate_selected(result); }
                        if icons::button(ui,Icon::Trash,crate::i18n::text("Supprimer l’élément · Suppr", "Delete element · Delete"),false).clicked() { self.remove_selected(result); }
                        let len=self.history.current.annotations.len();
                        let index=self.selected.unwrap_or(usize::MAX);
                        if ui.add_enabled_ui(index.saturating_add(1) < len, |ui| icons::button(ui,Icon::Up,crate::i18n::text("Avancer d’un calque", "Move forward one layer"),false)).inner.clicked() {
                            self.finish_properties(result);let mut doc=self.history.current.clone();doc.annotations.swap(index,index+1);self.selected=Some(index+1);self.history.commit(doc);result.save=true;
                        }
                        if ui.add_enabled_ui(index>0 && index<len, |ui| icons::button(ui,Icon::Down,crate::i18n::text("Reculer d’un calque", "Move back one layer"),false)).inner.clicked() {
                            self.finish_properties(result);let mut doc=self.history.current.clone();doc.annotations.swap(index,index-1);self.selected=Some(index-1);self.history.commit(doc);result.save=true;
                        }
                    });
                    let color=annotation.color_mut();
                    let mut rgb=[color[0],color[1],color[2]];
                    ui.horizontal(|ui| { ui.label(crate::i18n::text("Couleur", "Color")); if ui.color_edit_button_srgb(&mut rgb).changed() { *color=[rgb[0],rgb[1],rgb[2],255]; } });
                    match &mut annotation {
                        Annotation::Text {text,size,..} => {
                            let response=ui.add(egui::TextEdit::multiline(text).desired_rows(2).char_limit(1024).desired_width(f32::INFINITY));
                            #[cfg(test)] { self.last_text_rect=response.rect; }
                            let _=response;
                            ui.add(egui::Slider::new(size,8.0..=256.0).text(crate::i18n::text("Taille", "Size")));
                            if ui.button(crate::i18n::text("Modifier sur l’image", "Edit on image")).clicked(){self.finish_properties(result);self.begin_text(Point{x:0.0,y:0.0},Some(index));}
                        },
                        Annotation::Stroke {width,..} | Annotation::Arrow {width,..} => { ui.add(egui::Slider::new(width,1.0..=64.0).text(crate::i18n::text("Épaisseur", "Stroke width"))); },
                        Annotation::Frame {width,filled,..} | Annotation::Ellipse {width,filled,..} => { ui.add(egui::Slider::new(width,1.0..=64.0).text(crate::i18n::text("Épaisseur", "Stroke width")));ui.checkbox(filled,crate::i18n::text("Forme remplie", "Filled shape")); },
                    }
                    if let Some([min,max])=annotation.bounds() {
                        let mut x=min.x;let mut y=min.y;let mut w=max.x-min.x;let mut h=max.y-min.y;
                        let mut moved=false;let mut resized=false;
                        egui::Grid::new("object-geometry").num_columns(2).spacing([8.0,8.0]).show(ui,|ui| {
                            ui.label("X / Y");ui.horizontal(|ui| {
                                moved|=ui.add(egui::DragValue::new(&mut x).speed(1).range(0.0..=self.dimensions[0] as f32).prefix("X ")).changed();
                                moved|=ui.add(egui::DragValue::new(&mut y).speed(1).range(0.0..=self.dimensions[1] as f32).prefix("Y ")).changed();
                            });ui.end_row();
                            if !matches!(annotation,Annotation::Text{..}|Annotation::Arrow{..}) {
                                ui.label(crate::i18n::text("L / H", "W / H"));ui.horizontal(|ui| {
                                    resized|=ui.add(egui::DragValue::new(&mut w).speed(1).range(1.0..=self.dimensions[0] as f32).prefix(crate::i18n::text("L ", "W "))).changed();
                                    resized|=ui.add(egui::DragValue::new(&mut h).speed(1).range(1.0..=self.dimensions[1] as f32).prefix("H ")).changed();
                                });ui.end_row();
                            }
                        });
                        if resized { annotation=annotation.resized(w,h,self.dimensions); }
                        if moved { annotation=annotation.translated_clamped(x-min.x,y-min.y,self.dimensions); }
                    }
                    // A whole slider/text/drag gesture occupies one undo step.
                    if self.selected==Some(index) && before.annotations.get(index)!=Some(&annotation) && self.history.current==before {
                        self.property_before.get_or_insert(before);
                        self.history.current.annotations[index]=annotation;
                    }
                } else {
                    ui.label(egui::RichText::new(crate::i18n::text("Aucun élément sélectionné", "No element selected")).color(MUTED));
                    ui.label(egui::RichText::new(crate::i18n::text("Clique dans une forme ou sur un calque. Glisse pour déplacer ; attrape un coin pour redimensionner.", "Click inside a shape or on a layer. Drag to move; grab a corner to resize.")).small().color(MUTED));
                }
                ui.add_space(10.0);ui.separator();
                ui.label(egui::RichText::new(crate::translated_format!("CALQUES · {}", "LAYERS · {}",self.history.current.annotations.len())).strong().color(CYAN).small());
                let mut layer_clicked=None;
                egui::ScrollArea::vertical().id_salt("annotation-layers").max_height(140.0).show(ui,|ui| {
                    for (index,a) in self.history.current.annotations.iter().enumerate().rev() {
                        let title=match a { Annotation::Text {text,..}=>format!("{} · {}",index+1,text.chars().take(22).collect::<String>()), _=>format!("{} · {}",index+1,a.name()) };
                        if ui.selectable_label(self.selected==Some(index),title).clicked() { layer_clicked=Some(index); }
                    }
                });
                if let Some(index)=layer_clicked { self.finish_properties(result);self.selected=Some(index);self.tool=Tool::Select; }
                if !self.history.current.annotations.is_empty() && ui.small_button(crate::i18n::text("Effacer les annotations", "Clear annotations")).on_hover_text(crate::i18n::text("Annulable avec Ctrl+Z", "Undo with Ctrl+Z")).clicked() {
                    self.finish_properties(result);let mut doc=self.history.current.clone();doc.annotations.clear();self.history.commit(doc);self.selected=None;result.save=true;
                }
                ui.add_space(10.0);ui.separator();
                ui.collapsing(crate::i18n::text("Recadrage précis", "Precise crop"),|ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (label,a,b) in [("1:1",1,1),("16:9",16,9),("4:3",4,3),("9:16",9,16)] {
                            if ui.small_button(label).clicked() { self.finish_properties(result);let mut doc=self.history.current.clone();doc.crop=centered_crop(self.dimensions,a,b);self.history.commit(doc);result.save=true; }
                        }
                    });
                    if let Some(crop)=self.history.current.crop {
                        let mut c=crop;let mut changed=false;
                        ui.horizontal(|ui| { changed|=ui.add(egui::DragValue::new(&mut c.x).range(0..=self.dimensions[0]-1).prefix("X ")).changed();changed|=ui.add(egui::DragValue::new(&mut c.y).range(0..=self.dimensions[1]-1).prefix("Y ")).changed(); });
                        ui.horizontal(|ui| { changed|=ui.add(egui::DragValue::new(&mut c.width).range(1..=self.dimensions[0]-c.x.min(self.dimensions[0]-1)).prefix(crate::i18n::text("L ", "W "))).changed();changed|=ui.add(egui::DragValue::new(&mut c.height).range(1..=self.dimensions[1]-c.y.min(self.dimensions[1]-1)).prefix("H ")).changed(); });
                        c.x=c.x.min(self.dimensions[0]-1);c.y=c.y.min(self.dimensions[1]-1);c.width=c.width.clamp(1,self.dimensions[0]-c.x);c.height=c.height.clamp(1,self.dimensions[1]-c.y);
                        if changed { self.property_before.get_or_insert(self.history.current.clone());self.history.current.crop=Some(c); }
                        if ui.small_button(crate::i18n::text("Image entière", "Full image")).clicked() { self.finish_properties(result);let mut doc=self.history.current.clone();doc.crop=None;self.history.commit(doc);result.save=true; }
                    } else { ui.label(egui::RichText::new(crate::i18n::text("Image entière · ou glisse avec Recadrer", "Full image · or drag with Crop")).small().color(MUTED)); }
                });
                ui.collapsing(crate::i18n::text("Affichage et export", "View and export"),|ui| {
                    ui.checkbox(&mut self.grid,crate::i18n::text("Grille des tiers", "Rule of thirds grid"));
                    ui.checkbox(&mut self.show_original,crate::i18n::text("Voir l’original", "View original"));
                    ui.horizontal(|ui| { ui.selectable_value(&mut self.export_format,ExportFormat::Png,"PNG");ui.selectable_value(&mut self.export_format,ExportFormat::Jpeg,"JPEG"); });
                    if self.export_format==ExportFormat::Jpeg { ui.add(egui::Slider::new(&mut self.jpeg_quality,40..=100).text(crate::i18n::text("Qualité", "Quality"))); }
                });
                ui.add_space(8.0);
                ui.label(egui::RichText::new(crate::i18n::text("Ctrl+Z / Y : historique\nCtrl+D : dupliquer · Suppr : retirer\nFlèches : 1 px · Maj : 10 px", "Ctrl+Z / Y: undo / redo\nCtrl+D: duplicate · Delete: remove\nArrows: 1 px · Shift: 10 px")).small().color(MUTED));
            });
        });
    }
}
fn paint_annotation(
    painter: &egui::Painter,
    annotation: &Annotation,
    mapped: &impl Fn(Point) -> Pos2,
    scale: f32,
) {
    match annotation {
        Annotation::Stroke {
            points,
            color,
            width,
        } => {
            let stroke = egui::Stroke::new(
                width * scale,
                Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
            );
            if points.len() > 1 {
                painter.add(egui::Shape::line(
                    points.iter().copied().map(mapped).collect(),
                    stroke,
                ));
            } else if let Some(p) = points.first() {
                painter.circle_filled(mapped(*p), width * scale / 2.0, stroke.color);
            }
        }
        Annotation::Arrow {
            from,
            to,
            color,
            width,
        } => {
            let stroke = egui::Stroke::new(
                width * scale,
                Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
            );
            painter.line_segment([mapped(*from), mapped(*to)], stroke);
            for wing in arrow_wings(*from, *to, *width) {
                painter.line_segment([mapped(*to), mapped(wing)], stroke);
            }
        }
        Annotation::Text {
            at,
            text,
            color,
            size,
        } => {
            for (line_index, line) in text.split('\n').enumerate() {
                painter.text(
                    mapped(Point {
                        x: at.x,
                        y: at.y + line_index as f32 * size * 1.2,
                    }),
                    egui::Align2::LEFT_TOP,
                    line,
                    egui::FontId::proportional(size * scale),
                    Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]),
                );
            }
        }
        Annotation::Frame {
            from,
            to,
            color,
            width,
            filled,
        }
        | Annotation::Ellipse {
            from,
            to,
            color,
            width,
            filled,
        } => {
            let Some([min, max]) = shape_bounds(*from, *to) else {
                return;
            };
            let rect = Rect::from_min_max(mapped(min), mapped(max));
            let color = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
            let stroke = egui::Stroke::new(width * scale, color);
            if matches!(annotation, Annotation::Frame { .. }) {
                if *filled {
                    painter.rect_filled(rect, 0, color);
                }
                let corners = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                ];
                for i in 0..4 {
                    painter.line_segment([corners[i], corners[(i + 1) % 4]], stroke);
                    painter.circle_filled(corners[i], width * scale / 2.0, color);
                }
            } else {
                if *filled {
                    painter.add(egui::Shape::ellipse_filled(
                        rect.center(),
                        rect.size() * 0.5,
                        color,
                    ));
                }
                painter.add(egui::Shape::ellipse_stroke(
                    rect.center(),
                    rect.size() * 0.5,
                    stroke,
                ));
            }
        }
    }
}

#[cfg(test)]
mod interaction_tests {
    fn click(ctx: &egui::Context, editor: &mut Editor, pos: Pos2) {
        frame(
            ctx,
            editor,
            vec![egui::Event::PointerMoved(pos), button(pos, true, false)],
        );
        frame(ctx, editor, vec![button(pos, false, false)]);
    }
    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }
    #[test]
    fn text_is_typed_at_the_clicked_position_and_enter_commits_it() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Text);
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.min + egui::vec2(150.0, 150.0) * editor.view_scale;
        click(&ctx, &mut editor, pos);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::Text("Mon moment\nDeuxième ligne".into())],
        );
        frame(&ctx, &mut editor, vec![key(egui::Key::Enter)]);
        match &editor.history.current.annotations[..] {
            [Annotation::Text { text, at, .. }] => {
                assert_eq!(text, "Mon moment\nDeuxième ligne");
                assert!((at.x - 150.0).abs() < 0.1);
            }
            _ => panic!("Un seul texte validé attendu"),
        }
        editor.history.undo();
        assert!(editor.history.current.annotations.is_empty());
    }
    #[test]
    fn escape_cancels_new_text_without_leaving_an_annotation() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Text);
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.center();
        click(&ctx, &mut editor, pos);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::Text("Brouillon".into())],
        );
        frame(&ctx, &mut editor, vec![key(egui::Key::Escape)]);
        assert!(editor.history.current.annotations.is_empty());
        assert!(!editor.history.can_undo());
    }
    #[test]
    fn drawn_shape_can_immediately_be_dragged_from_the_middle() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Frame);
        frame(&ctx, &mut editor, vec![]);
        let a = editor.last_image_rect.min + egui::vec2(200.0, 200.0) * editor.view_scale;
        let b = editor.last_image_rect.min + egui::vec2(500.0, 450.0) * editor.view_scale;
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(a), button(a, true, false)],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(b)]);
        frame(&ctx, &mut editor, vec![button(b, false, false)]);
        let original = editor.history.current.annotations[0].clone();
        editor.tool = Tool::Select;
        let center = (a + b.to_vec2()) * 0.5;
        let end = center + egui::vec2(30.0, 20.0);
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::PointerMoved(center),
                button(center, true, false),
            ],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        assert_eq!(
            editor.history.current.annotations.len(),
            1,
            "Déplacer ne doit pas créer une seconde forme"
        );
        assert_ne!(editor.history.current.annotations[0], original);
        editor.history.undo();
        assert_eq!(editor.history.current.annotations[0], original);
    }
    #[test]
    fn a_drawing_tool_stays_selected_for_successive_shapes_until_changed_manually() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Frame);
        frame(&ctx, &mut editor, vec![]);
        for offset in [100., 350.] {
            let a = editor.last_image_rect.min + egui::vec2(offset, 100.) * editor.view_scale;
            let b = a + egui::vec2(90., 70.) * editor.view_scale;
            frame(
                &ctx,
                &mut editor,
                vec![egui::Event::PointerMoved(a), button(a, true, false)],
            );
            frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(b)]);
            frame(&ctx, &mut editor, vec![button(b, false, false)]);
            assert!(
                editor.tool == Tool::Frame,
                "Creating an annotation must keep its tool selected"
            );
        }
        assert_eq!(editor.history.current.annotations.len(), 2);
        editor.history.undo();
        assert_eq!(editor.history.current.annotations.len(), 1);
        editor.history.redo();
        assert_eq!(editor.history.current.annotations.len(), 2);
        editor.tool = Tool::Select;
        assert!(editor.tool == Tool::Select);
    }
    #[test]
    fn dragging_selected_corner_resizes_instead_of_translating() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        editor.history.current.annotations.push(Annotation::Frame {
            from: Point { x: 200.0, y: 200.0 },
            to: Point { x: 500.0, y: 450.0 },
            color: [255; 4],
            width: 4.0,
            filled: false,
        });
        editor.selected = Some(0);
        frame(&ctx, &mut editor, vec![]);
        let handle = editor.last_image_rect.min
            + egui::vec2(500.0, 450.0) * editor.view_scale
            + egui::vec2(5.0, 5.0);
        let end = handle + egui::vec2(60.0, 40.0);
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::PointerMoved(handle),
                button(handle, true, false),
            ],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        let [min, max] = editor.history.current.annotations[0].bounds().unwrap();
        assert_eq!(min, Point { x: 200.0, y: 200.0 });
        assert!(max.x > 550.0 && max.y > 490.0);
        editor.history.undo();
        assert_eq!(
            editor.history.current.annotations[0].bounds().unwrap()[1],
            Point { x: 500.0, y: 450.0 }
        );
    }
    #[test]
    fn shift_enter_inserts_a_line_and_escape_restores_an_existing_text() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let original = Annotation::Text {
            at: Point { x: 200.0, y: 200.0 },
            text: "Initial".into(),
            color: [255; 4],
            size: 28.0,
        };
        editor.history.current.annotations.push(original.clone());
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.min + egui::vec2(205.0, 205.0) * editor.view_scale;
        click(&ctx, &mut editor, pos);
        click(&ctx, &mut editor, pos);
        frame(&ctx, &mut editor, vec![egui::Event::Text("!".into())]);
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::SHIFT),
                egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: Some(egui::Key::Enter),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::SHIFT,
                },
            ],
        );
        assert!(
            editor
                .text_session
                .as_ref()
                .is_some_and(|s| s.text.contains('\n'))
        );
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                key(egui::Key::Escape),
            ],
        );
        assert_eq!(editor.history.current.annotations, vec![original]);
        assert!(!editor.history.can_undo());
    }
    #[test]
    fn escape_during_drag_restores_the_object_without_an_undo_step() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let original = Annotation::Frame {
            from: Point { x: 200.0, y: 200.0 },
            to: Point { x: 500.0, y: 450.0 },
            color: [255; 4],
            width: 3.0,
            filled: false,
        };
        editor.history.current.annotations.push(original.clone());
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.min + egui::vec2(350.0, 300.0) * editor.view_scale;
        let end = pos + egui::vec2(40.0, 30.0);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(pos), button(pos, true, false)],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut editor, vec![key(egui::Key::Escape)]);
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        assert_eq!(editor.history.current.annotations, vec![original]);
        assert!(!editor.history.can_undo());
    }
    #[test]
    fn handles_at_the_image_edge_can_be_dragged_from_their_visible_center() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        editor.history.current.annotations.push(Annotation::Frame {
            from: Point { x: 0.0, y: 0.0 },
            to: Point { x: 400.0, y: 300.0 },
            color: [255; 4],
            width: 3.0,
            filled: false,
        });
        editor.selected = Some(0);
        frame(&ctx, &mut editor, vec![]);
        let handle = editor.last_image_rect.min - egui::vec2(5.0, 5.0);
        let end = handle + egui::vec2(40.0, 30.0);
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::PointerMoved(handle),
                button(handle, true, false),
            ],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        let [min, max] = editor.history.current.annotations[0].bounds().unwrap();
        assert!(min.x > 40.0 && min.y > 25.0);
        assert!((max.x - 400.0).abs() < 0.001 && (max.y - 300.0).abs() < 0.001);
    }
    #[test]
    fn undo_button_during_text_entry_undoes_the_text_before_the_previous_shape() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let original = Annotation::Text {
            at: Point { x: 200.0, y: 200.0 },
            text: "Initial".into(),
            color: [255; 4],
            size: 28.0,
        };
        editor.history.current.annotations.push(original.clone());
        let mut doc = editor.history.current.clone();
        doc.annotations.push(Annotation::Frame {
            from: Point { x: 500.0, y: 400.0 },
            to: Point { x: 800.0, y: 650.0 },
            color: [255; 4],
            width: 3.0,
            filled: false,
        });
        editor.history.commit(doc);
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.min + egui::vec2(205.0, 205.0) * editor.view_scale;
        click(&ctx, &mut editor, pos);
        click(&ctx, &mut editor, pos);
        frame(&ctx, &mut editor, vec![egui::Event::Text("!".into())]);
        let undo = editor.last_undo_rect.center();
        click(&ctx, &mut editor, undo);
        assert_eq!(
            editor.history.current.annotations.len(),
            2,
            "Annuler la saisie ne doit pas retirer la forme précédente"
        );
        assert_eq!(editor.history.current.annotations[0], original);
        editor.history.redo();
        assert_ne!(editor.history.current.annotations[0], original);
    }
    #[test]
    fn validating_an_emptied_existing_text_removes_it_and_can_be_undone() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Text);
        let original = Annotation::Text {
            at: Point { x: 200.0, y: 200.0 },
            text: "Initial".into(),
            color: [255; 4],
            size: 28.0,
        };
        editor.history.current.annotations.push(original.clone());
        frame(&ctx, &mut editor, vec![]);
        let pos = editor.last_image_rect.min + egui::vec2(205.0, 205.0) * editor.view_scale;
        click(&ctx, &mut editor, pos);
        let control = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::ModifiersChanged(control),
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: Some(egui::Key::A),
                    pressed: true,
                    repeat: false,
                    modifiers: control,
                },
            ],
        );
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                key(egui::Key::Backspace),
            ],
        );
        frame(&ctx, &mut editor, vec![key(egui::Key::Enter)]);
        assert!(editor.history.current.annotations.is_empty());
        editor.history.undo();
        assert_eq!(editor.history.current.annotations, vec![original]);
    }
    #[test]
    fn ctrl_z_uses_the_key_event_modifiers_even_after_ctrl_is_released() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        editor.history.commit(Document {
            crop: None,
            annotations: vec![Annotation::Frame {
                from: Point { x: 200.0, y: 200.0 },
                to: Point { x: 500.0, y: 450.0 },
                color: [255; 4],
                width: 3.0,
                filled: false,
            }],
        });
        frame(&ctx, &mut editor, vec![]);
        frame(
            &ctx,
            &mut editor,
            vec![
                egui::Event::Key {
                    key: egui::Key::Z,
                    physical_key: Some(egui::Key::Z),
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::CTRL,
                },
                egui::Event::ModifiersChanged(egui::Modifiers::NONE),
            ],
        );
        assert!(editor.history.current.annotations.is_empty());
    }
    use super::*;

    fn frame(ctx: &egui::Context, editor: &mut Editor, events: Vec<egui::Event>) -> EditorResult {
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1024.0, 768.0))),
            events,
            ..Default::default()
        });
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("editor-test"),
            egui::UiBuilder::new(),
        );
        let result = editor.draw(&mut ui, ctx);
        let _ = ctx.end_pass();
        result
    }
    fn button(pos: Pos2, pressed: bool, shift: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers {
                shift,
                ..Default::default()
            },
        }
    }
    fn editor(ctx: &egui::Context, tool: Tool) -> Editor {
        let mut editor = Editor::new(
            ctx,
            "test".into(),
            image::RgbaImage::new(1000, 800),
            Document::default(),
            "Capture".into(),
        );
        editor.tool = tool;
        editor
    }

    #[test]
    fn hold_previews_without_saving_and_release_commits_only_once() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Frame);
        frame(&ctx, &mut editor, vec![]);
        let start = egui::pos2(350.0, 300.0);
        let end = egui::pos2(650.0, 500.0);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(start), button(start, true, false)],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        assert!(editor.history.current.annotations.is_empty());
        assert!(
            editor.start.is_some(),
            "Le geste doit avoir un aperçu actif"
        );
        let result = frame(&ctx, &mut editor, vec![button(end, false, false)]);
        assert!(result.save);
        assert_eq!(editor.history.current.annotations.len(), 1);
        assert!(editor.start.is_none());
        assert!(!frame(&ctx, &mut editor, vec![]).save);
        assert_eq!(editor.history.current.annotations.len(), 1);
        editor.history.undo();
        assert!(editor.history.current.annotations.is_empty());
        editor.history.redo();
        assert_eq!(editor.history.current.annotations.len(), 1);
    }

    #[test]
    fn selection_drag_saves_on_release_and_undo_restores_the_whole_object() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let annotation = Annotation::Frame {
            from: Point { x: 200.0, y: 200.0 },
            to: Point { x: 500.0, y: 450.0 },
            color: [255; 4],
            width: 6.0,
            filled: false,
        };
        editor.history.current.annotations.push(annotation.clone());
        frame(&ctx, &mut editor, vec![]);
        let start = editor.last_image_rect.min + egui::vec2(200.0, 300.0) * editor.view_scale;
        let end = start + egui::vec2(80.0, 35.0);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(start), button(start, true, false)],
        );
        let held = frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        assert!(!held.save);
        assert_eq!(editor.selected, Some(0));
        assert_ne!(editor.history.current.annotations[0], annotation);
        assert!(frame(&ctx, &mut editor, vec![button(end, false, false)]).save);
        assert_eq!(editor.history.current.annotations.len(), 1);
        let moved = editor.history.current.annotations[0].clone();
        editor.history.undo();
        assert_eq!(editor.history.current.annotations[0], annotation);
        editor.history.redo();
        assert_eq!(editor.history.current.annotations[0], moved);
    }

    #[test]
    fn settling_a_move_when_closing_still_keeps_an_undo_step() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let original = Annotation::Text {
            at: Point { x: 100.0, y: 100.0 },
            text: "Hello".into(),
            color: [255; 4],
            size: 24.0,
        };
        editor
            .history
            .current
            .annotations
            .push(original.translated_clamped(20.0, 30.0, editor.dimensions));
        editor.selected = Some(0);
        editor.drag_original = Some(original.clone());
        editor.start = Some(Point { x: 100.0, y: 100.0 });
        editor.settle();
        assert!(editor.start.is_none() && editor.drag_original.is_none());
        editor.history.undo();
        assert_eq!(editor.history.current.annotations[0], original);
    }

    #[test]
    fn typing_then_dragging_undoes_the_move_before_the_text_edit() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        editor.history.current.annotations.push(Annotation::Text {
            at: Point { x: 200.0, y: 200.0 },
            text: "Hello".into(),
            color: [255; 4],
            size: 28.0,
        });
        editor.selected = Some(0);
        let initial = editor.history.current.clone();
        frame(&ctx, &mut editor, vec![]);
        let field = editor.last_text_rect.center();
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(field), button(field, true, false)],
        );
        frame(&ctx, &mut editor, vec![button(field, false, false)]);
        assert!(ctx.text_edit_focused());
        frame(&ctx, &mut editor, vec![egui::Event::Text("!".into())]);
        assert!(editor.property_before.is_some());
        let typed = editor.history.current.clone();
        assert_ne!(typed, initial);
        let start = editor.last_image_rect.min + egui::vec2(205.0, 205.0) * editor.view_scale;
        let end = start + egui::vec2(30.0, 25.0);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(start), button(start, true, false)],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        assert_ne!(editor.history.current, typed);
        editor.history.undo();
        assert_eq!(editor.history.current, typed);
        editor.history.undo();
        assert_eq!(editor.history.current, initial);
    }

    #[test]
    fn creation_tools_keep_the_large_defaults_configured_in_settings() {
        for tool in [
            Tool::Brush,
            Tool::Arrow,
            Tool::Frame,
            Tool::Ellipse,
            Tool::Text,
        ] {
            let ctx = egui::Context::default();
            let mut editor = editor(&ctx, tool);
            editor.apply_defaults(&Settings {
                editor_width: 64.0,
                editor_text_size: 200.0,
                ..Default::default()
            });
            frame(&ctx, &mut editor, vec![]);
            assert_eq!(editor.width, 64.0);
            assert_eq!(editor.text_size, 200.0);
        }
    }

    #[test]
    fn document_shortcuts_do_not_interrupt_an_uncommitted_drag() {
        let ctx = egui::Context::default();
        let mut editor = editor(&ctx, Tool::Select);
        let object = Annotation::Frame {
            from: Point { x: 200.0, y: 200.0 },
            to: Point { x: 500.0, y: 450.0 },
            color: [255; 4],
            width: 6.0,
            filled: false,
        };
        editor.history.commit(Document {
            annotations: vec![object.clone()],
            crop: None,
        });
        frame(&ctx, &mut editor, vec![]);
        let start = editor.last_image_rect.min + egui::vec2(200.0, 300.0) * editor.view_scale;
        let end = start + egui::vec2(40.0, 30.0);
        frame(
            &ctx,
            &mut editor,
            vec![egui::Event::PointerMoved(start), button(start, true, false)],
        );
        frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
        let dragged = editor.history.current.clone();
        for key in [
            egui::Key::Z,
            egui::Key::D,
            egui::Key::Delete,
            egui::Key::ArrowRight,
        ] {
            frame(
                &ctx,
                &mut editor,
                vec![
                    egui::Event::ModifiersChanged(egui::Modifiers::CTRL),
                    egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::CTRL,
                    },
                ],
            );
            assert_eq!(editor.history.current, dragged);
        }
        frame(&ctx, &mut editor, vec![button(end, false, false)]);
        editor.history.undo();
        assert_eq!(editor.history.current.annotations, vec![object]);
        editor.history.undo();
        assert!(editor.history.current.annotations.is_empty());
    }

    #[test]
    fn release_keeps_its_position_and_shift_even_if_input_changes_later_in_the_frame() {
        for tool in [Tool::Frame, Tool::Ellipse] {
            let ctx = egui::Context::default();
            let mut editor = editor(&ctx, tool);
            frame(&ctx, &mut editor, vec![]);
            let start = egui::pos2(350.0, 300.0);
            let end = egui::pos2(650.0, 500.0);
            let shifted = egui::Modifiers {
                shift: true,
                ..Default::default()
            };
            frame(
                &ctx,
                &mut editor,
                vec![
                    egui::Event::ModifiersChanged(shifted),
                    egui::Event::PointerMoved(start),
                    button(start, true, true),
                ],
            );
            frame(&ctx, &mut editor, vec![egui::Event::PointerMoved(end)]);
            let expected = editor.end.expect("Aperçu de la forme");
            frame(
                &ctx,
                &mut editor,
                vec![
                    button(end, false, true),
                    egui::Event::ModifiersChanged(egui::Modifiers::NONE),
                    egui::Event::PointerMoved(egui::pos2(730.0, 550.0)),
                ],
            );
            let (from, to) = match editor.history.current.annotations.last().unwrap() {
                Annotation::Frame { from, to, .. } | Annotation::Ellipse { from, to, .. } => {
                    (*from, *to)
                }
                _ => panic!("Forme attendue"),
            };
            assert_eq!(to, expected);
            assert!(((to.x - from.x).abs() - (to.y - from.y).abs()).abs() < 0.001);
        }
    }
}
