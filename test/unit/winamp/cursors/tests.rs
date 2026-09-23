use super::*;

/// One image inside a cursor file, already laid out as a bottom-up DIB.
pub(in crate::winamp) struct TestImage {
    width: u32,
    height: u32,
    hotspot: (u16, u16),
    bit_count: u16,
    palette: Vec<[u8; 4]>,
    colors: Vec<u8>,
    mask: Vec<u8>,
}

impl TestImage {
    fn dib(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&(self.height * 2).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&self.bit_count.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(self.palette.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for entry in &self.palette {
            out.extend_from_slice(entry);
        }
        out.extend_from_slice(&self.colors);
        out.extend_from_slice(&self.mask);
        out
    }
}

/// Wraps images in an icon directory with the cursor resource type.
pub(in crate::winamp) fn cursor_file(images: &[TestImage]) -> Vec<u8> {
    cursor_file_of_type(images, CURSOR_RESOURCE_TYPE)
}

fn cursor_file_of_type(images: &[TestImage], resource_type: u16) -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&0u16.to_le_bytes());
    header.extend_from_slice(&resource_type.to_le_bytes());
    header.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let dibs: Vec<Vec<u8>> = images.iter().map(TestImage::dib).collect();
    let mut offset = 6 + images.len() * DIRECTORY_ENTRY_LEN;
    for (image, dib) in images.iter().zip(&dibs) {
        header.push(image.width as u8);
        header.push(image.height as u8);
        header.push(0);
        header.push(0);
        header.extend_from_slice(&image.hotspot.0.to_le_bytes());
        header.extend_from_slice(&image.hotspot.1.to_le_bytes());
        header.extend_from_slice(&(dib.len() as u32).to_le_bytes());
        header.extend_from_slice(&(offset as u32).to_le_bytes());
        offset += dib.len();
    }
    for dib in dibs {
        header.extend_from_slice(&dib);
    }
    header
}

/// A 2x2 two-colour cursor: white and black on the diagonal, with the
/// top-right pixel cleared by the transparency mask.
pub(in crate::winamp) fn monochrome_2x2() -> TestImage {
    TestImage {
        width: 2,
        height: 2,
        hotspot: (1, 0),
        bit_count: 1,
        palette: vec![[0, 0, 0, 0], [255, 255, 255, 0]],
        colors: vec![0x40, 0, 0, 0, 0x80, 0, 0, 0],
        mask: vec![0x00, 0, 0, 0, 0x40, 0, 0, 0],
    }
}

fn bgra_2x1(pixels: [[u8; 4]; 2], mask: u8) -> TestImage {
    let mut colors = Vec::new();
    for pixel in pixels {
        colors.extend_from_slice(&pixel);
    }
    TestImage {
        width: 2,
        height: 1,
        hotspot: (0, 0),
        bit_count: 32,
        palette: Vec::new(),
        colors,
        mask: vec![mask, 0, 0, 0],
    }
}

fn pixel(icon: &CustomPointerIcon, x: u32, y: u32) -> [u8; 4] {
    let at = ((y * icon.image().width() + x) * 4) as usize;
    let pixels = icon.image().pixels();
    [pixels[at], pixels[at + 1], pixels[at + 2], pixels[at + 3]]
}

#[test]
fn a_monochrome_cursor_decodes_bottom_up_with_its_palette() {
    let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

    assert_eq!(icon.image().width(), 2);
    assert_eq!(icon.image().height(), 2);
    assert_eq!(pixel(&icon, 0, 0), [255, 255, 255, 255]);
    assert_eq!(pixel(&icon, 0, 1), [0, 0, 0, 255]);
    assert_eq!(pixel(&icon, 1, 1), [255, 255, 255, 255]);
}

#[test]
fn the_transparency_mask_clears_the_pixels_it_covers() {
    let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

    assert_eq!(pixel(&icon, 1, 0)[3], 0, "the masked pixel is transparent");
}

#[test]
fn the_hotspot_comes_from_the_directory_entry() {
    let icon = decode_cur(&cursor_file(&[monochrome_2x2()])).expect("cursor decodes");

    assert_eq!(icon.hotspot_x(), 1);
    assert_eq!(icon.hotspot_y(), 0);
}

#[test]
fn a_32_bit_cursor_keeps_the_alpha_channel_it_carries() {
    let image = bgra_2x1([[10, 20, 30, 128], [40, 50, 60, 255]], 0b1100_0000);
    let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

    assert_eq!(
        pixel(&icon, 0, 0),
        [30, 20, 10, 128],
        "the stored alpha wins over a mask that would clear the pixel"
    );
    assert_eq!(pixel(&icon, 1, 0), [60, 50, 40, 255]);
}

#[test]
fn a_32_bit_cursor_with_an_empty_alpha_channel_falls_back_to_the_mask() {
    let image = bgra_2x1([[10, 20, 30, 0], [40, 50, 60, 0]], 0b1000_0000);
    let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

    assert_eq!(pixel(&icon, 0, 0)[3], 0, "the mask clears the first pixel");
    assert_eq!(pixel(&icon, 1, 0), [60, 50, 40, 255]);
}

#[test]
fn the_largest_image_in_the_directory_is_the_one_drawn() {
    let mut large = monochrome_2x2();
    large.width = 4;
    large.height = 4;
    large.colors = vec![0; 16];
    large.mask = vec![0; 16];
    large.hotspot = (2, 2);

    let icon = decode_cur(&cursor_file(&[monochrome_2x2(), large])).expect("cursor decodes");

    assert_eq!(icon.image().width(), 4);
    assert_eq!(icon.hotspot_x(), 2);
}

#[test]
fn an_icon_file_is_not_accepted_as_a_cursor() {
    let icon_resource_type = 1;
    let error = decode_cur(&cursor_file_of_type(
        &[monochrome_2x2()],
        icon_resource_type,
    ))
    .expect_err("an icon is refused");

    assert!(
        format!("{error}").contains("not a Windows cursor"),
        "{error}"
    );
}

#[test]
fn an_unsupported_bit_depth_is_refused_rather_than_guessed() {
    let mut image = monochrome_2x2();
    image.bit_count = 16;
    let error = decode_cur(&cursor_file(&[image])).expect_err("16bpp is refused");

    assert!(format!("{error}").contains("bit depth 16"), "{error}");
}

#[test]
fn a_truncated_file_is_refused_rather_than_panicking() {
    let full = cursor_file(&[monochrome_2x2()]);
    for length in 0..full.len() {
        assert!(
            decode_cur(&full[..length]).is_err(),
            "a {length}-byte prefix must not decode"
        );
    }
}

#[test]
fn a_cursor_larger_than_a_platform_draws_is_left_alone() {
    let mut image = monochrome_2x2();
    image.width = MAX_CURSOR_SIZE + 8;
    image.height = 2;
    image.colors = vec![0; 2 * row_stride(image.width, 1)];
    image.mask = vec![0; 2 * row_stride(image.width, 1)];

    assert!(decode_cur(&cursor_file(&[image])).is_err());
}

/// A 16-colour row where the two pixels pick palette entries 1 and 2.
fn indexed_4bpp_2x1() -> TestImage {
    let mut palette = vec![[0, 0, 0, 0]; 16];
    palette[1] = [11, 22, 33, 0];
    palette[2] = [44, 55, 66, 0];
    TestImage {
        width: 2,
        height: 1,
        hotspot: (0, 0),
        bit_count: 4,
        palette,
        colors: vec![0x12, 0, 0, 0],
        mask: vec![0, 0, 0, 0],
    }
}

#[test]
fn a_sixteen_colour_cursor_reads_two_pixels_from_one_byte() {
    let icon = decode_cur(&cursor_file(&[indexed_4bpp_2x1()])).expect("cursor decodes");

    assert_eq!(pixel(&icon, 0, 0), [33, 22, 11, 255]);
    assert_eq!(pixel(&icon, 1, 0), [66, 55, 44, 255]);
}

#[test]
fn a_256_colour_cursor_reads_one_pixel_per_byte() {
    let mut palette = vec![[0, 0, 0, 0]; 256];
    palette[7] = [1, 2, 3, 0];
    palette[9] = [4, 5, 6, 0];
    let image = TestImage {
        width: 2,
        height: 1,
        hotspot: (0, 0),
        bit_count: 8,
        palette,
        colors: vec![7, 9, 0, 0],
        mask: vec![0, 0, 0, 0],
    };

    let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

    assert_eq!(pixel(&icon, 0, 0), [3, 2, 1, 255]);
    assert_eq!(pixel(&icon, 1, 0), [6, 5, 4, 255]);
}

#[test]
fn a_24_bit_cursor_reads_three_bytes_per_pixel_over_a_padded_row() {
    let image = TestImage {
        width: 2,
        height: 1,
        hotspot: (0, 0),
        bit_count: 24,
        palette: Vec::new(),
        colors: vec![10, 20, 30, 40, 50, 60, 0, 0],
        mask: vec![0b0100_0000, 0, 0, 0],
    };

    let icon = decode_cur(&cursor_file(&[image])).expect("cursor decodes");

    assert_eq!(pixel(&icon, 0, 0), [30, 20, 10, 255]);
    assert_eq!(pixel(&icon, 1, 0)[3], 0, "the mask clears the second pixel");
}

#[test]
fn load_cursors_reads_the_regions_the_archive_names() {
    let mut files = HashMap::new();
    files.insert(
        SkinCursor::MainWindow.file_name().to_string(),
        cursor_file(&[monochrome_2x2()]),
    );
    files.insert(
        SkinCursor::VolumeBalance.file_name().to_string(),
        cursor_file(&[monochrome_2x2()]),
    );

    let cursors = load_cursors(&files);

    assert_eq!(cursors.len(), 2);
    assert!(cursors.get(SkinCursor::MainWindow).is_some());
    assert!(cursors.get(SkinCursor::VolumeBalance).is_some());
    assert!(cursors.get(SkinCursor::PlaylistResize).is_none());
}

fn size_of(cursors: &SkinCursors, region: SkinCursor) -> Option<(u32, u32)> {
    match cursors.get(region)? {
        PointerIcon::Custom(icon) => Some((icon.image().width(), icon.image().height())),
        _ => None,
    }
}

#[test]
fn the_rolled_up_window_borrows_the_full_windows_cursors_until_it_has_its_own() {
    let mut files = HashMap::new();
    for region in [SkinCursor::MainClose, SkinCursor::PositionBar] {
        files.insert(
            region.file_name().to_string(),
            cursor_file(&[monochrome_2x2()]),
        );
    }
    files.insert(
        SkinCursor::ShadePositionBar.file_name().to_string(),
        cursor_file(&[bgra_2x1([[1, 2, 3, 255], [4, 5, 6, 255]], 0)]),
    );

    let cursors = load_cursors(&files);

    assert_eq!(size_of(&cursors, SkinCursor::ShadeClose), Some((2, 2)));
    assert_eq!(
        size_of(&cursors, SkinCursor::ShadePositionBar),
        Some((2, 1)),
        "a skin's own rolled-up cursor wins over the one it would borrow"
    );
    assert!(
        cursors.get(SkinCursor::ShadeMinimize).is_none(),
        "nothing to borrow leaves the platform's pointer"
    );
    assert_eq!(cursors.len(), 4);
}

#[test]
fn a_malformed_cursor_costs_only_its_own_region() {
    let mut files = HashMap::new();
    files.insert(
        SkinCursor::MainWindow.file_name().to_string(),
        b"not a cursor at all".to_vec(),
    );
    files.insert(
        SkinCursor::MainClose.file_name().to_string(),
        cursor_file(&[monochrome_2x2()]),
    );

    let cursors = load_cursors(&files);

    assert!(cursors.get(SkinCursor::MainWindow).is_none());
    assert!(cursors.get(SkinCursor::MainClose).is_some());
}

#[test]
fn a_skin_with_no_cursor_files_carries_none() {
    assert!(load_cursors(&HashMap::new()).is_empty());
}

#[test]
fn every_region_reads_a_file_a_classic_player_reads() {
    for (region, name) in SkinCursor::FILES {
        assert!(
            CLASSIC_CURSORS.contains(&name),
            "{region:?} reads {name}, which the portability check would offer to drop"
        );
    }
}

#[test]
fn the_classic_cursor_names_are_sorted_and_distinct() {
    let mut sorted = CLASSIC_CURSORS;
    sorted.sort_unstable();
    assert_eq!(sorted, CLASSIC_CURSORS, "keep the list in name order");
    let mut unique = CLASSIC_CURSORS.to_vec();
    unique.dedup();
    assert_eq!(unique.len(), CLASSIC_CURSORS.len());
}

#[test]
fn every_region_names_a_distinct_lower_case_file() {
    let mut names: Vec<&str> = SkinCursor::FILES.iter().map(|(_, name)| *name).collect();
    names.sort_unstable();
    let count = names.len();
    names.dedup();

    assert_eq!(names.len(), count, "two regions share a file name");
    assert!(
        SkinCursor::FILES
            .iter()
            .all(|(_, name)| *name == name.to_ascii_lowercase()),
        "archive lookup is by lower-cased name"
    );
}

fn gradient(width: u32, height: u32) -> Vec<u8> {
    (0..width * height)
        .flat_map(|i| {
            let x = (i % width) as u8;
            let y = (i / width) as u8;
            [
                x.wrapping_mul(7),
                y.wrapping_mul(11),
                40,
                if x == y { 0 } else { 255 },
            ]
        })
        .collect()
}

#[test]
fn an_encoded_cursor_reads_back_pixel_for_pixel() {
    let rgba = gradient(16, 24);
    let bytes = encode_cur(16, 24, &rgba, 3, 5).expect("encodes");
    let icon = decode_cur(&bytes).expect("decodes");
    assert_eq!(icon.hotspot_x(), 3);
    assert_eq!(icon.hotspot_y(), 5);
    assert_eq!(icon.image().width(), 16);
    assert_eq!(icon.image().height(), 24);
    assert_eq!(icon.image().pixels(), rgba.as_slice());
}

#[test]
fn an_encoded_cursor_keeps_its_transparent_pixels_transparent() {
    let rgba = vec![0; 32 * 32 * 4];
    let bytes = encode_cur(32, 32, &rgba, 0, 0).expect("encodes");
    let icon = decode_cur(&bytes).expect("decodes");
    assert!(
        icon.image()
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 0),
        "an all-transparent cursor must not come back opaque"
    );
}

#[test]
fn an_image_of_the_wrong_length_is_refused() {
    assert!(encode_cur(8, 8, &[0; 10], 0, 0).is_err());
}

#[test]
fn a_hotspot_outside_the_image_is_refused() {
    let rgba = vec![0; 8 * 8 * 4];
    assert!(encode_cur(8, 8, &rgba, 8, 0).is_err());
    assert!(encode_cur(8, 8, &rgba, 0, 8).is_err());
}

#[test]
fn a_cursor_larger_than_the_format_allows_is_refused() {
    let side = MAX_CURSOR_SIZE + 1;
    let rgba = vec![0; (side * side * 4) as usize];
    assert!(encode_cur(side, side, &rgba, 0, 0).is_err());
}
