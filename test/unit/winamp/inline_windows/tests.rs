use super::*;

fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

const MAIN: Rect = Rect {
    x: 26.0,
    y: 22.0,
    width: 275.0,
    height: 116.0,
};

#[test]
fn a_window_put_down_near_the_bottom_of_another_lands_on_it() {
    let proposed = rect(31.0, 145.0, 275.0, 116.0);
    assert_eq!(
        snapped_origin(proposed, &[MAIN], None),
        Point::new(26.0, 138.0),
        "the top edge lands on the main window's bottom and the sides line up"
    );
}

#[test]
fn a_window_put_down_beside_another_lands_against_its_side() {
    let proposed = rect(307.0, 30.0, 275.0, 116.0);
    assert_eq!(
        snapped_origin(proposed, &[MAIN], None),
        Point::new(301.0, 22.0)
    );
}

#[test]
fn a_window_far_from_every_edge_stays_where_it_was_put() {
    let proposed = rect(400.0, 400.0, 275.0, 116.0);
    assert_eq!(
        snapped_origin(proposed, &[MAIN], Some(Size::new(1200.0, 900.0))),
        Point::new(400.0, 400.0)
    );
}

#[test]
fn a_window_does_not_snap_to_an_edge_it_is_nowhere_beside() {
    let proposed = rect(26.0, 600.0, 275.0, 116.0);
    assert_eq!(
        snapped_origin(proposed, &[rect(600.0, 22.0, 275.0, 116.0)], None),
        Point::new(26.0, 600.0),
        "the other window's column is far away, so its edges are not near this one"
    );
}

#[test]
fn a_window_near_the_canvas_edge_lands_on_it() {
    let canvas = Size::new(1000.0, 800.0);
    assert_eq!(
        snapped_origin(rect(6.0, 690.0, 275.0, 116.0), &[], Some(canvas)),
        Point::new(0.0, 684.0)
    );
}

#[test]
fn the_nearest_edge_wins() {
    let proposed = rect(26.0, 142.0, 275.0, 116.0);
    let below_main = rect(26.0, 146.0, 275.0, 116.0);
    assert_eq!(
        snapped_origin(proposed, &[MAIN, below_main], None),
        Point::new(26.0, 138.0),
        "138 is 4 away and 146 is 4 away the other way; the first nearest is kept"
    );
}

#[test]
fn windows_stacked_under_the_main_window_travel_with_it() {
    let equalizer = rect(26.0, 138.0, 275.0, 116.0);
    let playlist = rect(26.0, 254.0, 275.0, 261.0);
    assert_eq!(attached_to(0, &[MAIN, equalizer, playlist]), vec![1, 2]);
}

#[test]
fn a_window_set_apart_stays_behind() {
    let equalizer = rect(26.0, 138.0, 275.0, 116.0);
    let playlist = rect(500.0, 300.0, 275.0, 261.0);
    assert_eq!(attached_to(0, &[MAIN, equalizer, playlist]), vec![1]);
}

#[test]
fn windows_only_touching_at_a_corner_are_not_attached() {
    let corner = rect(301.0, 138.0, 275.0, 116.0);
    assert!(!touching(MAIN, corner));
}

#[test]
fn a_window_against_the_side_is_attached() {
    assert!(touching(MAIN, rect(301.0, 60.0, 275.0, 116.0)));
}
