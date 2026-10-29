use super::*;
use cranpose_app_shell::AppShell;
use cranpose_foundation::PointerSource;
use cranpose_ui::SemanticsNode;

use super::test_common as common;

type FixtureHandles = Rc<Cell<Option<(MutableState<WinampState>, PlaylistScrollHandles)>>>;

struct PlaylistFixture {
    shell: AppShell<common::HitGraphRenderer>,
    handles: FixtureHandles,
    shown: Rc<Cell<Option<MutableState<bool>>>>,
}

impl PlaylistFixture {
    fn new(count: usize) -> Self {
        let handles = Rc::new(Cell::new(None));
        let captured = handles.clone();
        let shown = Rc::new(Cell::new(None));
        let captured_shown = shown.clone();
        let artwork = load_skin(BUNDLED_SKINS[0].bytes).unwrap();
        let mut shell = AppShell::new(
            common::HitGraphRenderer::default(),
            cranpose_core::location_key(file!(), line!(), column!()),
            move || {
                let state = cranpose_core::rememberMutableStateOf(|| WinampState {
                    playlist: Rc::new(
                        (0..count)
                            .map(|index| Track {
                                title: format!("Track {index:04}"),
                                path: None,
                                duration_seconds: None,
                            })
                            .collect(),
                    ),
                    selected_indices: vec![0],
                    current_index: Some(0),
                    ..WinampState::default()
                });
                let snapshot = state.get();
                let shown = cranpose_core::rememberMutableStateOf(|| true);
                captured_shown.set(Some(shown));
                let scroll = PlaylistScrollHandles {
                    list: rememberLazyListState(),
                    dragging: cranpose_core::rememberMutableStateOf(|| false),
                    max_start: count.saturating_sub(playlist_visible_row_capacity(111.0, 2.0)),
                    row_height: 22.0,
                };
                captured.set(Some((state, scroll)));
                let artwork = artwork.clone();
                Box(
                    Modifier::empty().fill_max_size(),
                    BoxSpec::default(),
                    move || {
                        if !shown.get() {
                            return;
                        }
                        PlaylistEntries(
                            artwork.palette,
                            state,
                            snapshot.clone(),
                            scroll,
                            250.0,
                            111.0,
                            2.0,
                        );
                        PlaylistScrollbar(artwork.pledit.clone(), 260.0, 111.0, scroll, 2.0);
                    },
                );
            },
        );
        shell.set_buffer_size(560, 320);
        shell.set_viewport(560.0, 320.0);
        shell.set_semantics_enabled(true);
        common::pump(&mut shell);
        Self {
            shell,
            handles,
            shown,
        }
    }

    fn position(&self) -> f32 {
        let scroll = self.handles.get().unwrap().1;
        scroll.list.first_visible_item_index_non_reactive() as f32 * scroll.row_height
            + scroll.list.first_visible_item_scroll_offset_non_reactive()
    }

    fn swipe(&mut self) {
        self.shell.set_pointer_source(PointerSource::Touch);
        self.shell.set_cursor_at_time(120.0, 220.0, Some(1000));
        assert!(self.shell.pointer_pressed_at_time(Some(1000)));
        for step in 1..=6 {
            self.shell.set_cursor_at_time(
                120.0,
                220.0 - step as f32 * 20.0,
                Some(1000 + step * 16),
            );
            common::pump(&mut self.shell);
        }
        assert!(self.shell.pointer_released_at_time(Some(1112)));
        common::pump(&mut self.shell);
    }
}

fn find<'a>(node: &'a SemanticsNode, label: &str) -> Option<&'a SemanticsNode> {
    if node.hidden {
        return None;
    }
    if node.description.as_deref() == Some(label) {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, label))
}

#[test]
fn touch_swipe_scrolls_and_flings_without_selecting_a_track() {
    let mut fixture = PlaylistFixture::new(10_000);
    fixture.swipe();
    let released = fixture.position();
    assert!(
        released > 40.0,
        "The rows did not follow the finger: {released}"
    );
    let (state, scroll) = fixture.handles.get().unwrap();
    assert!(
        scroll.list.is_scroll_in_progress(),
        "A quick swipe should keep moving after release"
    );
    for _ in 0..15 {
        std::thread::sleep(Duration::from_millis(16));
        common::pump(&mut fixture.shell);
    }
    assert!(
        fixture.position() > released + 10.0,
        "The fling did not advance"
    );
    assert_eq!(state.get_non_reactive().selected_indices, vec![0]);
    assert_eq!(state.get_non_reactive().current_index, Some(0));
    assert_eq!(state.get_non_reactive().playlist_last_click_index, None);
    let tree = fixture.shell.semantics_tree().unwrap();
    let thumb = find(tree.root(), "Playlist scroll position").unwrap();
    let progress = thumb.details().progress.as_ref().unwrap().current;
    assert!(progress > 0.0);
    assert!((progress - scroll.fraction()).abs() < 0.001);
    assert!(
        scroll.list.layout_info().visible_items_info.len() < 30,
        "The list must remain virtualized"
    );
}

#[test]
fn short_playlists_stay_finite_and_still_accept_taps() {
    let mut fixture = PlaylistFixture::new(3);
    fixture.swipe();
    assert_eq!(fixture.position(), 0.0);
    let (state, scroll) = fixture.handles.get().unwrap();
    assert_eq!(scroll.fraction(), 0.0);
    assert!(state.get_non_reactive().playlist_scroll.is_finite());
    let bounds = find(fixture.shell.semantics_tree().unwrap().root(), "Track 0001")
        .unwrap()
        .bounds;
    fixture
        .shell
        .set_cursor(bounds.x + 20.0, bounds.y + bounds.height * 0.5);
    fixture.shell.pointer_pressed();
    fixture.shell.pointer_released();
    common::pump(&mut fixture.shell);
    assert_eq!(state.get_non_reactive().selected_indices, vec![1]);
}

#[test]
fn scrollbar_and_saved_position_move_the_same_list() {
    let mut fixture = PlaylistFixture::new(200);
    let (state, scroll) = fixture.handles.get().unwrap();
    state.update(|s| s.playlist_scroll = 0.5);
    common::pump(&mut fixture.shell);
    assert!((scroll.fraction() - 0.5).abs() < 0.001);
    scroll.dragging.set(true);
    scroll.scroll_to(1.0);
    common::pump(&mut fixture.shell);
    scroll.dragging.set(false);
    common::pump(&mut fixture.shell);
    assert!((state.get_non_reactive().playlist_scroll - 1.0).abs() < 0.001);
    assert!(find(fixture.shell.semantics_tree().unwrap().root(), "Track 0199").is_some());
}

#[test]
fn mouse_wheel_scrolls_rows_without_selecting_them() {
    let mut fixture = PlaylistFixture::new(200);
    fixture.shell.set_pointer_source(PointerSource::Mouse);
    fixture.shell.set_cursor(120.0, 150.0);
    assert!(fixture.shell.pointer_scrolled(0.0, -120.0));
    common::pump(&mut fixture.shell);
    assert!(fixture.position() > 0.0);
    let (state, _) = fixture.handles.get().unwrap();
    assert_eq!(state.get_non_reactive().selected_indices, vec![0]);
}

#[test]
fn empty_playlist_keeps_zero_scroll_and_its_status_text() {
    let mut fixture = PlaylistFixture::new(0);
    fixture.swipe();
    let (state, scroll) = fixture.handles.get().unwrap();
    assert_eq!(scroll.fraction(), 0.0);
    assert_eq!(state.get_non_reactive().playlist_scroll, 0.0);
    let status = state.get_non_reactive().status;
    assert!(common::visible_texts(&mut fixture.shell)
        .iter()
        .any(|text| text == &status));
}

#[test]
fn leaving_the_player_during_a_fling_keeps_its_position() {
    let mut fixture = PlaylistFixture::new(200);
    fixture.swipe();
    let (state, scroll) = fixture.handles.get().unwrap();
    let position = scroll.fraction();
    assert!(position > 0.0);
    fixture.shown.get().unwrap().set(false);
    common::pump(&mut fixture.shell);
    assert!((state.get_non_reactive().playlist_scroll - position).abs() < 0.01);
    assert!(!playlist_scroll_drag_active());
    fixture.shown.get().unwrap().set(true);
    common::pump(&mut fixture.shell);
    assert!((scroll.fraction() - position).abs() < 0.01);
}
