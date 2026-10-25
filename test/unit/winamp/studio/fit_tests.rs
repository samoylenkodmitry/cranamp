use super::{
    layout::{Layout, WidthClass},
    viewport::Viewport,
};
use cranpose_ui_graphics::Point;

#[test]
fn fitted_canvas_remains_fitted_after_resizing() {
    let mut viewport = Viewport::default();
    viewport.fit();
    for room in [[528., 116.], [220., 360.], [800., 700.], [1200., 900.]] {
        viewport.resize(room, [275., 377.]);
        assert!(viewport.content[0] * viewport.scale <= room[0] + 0.001);
        assert!(viewport.content[1] * viewport.scale <= room[1] + 0.001);
        assert_eq!(viewport.limits(), [0., 0.]);
        assert_eq!(viewport.scroll, [0., 0.]);
    }
}
#[test]
fn width_selects_the_same_layout_at_every_height() {
    for (width, class) in [
        (320., WidthClass::SmallPhone),
        (379., WidthClass::SmallPhone),
        (380., WidthClass::Phone),
        (599., WidthClass::Phone),
        (600., WidthClass::Tablet),
        (899., WidthClass::Tablet),
        (900., WidthClass::WideTablet),
        (1279., WidthClass::WideTablet),
        (1280., WidthClass::Desktop),
        (1920., WidthClass::Desktop),
    ] {
        for height in [240., 390., 780., 1200.] {
            let layout = Layout::new(width, height);
            assert_eq!(layout.class, class);
            assert!(layout.canvas.w > 0. && layout.canvas.h > 0.);
            assert!(layout.canvas.x + layout.canvas.w <= width);
            assert!(layout.canvas.y + layout.canvas.h <= height);
        }
    }
}
#[test]
fn tablet_and_desktop_preserve_canvas_space() {
    for width in [600., 768., 900., 1100., 1280., 1920.] {
        let layout = Layout::new(width, 900.);
        assert!(layout.canvas.w >= 360.);
        assert_eq!(layout.rail.is_some(), width >= 1280.);
        assert_eq!(layout.inspector.is_some(), width >= 900.);
    }
}
#[test]
fn pinch_keeps_the_artwork_under_the_gesture_anchor() {
    let mut view = Viewport {
        scale: 4.,
        scroll: [120., 140.],
        content: [275., 377.],
        room: [300., 400.],
    };
    let at = Point::new(100., 160.);
    let before = view.point(at);
    view.transform(at, Point::new(12., -8.), 1.25);
    assert_eq!(view.point(Point::new(at.x + 12., at.y - 8.)), before);
    assert_eq!(view.scale, 5.);
}
#[test]
fn resizing_preserves_the_centre_pixel_and_scroll_is_bounded() {
    let mut view = Viewport {
        scale: 4.,
        scroll: [120., 140.],
        content: [275., 377.],
        room: [300., 400.],
    };
    let before = view.point(Point::new(150., 200.));
    view.resize([500., 600.], [275., 377.]);
    assert_eq!(view.point(Point::new(250., 300.)), before);
    view.transform(Point::ZERO, Point::new(90000., -90000.), 1.);
    assert_eq!(view.scroll, [0., view.limits()[1]]);
    view.transform(Point::ZERO, Point::ZERO, f32::NAN);
    assert!(view.scale.is_finite());
}
