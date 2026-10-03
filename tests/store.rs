use frame_studio::{edit::*, model::*, store::Store};
use image::{Rgba, RgbaImage};
fn temp() -> Store {
    Store::new(std::env::temp_dir().join(format!(
            "frame-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )))
    .unwrap()
}
#[test]
fn save_reload_edits_and_trash_preserve_original_bytes() {
    let store = temp();
    let image = RgbaImage::from_pixel(8, 6, Rgba([50, 60, 70, 255]));
    let meta = store
        .save_capture(
            &image,
            GameSnapshot {
                level: "Stereo Madness".into(),
                percent: 72.5,
                ..Default::default()
            },
            true,
        )
        .unwrap();
    let path = store.image_path(&meta.id).unwrap();
    let original = std::fs::read(&path).unwrap();
    assert_eq!(store.list().unwrap().0[0].game.percent, 72.5);
    store
        .save_document(
            &meta.id,
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
    assert_eq!(
        store.load_document(&meta.id).unwrap().crop.unwrap().width,
        2
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    let export = store
        .export(&meta.id, &store.load_document(&meta.id).unwrap())
        .unwrap();
    assert!(export.starts_with(store.root.join("exports")));
    assert_ne!(export, path);
    let exported = image::open(export).unwrap().to_rgba8();
    assert_eq!(exported.dimensions(), (2, 2));
    assert_eq!(exported.get_pixel(0, 0).0, [50, 60, 70, 255]);
    assert_eq!(std::fs::read(&path).unwrap(), original);
    store.trash(&meta.id).unwrap();
    assert!(store.list().unwrap().0.is_empty());
    assert!(
        store
            .root
            .join("trash")
            .join(&meta.id)
            .join("original.png")
            .is_file()
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn corrupt_metadata_does_not_hide_good_captures_and_ids_cannot_escape_root() {
    let store = temp();
    store
        .save_capture(&RgbaImage::new(3, 3), GameSnapshot::default(), false)
        .unwrap();
    std::fs::write(store.root.join("captures").join("broken.json"), "{broken").unwrap();
    let (items, corrupt) = store.list().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(corrupt, 1);
    assert!(store.image_path("../../outside").is_err());
    assert!(
        store
            .save_document("/absolute", &Document::default())
            .is_err()
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}

#[test]
fn jpeg_and_png_exports_keep_crop_dimensions_and_original_bytes() {
    let store = temp();
    let source = RgbaImage::from_pixel(32, 24, Rgba([40, 80, 120, 255]));
    let meta = store
        .save_capture(&source, GameSnapshot::default(), false)
        .unwrap();
    let original = std::fs::read(store.image_path(&meta.id).unwrap()).unwrap();
    let doc = Document {
        crop: Some(Crop {
            x: 4,
            y: 3,
            width: 20,
            height: 16,
        }),
        annotations: vec![],
    };
    for format in [ExportFormat::Png, ExportFormat::Jpeg] {
        let path = store.export_as(&meta.id, &doc, format, 95).unwrap();
        assert_eq!(
            path.extension().unwrap(),
            if format == ExportFormat::Png {
                "png"
            } else {
                "jpg"
            }
        );
        let output = image::open(&path).unwrap().to_rgb8();
        assert_eq!(output.dimensions(), (20, 16));
        for (actual, expected) in output.get_pixel(5, 5).0.into_iter().zip([40u8, 80, 120]) {
            assert!(actual.abs_diff(expected) <= 3);
        }
    }
    assert_eq!(
        std::fs::read(store.image_path(&meta.id).unwrap()).unwrap(),
        original
    );
    std::fs::remove_dir_all(store.root).unwrap();
}
