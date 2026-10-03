use image::RgbaImage;

/// CF_DIB: BITMAPINFOHEADER followed by bottom-up BGRA pixels, without a BMP file header.
pub fn dib_bytes(image: &RgbaImage) -> Result<Vec<u8>, String> {
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 || width as u64 * height as u64 > 16_000_000 {
        return Err(crate::i18n::text(
            "Dimensions invalides pour le presse-papiers.",
            "Invalid image dimensions for the clipboard.",
        )
        .into());
    }
    let size = width as usize * height as usize * 4;
    let mut bytes = vec![0u8; 40 + size];
    bytes[0..4].copy_from_slice(&40u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    bytes[8..12].copy_from_slice(&(height as i32).to_le_bytes());
    bytes[12..14].copy_from_slice(&1u16.to_le_bytes());
    bytes[14..16].copy_from_slice(&32u16.to_le_bytes());
    bytes[20..24].copy_from_slice(&(size as u32).to_le_bytes());
    for y in 0..height {
        for x in 0..width {
            let p = image.get_pixel(x, y);
            let offset = 40 + ((height - 1 - y) as usize * width as usize + x as usize) * 4;
            bytes[offset..offset + 4].copy_from_slice(&[p[2], p[1], p[0], p[3]]);
        }
    }
    Ok(bytes)
}

#[cfg(feature = "native")]
pub fn copy_image(image: &RgbaImage) -> Result<(), String> {
    use std::ffi::c_void;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(owner: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EmptyClipboard() -> i32;
        fn SetClipboardData(format: u32, data: *mut c_void) -> *mut c_void;
        fn GetForegroundWindow() -> *mut c_void;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalAlloc(flags: u32, size: usize) -> *mut c_void;
        fn GlobalLock(data: *mut c_void) -> *mut c_void;
        fn GlobalUnlock(data: *mut c_void) -> i32;
        fn GlobalFree(data: *mut c_void) -> *mut c_void;
    }
    let bytes = dib_bytes(image)?;
    unsafe {
        let memory = GlobalAlloc(2, bytes.len());
        if memory.is_null() {
            return Err(crate::i18n::text(
                "Mémoire insuffisante pour copier l’image.",
                "Not enough memory to copy the image.",
            )
            .into());
        }
        let target = GlobalLock(memory);
        if target.is_null() {
            GlobalFree(memory);
            return Err(crate::i18n::text(
                "Impossible de préparer l’image à copier.",
                "Unable to prepare the image for copying.",
            )
            .into());
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.cast::<u8>(), bytes.len());
        GlobalUnlock(memory);
        let mut opened = false;
        for _ in 0..10 {
            if OpenClipboard(GetForegroundWindow()) != 0 {
                opened = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if !opened {
            GlobalFree(memory);
            return Err(crate::i18n::text(
                "Presse-papiers occupé. Réessaie Copier dans un instant.",
                "Clipboard is busy. Try Copy again in a moment.",
            )
            .into());
        }
        let success = EmptyClipboard() != 0 && !SetClipboardData(8, memory).is_null();
        CloseClipboard();
        if !success {
            GlobalFree(memory);
            return Err(crate::i18n::text(
                "Impossible de copier l’image dans le presse-papiers.",
                "Unable to copy the image to the clipboard.",
            )
            .into());
        }
    }
    Ok(())
}
#[cfg(not(feature = "native"))]
pub fn copy_image(_image: &RgbaImage) -> Result<(), String> {
    Err(crate::i18n::text(
        "Le presse-papiers nécessite le mod Windows natif.",
        "Clipboard support requires the native Windows mod.",
    )
    .into())
}
