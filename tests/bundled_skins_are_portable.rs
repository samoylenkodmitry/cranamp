#![cfg(not(target_arch = "wasm32"))]
use cranamp::winamp::skin;
#[test]
fn every_bundled_skin_plays_the_same_in_any_player_that_reads_wsz() {
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
            "{} carries something no other player honours: {found:#?}",
            path.display()
        );
        skin::load_skin(&bytes).expect("the player's own loader takes it");
        checked += 1;
    }
    assert!(checked >= 6, "only {checked} skins were checked");
}
