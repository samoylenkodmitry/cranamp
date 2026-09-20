use super::presentation_origin;
#[test]
fn presentation_centers_on_native_pixels_in_odd_and_even_scenes() {
    assert_eq!(
        presentation_origin([1157., 871.], [550., 754.]),
        [303., 58.]
    );
    assert_eq!(
        presentation_origin([1160., 850.], [550., 754.]),
        [305., 48.]
    );
    for scene_width in [1157., 1158.] {
        for scene_height in [870., 871.] {
            for player in [[275., 377.], [550., 754.], [275., 754.]] {
                let scene = [scene_width, scene_height];
                let origin = presentation_origin(scene, player);
                for axis in 0..2 {
                    let far_margin = scene[axis] - player[axis] - origin[axis];
                    assert_eq!(origin[axis].fract(), 0.);
                    assert!((0. ..=1.).contains(&(far_margin - origin[axis])));
                }
            }
        }
    }
}
#[test]
fn presentation_keeps_centered_overflow_when_scene_is_smaller() {
    assert_eq!(
        presentation_origin([501., 701.], [550., 754.]),
        [-25., -27.]
    );
    assert_eq!(
        presentation_origin([500., 700.], [550., 754.]),
        [-25., -27.]
    );
}
