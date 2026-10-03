use super::*;
use std::io::Write;

fn archive(names: &[&str]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for name in names {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"tiny payload").unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn archive_limits_reject_oversized_entries_before_inflating() {
    let mut bytes = archive(&["main.bmp"]);
    let header = bytes.windows(4).position(|b| b == b"PK\x01\x02").unwrap();
    bytes[header + 24..header + 28].copy_from_slice(&((MAX_ENTRY_BYTES + 1) as u32).to_le_bytes());
    let error = crate::winamp::skin::load_skin(&bytes).err().unwrap();
    assert!(format!("{error:#}").contains("16 MiB"));
}

#[test]
fn duplicate_case_names_and_excessive_directory_counts_are_rejected() {
    assert!(open_archive(&archive(&["main.bmp", "MAIN.BMP"])).is_err());
    let mut bytes = archive(&["main.bmp"]);
    let end = bytes.len() - 22;
    bytes[end + 10..end + 12].copy_from_slice(&2049u16.to_le_bytes());
    assert!(open_archive(&bytes).is_err());
    for size in 0..22 {
        assert!(open_archive(&vec![0; size]).is_err());
    }
}

#[test]
fn image_dimensions_are_checked_before_allocating_pixels() {
    let image = image::RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Bmp).unwrap();
    let mut bytes = bytes.into_inner();
    assert_eq!(decode_image(&bytes).unwrap().dimensions(), (1, 1));
    bytes[18..22].copy_from_slice(&100_000u32.to_le_bytes());
    assert!(decode_image(&bytes).is_err());
}

#[test]
fn limited_reader_rejects_a_source_that_exceeds_its_declared_size() {
    assert!(read_limited(std::io::repeat(0), 1024).is_err());
    assert_eq!(read_limited(Cursor::new(b"small"), 1024).unwrap(), b"small");
}
