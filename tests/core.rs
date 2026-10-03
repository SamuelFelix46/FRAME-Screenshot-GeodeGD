use frame_studio::edit::*;
use frame_studio::model::*;
use image::{Rgba, RgbaImage};

fn death(percent: f32, practice: bool) -> GameSnapshot {
    GameSnapshot {
        percent,
        practice,
        playing: true,
        ..Default::default()
    }
}
fn auto() -> Settings {
    Settings {
        auto_enabled: true,
        ..Default::default()
    }
}

#[test]
fn threshold_includes_exact_boundary_and_rejects_below() {
    let s = auto();
    let mut g = DeathGate::default();
    assert!(!g.accept(&s, &death(69.99, false), 10.0));
    assert!(g.accept(&s, &death(70.0, false), 10.0));
}
#[test]
fn dual_death_fires_once_and_new_attempt_can_fire_after_cooldown() {
    let s = auto();
    let mut g = DeathGate::default();
    assert!(g.accept(&s, &death(85.0, false), 10.0));
    assert!(!g.accept(&s, &death(85.0, false), 10.01));
    g.reset_attempt();
    assert!(!g.accept(&s, &death(90.0, false), 12.99));
    assert!(g.accept(&s, &death(90.0, false), 13.0));
}
#[test]
fn range_is_inclusive_and_practice_filter_is_respected() {
    let mut s = auto();
    s.range_mode = true;
    s.minimum_percent = 42.0;
    s.maximum_percent = 45.0;
    let mut g = DeathGate::default();
    assert!(!g.accept(&s, &death(46.0, false), 1.0));
    assert!(!g.accept(&s, &death(42.0, true), 1.0));
    assert!(g.accept(&s, &death(45.0, false), 1.0));
    g.reset_attempt();
    s.include_practice = true;
    assert!(g.accept(&s, &death(42.0, true), 5.0));
}
#[test]
fn invalid_progress_and_disabled_auto_never_capture() {
    let mut g = DeathGate::default();
    let s = auto();
    for p in [f32::NAN, f32::INFINITY, -1.0, 101.0] {
        assert!(!g.accept(&s, &death(p, false), 10.0));
    }
    assert!(!g.accept(&Settings::default(), &death(90.0, false), 10.0));
    assert!(!g.accept(
        &s,
        &GameSnapshot {
            percent: 90.0,
            ..Default::default()
        },
        10.0
    ));
}
#[test]
fn normalization_recovers_nonfinite_and_reversed_bounds() {
    let mut s = auto();
    s.minimum_percent = 120.0;
    s.maximum_percent = -5.0;
    s.cooldown_seconds = f64::NAN;
    s.range_mode = true;
    s.capture_key = "garbage".into();
    s.flash_strength = f32::NAN;
    s.normalize();
    assert_eq!((s.minimum_percent, s.maximum_percent), (0.0, 100.0));
    assert!(s.cooldown_seconds.is_finite());
    assert!(s.flash_strength.is_finite());
    assert_eq!(s.capture_key, "F8");
}

#[test]
fn previous_settings_gain_preview_and_gallery_defaults_without_losing_shortcuts() {
    let old: Settings = serde_json::from_str(
        r#"{"capture_key":"F9","studio_key":"F7","auto_enabled":true,"minimum_percent":82}"#,
    )
    .unwrap();
    assert!(old.show_preview);
    assert_eq!(old.gallery_tile_size, 250.0);
    assert_eq!(old.capture_key, "F9");
    assert_eq!(old.minimum_percent, 82.0);
    let mut next = old;
    next.show_preview = false;
    next.gallery_tile_size = f32::NAN;
    next.normalize();
    assert_eq!(next.gallery_tile_size, 250.0);
    let restored: Settings = serde_json::from_str(&serde_json::to_string(&next).unwrap()).unwrap();
    assert!(!restored.show_preview);
}

#[test]
fn threshold_normalization_ignores_previous_range_maximum() {
    let mut s = Settings {
        range_mode: false,
        minimum_percent: 95.0,
        maximum_percent: 45.0,
        ..Default::default()
    };
    s.normalize();
    assert_eq!(s.minimum_percent, 95.0);
}
#[test]
fn full_image_crop_includes_last_row_and_column() {
    let crop = crop_from_drag(
        Point { x: 0.0, y: 0.0 },
        Point { x: 100.0, y: 80.0 },
        [100, 80],
    )
    .unwrap();
    assert_eq!(
        crop,
        Crop {
            x: 0,
            y: 0,
            width: 100,
            height: 80
        }
    );
    let original = RgbaImage::from_pixel(100, 80, Rgba([20, 30, 40, 255]));
    assert_eq!(
        render_edit(
            &original,
            &Document {
                crop: Some(crop),
                ..Default::default()
            }
        )
        .unwrap()
        .dimensions(),
        (100, 80)
    );
}
#[test]
fn reverse_edge_crop_stays_inside_image_and_zero_size_is_rejected() {
    assert_eq!(
        crop_from_drag(
            Point { x: 100.0, y: 80.0 },
            Point { x: 90.0, y: 70.0 },
            [100, 80]
        )
        .unwrap(),
        Crop {
            x: 90,
            y: 70,
            width: 10,
            height: 10
        }
    );
    assert!(crop_from_drag(Point { x: 1.0, y: 1.0 }, Point { x: 2.0, y: 2.0 }, [0, 0]).is_none());
}
#[test]
fn capture_budget_caps_raw_images_and_recovers_on_completion() {
    let mut budget = CaptureBudget::default();
    assert!(budget.acquire());
    assert!(budget.acquire());
    assert!(!budget.acquire());
    budget.release();
    assert!(budget.acquire());
    assert!(!budget.acquire());
    budget.release();
    budget.release();
    budget.release();
    assert!(budget.acquire());
}
#[test]
fn crop_uses_source_coordinates_and_never_mutates_original() {
    let original = RgbaImage::from_fn(4, 3, |x, y| Rgba([x as u8, y as u8, 0, 255]));
    let before = original.clone();
    let out = render_edit(
        &original,
        &Document {
            crop: Some(Crop {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            }),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(out.dimensions(), (2, 2));
    assert_eq!(out.get_pixel(0, 0).0, [1, 1, 0, 255]);
    assert_eq!(original, before);
}
#[test]
fn crop_outside_image_is_rejected_without_panicking() {
    let original = RgbaImage::new(4, 3);
    for c in [
        Crop {
            x: 5,
            y: 0,
            width: 1,
            height: 1,
        },
        Crop {
            x: 0,
            y: 0,
            width: 0,
            height: 1,
        },
        Crop {
            x: u32::MAX,
            y: 0,
            width: 2,
            height: 1,
        },
    ] {
        assert!(
            render_edit(
                &original,
                &Document {
                    crop: Some(c),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn arrow_and_brush_export_pixels_at_original_coordinates() {
    let original = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 255]));
    let doc = Document {
        annotations: vec![
            Annotation::Stroke {
                points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 20.0, y: 10.0 }],
                color: [255, 0, 0, 255],
                width: 3.0,
            },
            Annotation::Arrow {
                from: Point { x: 30.0, y: 30.0 },
                to: Point { x: 50.0, y: 30.0 },
                color: [0, 255, 0, 255],
                width: 2.0,
            },
        ],
        ..Default::default()
    };
    let out = render_edit(&original, &doc).unwrap();
    assert_eq!(out.get_pixel(15, 10).0, [255, 0, 0, 255]);
    assert_eq!(out.get_pixel(40, 30).0, [0, 255, 0, 255]);
    assert_eq!(original.get_pixel(15, 10).0, [0, 0, 0, 255]);
}
#[test]
fn history_undo_redo_and_new_branch_discard_future() {
    let mut h = History::default();
    h.commit(Document {
        crop: Some(Crop {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        }),
        ..Default::default()
    });
    h.undo();
    assert!(h.current.crop.is_none());
    h.redo();
    assert_eq!(h.current.crop.unwrap().x, 1);
    h.undo();
    h.commit(Document {
        crop: Some(Crop {
            x: 2,
            y: 1,
            width: 1,
            height: 1,
        }),
        ..Default::default()
    });
    h.redo();
    assert_eq!(h.current.crop.unwrap().x, 2);
}
