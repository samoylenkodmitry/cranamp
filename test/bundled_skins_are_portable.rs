#![cfg(not(target_arch = "wasm32"))]
use cranamp::winamp::skin;
#[test]
fn every_bundled_skin_uses_classic_entries_and_loads() {
    let mut checked = 0;
    for entry in std::fs::read_dir("assets/skins").expect("assets/skins") {
        let path = entry.expect("a directory entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("wsz") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("a readable skin");
        let entries = skin::entries_of(&bytes).expect("a readable archive");
        let found = skin::divergences(&entries);
        assert!(
            found.is_empty(),
            "{} has unsupported archive differences: {found:#?}",
            path.display()
        );
        skin::load_skin(&bytes).expect("the player's own loader takes it");
        let audit = skin::audit_classic_archive(&bytes).unwrap();
        assert!(audit.exportable, "{}: {:#?}", path.display(), audit.errors);
        assert_eq!(
            audit.magenta_pixels,
            0,
            "{} retains obsolete magenta placeholders",
            path.display()
        );
        let loaded = skin::load_skin(&bytes).unwrap();
        assert!(loaded
            .text
            .pixels()
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| p[3] == 255));
        assert!(loaded.regions.portable());
        checked += 1;
    }
    assert!(checked >= 18, "only {checked} skins were checked");
}

#[test]
fn bundled_clock_digits_are_distinct() {
    for entry in std::fs::read_dir("assets/skins").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("wsz") {
            continue;
        }
        let skin = skin::load_skin(&std::fs::read(&path).unwrap()).unwrap();
        let stride = skin.numbers.width() as usize * 4;
        let glyphs: Vec<Vec<u8>> = (0..10)
            .map(|digit| {
                (0..13)
                    .flat_map(|y| {
                        let start = y * stride + digit * 9 * 4;
                        skin.numbers.pixels()[start..start + 9 * 4].iter().copied()
                    })
                    .collect()
            })
            .collect();
        for a in 0..10 {
            for b in a + 1..10 {
                assert_ne!(
                    glyphs[a],
                    glyphs[b],
                    "{}: digits {a} and {b} are identical",
                    path.display()
                );
            }
        }
    }
}
