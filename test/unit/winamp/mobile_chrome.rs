use super::*;

#[test]
fn album_cover_reads_are_bounded_and_leave_original_files_unchanged() {
    let directory = std::env::temp_dir().join(format!(
        "cranamp-cover-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&directory).unwrap();
    let audio = directory.join("song.flac");
    std::fs::write(&audio, b"original audio must not be opened as artwork").unwrap();
    let oversized = std::fs::File::create(directory.join("cover.jpg")).unwrap();
    oversized.set_len(4 * 1024 * 1024 + 1).unwrap();
    let original = image::RgbaImage::from_pixel(320, 320, image::Rgba([12, 34, 56, 255]));
    original.save(directory.join("folder.png")).unwrap();
    let cover = read_cover(directory.clone()).unwrap();
    assert_eq!((cover.width(), cover.height()), (160, 160));
    assert_eq!(&cover.pixels()[..4], &[12, 34, 56, 255]);
    assert_eq!(
        std::fs::read(&audio).unwrap(),
        b"original audio must not be opened as artwork"
    );
    assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 3);
    std::fs::remove_file(directory.join("folder.png")).unwrap();
    assert!(
        read_cover(directory.clone()).is_some(),
        "Cover must come from memory after first read"
    );
    std::fs::remove_dir_all(directory).unwrap();
}
