use frame_studio::edit::{Document, render_edit};
use image::{Rgba, RgbaImage};

fn shape(kind: &str, filled: bool, reverse: bool) -> Document {
    let (from, to) = if reverse {
        ((44, 40), (12, 8))
    } else {
        ((12, 8), (44, 40))
    };
    serde_json::from_value(serde_json::json!({
        "crop": null,
        "annotations": [{kind: {
            "from":{"x":from.0,"y":from.1}, "to":{"x":to.0,"y":to.1},
            "color":[255,180,40,255], "width":2, "filled":filled
        }}]
    }))
    .expect("Les formes enregistrées doivent pouvoir être relues")
}

#[test]
fn frame_exports_four_edges_with_empty_center_and_preserves_original() {
    let original = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255]));
    let before = original.clone();
    let out = render_edit(&original, &shape("Frame", false, false)).unwrap();
    for (x, y) in [
        (12, 8),
        (44, 8),
        (12, 40),
        (44, 40),
        (28, 8),
        (12, 24),
        (44, 24),
        (28, 40),
    ] {
        assert_eq!(out.get_pixel(x, y).0, [255, 180, 40, 255]);
    }
    assert_eq!(out.get_pixel(28, 24).0, [10, 20, 30, 255]);
    assert_eq!(out.get_pixel(3, 3).0, [10, 20, 30, 255]);
    assert_eq!(original, before);
    assert_eq!(
        out,
        render_edit(&original, &shape("Frame", false, true)).unwrap()
    );
}

#[test]
fn ellipse_exports_outline_in_both_drag_directions() {
    let original = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255]));
    let out = render_edit(&original, &shape("Ellipse", false, false)).unwrap();
    for (x, y) in [(28, 8), (44, 24), (28, 40), (12, 24)] {
        assert_eq!(out.get_pixel(x, y).0, [255, 180, 40, 255]);
    }
    for (x, y) in [(12, 8), (28, 24), (3, 3)] {
        assert_eq!(out.get_pixel(x, y).0, [10, 20, 30, 255]);
    }
    assert_eq!(
        out,
        render_edit(&original, &shape("Ellipse", false, true)).unwrap()
    );
}

#[test]
fn filled_shapes_and_crop_use_original_pixel_coordinates() {
    let original = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255]));
    for kind in ["Frame", "Ellipse"] {
        let mut doc = shape(kind, true, false);
        doc.crop = Some(frame_studio::edit::Crop {
            x: 20,
            y: 20,
            width: 16,
            height: 12,
        });
        let out = render_edit(&original, &doc).unwrap();
        assert_eq!(out.dimensions(), (16, 12));
        assert_eq!(out.get_pixel(8, 4).0, [255, 180, 40, 255]);
        let restored: Document =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(render_edit(&original, &restored).unwrap(), out);
    }
}

#[test]
fn old_annotations_load_with_new_shapes_and_shape_fill_defaults_to_outline() {
    let doc: Document = serde_json::from_str(r#"{"crop":null,"annotations":[{"Stroke":{"points":[{"x":2,"y":2}],"color":[20,200,80,255],"width":2}},{"Frame":{"from":{"x":12,"y":8},"to":{"x":44,"y":40},"color":[255,180,40,255],"width":2}}]}"#).unwrap();
    let out = render_edit(
        &RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255])),
        &doc,
    )
    .unwrap();
    assert_eq!(out.get_pixel(2, 2).0, [20, 200, 80, 255]);
    assert_eq!(out.get_pixel(28, 24).0, [10, 20, 30, 255]);
}

#[test]
fn square_stays_square_and_inside_image_when_dragged_towards_each_corner() {
    use frame_studio::edit::{Point, square_endpoint};
    let from = Point { x: 20.0, y: 20.0 };
    for to in [
        Point { x: 99.0, y: 25.0 },
        Point { x: 0.0, y: 18.0 },
        Point { x: 22.0, y: 79.0 },
        Point { x: 21.0, y: 0.0 },
    ] {
        let end = square_endpoint(from, to, [100, 80]);
        assert_eq!((end.x - from.x).abs(), (end.y - from.y).abs());
        assert!((0.0..=99.0).contains(&end.x));
        assert!((0.0..=79.0).contains(&end.y));
    }
}

#[test]
fn empty_or_nonfinite_shapes_are_ignored_without_changing_the_image() {
    use frame_studio::edit::{Annotation, Point, shape_bounds};
    let original = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255]));
    let from = Point { x: 12.0, y: 8.0 };
    for to in [
        from,
        Point {
            x: f32::NAN,
            y: 20.0,
        },
        Point { x: 12.0, y: 40.0 },
    ] {
        assert!(shape_bounds(from, to).is_none());
        let doc = Document {
            annotations: vec![Annotation::Frame {
                from,
                to,
                color: [255, 255, 255, 255],
                width: 6.0,
                filled: false,
            }],
            ..Default::default()
        };
        assert_eq!(render_edit(&original, &doc).unwrap(), original);
    }
}
