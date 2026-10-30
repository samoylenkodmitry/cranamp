use super::*;

#[test]
fn growing_and_shrinking_return_to_the_same_size() {
    for scale in [1., 1.5 * 96. / 130., 1.5, 2.] {
        for margin in [0., 24.] {
            let initial = Size::new(275., 261.);
            let larger = stretched(initial, Point::new(100., 200.), scale, 232., margin);
            let restored = stretched(larger, Point::new(-100., -200.), scale, 232., margin);
            assert!((restored.width - initial.width).abs() < 0.001);
            assert!((restored.height - initial.height).abs() < 0.001);
        }
    }
}

#[test]
fn vertical_drag_changes_only_playlist_height() {
    let initial = Size::new(330., 261.);
    let taller = stretched(initial, Point::new(0., 54.), 1.5, 232., 0.);
    assert_eq!(taller.width, initial.width);
    assert!((taller.height - 291.).abs() < 0.001);
}

#[test]
fn width_drag_scales_panels_and_keeps_the_bottom_at_the_pointer() {
    let initial = Size::new(275., 400.);
    let wider = stretched(initial, Point::new(82.5, 0.), 1.5, 232., 0.);
    assert_eq!(wider.width, 330.);
    let main_height = 116. * 1.5 * wider.width / WIDTH;
    assert!((main_height - 208.8).abs() < 0.001);
    assert!(((232. + wider.height) * 1.8 - (232. + initial.height) * 1.5).abs() < 0.001);
    let smaller = stretched(initial, Point::new(-80., -50.), 1.5, 232., 0.);
    assert!(smaller.width < initial.width);
}
