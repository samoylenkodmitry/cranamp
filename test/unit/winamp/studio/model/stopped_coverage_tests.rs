use super::*;
#[test]
fn stopped_spectrum_reveals_editable_background() {
    let mut doc = Document::blank();
    doc.open_on_whole_skin();
    doc.view.playback = 0;
    let (coverage, _) = doc.coverage([24, 43, 76, 16], false).unwrap();
    assert_eq!(coverage["counts"]["opaque_runtime"], 0);
    assert_eq!(coverage["counts"]["paintable"], 76 * 16);
}
