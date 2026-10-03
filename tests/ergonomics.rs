use frame_studio::{edit::*, model::*};

#[test]
fn configured_letters_and_modifier_combinations_survive_normalization() {
    let mut settings = Settings {
        capture_key: "shift + ctrl + p".into(),
        studio_key: "alt+s".into(),
        ..Default::default()
    };
    settings.normalize();
    assert_eq!(settings.capture_key, "Ctrl+Maj+P");
    assert_eq!(settings.studio_key, "Alt+S");
    assert!(valid_key("A"));
    assert!(valid_key("Ctrl+F10"));
    for invalid in ["Ctrl", "Ctrl+A+B", "F13", "Escape", "Ctrl++"] {
        assert!(!valid_key(invalid), "{invalid}");
    }
}

#[test]
fn hollow_shapes_can_be_selected_and_moved_from_their_interior() {
    let frame = Annotation::Frame {
        from: Point { x: 20.0, y: 30.0 },
        to: Point { x: 120.0, y: 130.0 },
        color: [255; 4],
        width: 3.0,
        filled: false,
    };
    let ellipse = Annotation::Ellipse {
        from: Point { x: 150.0, y: 30.0 },
        to: Point { x: 250.0, y: 130.0 },
        color: [255; 4],
        width: 3.0,
        filled: false,
    };
    let doc = Document {
        annotations: vec![frame, ellipse],
        crop: None,
    };
    assert_eq!(doc.pick(Point { x: 70.0, y: 80.0 }, 4.0), Some(0));
    assert_eq!(doc.pick(Point { x: 200.0, y: 80.0 }, 4.0), Some(1));
    assert_eq!(doc.pick(Point { x: 151.0, y: 31.0 }, 0.0), None);
}

#[test]
fn multiline_text_has_separate_lines_in_bounds_and_export() {
    let text = Annotation::Text {
        at: Point { x: 10.0, y: 10.0 },
        text: "A\nA".into(),
        color: [255; 4],
        size: 30.0,
    };
    let bounds = text.bounds().unwrap();
    assert!(
        bounds[1].y >= 75.0,
        "La seconde ligne doit faire partie de la sélection"
    );
    let image = render_edit(
        &image::RgbaImage::new(120, 120),
        &Document {
            annotations: vec![text],
            crop: None,
        },
    )
    .unwrap();
    assert!(
        (46..80).any(|y| (10..40).any(|x| image.get_pixel(x, y)[3] > 0)),
        "La seconde ligne doit être exportée"
    );
}

#[test]
fn chords_require_exact_modifiers_and_never_fire_while_typing() {
    let chord = Shortcut::parse("Ctrl+P").unwrap();
    assert!(chord.matches(80, true, false, false, false));
    assert!(!chord.matches(80, false, false, false, false));
    assert!(!chord.matches(80, true, true, false, false));
    assert!(!chord.matches(80, true, false, false, true));
    let mut settings = Settings {
        capture_key: "ctrl+p".into(),
        studio_key: "Ctrl+P".into(),
        ..Default::default()
    };
    settings.normalize();
    assert_eq!(settings.capture_key, "Ctrl+P");
    assert_eq!(settings.studio_key, "F6");
}

#[test]
fn resize_from_top_left_keeps_opposite_corner_and_stays_in_image() {
    let a = Annotation::Frame {
        from: Point { x: 20.0, y: 30.0 },
        to: Point { x: 120.0, y: 130.0 },
        color: [255; 4],
        width: 3.0,
        filled: false,
    };
    let resized = a.resized_corner(0, -100.0, -100.0, false, [200, 200]);
    assert_eq!(
        resized.bounds(),
        Some([Point { x: 0.0, y: 0.0 }, Point { x: 120.0, y: 130.0 }])
    );
    let crossed = a.resized_corner(0, 150.0, 150.0, false, [200, 200]);
    assert_eq!(
        crossed.bounds(),
        Some([Point { x: 119.0, y: 129.0 }, Point { x: 120.0, y: 130.0 }])
    );
}
