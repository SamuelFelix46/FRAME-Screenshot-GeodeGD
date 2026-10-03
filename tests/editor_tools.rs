use frame_studio::{edit::*, model::*};

fn rectangle() -> Annotation {
    Annotation::Frame {
        from: Point { x: 20.0, y: 30.0 },
        to: Point { x: 80.0, y: 70.0 },
        color: [255; 4],
        width: 4.0,
        filled: false,
    }
}

#[test]
fn selection_respects_hollow_shapes_and_picks_the_topmost_visible_edge() {
    let lower = rectangle();
    assert!(lower.hit_test(Point { x: 20.0, y: 50.0 }, 3.0));
    assert!(!lower.hit_test(Point { x: 50.0, y: 50.0 }, 3.0));
    let top = Annotation::Stroke {
        points: vec![Point { x: 0.0, y: 50.0 }, Point { x: 100.0, y: 50.0 }],
        color: [255; 4],
        width: 4.0,
    };
    let doc = Document {
        annotations: vec![lower, top],
        crop: None,
    };
    assert_eq!(doc.pick(Point { x: 20.0, y: 50.0 }, 3.0), Some(1));
    assert_eq!(doc.pick(Point { x: 80.0, y: 35.0 }, 3.0), Some(0));
    assert_eq!(doc.pick(Point { x: 150.0, y: 150.0 }, 3.0), None);
}

#[test]
fn dragging_an_object_clamps_the_whole_object_without_changing_its_size() {
    let object = rectangle().translated_clamped(500.0, -100.0, [120, 100]);
    let [min, max] = object.bounds().unwrap();
    assert_eq!(min, Point { x: 59.0, y: 0.0 });
    assert_eq!(max, Point { x: 119.0, y: 40.0 });
    assert_eq!(
        rectangle().translated_clamped(f32::NAN, 0.0, [120, 100]),
        rectangle()
    );
}

#[test]
fn resizing_reversed_shapes_and_strokes_preserves_their_geometry() {
    let shape = Annotation::Ellipse {
        from: Point { x: 80.0, y: 70.0 },
        to: Point { x: 20.0, y: 30.0 },
        color: [255; 4],
        width: 3.0,
        filled: false,
    };
    let resized = shape.resized(120.0, 80.0, [300, 200]);
    assert_eq!(
        resized.bounds().unwrap(),
        [Point { x: 20.0, y: 30.0 }, Point { x: 140.0, y: 110.0 }]
    );
    assert!(resized.hit_test(Point { x: 80.0, y: 30.0 }, 2.0));
    assert!(!resized.hit_test(Point { x: 80.0, y: 70.0 }, 2.0));
    let line = Annotation::Stroke {
        points: vec![Point { x: 10.0, y: 10.0 }, Point { x: 30.0, y: 30.0 }],
        color: [255; 4],
        width: 2.0,
    };
    assert_eq!(
        line.resized(40.0, 60.0, [100, 100]).bounds().unwrap()[1],
        Point { x: 50.0, y: 70.0 }
    );
}

#[test]
fn a_flat_ellipse_does_not_select_points_far_beyond_its_end() {
    let ellipse = Annotation::Ellipse {
        from: Point { x: 20.0, y: 30.0 },
        to: Point { x: 820.0, y: 130.0 },
        color: [255; 4],
        width: 4.0,
        filled: false,
    };
    assert!(ellipse.hit_test(Point { x: 823.0, y: 80.0 }, 3.0));
    assert!(!ellipse.hit_test(Point { x: 850.0, y: 80.0 }, 3.0));
    assert!(!ellipse.hit_test(Point { x: 420.0, y: 80.0 }, 3.0));
}

#[test]
fn centered_crop_presets_are_inside_the_source_and_keep_the_requested_ratio() {
    for (a, b) in [(1, 1), (16, 9), (4, 3), (9, 16)] {
        let crop = centered_crop([1920, 1200], a, b).unwrap();
        assert!(crop.x + crop.width <= 1920 && crop.y + crop.height <= 1200);
        assert!((crop.width as f32 / crop.height as f32 - a as f32 / b as f32).abs() < 0.002);
        assert!((2 * crop.x + crop.width).abs_diff(1920) <= 1);
        assert!((2 * crop.y + crop.height).abs_diff(1200) <= 1);
    }
    assert!(centered_crop([0, 1200], 16, 9).is_none());
    assert!(centered_crop([1920, 1200], 0, 9).is_none());
}

#[test]
fn old_settings_upgrade_and_new_options_recover_invalid_values() {
    let mut settings: Settings =
        serde_json::from_str(r#"{"capture_key":"F9","minimum_percent":82}"#).unwrap();
    assert_eq!(settings.preview_corner, PreviewCorner::TopRight);
    assert_eq!(settings.export_format, ExportFormat::Png);
    assert!(settings.confirm_trash);
    settings.preview_width = f32::NAN;
    settings.preview_border = -10.0;
    settings.flash_seconds = f32::INFINITY;
    settings.editor_width = 1000.0;
    settings.editor_text_size = -5.0;
    settings.jpeg_quality = 0;
    settings.normalize();
    assert_eq!(settings.preview_width, 216.0);
    assert_eq!(settings.preview_border, 0.0);
    assert_eq!(settings.flash_seconds, 0.2);
    assert_eq!(settings.editor_width, 64.0);
    assert_eq!(settings.editor_text_size, 8.0);
    assert_eq!(settings.jpeg_quality, 40);
    assert_eq!(settings.capture_key, "F9");
}

#[test]
fn inline_titles_are_trimmed_single_line_nonempty_and_unicode_safe() {
    assert_eq!(
        clean_title("  mon\n meilleur\t moment  "),
        Some("mon meilleur moment".into())
    );
    assert!(clean_title("\n \t").is_none());
    assert_eq!(clean_title(&"é".repeat(150)).unwrap().chars().count(), 120);
}
