use super::*;

#[test]
fn a_playlist_takes_only_whole_steps_from_the_base() {
    assert_eq!(classic(Size::new(275.0, 116.0)), Size::new(275.0, 116.0));
    assert_eq!(classic(Size::new(275.0, 261.0)), Size::new(275.0, 261.0));
    assert_eq!(
        classic(Size::new(313.0, 150.0)),
        Size::new(325.0, 145.0),
        "38 across is nearer two steps than one; 34 down nearer one than two"
    );
    assert_eq!(
        classic(Size::new(100.0, 20.0)),
        Size::new(275.0, 116.0),
        "never smaller than the base"
    );
    assert_eq!(
        classic(Size::new(f32::NAN, f32::INFINITY)),
        Size::new(275.0, 116.0)
    );
}

#[test]
fn a_corner_dragged_part_of_a_step_lands_on_the_nearest_one() {
    let held = Size::new(275.0, 261.0);
    assert_eq!(stretched(held, Point::new(12.0, 14.0)), held);
    assert_eq!(
        stretched(held, Point::new(13.0, 15.0)),
        Size::new(300.0, 290.0)
    );
    assert_eq!(
        stretched(held, Point::new(-60.0, -200.0)),
        Size::new(275.0, 116.0)
    );
}

#[test]
fn the_visualizer_panel_appears_three_steps_past_the_base_beside_the_right_corner() {
    assert_eq!(visualizer_x(275.0), None);
    assert_eq!(visualizer_x(325.0), None);
    assert_eq!(visualizer_x(350.0), Some(125.0));
    assert_eq!(visualizer_x(400.0), Some(175.0));
}

#[test]
fn the_right_corner_moves_with_the_right_edge() {
    assert_eq!(right_corner_shift(275.0), 0.0);
    assert_eq!(right_corner_shift(400.0), 125.0);
}
