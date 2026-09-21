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
        checked += 1;
    }
    assert!(checked >= 6, "only {checked} skins were checked");
}
