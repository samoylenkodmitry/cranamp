use super::*;
#[test]
fn generated_masks_round_trip_with_exact_holes_and_disconnected_islands() {
    let mut mask = vec![false; 275 * 116];
    for y in 0..116 {
        for x in 0..275 {
            mask[y * 275 + x] =
                (x > y / 2 && x < 275 - y / 2) && !(x > 75 && x < 115 && y > 30 && y < 60);
        }
    }
    mask[0] = true;
    let mut regions = Regions::default();
    regions.set_mask("Normal", 275, 116, &mask).unwrap();
    assert!(regions.portable());
    let bytes = regions.encode().unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("PointList="));
    assert_eq!(
        Regions::parse(&bytes).unwrap().mask("Normal", 275, 116),
        mask
    );
    assert!(Regions::parse(&bytes)
        .unwrap()
        .mask("Equalizer", 275, 116)
        .iter()
        .all(|p| *p));
}
#[test]
fn strict_parser_catches_ambiguous_or_broken_configs() {
    for input in [
        "[Normal]\nNumPoints=4\nPointList=0,0",
        "[Normal]\nNumPoints=-3\nPointList=0,0",
        "[Normal]\nNumPoints=4\nNumPoints=4\nPointList=0,0,2,0,2,2,0,2",
    ] {
        assert!(Regions::parse(input.as_bytes()).is_err(), "{input}");
    }
    let parsed =
        Regions::parse(b";comment\n[normal]\nnumpoints=4\npointlist=,0,0,2,0,2,2,0,2 ;comment")
            .unwrap();
    assert_eq!(
        parsed.mask("Normal", 3, 3),
        vec![true, true, false, true, true, false, false, false, false]
    );
}
#[test]
fn winding_holes_import_but_require_rectangular_normalization_for_audacious() {
    let input = b"[Normal]\nNumPoints=4,4\nPointList=0,0,4,0,4,4,0,4,1,1,1,3,3,3,3,1";
    let regions = Regions::parse(input).unwrap();
    let mask = regions.mask("Normal", 4, 4);
    assert!(!mask[5]);
    assert!(mask[0]);
    let triangle = Regions::parse(b"[Normal]\nNumPoints=3\nPointList=0,0,4,0,0,4").unwrap();
    assert!(!triangle.portable());
}
#[test]
fn impossible_and_overlong_masks_are_rejected() {
    assert!(Regions::default()
        .set_mask("Normal", 275, 116, &vec![false; 275 * 116])
        .is_err());
    let checker = (0..275 * 116)
        .map(|i| (i % 275 + i / 275) % 2 == 0)
        .collect::<Vec<_>>();
    assert!(Regions::default()
        .set_mask("Normal", 275, 116, &checker)
        .is_err());
}
