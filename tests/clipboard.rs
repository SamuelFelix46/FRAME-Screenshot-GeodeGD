#[test]
fn clipboard_bitmap_keeps_colors_and_bottom_up_rows() {
    let image = image::RgbaImage::from_raw(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 70, 80, 90, 255,
        ],
    )
    .unwrap();
    let dib = frame_studio::clipboard::dib_bytes(&image).unwrap();
    assert_eq!(&dib[0..4], &40u32.to_le_bytes());
    assert_eq!(&dib[4..8], &2i32.to_le_bytes());
    assert_eq!(&dib[8..12], &2i32.to_le_bytes());
    assert_eq!(
        &dib[40..],
        &[
            255, 0, 0, 255, 90, 80, 70, 255, 0, 0, 255, 255, 0, 255, 0, 255
        ]
    );
}
