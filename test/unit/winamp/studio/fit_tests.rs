use super::{fit_zoom, Scene, CANVAS_MIN, DRAWER_WIDTH};
#[test]
fn the_default_window_fits_the_whole_skin_at_two_times() {
    let scene = Scene::new(1388., 1000.);
    let (canvas_w, drawer_w) = scene.split();
    let canvas_h = scene.canvas().3;
    assert!(drawer_w >= DRAWER_WIDTH, "the tool column stays docked");
    assert_eq!(fit_zoom((canvas_w, canvas_h), (275, 377)), 2);
}
#[test]
fn a_narrow_window_gives_the_canvas_back_the_room_the_column_took() {
    for (w, h) in [
        (0., 0.),
        (393., 780.),
        (820., 1180.),
        (900., 800.),
        (1160., 850.),
        (1388., 1000.),
        (1680., 1050.),
    ] {
        let scene = Scene::new(w, h);
        let (canvas_w, drawer_w) = scene.split();
        assert!(canvas_w > 0., "{w}x{h} has no canvas at all");
        if drawer_w > 0. {
            assert!(
                canvas_w >= CANVAS_MIN,
                "{w}x{h} docked the column over a {canvas_w}pt canvas"
            );
            assert!(drawer_w >= DRAWER_WIDTH, "{w}x{h} docked a narrow column");
        } else {
            assert_eq!(
                canvas_w,
                scene.painting().2,
                "{w}x{h} undocked and kept the room anyway"
            );
        }
    }
}
