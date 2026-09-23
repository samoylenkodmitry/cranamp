use super::*;

#[test]
fn the_window_icon_is_the_square_the_icon_script_renders() {
    let icon = window_icon().expect("the committed window icon decodes");
    assert_eq!((icon.width(), icon.height()), (128, 128));
}

#[test]
fn the_window_icon_is_a_plate_with_transparent_corners() {
    let icon = window_icon().expect("the committed window icon decodes");
    let alpha = |x: u32, y: u32| icon.pixels()[((y * icon.width() + x) * 4 + 3) as usize];

    assert_eq!(
        alpha(0, 0),
        0,
        "a square corner would show as a box in the taskbar"
    );
    assert_eq!(alpha(64, 64), 255, "the plate under the cat is opaque");
}
