use super::*;

pub(super) const HEIGHT: f32 = 86.;
pub(super) const TOP_INSET: f32 = 10.;
pub(super) const SIDE_MARGIN: f32 = 24.;
pub(super) const TOP_MARGIN: f32 = 16.;
pub(super) const PLAYER_GAP: f32 = SIDE_MARGIN * 1.5;
#[cfg(not(target_arch = "wasm32"))]
pub(super) const REFLECTION_GAP: f32 = 8.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Navigation {
    External,
    Desktop(bool),
    Mobile(bool),
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    MobileAutomatic(bool),
}

impl Navigation {
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn mobile(width: f32) -> Self {
        Self::MobileAutomatic(width < 600.)
    }

    pub(super) fn visible(self) -> bool {
        matches!(
            self,
            Self::Desktop(true) | Self::Mobile(true) | Self::MobileAutomatic(true)
        )
    }

    pub(super) fn toggle(&mut self) {
        match self {
            Self::Desktop(visible) => *visible = !*visible,
            Self::Mobile(visible) => *visible = !*visible,
            Self::MobileAutomatic(visible) => *self = Self::Mobile(!*visible),
            Self::External => {}
        }
    }
}

pub(super) fn desktop_frame(width: f32, height: f32, visible: bool) -> (Rect, f32, f32) {
    if !visible {
        return (Rect::from_size(Size::new(width, height)), height, 1.);
    }
    let scale = (width - SIDE_MARGIN * 2.).max(1.) / width;
    let player = Rect::from_origin_size(
        Point::new(SIDE_MARGIN, TOP_MARGIN),
        Size::new(width * scale, height * scale),
    );
    let window_height = (player.y + player.height + HEIGHT - TOP_INSET + PLAYER_GAP).ceil();
    (player, window_height, scale)
}
