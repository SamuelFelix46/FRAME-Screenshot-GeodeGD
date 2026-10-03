use crate::{
    model::*,
    store::Store,
    worker::{self, Command},
};
use geode_rs::{
    classes::{
        CCEGLView, CCIMEDispatcher, CCKeyboardDispatcher, GameObject, PlayLayer, PlayerObject,
    },
    cocos::enumKeyCodes,
    geode_main, modify,
};
use image::RgbaImage;
use std::{
    sync::{Arc, Mutex, OnceLock, mpsc::SyncSender},
    time::Instant,
};

geode_egui::install_input_hooks!();

struct Bridge {
    settings: Settings,
    game: GameSnapshot,
    gate: DeathGate,
    budget: CaptureBudget,
    sender: Option<SyncSender<Command>>,
    pending: bool,
    toggle: bool,
    pending_death: Option<GameSnapshot>,
    error: Option<String>,
    typing: bool,
    recording: bool,
    recorded: Option<String>,
    consumed_keys: Vec<u32>,
    chosen_app: Option<Option<String>>,
    choosing_app: bool,
    chosen_export: Option<Option<std::path::PathBuf>>,
    choosing_export: bool,
}
impl Default for Bridge {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            game: GameSnapshot::default(),
            gate: DeathGate::default(),
            budget: CaptureBudget::default(),
            sender: None,
            pending: false,
            toggle: false,
            pending_death: None,
            error: None,
            typing: false,
            recording: false,
            recorded: None,
            consumed_keys: Vec::new(),
            chosen_app: None,
            choosing_app: false,
            chosen_export: None,
            choosing_export: false,
        }
    }
}
fn bridge() -> std::sync::MutexGuard<'static, Bridge> {
    static BRIDGE: OnceLock<Mutex<Bridge>> = OnceLock::new();
    BRIDGE
        .get_or_init(|| Mutex::new(Bridge::default()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}
pub fn seconds() -> f64 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_secs_f64()
}
pub fn request_capture() {
    bridge().pending = true;
}
pub fn finish_capture() {
    bridge().budget.release();
}
pub fn set_settings(settings: Settings) {
    let mut b = bridge();
    if !settings.auto_enabled {
        b.pending_death = None;
    }
    b.settings = settings;
}
pub fn game_text_focused() -> bool {
    CCIMEDispatcher::shared_dispatcher().has_delegate()
}
pub fn take_toggle() -> bool {
    std::mem::take(&mut bridge().toggle)
}
pub fn take_error() -> Option<String> {
    bridge().error.take()
}
pub fn keyboard_context(typing: bool, recording: bool) {
    let mut b = bridge();
    b.typing = typing;
    b.recording = recording;
}
pub fn take_recorded_key() -> Option<String> {
    bridge().recorded.take()
}
pub fn take_chosen_app() -> Option<Option<String>> {
    bridge().chosen_app.take()
}
pub fn take_export_path() -> Option<Option<std::path::PathBuf>> {
    bridge().chosen_export.take()
}
pub fn choose_export(format: ExportFormat, id: &str) -> bool {
    {
        let mut b = bridge();
        if b.choosing_export {
            return false;
        }
        b.choosing_export = true;
    }
    let filename = format!(
        "{id}-edited.{}",
        if format == ExportFormat::Png {
            "png"
        } else {
            "jpg"
        }
    );
    std::thread::spawn(move || {
        let result = file_dialog(Some((format, filename))).map(std::path::PathBuf::from);
        let mut b = bridge();
        b.chosen_export = Some(result);
        b.choosing_export = false;
    });
    true
}
pub fn choose_app() {
    {
        let mut b = bridge();
        if b.choosing_app {
            return;
        }
        b.choosing_app = true;
    }
    std::thread::spawn(|| {
        let result = file_dialog(None);
        let mut b = bridge();
        b.chosen_app = Some(result);
        b.choosing_app = false;
    });
}
fn file_dialog(save: Option<(ExportFormat, String)>) -> Option<String> {
    #[repr(C)]
    struct OpenFile {
        size: u32,
        owner: *mut std::ffi::c_void,
        instance: *mut std::ffi::c_void,
        filter: *const u16,
        custom_filter: *mut u16,
        max_custom_filter: u32,
        filter_index: u32,
        file: *mut u16,
        max_file: u32,
        file_title: *mut u16,
        max_file_title: u32,
        initial_dir: *const u16,
        title: *const u16,
        flags: u32,
        file_offset: u16,
        file_extension: u16,
        default_extension: *const u16,
        custom_data: isize,
        hook: *mut std::ffi::c_void,
        template: *const u16,
        reserved: *mut std::ffi::c_void,
        reserved_word: u32,
        flags_ex: u32,
    }
    #[link(name = "comdlg32")]
    unsafe extern "system" {
        fn GetOpenFileNameW(info: *mut OpenFile) -> i32;
        fn GetSaveFileNameW(info: *mut OpenFile) -> i32;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> *mut std::ffi::c_void;
    }
    let filter: Vec<u16> = if save.is_some() {
        "PNG (*.png)\0*.png\0JPEG (*.jpg)\0*.jpg\0\0"
    } else {
        "Applications Windows (*.exe)\0*.exe\0\0"
    }
    .encode_utf16()
    .collect();
    let title: Vec<u16> = if save.is_some() {
        crate::i18n::text(
            "Enregistrer la copie modifiée sous…",
            "Save edited copy as…",
        )
    } else {
        crate::i18n::text(
            "Choisir l’application pour les exports FRAME",
            "Choose an application for FRAME exports",
        )
    }
    .encode_utf16()
    .chain([0])
    .collect();
    let mut file = vec![0u16; 32768];
    if let Some((_, name)) = &save {
        for (i, c) in name.encode_utf16().take(file.len() - 1).enumerate() {
            file[i] = c;
        }
    }
    let extension: Vec<u16> = if save.as_ref().is_some_and(|(f, _)| *f == ExportFormat::Jpeg) {
        "jpg\0"
    } else {
        "png\0"
    }
    .encode_utf16()
    .collect();
    // Windows x64 OPENFILENAMEW: every optional field starts null, buffers remain alive for the call.
    let mut info: OpenFile = unsafe { std::mem::zeroed() };
    info.size = std::mem::size_of::<OpenFile>() as u32;
    info.owner = unsafe { GetForegroundWindow() };
    info.filter = filter.as_ptr();
    info.title = title.as_ptr();
    info.file = file.as_mut_ptr();
    info.max_file = file.len() as u32;
    info.filter_index = if save.as_ref().is_some_and(|(f, _)| *f == ExportFormat::Jpeg) {
        2
    } else {
        1
    };
    info.default_extension = extension.as_ptr();
    info.flags = 0x00080000 | 0x00001000 | 0x00000800 | 0x00000008; // Explorer, existing file/path, don't change game working directory.
    let success = if save.is_some() {
        info.flags = 0x00080000 | 0x00000800 | 0x00000008 | 0x00000002;
        unsafe { GetSaveFileNameW(&mut info) }
    } else {
        unsafe { GetOpenFileNameW(&mut info) }
    };
    if success == 0 {
        return None;
    }
    let length = file.iter().position(|c| *c == 0)?;
    Some(String::from_utf16_lossy(&file[..length]))
}

#[geode_main]
fn main() {
    let root = geode_rs::Mod::get()
        .and_then(|current| current.config_dir(true))
        .or_else(|| {
            std::env::current_exe().ok().and_then(|exe| {
                exe.parent()
                    .map(|parent| parent.join("geode/config/zemci.frame"))
            })
        });
    let Some(root) = root else {
        eprintln!("[FRAME] Could not locate the Geode configuration directory");
        return;
    };
    match Store::new(root) {
        Ok(store) => {
            let loaded = store.load_settings();
            let settings = loaded.as_ref().cloned().unwrap_or_default();
            crate::i18n::set_language(settings.language);
            let (sender, events) = worker::start(store.clone());
            {
                let mut b = bridge();
                b.settings = settings.clone();
                b.sender = Some(sender.clone());
                if let Err(e) = loaded {
                    b.error = Some(crate::translated_format!(
                        "Réglages illisibles : {e}",
                        "Unable to read settings: {e}"
                    ));
                }
            }
            let mut studio = crate::ui::Studio::new(store, sender, events, settings);
            geode_egui::set_ui(move |ctx| studio.draw(ctx));
            eprintln!("[FRAME] Native Rust studio ready · GD 2.2081 / Geode 5.10.1");
        }
        Err(e) => eprintln!("[FRAME] Could not initialize local store: {e}"),
    }
}

#[derive(Default)]
#[modify(CCEGLView)]
struct FrameRenderer;
#[modify(CCEGLView)]
impl FrameRenderer {
    fn swap_buffers(&mut self, this: &mut CCEGLView) {
        before_ui();
        geode_egui::paint_frame();
        this.swap_buffers();
    }
}
fn before_ui() {
    capture_requests(&mut bridge(), read_frame);
}
fn capture_requests(b: &mut Bridge, read: impl FnOnce() -> Result<RgbaImage, String>) {
    let pending = std::mem::take(&mut b.pending);
    let death = b.pending_death.take();
    if !pending && death.is_none() {
        return;
    }
    let automatic = death.is_some();
    let game = death.unwrap_or_else(|| b.game.clone());
    match read() {
        Ok(image) => {
            submit(b, Arc::new(image), game, automatic);
        }
        Err(e) => {
            b.error = Some(e);
        }
    }
}
fn queue_death(b: &mut Bridge, event: GameSnapshot, was_dead: bool, is_dead: bool, now: f64) {
    if !was_dead && is_dead && b.gate.accept(&b.settings, &event, now) {
        // Read the newly rendered death frame once, before FRAME draws its own UI.
        b.pending_death = Some(event);
    }
}
fn reset_capture_attempt(b: &mut Bridge) {
    b.gate.reset_attempt();
    b.pending_death = None;
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn enabling_auto_capture_never_reads_frames_while_player_is_alive() {
        let mut b = Bridge::default();
        b.settings.auto_enabled = true;
        b.game.playing = true;
        let reads = Cell::new(0);
        for _ in 0..1000 {
            capture_requests(&mut b, || {
                reads.set(reads.get() + 1);
                Ok(RgbaImage::new(8, 8))
            });
        }
        assert_eq!(
            reads.get(),
            0,
            "No image read is needed until an actual death"
        );
    }

    #[test]
    fn confirmed_death_reads_one_rendered_frame_with_frozen_metadata_and_no_dual_duplicate() {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let event = GameSnapshot {
            level: "Stereo Madness".into(),
            percent: 82.5,
            attempt: 7,
            playing: true,
            ..Default::default()
        };
        let mut b = Bridge {
            settings: Settings {
                auto_enabled: true,
                ..Default::default()
            },
            game: event.clone(),
            sender: Some(tx),
            ..Default::default()
        };
        queue_death(&mut b, event.clone(), false, false, 10.0);
        capture_requests(&mut b, || panic!("Rejected death must not read an image"));
        assert!(rx.try_recv().is_err());

        queue_death(&mut b, event.clone(), false, true, 10.0);
        queue_death(&mut b, event.clone(), false, true, 10.01);
        queue_death(&mut b, event, true, true, 10.02);
        b.game.percent = 0.0;
        b.game.attempt = 8;
        let rendered_death = RgbaImage::from_pixel(8, 8, image::Rgba([40, 70, 90, 255]));
        let reads = Cell::new(0);
        capture_requests(&mut b, || {
            reads.set(reads.get() + 1);
            Ok(rendered_death.clone())
        });
        let Command::Capture(image, metadata, automatic) = rx.try_recv().unwrap() else {
            panic!("Expected automatic capture");
        };
        assert!(automatic);
        assert_eq!(metadata.percent, 82.5);
        assert_eq!(metadata.attempt, 7);
        assert_eq!(*image, rendered_death);
        for _ in 0..1000 {
            capture_requests(&mut b, || {
                reads.set(reads.get() + 1);
                Ok(RgbaImage::new(8, 8))
            });
        }
        assert_eq!(reads.get(), 1);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn reset_cancels_an_unrendered_death_and_keeps_manual_capture_working() {
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let event = GameSnapshot {
            percent: 90.0,
            playing: true,
            ..Default::default()
        };
        let mut b = Bridge {
            settings: Settings {
                auto_enabled: true,
                ..Default::default()
            },
            sender: Some(tx),
            ..Default::default()
        };
        queue_death(&mut b, event.clone(), false, true, 10.0);
        reset_capture_attempt(&mut b);
        capture_requests(&mut b, || {
            panic!("Never capture another attempt for an old death")
        });
        queue_death(&mut b, event, false, true, 13.0);
        capture_requests(&mut b, || Ok(RgbaImage::new(8, 8)));
        assert!(matches!(
            rx.try_recv().unwrap(),
            Command::Capture(_, _, true)
        ));
        b.budget.release();
        b.game.attempt = 9;
        b.pending = true;
        capture_requests(&mut b, || Ok(RgbaImage::new(8, 8)));
        let Command::Capture(_, metadata, automatic) = rx.try_recv().unwrap() else {
            panic!()
        };
        assert!(!automatic);
        assert_eq!(metadata.attempt, 9);
        capture_requests(&mut b, || panic!("Manual capture also reads only once"));
    }
}
fn submit(b: &mut Bridge, image: Arc<RgbaImage>, game: GameSnapshot, auto: bool) {
    if !b.budget.acquire() {
        b.error = Some(
            crate::i18n::text(
                "Deux captures sont déjà en cours. Réessaie dans un instant.",
                "Two screenshots are already being processed. Try again in a moment.",
            )
            .into(),
        );
        return;
    }
    if b.sender
        .as_ref()
        .is_none_or(|tx| tx.try_send(Command::Capture(image, game, auto)).is_err())
    {
        b.budget.release();
        b.error = Some(
            crate::i18n::text(
                "FRAME est occupé. Réessaie dans un instant.",
                "FRAME is busy. Try again in a moment.",
            )
            .into(),
        );
    }
}
fn snapshot(this: &mut PlayLayer) -> GameSnapshot {
    let level = unsafe { this.level.as_ref() }
        .map(|l| {
            use std::fmt::Write;
            let mut name = String::new();
            if write!(&mut name, "{}", l.level_name).is_err() || name.is_empty() {
                "Niveau".into()
            } else {
                name
            }
        })
        .unwrap_or_else(|| "Niveau".into());
    GameSnapshot {
        level,
        percent: this.get_current_percent(),
        attempt: this.attempts.max(0) as u32,
        practice: this.is_practice_mode,
        playing: true,
    }
}
#[derive(Default)]
#[modify(PlayLayer)]
struct FramePlayLayer;
#[modify(PlayLayer)]
impl FramePlayLayer {
    fn post_update(&mut self, this: &mut PlayLayer, dt: f32) {
        this.post_update(dt);
        bridge().game = snapshot(this);
    }
    fn reset_level(&mut self, this: &mut PlayLayer) {
        {
            let mut b = bridge();
            reset_capture_attempt(&mut b);
        }
        this.reset_level();
        bridge().game = snapshot(this);
    }
    fn on_exit(&mut self, this: &mut PlayLayer) {
        {
            let mut b = bridge();
            b.game.playing = false;
            b.pending_death = None;
        }
        this.on_exit();
    }
    fn destroy_player(
        &mut self,
        this: &mut PlayLayer,
        player: *mut PlayerObject,
        object: *mut GameObject,
    ) {
        let event = snapshot(this);
        // GD permits null arguments, including a null hazard for forced deaths.
        // Forward raw pointers rather than creating references to null objects.
        let actual = if player.is_null() {
            this.player1
        } else {
            player
        };
        let was_dead = unsafe { actual.as_ref() }.is_none_or(|p| p.is_dead);
        let original: extern "C" fn(*mut PlayLayer, *mut PlayerObject, *mut GameObject) =
            unsafe { std::mem::transmute(PlayLayer::DESTROY_PLAYER_ADDR()) };
        original(this, player, object);
        let is_dead = unsafe { actual.as_ref() }.is_some_and(|p| p.is_dead);
        queue_death(&mut bridge(), event, was_dead, is_dead, seconds());
    }
}
#[derive(Default)]
#[modify(CCKeyboardDispatcher)]
struct FrameKeys;
#[modify(CCKeyboardDispatcher)]
impl FrameKeys {
    fn dispatch_keyboard_msg(
        &mut self,
        this: &mut CCKeyboardDispatcher,
        key: enumKeyCodes,
        down: bool,
        repeat: bool,
        time: f64,
    ) -> bool {
        let code = key as u32;
        let game_typing = game_text_focused();
        {
            let mut b = bridge();
            if let Some(index) = b.consumed_keys.iter().position(|k| *k == code) {
                if !down {
                    b.consumed_keys.swap_remove(index);
                }
                return false;
            }
            let modifiers = geode_egui::modifiers();
            if b.recording && down && !repeat {
                if let Some(chord) =
                    Shortcut::from_code(code, modifiers.ctrl, modifiers.shift, modifiers.alt)
                {
                    b.recorded = Some(chord.label());
                    b.consumed_keys.push(code);
                    return false;
                }
            }
            let blocked = b.typing || b.recording || game_typing;
            let capture = Shortcut::parse(&b.settings.capture_key).is_some_and(|k| {
                k.matches(
                    code,
                    modifiers.ctrl,
                    modifiers.shift,
                    modifiers.alt,
                    blocked,
                )
            });
            let studio = Shortcut::parse(&b.settings.studio_key).is_some_and(|k| {
                k.matches(
                    code,
                    modifiers.ctrl,
                    modifiers.shift,
                    modifiers.alt,
                    blocked,
                )
            });
            if down && (capture || studio) {
                if down && !repeat {
                    b.consumed_keys.push(code);
                    if capture {
                        b.pending = true;
                    } else {
                        b.toggle = true;
                    }
                }
                return false;
            }
        }
        geode_egui::dispatch_keyboard_msg_hook(this, key, down, repeat, time)
    }
}

#[link(name = "opengl32")]
unsafe extern "system" {
    fn glGetIntegerv(name: u32, value: *mut i32);
    fn glReadPixels(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        format: u32,
        ty: u32,
        pixels: *mut std::ffi::c_void,
    );
    fn glPixelStorei(name: u32, value: i32);
    fn wglGetProcAddress(name: *const i8) -> *const std::ffi::c_void;
}
fn read_frame() -> Result<RgbaImage, String> {
    unsafe {
        let mut vp = [0i32; 4];
        glGetIntegerv(0x0ba2, vp.as_mut_ptr());
        let (w, h) = (vp[2], vp[3]);
        if w <= 0 || h <= 0 || w > 7680 || h > 4320 || (w as u64) * (h as u64) > 16_777_216 {
            return Err(crate::i18n::text(
                "La résolution de capture est invalide ou dépasse 16 mégapixels.",
                "Capture resolution is invalid or exceeds 16 megapixels.",
            )
            .into());
        }
        let names = [0x0d05, 0x0d02, 0x0d03, 0x0d04, 0x0d00, 0x0d01];
        let mut previous = [0i32; 6];
        for (i, name) in names.iter().enumerate() {
            glGetIntegerv(*name, &mut previous[i]);
            glPixelStorei(*name, if i == 0 { 1 } else { 0 });
        }
        let mut pack_buffer = 0;
        glGetIntegerv(0x88ed, &mut pack_buffer);
        let ptr = wglGetProcAddress(c"glBindBuffer".as_ptr());
        let bind: Option<unsafe extern "system" fn(u32, u32)> =
            if ptr as usize > 3 && ptr as usize != usize::MAX {
                Some(std::mem::transmute(ptr))
            } else {
                None
            };
        if pack_buffer != 0 {
            if let Some(f) = bind {
                f(0x88eb, 0);
            } else {
                for (i, name) in names.iter().enumerate() {
                    glPixelStorei(*name, previous[i]);
                }
                return Err(crate::i18n::text(
                    "Le buffer de capture OpenGL est indisponible.",
                    "The OpenGL capture buffer is unavailable.",
                )
                .into());
            }
        }
        let stride = w as usize * 4;
        let mut data = vec![0u8; stride * h as usize];
        glReadPixels(vp[0], vp[1], w, h, 0x1908, 0x1401, data.as_mut_ptr().cast());
        if pack_buffer != 0 {
            if let Some(f) = bind {
                f(0x88eb, pack_buffer as u32);
            }
        }
        for (i, name) in names.iter().enumerate() {
            glPixelStorei(*name, previous[i]);
        }
        for y in 0..h as usize / 2 {
            let offset = (h as usize - 1 - y) * stride;
            let (a, b) = data.split_at_mut(offset);
            a[y * stride..(y + 1) * stride].swap_with_slice(&mut b[..stride]);
        }
        for pixel in data.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
        RgbaImage::from_raw(w as u32, h as u32, data).ok_or_else(|| {
            crate::i18n::text("La capture OpenGL a échoué.", "OpenGL capture failed.").into()
        })
    }
}
#[link(name = "winmm")]
unsafe extern "system" {
    fn PlaySoundW(sound: *const u16, module: *mut std::ffi::c_void, flags: u32) -> i32;
}
pub fn play_shutter() {
    static WAV: OnceLock<Vec<u32>> = OnceLock::new();
    let data = WAV.get_or_init(|| {
        let bytes = include_bytes!("../assets/shutter.wav");
        let mut data = vec![0u32; bytes.len().div_ceil(4)];
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data.as_mut_ptr().cast(), bytes.len());
        }
        data
    });
    unsafe {
        PlaySoundW(
            data.as_ptr().cast(),
            std::ptr::null_mut(),
            0x0004 | 0x0001 | 0x0002 | 0x2000,
        );
    }
}
pub fn open_path(path: &std::path::Path) {
    let _ = std::process::Command::new("explorer.exe").arg(path).spawn();
}
pub fn open_in_app(application: &str, path: &std::path::Path) -> Result<(), String> {
    let application = std::path::Path::new(application.trim().trim_matches('"'));
    if !application.is_absolute()
        || !application.is_file()
        || !application
            .extension()
            .is_some_and(|s| s.eq_ignore_ascii_case("exe"))
    {
        return Err(crate::i18n::text("Application introuvable : indique le chemin complet d’un fichier .exe dans les réglages Galerie et export.", "Application not found: enter the full path to an .exe file in Gallery and export settings.").into());
    }
    std::process::Command::new(application)
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            crate::translated_format!(
                "Impossible d’ouvrir cette application : {e}",
                "Could not open this application: {e}"
            )
        })
}
