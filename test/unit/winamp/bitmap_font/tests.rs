use super::*;

fn atlas(height: u32) -> ImageBitmap {
    let mut p = Vec::new();
    for y in 0..height {
        for x in 0..155 {
            p.extend_from_slice(&[x as u8, y as u8, 91, 255]);
        }
    }
    ImageBitmap::from_rgba8(155, height, p).unwrap()
}
fn pixel(bitmap: &ImageBitmap, x: u32, y: u32) -> &[u8] {
    let i = ((y * bitmap.width() + x) * 4) as usize;
    &bitmap.pixels()[i..i + 4]
}
#[test]
fn reads_authored_letters_numbers_punctuation_and_third_row_without_recoloring() {
    let b = render(&atlas(18), "A0:?", 20);
    assert_eq!((b.width(), b.height()), (20, 6));
    assert_eq!(pixel(&b, 0, 0), &[0, 0, 91, 255]);
    assert_eq!(pixel(&b, 4, 5), &[4, 5, 91, 255]);
    assert_eq!(pixel(&b, 5, 0), &[0, 6, 91, 255]);
    assert_eq!(pixel(&b, 10, 0), &[60, 6, 91, 255]);
    assert_eq!(pixel(&b, 19, 5), &[19, 17, 91, 255]);
}
#[test]
fn lowercase_shares_uppercase_cells_and_padding_copies_the_space_background() {
    let a = atlas(18);
    assert_eq!(
        render(&a, "Cat", 15).pixels(),
        render(&a, "CAT", 15).pixels()
    );
    let b = render(&a, "A", 12);
    assert_eq!(pixel(&b, 5, 0), &[150, 0, 91, 255]);
    assert_eq!(pixel(&b, 11, 5), &[151, 5, 91, 255]);
}
#[test]
fn legacy_two_row_fonts_and_unknown_characters_fall_back_to_space() {
    let b = render(&atlas(12), "?猫", 10);
    assert_eq!(pixel(&b, 0, 0), &[150, 0, 91, 255]);
    assert_eq!(pixel(&b, 5, 0), &[150, 0, 91, 255]);
}
#[test]
fn cropped_cells_keep_their_exact_alpha_and_color() {
    let mut p = [255, 0, 255, 255].repeat(155 * 18);
    p[0..4].copy_from_slice(&[0, 0, 0, 0]);
    let a = ImageBitmap::from_rgba8(155, 18, p).unwrap();
    let b = render(&a, "AA", 6);
    assert_eq!(pixel(&b, 0, 0), &[0, 0, 0, 0]);
    assert_eq!(pixel(&b, 1, 0), &[255, 0, 255, 255]);
    assert_eq!(pixel(&b, 5, 0), &[0, 0, 0, 0]);
}
