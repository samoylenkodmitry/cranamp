#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WidthClass {
    SmallPhone,
    Phone,
    Tablet,
    WideTablet,
    Desktop,
}
impl WidthClass {
    pub(super) fn at(width: f32) -> Self {
        match width {
            w if w < 380. => Self::SmallPhone,
            w if w < 600. => Self::Phone,
            w if w < 900. => Self::Tablet,
            w if w < 1280. => Self::WideTablet,
            _ => Self::Desktop,
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::SmallPhone => "Small phone",
            Self::Phone => "Phone",
            Self::Tablet => "Tablet",
            Self::WideTablet => "Wide tablet",
            Self::Desktop => "Desktop",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}
impl Area {
    pub(super) fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w: w.max(1.),
            h: h.max(1.),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layout {
    pub class: WidthClass,
    pub width: f32,
    pub height: f32,
    pub canvas: Area,
    pub rail: Option<Area>,
    pub inspector: Option<Area>,
    pub references: Option<Area>,
    pub tools: Area,
    pub context: Area,
    pub properties: Option<Area>,
    pub side_tools: bool,
}
impl Layout {
    pub(super) fn new(width: f32, height: f32) -> Self {
        let width = width.max(240.);
        let height = height.max(200.);
        let class = WidthClass::at(width);
        let short = height < 500.;
        let gap = 8.;
        let top = if class == WidthClass::Desktop || height < 500. {
            64.
        } else {
            112.
        };
        let rail_width = if class == WidthClass::Desktop {
            224.
        } else {
            0.
        };
        let inspector_width = match class {
            WidthClass::WideTablet => 272.,
            WidthClass::Desktop => 304.,
            _ => 0.,
        };
        let side_tools = short
            || matches!(
                class,
                WidthClass::Tablet | WidthClass::WideTablet | WidthClass::Desktop
            );
        let tools_x = gap
            + if rail_width > 0. {
                rail_width + gap
            } else {
                0.
            };
        let left = tools_x + if side_tools { 48. + gap } else { 0. };
        let reference_width = if width >= 1680. { 248. } else { 0. };
        let inspector_x = width - gap - inspector_width;
        let reference_x = inspector_x
            - if reference_width > 0. {
                reference_width + gap
            } else {
                0.
            };
        let right = reference_x - if inspector_width > 0. { gap } else { 0. };
        let room = right - left;
        let properties_h = if inspector_width == 0. { 68. } else { 0. };
        let toolbar_h = if side_tools { 0. } else { 48. };
        let bottom = height - gap - properties_h - toolbar_h;
        Self {
            class,
            width,
            height,
            side_tools,
            tools: if side_tools {
                Area::new(tools_x, top + 48., 48., height - top - 56.)
            } else {
                Area::new(gap, bottom, width - gap * 2., 44.)
            },
            context: Area::new(left, top, room, 48.),
            properties: (properties_h > 0.)
                .then(|| Area::new(left, height - gap - properties_h, room, properties_h)),
            canvas: Area::new(left, top + 48., room, (bottom - top - 92.).max(1.)),
            rail: (rail_width > 0.).then(|| Area::new(gap, top, rail_width, height - top - gap)),
            references: (reference_width > 0.)
                .then(|| Area::new(reference_x, top, reference_width, height - top - gap)),
            inspector: (inspector_width > 0.)
                .then(|| Area::new(inspector_x, top, inspector_width, height - top - gap)),
        }
    }
    pub(super) fn for_navigation(mut self) -> Self {
        if let Some(properties) = self.properties.take() {
            self.canvas.h += properties.h;
            if !self.side_tools {
                self.tools.y += properties.h;
            }
        }
        self
    }
    pub(super) fn panel(self, full: bool) -> Area {
        if full || self.class == WidthClass::SmallPhone || self.height < 500. {
            Area::new(0., 0., self.width, self.height)
        } else if let Some(inspector) = self.inspector {
            inspector
        } else {
            let h = (self.height * 0.55).min(self.height - 104.);
            Area::new(0., self.height - h, self.width, h)
        }
    }
}
