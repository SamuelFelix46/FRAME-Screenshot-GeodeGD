use frame_studio::{edit::*, model::*, store::Store};
use image::{GenericImageView, Rgba, RgbaImage};

#[test]
fn save_as_writes_the_rendered_crop_to_a_chosen_path_and_protects_originals() {
    let root = std::env::temp_dir().join(format!(
        "frame-save-as-{}-{}",
        std::process::id(),
        frame_studio::store::now_ms()
    ));
    let store = Store::new(root.clone()).unwrap();
    let source = RgbaImage::from_pixel(12, 10, Rgba([21, 40, 90, 255]));
    let meta = store
        .save_capture(&source, GameSnapshot::default(), false)
        .unwrap();
    let original_path = store.image_path(&meta.id).unwrap();
    let original = std::fs::read(&original_path).unwrap();
    let doc = Document {
        crop: Some(Crop {
            x: 2,
            y: 2,
            width: 5,
            height: 4,
        }),
        annotations: vec![Annotation::Frame {
            from: Point { x: 2., y: 2. },
            to: Point { x: 6., y: 5. },
            color: [250, 20, 10, 255],
            width: 1.,
            filled: true,
        }],
    };
    for extension in ["png", "jpg"] {
        let path = root.join(format!("my edited picture.{extension}"));
        assert_eq!(store.export_to(&meta.id, &doc, &path, 96).unwrap(), path);
        let result = image::open(&path).unwrap().to_rgba8();
        assert_eq!(result.dimensions(), (5, 4));
        assert!(result.get_pixel(2, 2)[0] > 200);
        // A second save replaces the user's chosen export without touching the source.
        store
            .export_to(&meta.id, &Document::default(), &path, 96)
            .unwrap();
        assert_eq!(image::open(&path).unwrap().dimensions(), (12, 10));
    }
    assert!(store.export_to(&meta.id, &doc, &original_path, 96).is_err());
    assert!(
        store
            .export_to(&meta.id, &doc, &root.join("captures/other.png"), 96)
            .is_err()
    );
    assert!(
        store
            .export_to(&meta.id, &doc, &root.join("unsupported.bmp"), 96)
            .is_err()
    );
    assert_eq!(std::fs::read(original_path).unwrap(), original);
    store.trash(&meta.id).unwrap();
    let recoverable = root.join("trash").join(&meta.id).join("original.png");
    let another = store
        .save_capture(&source, GameSnapshot::default(), false)
        .unwrap();
    assert!(
        store
            .export_to(&another.id, &doc, &recoverable, 96)
            .is_err()
    );
    assert_eq!(std::fs::read(recoverable).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn previous_settings_keep_shortcuts_and_do_not_enable_automatic_clipboard_replacement() {
    let settings: Settings =
        serde_json::from_str(r#"{"capture_key":"F9","studio_key":"F7"}"#).unwrap();
    assert!(!settings.auto_copy);
    assert_eq!(settings.capture_key, "F9");
    assert_eq!(settings.studio_key, "F7");
}
