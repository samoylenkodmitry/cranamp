//! Scrollbars for Library lists. Lazy lists jump by item instead of measuring
//! every intervening row when the user drags across a large music collection.
use super::*;
use cranpose_foundation::lazy::LazyListIntervalContent;
use cranpose_ui::scroll::{ScrollMetrics, ScrollState};
use cranpose_ui::scrollbar::{content_delta_for_thumb_drag, ThumbBounds};
use cranpose_ui::{Axis, Brush, CornerRadii, DrawScope, Point, Rect, Size};

const HIT_WIDTH: f32 = 44.;
const GUTTER: f32 = 32.;

#[derive(Clone, Copy, PartialEq)]
struct LazyMetrics {
    state: LazyListState,
    count: usize,
    row_height: f32,
    viewport: f32,
}

impl LazyMetrics {
    fn row_extent(self) -> f32 {
        // Measurement rounds row sizes to pixels. Ignore stale layout data
        // when switching between lists that have different row heights.
        self.state
            .with_layout_info(|info| {
                info.visible_items_info
                    .first()
                    .map(|item| item.size)
                    .filter(|size| (*size - self.row_height).abs() <= 1.)
            })
            .flatten()
            .unwrap_or(self.row_height)
    }

    fn get(self) -> ScrollMetrics {
        let extent = self.row_extent();
        let max_offset = (self.count as f32 * extent - self.viewport).max(0.);
        let mut offset = self.state.first_visible_item_index_non_reactive() as f32 * extent
            + self.state.first_visible_item_scroll_offset_non_reactive();
        if self.state.can_scroll_backward_non_reactive()
            && !self.state.can_scroll_forward_non_reactive()
        {
            offset = max_offset;
        }
        ScrollMetrics {
            offset: offset.clamp(0., max_offset),
            max_offset,
            viewport_extent: self.viewport,
        }
    }

    fn seek(self, offset: f32) {
        if self.count == 0 || !offset.is_finite() {
            return;
        }
        let offset = offset.clamp(0., self.get().max_offset);
        let extent = self.row_extent();
        let index = ((offset / extent) as usize).min(self.count - 1);
        self.state.stop_scroll_animation();
        self.state
            .scroll_to_item(index, offset - index as f32 * extent);
    }
}

#[composable]
pub(super) fn List(
    modifier: Modifier,
    state: LazyListState,
    spec: cranpose_ui::LazyColumnSpec,
    count: usize,
    row_height: f32,
    content: impl FnMut(&mut LazyListIntervalContent) + 'static,
) {
    let mut content = content;
    BoxWithConstraints(modifier, move |scope| {
        let bounds = scope.constraints();
        // Bottom content padding makes room for the player dock, navigation
        // and selection toolbar. Keep the scrollbar above those controls.
        let viewport = (bounds.max_height - spec.content_padding.bottom).max(0.);
        let metrics = LazyMetrics {
            state,
            count,
            row_height,
            viewport,
        };
        let overflow = metrics.get().max_offset > 0. && viewport > 0.;
        let mut spec = spec.clone();
        if overflow {
            spec.content_padding.end += GUTTER;
        }
        cranpose_ui::LazyColumn(Modifier::empty().fill_max_size(), state, spec, |scope| {
            content(scope)
        });
        if overflow {
            // Subscribe only the scrollbar to item-position changes. The
            // content remains lazy even during a jump through thousands of rows.
            LazyRail(
                Modifier::empty()
                    .offset((bounds.max_width - HIT_WIDTH).max(0.), 2.)
                    .size_points(HIT_WIDTH, (viewport - 4.).max(0.)),
                metrics,
            );
        }
    });
}

#[composable]
fn LazyRail(modifier: Modifier, metrics: LazyMetrics) {
    let _position = (
        metrics.state.first_visible_item_index(),
        metrics.state.first_visible_item_scroll_offset(),
        metrics.state.can_scroll_forward(),
        metrics.state.can_scroll_backward(),
    );
    Rail(
        modifier,
        Rc::new(move || metrics.get()),
        Rc::new(move |offset| metrics.seek(offset)),
    );
}

#[composable(no_skip)]
fn Rail(modifier: Modifier, metrics: Rc<dyn Fn() -> ScrollMetrics>, seek: Rc<dyn Fn(f32)>) {
    let colors = liquid_colors();
    BoxWithConstraints(modifier, move |scope| {
        let height = scope.constraints().max_height;
        let bounds = ThumbBounds::at_least(44_f32.min(height * 0.5), height);
        let drag_metrics = metrics.clone();
        let drag_seek = seek.clone();
        let dragged = cranpose_ui::draggable::rememberDraggableState(move |delta| {
            let metrics = drag_metrics();
            if let Some(thumb) = metrics.thumb(bounds) {
                drag_seek(
                    metrics.offset
                        + content_delta_for_thumb_drag(delta, height, thumb, metrics.max_offset),
                );
            }
        });
        let progress_metrics = metrics.clone();
        let progress_seek = seek.clone();
        let draw_metrics = metrics.clone();
        let drawing_drag = dragged.clone();
        Canvas(
            Modifier::empty()
                .fill_max_size()
                .stable_semantics(player_slider_semantics(
                    "Library scrollbar".into(),
                    metrics().progress(),
                    Rc::new(move |progress| {
                        progress_seek(progress * progress_metrics().max_offset);
                    }),
                    Rc::new(|_| {}),
                ))
                .draggable(Axis::Vertical, dragged),
            move |scope: &mut dyn DrawScope| {
                let Some(thumb) = draw_metrics().thumb(bounds) else {
                    return;
                };
                let x = (scope.size().width - 10.).max(0.);
                let radii = CornerRadii::uniform(3.);
                scope.draw_round_rect_at(
                    Rect::from_origin_size(Point::new(x, 0.), Size::new(6., height)),
                    Brush::Solid(colors.separator.with_alpha(0.15)),
                    radii,
                );
                scope.draw_round_rect_at(
                    Rect::from_origin_size(
                        Point::new(x, thumb.offset * height),
                        Size::new(6., thumb.length * height),
                    ),
                    Brush::Solid(if drawing_drag.is_dragging() {
                        colors.accent
                    } else {
                        colors.secondary_label.with_alpha(0.6)
                    }),
                    radii,
                );
            },
        );
    });
}

/// Measure sheet content first so adding a scrollbar does not stretch a short
/// action sheet to the screen's full height.
#[composable]
pub(super) fn SheetColumn(content: impl FnMut() + 'static) {
    use cranpose_core::SlotId;
    use cranpose_ui::{Constraints, Placement, SubcomposeLayoutScope, SubcomposeMeasureScope};
    let scroll: ScrollState = cranpose_ui::rememberScrollState!(0.);
    let content = Rc::new(RefCell::new(content));
    cranpose_ui::SubcomposeLayout(
        Modifier::empty().fill_max_width(),
        move |scope, constraints| {
            let child_content = content.clone();
            let children = scope.subcompose(SlotId::new(0), constraints, move || {
                let _offset = scroll.value();
                let child_content = child_content.clone();
                Column(
                    Modifier::empty()
                        .fill_max_width()
                        .vertical_scroll(scroll, false)
                        .padding_each(20., 8., HIT_WIDTH, 32.),
                    ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(10.)),
                    move || (child_content.borrow_mut())(),
                );
            });
            let child = scope.measure(children[0], constraints);
            let width = child.width();
            let height = child.height();
            child.place(0., 0.);
            let mut placements = vec![Placement::new(child.node_id(), 0., 0., 0)];
            if scroll.max_value() > 0. && height > 4. {
                let rail_constraints = Constraints::tight(HIT_WIDTH, height - 4.);
                let bars = scope.subcompose(SlotId::new(1), rail_constraints, move || {
                    let _offset = scroll.value();
                    Rail(
                        Modifier::empty().fill_max_size(),
                        Rc::new(move || scroll.metrics()),
                        Rc::new(move |offset| {
                            scroll.dispatch_raw_delta(offset - scroll.value_non_reactive());
                        }),
                    );
                });
                let bar = scope.measure(bars[0], rail_constraints);
                bar.place(width - HIT_WIDTH, 2.);
                placements.push(Placement::new(bar.node_id(), width - HIT_WIDTH, 2., 1));
            }
            scope.layout(width, height, placements)
        },
    );
}
