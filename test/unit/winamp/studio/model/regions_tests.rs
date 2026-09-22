use super::*;
#[test]
fn region_generation_is_undoable_and_survives_both_archive_formats() {
    let mut doc = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    let rows = (0..116)
        .map(|y| {
            if y < 3 {
                ".".repeat(275)
            } else {
                format!("..{}..", "#".repeat(271))
            }
        })
        .collect::<Vec<_>>();
    doc.window_regions(&json!({"action":"generate","section":"Normal","rows":rows}))
        .unwrap();
    let saved = doc.files["region.txt"].clone();
    assert!(Regions::parse(&saved).unwrap().portable());
    let reopened = Document::open(&doc.archive().unwrap(), None).unwrap();
    assert_eq!(reopened.files["region.txt"], saved);
    let project = Document::open_project(&doc.project_bytes().unwrap()).unwrap();
    assert_eq!(project.files["region.txt"], saved);
    doc.undo();
    assert!(!doc.files.contains_key("region.txt"));
}
#[test]
fn project_preserves_unfinished_base_alpha_and_export_refuses_it() {
    let mut doc = Document::open(include_bytes!("../../../../../assets/winamp.wsz"), None).unwrap();
    doc.images
        .get_mut("main.bmp")
        .unwrap()
        .put_pixel(0, 0, Rgba([12, 34, 56, 100]));
    let project = doc.project_bytes().unwrap();
    let restored = Document::open_project(&project).unwrap();
    assert_eq!(
        restored.images["main.bmp"].get_pixel(0, 0).0,
        [12, 34, 56, 100]
    );
    assert!(restored.export_archive().is_err());
}
