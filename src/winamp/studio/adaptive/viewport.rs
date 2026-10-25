use cranpose_ui_graphics::Point;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Viewport {
    pub scale: f32,
    pub scroll: [f32; 2],
    pub content: [f32; 2],
    pub room: [f32; 2],
}
impl Default for Viewport {
    fn default() -> Self {
        Self {
            scale: 1.,
            scroll: [0.; 2],
            content: [275., 377.],
            room: [275., 377.],
        }
    }
}
impl Viewport {
    pub fn limits(self) -> [f32; 2] {
        std::array::from_fn(|axis| {
            let overflow = self.content[axis] * self.scale - self.room[axis];
            if overflow < 0.001 {
                0.
            } else {
                overflow
            }
        })
    }
    pub fn origin(self) -> [f32; 2] {
        [
            ((self.room[0] - self.content[0] * self.scale) / 2.).max(0.) - self.scroll[0],
            ((self.room[1] - self.content[1] * self.scale) / 2.).max(0.) - self.scroll[1],
        ]
    }
    pub fn constrain(&mut self) {
        let limits = self.limits();
        for (v, max) in self.scroll.iter_mut().zip(limits) {
            *v = v.clamp(0., max);
        }
    }
    pub fn resize(&mut self, room: [f32; 2], content: [f32; 2]) {
        if self.room == room && self.content == content {
            return;
        }
        let fitted = (self.scale
            - (self.room[0] / self.content[0])
                .min(self.room[1] / self.content[1])
                .clamp(0.05, 32.))
        .abs()
            < 0.001;
        let old = self.origin();
        let centre = [
            (self.room[0] / 2. - old[0]) / self.scale,
            (self.room[1] / 2. - old[1]) / self.scale,
        ];
        self.room = room;
        self.content = content;
        if fitted {
            self.fit();
            return;
        }
        self.scroll = [
            centre[0] * self.scale - room[0] / 2.,
            centre[1] * self.scale - room[1] / 2.,
        ];
        self.constrain();
    }
    pub fn transform(&mut self, anchor: Point, pan: Point, factor: f32) {
        if !factor.is_finite() || factor <= 0. {
            return;
        }
        let old = self.origin();
        let next = (self.scale * factor).clamp(0.05, 32.);
        let pixel = [
            (anchor.x - old[0]) / self.scale,
            (anchor.y - old[1]) / self.scale,
        ];
        self.scale = next;
        self.scroll = [
            pixel[0] * next - anchor.x - pan.x,
            pixel[1] * next - anchor.y - pan.y,
        ];
        self.constrain();
    }
    pub fn point(self, point: Point) -> [i32; 2] {
        let o = self.origin();
        [
            ((point.x - o[0]) / self.scale).floor() as i32,
            ((point.y - o[1]) / self.scale).floor() as i32,
        ]
    }
    pub fn fit(&mut self) {
        self.scale = (self.room[0] / self.content[0])
            .min(self.room[1] / self.content[1])
            .clamp(0.05, 32.);
        self.scroll = [0.; 2];
    }
}
