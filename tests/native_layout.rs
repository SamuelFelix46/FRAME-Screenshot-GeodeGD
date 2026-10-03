#![cfg(all(feature = "native", target_os = "windows", target_pointer_width = "64"))]
use geode_rs::classes::{GJBaseGameLayer, GJGameLevel, PlayerObject};

#[test]
fn gd_22081_fields_used_by_frame_match_the_native_layout() {
    assert_eq!(std::mem::offset_of!(GJBaseGameLayer, level), 0x878);
    assert_eq!(std::mem::offset_of!(GJBaseGameLayer, attempts), 0x3084);
    assert_eq!(
        std::mem::offset_of!(GJBaseGameLayer, is_practice_mode),
        0x31f0
    );
    assert_eq!(std::mem::offset_of!(GJGameLevel, level_name), 0x158);
    assert_eq!(std::mem::offset_of!(PlayerObject, is_dead), 0x9c0);
    assert_eq!(std::mem::size_of::<geode_rs::stl::StlString>(), 32);
}
