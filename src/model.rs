use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviewCorner {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    #[default]
    Png,
    Jpeg,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GallerySort {
    #[default]
    Newest,
    Oldest,
    Title,
}
pub fn clean_title(input: &str) -> Option<String> {
    let title: String = input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(120)
        .collect();
    (!title.is_empty()).then_some(title)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub auto_enabled: bool,
    pub minimum_percent: f32,
    pub maximum_percent: f32,
    pub range_mode: bool,
    pub include_practice: bool,
    pub cooldown_seconds: f64,
    pub flash: bool,
    pub flash_strength: f32,
    pub sound: bool,
    pub show_preview: bool,
    pub preview_seconds: f32,
    pub gallery_tile_size: f32,
    pub capture_key: String,
    pub studio_key: String,
    pub ui_scale: f32,
    pub flash_seconds: f32,
    pub preview_width: f32,
    pub preview_border: f32,
    pub preview_corner: PreviewCorner,
    pub confirm_trash: bool,
    pub open_exports: bool,
    pub export_format: ExportFormat,
    pub jpeg_quality: u8,
    pub gallery_sort: GallerySort,
    pub show_capture_details: bool,
    pub editor_width: f32,
    pub editor_text_size: f32,
    pub editor_color: [u8; 3],
    pub editor_grid: bool,
    pub external_app: String,
    pub auto_copy: bool,
    pub language: crate::i18n::Language,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_enabled: false,
            minimum_percent: 70.0,
            maximum_percent: 100.0,
            range_mode: false,
            include_practice: false,
            cooldown_seconds: 3.0,
            flash: true,
            flash_strength: 0.22,
            sound: true,
            show_preview: true,
            preview_seconds: 3.0,
            gallery_tile_size: 250.0,
            capture_key: "F8".into(),
            studio_key: "F6".into(),
            ui_scale: 1.0,
            flash_seconds: 0.2,
            preview_width: 216.0,
            preview_border: 2.0,
            preview_corner: PreviewCorner::TopRight,
            confirm_trash: true,
            open_exports: true,
            export_format: ExportFormat::Png,
            jpeg_quality: 92,
            gallery_sort: GallerySort::Newest,
            show_capture_details: true,
            editor_width: 6.0,
            editor_text_size: 28.0,
            editor_color: [255; 3],
            editor_grid: false,
            external_app: String::new(),
            auto_copy: false,
            language: Default::default(),
        }
    }
}
impl Settings {
    pub fn normalize(&mut self) {
        fn finite(v: f32, fallback: f32, lo: f32, hi: f32) -> f32 {
            if v.is_finite() {
                v.clamp(lo, hi)
            } else {
                fallback
            }
        }
        self.minimum_percent = finite(self.minimum_percent, 70.0, 0.0, 100.0);
        self.maximum_percent = finite(self.maximum_percent, 100.0, 0.0, 100.0);
        if self.range_mode && self.minimum_percent > self.maximum_percent {
            std::mem::swap(&mut self.minimum_percent, &mut self.maximum_percent);
        }
        self.cooldown_seconds = if self.cooldown_seconds.is_finite() {
            self.cooldown_seconds.clamp(0.0, 120.0)
        } else {
            3.0
        };
        self.flash_strength = finite(self.flash_strength, 0.22, 0.0, 0.5);
        self.preview_seconds = finite(self.preview_seconds, 3.0, 0.5, 10.0);
        self.gallery_tile_size = finite(self.gallery_tile_size, 250.0, 180.0, 360.0);
        self.ui_scale = finite(self.ui_scale, 1.0, 0.7, 1.5);
        self.flash_seconds = finite(self.flash_seconds, 0.2, 0.05, 1.0);
        self.preview_width = finite(self.preview_width, 216.0, 140.0, 420.0);
        self.preview_border = finite(self.preview_border, 2.0, 0.0, 6.0);
        self.editor_width = finite(self.editor_width, 6.0, 1.0, 64.0);
        self.editor_text_size = finite(self.editor_text_size, 28.0, 8.0, 256.0);
        self.jpeg_quality = self.jpeg_quality.clamp(40, 100);
        self.capture_key = Shortcut::parse(&self.capture_key)
            .map(|k| k.label())
            .unwrap_or_else(|| "F8".into());
        self.studio_key = Shortcut::parse(&self.studio_key)
            .map(|k| k.label())
            .unwrap_or_else(|| "F6".into());
        if self.studio_key == self.capture_key {
            self.studio_key = if self.capture_key == "F6" { "F7" } else { "F6" }.into();
        }
    }
}
pub fn valid_key(key: &str) -> bool {
    Shortcut::parse(key).is_some()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub code: u32,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}
impl Shortcut {
    pub fn parse(value: &str) -> Option<Self> {
        let mut shortcut = Self {
            code: 0,
            ctrl: false,
            shift: false,
            alt: false,
        };
        for part in value.split('+').map(|s| s.trim().to_uppercase()) {
            match part.as_str() {
                "CTRL" | "CONTROL" if !shortcut.ctrl => shortcut.ctrl = true,
                "MAJ" | "SHIFT" if !shortcut.shift => shortcut.shift = true,
                "ALT" if !shortcut.alt => shortcut.alt = true,
                _ if shortcut.code == 0 => shortcut.code = key_number(&part)?,
                _ => return None,
            }
        }
        (shortcut.code != 0).then_some(shortcut)
    }
    pub fn from_code(code: u32, ctrl: bool, shift: bool, alt: bool) -> Option<Self> {
        key_name(code).map(|_| Self {
            code,
            ctrl,
            shift,
            alt,
        })
    }
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Maj".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        parts.push(key_name(self.code).unwrap_or_default());
        parts.join("+")
    }
    pub fn matches(
        self,
        code: u32,
        ctrl: bool,
        shift: bool,
        alt: bool,
        typing_or_recording: bool,
    ) -> bool {
        !typing_or_recording
            && self.code == code
            && self.ctrl == ctrl
            && self.shift == shift
            && self.alt == alt
    }
}
fn key_number(value: &str) -> Option<u32> {
    if value.len() == 1 {
        let c = value.as_bytes()[0];
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            return Some(c as u32);
        }
    }
    if let Some(n) = value
        .strip_prefix('F')
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| (1..=12).contains(n))
    {
        return Some(111 + n);
    }
    Some(match value {
        "ESPACE" | "SPACE" => 32,
        "TAB" => 9,
        "INSERT" => 45,
        "SUPPR" | "DELETE" => 46,
        "DÉBUT" | "DEBUT" | "HOME" => 36,
        "FIN" | "END" => 35,
        "PAGEUP" => 33,
        "PAGEDOWN" => 34,
        "GAUCHE" | "LEFT" => 37,
        "HAUT" | "UP" => 38,
        "DROITE" | "RIGHT" => 39,
        "BAS" | "DOWN" => 40,
        _ => return None,
    })
}
fn key_name(code: u32) -> Option<String> {
    if (48..=57).contains(&code) || (65..=90).contains(&code) {
        return char::from_u32(code).map(|c| c.to_string());
    }
    if (112..=123).contains(&code) {
        return Some(format!("F{}", code - 111));
    }
    Some(
        match code {
            32 => "Espace",
            9 => "Tab",
            45 => "Insert",
            46 => "Suppr",
            36 => "Début",
            35 => "Fin",
            33 => "PageUp",
            34 => "PageDown",
            37 => "Gauche",
            38 => "Haut",
            39 => "Droite",
            40 => "Bas",
            _ => return None,
        }
        .into(),
    )
}
#[derive(Default)]
pub struct CaptureBudget {
    inflight: usize,
}
impl CaptureBudget {
    pub fn acquire(&mut self) -> bool {
        if self.inflight >= 2 {
            return false;
        }
        self.inflight += 1;
        true
    }
    pub fn release(&mut self) {
        self.inflight = self.inflight.saturating_sub(1);
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameSnapshot {
    pub level: String,
    pub percent: f32,
    pub attempt: u32,
    pub practice: bool,
    pub playing: bool,
}

#[derive(Default)]
pub struct DeathGate {
    fired: bool,
    last_capture: Option<f64>,
}
impl DeathGate {
    pub fn reset_attempt(&mut self) {
        self.fired = false;
    }
    pub fn accept(&mut self, settings: &Settings, event: &GameSnapshot, now: f64) -> bool {
        if self.fired
            || !settings.auto_enabled
            || !event.playing
            || !now.is_finite()
            || !event.percent.is_finite()
            || !(0.0..=100.0).contains(&event.percent)
            || (event.practice && !settings.include_practice)
            || event.percent < settings.minimum_percent
            || (settings.range_mode && event.percent > settings.maximum_percent)
        {
            return false;
        }
        if self
            .last_capture
            .is_some_and(|last| now - last < settings.cooldown_seconds)
        {
            return false;
        }
        self.fired = true;
        self.last_capture = Some(now);
        true
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureMeta {
    pub id: String,
    pub title: String,
    pub game: GameSnapshot,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub automatic: bool,
    pub favorite: bool,
}
