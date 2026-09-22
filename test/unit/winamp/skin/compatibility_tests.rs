use super::*;
#[test]
fn palette_gate_rejects_player_dependent_defaults_and_duplicates() {
    assert!(!complete_palette("viscolor.txt", b"0,0,0"));
    assert!(complete_palette(
        "viscolor.txt",
        "0,0,0\n".repeat(24).as_bytes()
    ));
    assert!(!complete_palette(
        "viscolor.txt",
        "0,0,256\n".repeat(24).as_bytes()
    ));
    let valid = "[Text]\nNormal=#abcdef\nCurrent=#ffffff\nNormalBG=#000000\nSelectedBG=#333333\nMbFG=#ffffff\nMbBG=#000000\n";
    assert!(complete_palette("pledit.txt", valid.as_bytes()));
    assert!(!complete_palette(
        "pledit.txt",
        valid.replace("[Text]", "[Other]").as_bytes()
    ));
    assert!(!complete_palette(
        "pledit.txt",
        format!("{valid}normal=#222222\n").as_bytes()
    ));
}
