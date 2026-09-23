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
fn the_windows_under_a_window_follow_its_bottom_edge() {
    let equalizer = rect(26.0, 138.0, 275.0, 116.0);
    let beside = rect(301.0, 22.0, 275.0, 116.0);
    let playlist = rect(26.0, 254.0, 275.0, 261.0);
    let under_beside = rect(400.0, 138.0, 275.0, 116.0);
    assert_eq!(
        hanging_below(0, &[MAIN, equalizer, beside, playlist, under_beside]),
        vec![1, 3],
        "the window beside the main window, and the one under it, keep their places"
    );
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

#[test]
fn a_docked_stack_the_page_shrank_past_moves_back_in_as_one() {
    let equalizer = rect(700.0, 616.0, 275.0, 116.0);
    let main = rect(700.0, 500.0, 275.0, 116.0);
    let origins = kept_inside(&[main, equalizer], Size::new(800.0, 600.0));
    assert_eq!(
        origins,
        vec![Point::new(525.0, 368.0), Point::new(525.0, 484.0)]
    );
}

#[test]
fn windows_already_inside_stay_where_they_are() {
    let origins = kept_inside(
        &[MAIN, rect(400.0, 300.0, 275.0, 261.0)],
        Size::new(1200.0, 900.0),
    );
    assert_eq!(
        origins,
        vec![Point::new(26.0, 22.0), Point::new(400.0, 300.0)]
    );
}

#[test]
fn a_window_set_apart_is_brought_in_on_its_own() {
    let apart = rect(1100.0, 22.0, 275.0, 116.0);
    let origins = kept_inside(&[MAIN, apart], Size::new(1000.0, 800.0));
    assert_eq!(
        origins,
        vec![Point::new(26.0, 22.0), Point::new(725.0, 22.0)]
    );
}

#[test]
fn a_group_larger_than_the_canvas_keeps_its_top_left_corner_on_screen() {
    let origins = kept_inside(&[rect(-40.0, -10.0, 275.0, 493.0)], Size::new(200.0, 300.0));
    assert_eq!(origins, vec![Point::new(0.0, 0.0)]);
}

#[test]
fn positions_survive_the_trip_through_the_preferences() {
    let positions = [
        Point::new(26.0, 22.0),
        Point::new(301.5, 22.0),
        Point::new(26.0, 254.0),
    ];
    assert_eq!(
        decode_positions(&encode_positions(&positions)),
        Some(positions)
    );
    assert_eq!(decode_positions("1,2;3,4"), None);
    assert_eq!(decode_positions("1,2;3,x;5,6"), None);
    assert_eq!(decode_positions("NaN,2;3,4;5,6"), None);
}

#[test]
fn the_playlist_size_reads_back_as_it_was_kept() {
    let size = Size::new(400.0, 312.5);
    assert_eq!(decode_size(&encode_size(size)), Some(size));
    assert_eq!(decode_size("400"), None);
    assert_eq!(decode_size("400,x"), None);
    assert_eq!(decode_size("inf,300"), None);
    assert_eq!(decode_size("0,300"), None, "a window has an area");
}

#[test]
fn a_window_stretched_from_its_corner_grows_with_the_pointer_and_stops_at_its_minimum() {
    let held = Size::new(275.0, 261.0);
    let minimum = Size::new(275.0, 145.0);
    assert_eq!(
        stretched(held, Point::new(50.0, 40.0), minimum),
        Size::new(325.0, 301.0)
    );
    assert_eq!(
        stretched(held, Point::new(-80.0, -200.0), minimum),
        minimum,
        "pushed past its smallest size it stays there"
    );
}
