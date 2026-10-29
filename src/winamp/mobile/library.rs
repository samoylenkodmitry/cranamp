//! A touch library over the player's existing playlist. No media is copied or indexed here.
use super::*;

const MUSIC: &str = "M9 3v12.2A3.5 3.5 0 1 0 11 18V7h8v6.2A3.5 3.5 0 1 0 21 16V3H9z";
const PAUSE: &str = "M6 5h4v14H6zM14 5h4v14h-4z";
const NEXT: &str = "M5 5v14l10-7zM17 5h3v14h-3z";

#[derive(Clone, Debug, PartialEq)]
enum Sheet {
    None,
    Add,
    Manage,
    Sort,
    Track(TrackTarget),
    Remove(Removal),
    Import,
    Address(bool),
}

#[derive(Clone, Debug, PartialEq)]
struct TrackTarget {
    playlist: Rc<Vec<Track>>,
    index: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct Removal {
    playlist: Rc<Vec<Track>>,
    indices: Vec<usize>,
}

/// Appending scan results keeps these indices valid. Replacing or reordering the
/// playlist invalidates a pending action, so it cannot affect a different track.
fn unchanged_prefix(before: &[Track], after: &[Track]) -> bool {
    after.starts_with(before)
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct LibraryUi {
    query: TextFieldState,
    search_focused: MutableState<bool>,
    last_query: MutableState<String>,
    address: TextFieldState,
    editing: MutableState<bool>,
    sheet: MutableState<Sheet>,
    notice: MutableState<String>,
    list: LazyListState,
}

#[composable]
pub(super) fn remember_library_ui() -> LibraryUi {
    cranpose_core::remember(|| LibraryUi {
        query: TextFieldState::new(""),
        search_focused: cranpose_core::mutableStateOf(false),
        last_query: cranpose_core::mutableStateOf(String::new()),
        address: TextFieldState::new(""),
        editing: cranpose_core::mutableStateOf(false),
        sheet: cranpose_core::mutableStateOf(Sheet::None),
        notice: cranpose_core::mutableStateOf(String::new()),
        list: LazyListState::new(0, 0.0),
    })
    .with(|ui| *ui)
}

#[derive(Default)]
struct FilterCache {
    playlist: Option<Rc<Vec<Track>>>,
    query: String,
    rows: Rc<Vec<usize>>,
    summary: String,
}

impl FilterCache {
    fn project(&mut self, playlist: &Rc<Vec<Track>>, query: &str) -> Rc<Vec<usize>> {
        let changed = self
            .playlist
            .as_ref()
            .is_none_or(|old| !Rc::ptr_eq(old, playlist));
        if changed || self.query != query {
            let terms: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
            self.rows = Rc::new(
                playlist
                    .iter()
                    .enumerate()
                    .filter_map(|(index, track)| {
                        let matches = terms.is_empty() || {
                            let haystack =
                                format!("{} {}", track.title, track.path.as_deref().unwrap_or(""))
                                    .to_lowercase();
                            terms.iter().all(|term| haystack.contains(term))
                        };
                        matches.then_some(index)
                    })
                    .collect(),
            );
            self.query = query.to_owned();
        }
        if changed {
            self.summary = library_summary(playlist);
            self.playlist = Some(playlist.clone());
        }
        self.rows.clone()
    }
}

fn library_summary(tracks: &[Track]) -> String {
    let count = tracks.len();
    let label = if count == 1 { "track" } else { "tracks" };
    if tracks.iter().any(|track| {
        track
            .duration_seconds
            .is_none_or(|seconds| !seconds.is_finite() || seconds <= 0.0)
    }) {
        return format!("{count} {label}");
    }
    match playlist_total_duration_seconds(tracks) {
        Some(seconds) if seconds > 0.0 => {
            format!("{count} {label} · {}", format_duration_compact(seconds))
        }
        _ => format!("{count} {label}"),
    }
}

fn track_subtitle(track: &Track) -> String {
    parsed_track_artist(&track.title)
        .map(str::to_owned)
        .filter(|artist| !artist.is_empty())
        .unwrap_or_else(|| {
            let folder = playlist_parent_folder(track);
            if folder.is_empty() {
                "Audio track".into()
            } else {
                folder
            }
        })
}

/// Move by index, preserving the exact current occurrence even with duplicate paths.
fn move_track(state: &mut WinampState, from: usize, to: usize) -> bool {
    let len = state.playlist.len();
    if from >= len || to >= len || from == to {
        return false;
    }
    let remap = |index: usize| {
        if index == from {
            to
        } else if from < to && index > from && index <= to {
            index - 1
        } else if to < from && index >= to && index < from {
            index + 1
        } else {
            index
        }
    };
    let tracks = playlist_tracks_mut(state);
    let moved = tracks.remove(from);
    tracks.insert(to, moved);
    state.current_index = state.current_index.map(remap);
    state
        .selected_indices
        .iter_mut()
        .for_each(|index| *index = remap(*index));
    state.selection_anchor = state.selection_anchor.map(remap);
    state.playlist_last_click_index = None;
    state
        .shuffle_order
        .iter_mut()
        .for_each(|index| *index = remap(*index));
    normalize_playlist_selection(state);
    state.status = "Playlist order updated".into();
    true
}

fn play_next(state: &mut WinampState, index: usize) {
    if index >= state.playlist.len() || state.current_index == Some(index) {
        return;
    }
    let destination = state
        .current_index
        .map_or(0, |current| current + usize::from(index > current));
    move_track(state, index, destination);
    if state.shuffle {
        state.shuffle_order.retain(|item| *item != destination);
        let at = state
            .current_index
            .and_then(|current| state.shuffle_order.iter().position(|item| *item == current))
            .map_or(0, |at| at + 1);
        state.shuffle_order.insert(at, destination);
    }
    state.status = "Track queued next".into();
}

fn toggle_selection(player: MutableState<WinampState>, index: usize) {
    player.update(|state| {
        if index >= state.playlist.len() {
            return;
        }
        if state.selected_indices.contains(&index) {
            state.selected_indices.retain(|item| *item != index);
        } else {
            state.selected_indices.push(index);
        }
        normalize_playlist_selection(state);
    });
}

#[composable]
pub(super) fn LibraryScreen(player: MutableState<WinampState>, ui: LibraryUi) {
    let focus = cranpose_ui::local_focus_manager().current();
    cranpose_services::BackHandler(ui.editing.get(), move || ui.editing.set(false));
    cranpose_services::BackHandler(ui.sheet.get() != Sheet::None, move || {
        ui.sheet.set(Sheet::None)
    });
    let snapshot = player.get();
    let query = ui.query.text();
    let cache = cranpose_core::remember(|| RefCell::new(FilterCache::default()));
    let rows = cache.with(|cell| cell.borrow_mut().project(&snapshot.playlist, &query));
    let summary = cache.with(|cell| cell.borrow().summary.clone());
    let changed_query = query.clone();
    cranpose_core::LaunchedEffect(query.clone(), move |_| {
        if ui.last_query.get_non_reactive() != changed_query {
            ui.last_query.set(changed_query);
            ui.list.scroll_to_item(0, 0.0);
        }
    });
    let colors = liquid_colors();
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(colors.background),
        move |scope| {
            let width = scope.max_width().0.min(760.0);
            let compact = scope.max_height().0 < 560.0;
            let rows = rows.clone();
            let snapshot = snapshot.clone();
            let summary = summary.clone();
            let query = query.clone();
            Box(
                Modifier::empty()
                    .size_points(width, scope.max_height().0)
                    .absolute_offset((scope.max_width().0 - width) * 0.5, 0.0),
                BoxSpec::default(),
                move || {
                    let rows = rows.clone();
                    let snapshot = snapshot.clone();
                    let summary = summary.clone();
                    let query = query.clone();
                    let has_selection = !snapshot.selected_indices.is_empty();
                    Column(
                        Modifier::empty().fill_max_size(),
                        ColumnSpec::default(),
                        move || {
                            let summary = summary.clone();
                            let snapshot_header = snapshot.clone();
                            let rows_header = rows.clone();
                            let query_header = query.clone();
                            Column(
                                Modifier::empty().fill_max_width().padding_each(
                                    20.0,
                                    if compact { 8.0 } else { 20.0 },
                                    20.0,
                                    8.0,
                                ),
                                ColumnSpec::default().vertical_arrangement(
                                    LinearArrangement::SpacedBy(if compact { 6.0 } else { 12.0 }),
                                ),
                                move || {
                                    let summary = summary.clone();
                                    Row(
                                        Modifier::empty().fill_max_width(),
                                        RowSpec::default()
                                            .vertical_alignment(VerticalAlignment::CenterVertically)
                                            .horizontal_arrangement(LinearArrangement::SpacedBy(
                                                8.0,
                                            )),
                                        move || {
                                            let summary = summary.clone();
                                            Column(
                                                Modifier::empty().weight(1.0),
                                                ColumnSpec::default(),
                                                move || {
                                                    Text(
                                                        "Library",
                                                        Modifier::empty(),
                                                        settings_text_style(
                                                            if compact { 26.0 } else { 32.0 },
                                                            colors.label,
                                                        ),
                                                    );
                                                    if !compact {
                                                        Text(
                                                            summary.clone(),
                                                            Modifier::empty(),
                                                            settings_text_style(
                                                                14.0,
                                                                colors.secondary_label,
                                                            ),
                                                        );
                                                    }
                                                },
                                            );
                                            GlassIconButton(
                                                Modifier::empty().content_description("Add music"),
                                                GlassButtonSpec::prominent(),
                                                48.0,
                                                move || ui.sheet.set(Sheet::Add),
                                                icons::PLUS,
                                            );
                                            GlassIconButton(
                                                Modifier::empty()
                                                    .content_description("Library options"),
                                                GlassButtonSpec::glass(),
                                                48.0,
                                                move || ui.sheet.set(Sheet::Manage),
                                                icons::MORE_HORIZ,
                                            );
                                        },
                                    );
                                    let has_query = !query_header.is_empty();
                                    Row(
                                        Modifier::empty().fill_max_width(),
                                        RowSpec::default().vertical_alignment(
                                            VerticalAlignment::CenterVertically,
                                        ),
                                        move || {
                                            LiquidSearchField(
                                                Modifier::empty().weight(1.0).on_focus_changed(
                                                    move |state| {
                                                        ui.search_focused.set(state.has_focus())
                                                    },
                                                ),
                                                ui.query,
                                                LiquidSearchFieldSpec {
                                                    placeholder: Some(
                                                        "Search your playlist".into(),
                                                    ),
                                                    ..Default::default()
                                                },
                                            );
                                            if has_query {
                                                GlassIconButton(
                                                    Modifier::empty()
                                                        .content_description("Clear search"),
                                                    GlassButtonSpec::plain(),
                                                    44.0,
                                                    move || {
                                                        set_text_field_text(&ui.query, "");
                                                        focus.clear_focus();
                                                    },
                                                    icons::CLOSE,
                                                );
                                            }
                                            if has_query || ui.search_focused.get() {
                                                GlassButton(
                                                    Modifier::empty()
                                                        .height(44.0)
                                                        .content_description("Finish search"),
                                                    GlassButtonSpec::plain(),
                                                    move || {
                                                        focus.clear_focus();
                                                    },
                                                    || {
                                                        GlassButtonLabel(
                                                            "Done".into(),
                                                            GlassButtonSpec::plain(),
                                                        )
                                                    },
                                                );
                                            }
                                        },
                                    );
                                    if !compact && !ui.editing.get() {
                                        NowPlaying(player, ui, snapshot_header.clone());
                                    }
                                    let selected = snapshot_header.selected_indices.len();
                                    let count = rows_header.len();
                                    let all = snapshot_header.playlist.len();
                                    let visible = rows_header.clone();
                                    Row(
                                        Modifier::empty().fill_max_width().height(44.0),
                                        RowSpec::default()
                                            .vertical_alignment(VerticalAlignment::CenterVertically)
                                            .horizontal_arrangement(LinearArrangement::SpacedBy(
                                                8.0,
                                            )),
                                        move || {
                                            let label = if ui.editing.get() {
                                                format!("{selected} selected")
                                            } else if count != all {
                                                format!("{count} of {all} tracks")
                                            } else {
                                                "Your playlist".into()
                                            };
                                            OneLine(
                                                label,
                                                Modifier::empty().weight(1.0),
                                                16.0,
                                                colors.label,
                                            );
                                            if ui.editing.get() {
                                                let visible = visible.clone();
                                                SmallAction(
                                                    "Select results",
                                                    GlassButtonSpec::plain(),
                                                    move || {
                                                        player.update(|s| {
                                                            set_playlist_selection(
                                                                s,
                                                                visible.iter().copied(),
                                                            )
                                                        })
                                                    },
                                                );
                                                SmallAction(
                                                    "Done",
                                                    GlassButtonSpec::glass(),
                                                    move || ui.editing.set(false),
                                                );
                                            } else {
                                                GlassIconButton(
                                                    Modifier::empty()
                                                        .content_description("Sort playlist"),
                                                    GlassButtonSpec::plain(),
                                                    44.0,
                                                    move || ui.sheet.set(Sheet::Sort),
                                                    icons::FILTER,
                                                );
                                                SmallAction(
                                                    "Select",
                                                    GlassButtonSpec::glass(),
                                                    move || {
                                                        player.update(|s| {
                                                            set_playlist_selection(s, [])
                                                        });
                                                        ui.editing.set(true);
                                                    },
                                                );
                                            }
                                        },
                                    );
                                    let notice = ui.notice.get();
                                    let status = if notice.is_empty() {
                                        snapshot_header.status.clone()
                                    } else {
                                        notice
                                    };
                                    if !status.is_empty()
                                        && !status.starts_with("Restored ")
                                        && !matches!(
                                            status.as_str(),
                                            "Stopped" | "Playing" | "Paused"
                                        )
                                    {
                                        OneLine(
                                            status,
                                            Modifier::empty().fill_max_width(),
                                            13.0,
                                            colors.secondary_label,
                                        );
                                    }
                                },
                            );
                            let rows_list = rows.clone();
                            let snapshot_list = Rc::new(snapshot.clone());
                            let empty = snapshot.playlist.is_empty();
                            let empty_result = rows.is_empty();
                            let query_empty = query.trim().is_empty();
                            Box(
                                Modifier::empty()
                                    .fill_max_width()
                                    .weight(1.0)
                                    .clip_to_bounds(),
                                BoxSpec::default(),
                                move || {
                                    if empty_result {
                                        EmptyLibrary(ui, empty && query_empty);
                                    } else {
                                        let rows = rows_list.clone();
                                        let snapshot = snapshot_list.clone();
                                        cranpose_ui::LazyColumn(
                                            Modifier::empty()
                                                .fill_max_size()
                                                .content_description("Library tracks"),
                                            ui.list,
                                            cranpose_ui::LazyColumnSpec::default().content_padding(
                                                cranpose_ui::PaddingValues {
                                                    start: 12.0,
                                                    end: 12.0,
                                                    bottom: NAV_HEIGHT
                                                        + if ui.editing.get() {
                                                            64.0
                                                        } else {
                                                            12.0
                                                        },
                                                    ..Default::default()
                                                },
                                            ),
                                            move |scope| {
                                                let rows = rows.clone();
                                                let keys = rows.clone();
                                                let snapshot = snapshot.clone();
                                                scope.items(
                                                    LazyItems::new(rows.len())
                                                        .key(move |row| keys[row] as u64),
                                                    move |row| {
                                                        LibraryRow(
                                                            player,
                                                            ui,
                                                            snapshot.clone(),
                                                            rows[row],
                                                        )
                                                    },
                                                );
                                            },
                                        );
                                    }
                                },
                            );
                        },
                    );
                    if ui.editing.get() && has_selection {
                        let bottom = (scope.max_height().0 - NAV_HEIGHT - 64.0).max(0.0);
                        Box(
                            Modifier::empty()
                                .absolute_offset(20.0, bottom)
                                .size_points(width - 40.0, 52.0),
                            BoxSpec::default(),
                            move || {
                                GlassButton(
                                    Modifier::empty()
                                        .fill_max_size()
                                        .content_description("Remove selected tracks"),
                                    GlassButtonSpec::glass().with_content_color(colors.destructive),
                                    move || {
                                        let s = player.get_non_reactive();
                                        ui.sheet.set(Sheet::Remove(Removal {
                                            playlist: s.playlist,
                                            indices: s.selected_indices,
                                        }));
                                    },
                                    move || {
                                        GlassButtonLabel(
                                            "Remove selected".into(),
                                            GlassButtonSpec::destructive(),
                                        )
                                    },
                                );
                            },
                        );
                    }
                },
            );
        },
    );
    if ui.sheet.get() != Sheet::None {
        LibrarySheet(player, ui);
    }
}

#[composable]
fn OneLine(text: String, modifier: Modifier, size: f32, color: Color) {
    BasicText(
        text,
        modifier,
        settings_text_style(size, color),
        TextOverflow::Ellipsis,
        false,
        1,
        1,
    );
}

#[composable]
fn SmallAction(label: &'static str, spec: GlassButtonSpec, action: impl Fn() + 'static) {
    GlassButton(
        Modifier::empty().height(44.0).content_description(label),
        spec.clone(),
        action,
        move || GlassButtonLabel(label.into(), spec.clone()),
    );
}

#[composable]
fn NowPlaying(player: MutableState<WinampState>, ui: LibraryUi, snapshot: WinampState) {
    let Some(track) = snapshot
        .current_index
        .and_then(|i| snapshot.playlist.get(i))
        .cloned()
    else {
        return;
    };
    let colors = liquid_colors();
    let playing = snapshot.playback == PlaybackState::Playing;
    GlassSurface(
        Modifier::empty().fill_max_width(),
        Glass::regular()
            .shape(LiquidShape::RoundedRect(24.0))
            .tint(colors.accent.with_alpha(0.07)),
        move || {
            let track = track.clone();
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .padding_each(16.0, 12.0, 8.0, 12.0),
                RowSpec::default()
                    .vertical_alignment(VerticalAlignment::CenterVertically)
                    .horizontal_arrangement(LinearArrangement::SpacedBy(8.0)),
                move || {
                    let track = track.clone();
                    Column(
                        Modifier::empty().weight(1.0),
                        ColumnSpec::default()
                            .vertical_arrangement(LinearArrangement::SpacedBy(3.0)),
                        move || {
                            Text(
                                if playing {
                                    "NOW PLAYING"
                                } else {
                                    "READY TO PLAY"
                                },
                                Modifier::empty(),
                                settings_text_style(11.0, colors.accent),
                            );
                            OneLine(
                                parsed_track_title(&track.title).to_owned(),
                                Modifier::empty().fill_max_width(),
                                17.0,
                                colors.label,
                            );
                            OneLine(
                                track_subtitle(&track),
                                Modifier::empty().fill_max_width(),
                                13.0,
                                colors.secondary_label,
                            );
                        },
                    );
                    GlassIconButton(
                        Modifier::empty().content_description(if playing {
                            "Pause library playback"
                        } else {
                            "Play library selection"
                        }),
                        GlassButtonSpec::prominent(),
                        48.0,
                        move || {
                            ui.notice.set(String::new());
                            if playing {
                                pause_playback(player)
                            } else {
                                play_or_resume(player)
                            }
                        },
                        if playing { PAUSE } else { PLAY_ICON },
                    );
                    GlassIconButton(
                        Modifier::empty().content_description("Next library track"),
                        GlassButtonSpec::plain(),
                        44.0,
                        move || {
                            ui.notice.set(String::new());
                            next_track(player);
                        },
                        NEXT,
                    );
                },
            );
        },
    );
}

#[composable]
fn LibraryRow(
    player: MutableState<WinampState>,
    ui: LibraryUi,
    snapshot: Rc<WinampState>,
    index: usize,
) {
    let track = &snapshot.playlist[index];
    let title = parsed_track_title(&track.title).to_owned();
    let subtitle = track_subtitle(track);
    let duration = playlist_duration_text(track.duration_seconds).unwrap_or_default();
    let current = snapshot.current_index == Some(index);
    let selected = ui.editing.get() && snapshot.selected_indices.contains(&index);
    let editing = ui.editing.get();
    let colors = liquid_colors();
    let background = if selected || current {
        colors.accent.with_alpha(if selected { 0.13 } else { 0.07 })
    } else {
        Color::TRANSPARENT
    };
    let target = TrackTarget {
        playlist: snapshot.playlist.clone(),
        index,
    };
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(76.0)
            .rounded_corners(18.0)
            .background(background)
            .padding_symmetric(8.0, 4.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            let title = title.clone();
            let subtitle = subtitle.clone();
            let row_label = if editing {
                format!(
                    "{} track {}: {title}",
                    if selected { "Deselect" } else { "Select" },
                    index + 1
                )
            } else {
                format!("Play track {}: {title}", index + 1)
            };
            Row(
                Modifier::empty()
                    .weight(1.0)
                    .fill_max_height()
                    .content_description(row_label)
                    .clickable(move |_| {
                        if editing {
                            toggle_selection(player, index);
                        } else {
                            ui.notice.set(String::new());
                            start_track(player, index);
                        }
                    }),
                RowSpec::default()
                    .vertical_alignment(VerticalAlignment::CenterVertically)
                    .horizontal_arrangement(LinearArrangement::SpacedBy(12.0)),
                move || {
                    Box(
                        Modifier::empty()
                            .size_points(40.0, 40.0)
                            .rounded_corners(12.0)
                            .background(if current || selected {
                                colors.accent.with_alpha(0.12)
                            } else {
                                colors.fill
                            }),
                        BoxSpec::new().content_alignment(Alignment::CENTER),
                        move || {
                            if editing && selected {
                                icons::Icon(icons::CHECK, None, 22.0, colors.accent);
                            } else if current {
                                icons::Icon(MUSIC, None, 22.0, colors.accent);
                            } else {
                                Text(
                                    format!("{}", index + 1),
                                    Modifier::empty(),
                                    settings_text_style(13.0, colors.secondary_label),
                                );
                            }
                        },
                    );
                    let title = title.clone();
                    let subtitle = subtitle.clone();
                    Column(
                        Modifier::empty().weight(1.0),
                        ColumnSpec::default()
                            .vertical_arrangement(LinearArrangement::SpacedBy(5.0)),
                        move || {
                            OneLine(
                                title.clone(),
                                Modifier::empty().fill_max_width(),
                                16.0,
                                if current { colors.accent } else { colors.label },
                            );
                            OneLine(
                                subtitle.clone(),
                                Modifier::empty().fill_max_width(),
                                13.0,
                                colors.secondary_label,
                            );
                        },
                    );
                },
            );
            if !editing {
                Text(
                    duration.clone(),
                    Modifier::empty().padding_symmetric(4.0, 0.0),
                    settings_text_style(12.0, colors.secondary_label),
                );
                let target = target.clone();
                GlassIconButton(
                    Modifier::empty().content_description(format!("Track {} options", index + 1)),
                    GlassButtonSpec::plain(),
                    44.0,
                    move || ui.sheet.set(Sheet::Track(target.clone())),
                    icons::MORE_HORIZ,
                );
            }
        },
    );
}

#[composable]
fn EmptyLibrary(ui: LibraryUi, empty: bool) {
    let colors = liquid_colors();
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    Column(
        Modifier::empty()
            .fill_max_width()
            .vertical_scroll(scroll, false)
            .padding_each(28.0, 20.0, 28.0, NAV_HEIGHT),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(14.0)),
        move || {
            icons::Icon(
                if empty { MUSIC } else { icons::SEARCH },
                None,
                40.0,
                colors.accent,
            );
            Text(
                if empty {
                    "Add your music"
                } else {
                    "No matching tracks"
                },
                Modifier::empty(),
                settings_text_style(24.0, colors.label),
            );
            Text(
                if empty {
                    "Choose files or a folder to start your playlist. Your music stays in its original location."
                } else {
                    "Try another title, artist, or folder name."
                },
                Modifier::empty().fill_max_width(),
                settings_text_style(15.0, colors.secondary_label),
            );
            if empty {
                GlassAction("Add music to Library", move || ui.sheet.set(Sheet::Add));
            } else {
                GlassAction("Show all tracks", move || {
                    set_text_field_text(&ui.query, "")
                });
            }
        },
    );
}

#[composable]
fn LibrarySheet(player: MutableState<WinampState>, ui: LibraryUi) {
    let sheet = rememberLiquidSheetState(true);
    LiquidModalBottomSheet(
        sheet,
        move || ui.sheet.set(Sheet::None),
        move || {
            let scroll = cranpose_ui::rememberScrollState!(0.0);
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .vertical_scroll(scroll, false)
                    .padding_each(20.0, 8.0, 20.0, 32.0),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(10.0)),
                move || {
                    SheetContents(player, ui);
                    GlassAction("Cancel", move || ui.sheet.set(Sheet::None));
                },
            );
        },
    );
}

#[composable]
fn SheetContents(player: MutableState<WinampState>, ui: LibraryUi) {
    let colors = liquid_colors();
    match ui.sheet.get() {
        Sheet::None => {}
        Sheet::Add => {
            Heading("Add music");
            Text(
                "Play directly from the files you choose.",
                Modifier::empty(),
                settings_text_style(15.0, colors.secondary_label),
            );
            GlassAction("Choose audio files", move || {
                ui.notice.set(String::new());
                ui.sheet.set(Sheet::None);
                add_audio_files(player);
            });
            GlassAction("Choose a music folder", move || {
                ui.notice.set(String::new());
                ui.sheet.set(Sheet::None);
                add_audio_folder(player);
            });
            GlassAction("Add an audio URL", move || {
                set_text_field_text(&ui.address, "");
                ui.sheet.set(Sheet::Address(false));
            });
            GlassAction("Open a network folder", move || {
                set_text_field_text(&ui.address, "");
                ui.sheet.set(Sheet::Address(true));
            });
        }
        Sheet::Manage => {
            Heading("Playlist options");
            GlassAction("Sort tracks", move || ui.sheet.set(Sheet::Sort));
            GlassAction("Import a playlist", move || ui.sheet.set(Sheet::Import));
            if !player.get().playlist.is_empty() {
                GlassAction("Export playlist", move || {
                    ui.notice.set(String::new());
                    ui.sheet.set(Sheet::None);
                    export_playlist(player);
                });
                GlassAction("Remove duplicates", move || {
                    let s = player.get_non_reactive();
                    let indices = duplicate_playlist_indices(&s.playlist);
                    if indices.is_empty() {
                        ui.notice.set("No duplicate tracks found".into());
                        ui.sheet.set(Sheet::None);
                    } else {
                        ui.sheet.set(Sheet::Remove(Removal {
                            playlist: s.playlist,
                            indices,
                        }));
                    }
                });
                GlassAction("Clear playlist", move || {
                    let s = player.get_non_reactive();
                    ui.sheet.set(Sheet::Remove(Removal {
                        indices: (0..s.playlist.len()).collect(),
                        playlist: s.playlist,
                    }));
                });
            }
        }
        Sheet::Sort => {
            Heading("Sort playlist");
            for (label, field) in [
                ("Track title", PlaylistSortField::Title),
                ("Artist", PlaylistSortField::Artist),
                ("File name", PlaylistSortField::FileName),
                ("Folder", PlaylistSortField::Path),
                ("Duration", PlaylistSortField::Duration),
            ] {
                GlassAction(label, move || {
                    player.update(|s| {
                        sort_playlist_tracks_by_field(s, field);
                    });
                    ui.notice.set(format!("Sorted by {}", label.to_lowercase()));
                    ui.sheet.set(Sheet::None);
                    ui.list.scroll_to_item(0, 0.0);
                });
            }
        }
        Sheet::Track(target) => {
            let Some(track) = target.playlist.get(target.index) else {
                return;
            };
            Text(
                parsed_track_title(&track.title).to_owned(),
                Modifier::empty().fill_max_width(),
                settings_text_style(22.0, colors.label),
            );
            Text(
                track_subtitle(track),
                Modifier::empty().fill_max_width(),
                settings_text_style(14.0, colors.secondary_label),
            );
            for (label, action) in [
                ("Play now", 0),
                ("Play next", 1),
                ("Move up", 2),
                ("Move down", 3),
                ("Remove from playlist", 4),
            ] {
                if (action == 2 && target.index == 0)
                    || (action == 3 && target.index + 1 == target.playlist.len())
                {
                    continue;
                }
                let target = target.clone();
                GlassAction(label, move || {
                    let s = player.get_non_reactive();
                    if !unchanged_prefix(&target.playlist, &s.playlist) {
                        ui.notice
                            .set("Playlist changed. Choose the track again.".into());
                        ui.sheet.set(Sheet::None);
                        return;
                    }
                    ui.sheet.set(Sheet::None);
                    match action {
                        0 => {
                            ui.notice.set(String::new());
                            start_track(player, target.index);
                        }
                        1 => {
                            player.update(|s| play_next(s, target.index));
                            ui.notice.set("Track queued next".into());
                        }
                        2 | 3 => player.update(|s| {
                            move_track(
                                s,
                                target.index,
                                if action == 2 {
                                    target.index - 1
                                } else {
                                    target.index + 1
                                },
                            );
                        }),
                        _ => ui.sheet.set(Sheet::Remove(Removal {
                            playlist: s.playlist,
                            indices: vec![target.index],
                        })),
                    }
                });
            }
        }
        Sheet::Remove(removal) => {
            Heading("Remove tracks?");
            Text(format!("Remove {} track(s) from this playlist? The audio files will stay in their original location.", removal.indices.len()), Modifier::empty().fill_max_width(), settings_text_style(16.0, colors.secondary_label));
            let removal = removal.clone();
            GlassButton(
                Modifier::empty()
                    .fill_max_width()
                    .height(48.0)
                    .content_description("Confirm remove tracks"),
                GlassButtonSpec::destructive(),
                move || {
                    let current = player.get_non_reactive();
                    if unchanged_prefix(&removal.playlist, &current.playlist) {
                        remove_playlist_indices_action(player, removal.indices.clone());
                        ui.editing.set(false);
                        ui.notice.set("Tracks removed from the playlist".into());
                    } else {
                        ui.notice
                            .set("Playlist changed. Select tracks again.".into());
                    }
                    ui.sheet.set(Sheet::None);
                },
                move || GlassButtonLabel("Remove tracks".into(), GlassButtonSpec::destructive()),
            );
        }
        Sheet::Import => {
            Heading("Import playlist");
            Text("Choose an M3U or PLS file. It will replace the current playlist. You can export your current list first from Playlist options.", Modifier::empty().fill_max_width(), settings_text_style(16.0, colors.secondary_label));
            GlassAction("Choose playlist file", move || {
                ui.notice.set(String::new());
                ui.sheet.set(Sheet::None);
                ui.editing.set(false);
                import_playlist(player);
            });
        }
        Sheet::Address(folder) => {
            Heading(if folder {
                "Network folder"
            } else {
                "Audio URL"
            });
            Text(
                if folder {
                    "Enter the WebDAV address of your music folder. It will replace the current playlist."
                } else {
                    "Enter a link to an audio file or stream. A playlist link will replace the current playlist."
                },
                Modifier::empty().fill_max_width(),
                settings_text_style(15.0, colors.secondary_label),
            );
            Text(
                "Address",
                Modifier::empty(),
                settings_text_style(14.0, colors.secondary_label),
            );
            cranpose_ui::BasicTextFieldWithOptions(
                ui.address,
                Modifier::empty()
                    .fill_max_width()
                    .height(48.0)
                    .glass_effect(Glass::regular().shape(LiquidShape::RoundedRect(16.0)))
                    .padding_symmetric(12.0, 12.0)
                    .content_description("Music address"),
                cranpose_ui::BasicTextFieldOptions {
                    text_style: settings_text_style(16.0, colors.label),
                    line_limits: cranpose_ui::TextFieldLineLimits::SingleLine,
                    keyboard_options: cranpose_ui::KeyboardOptions {
                        keyboard_type: cranpose_ui::KeyboardType::Uri,
                        auto_correct: false,
                        ime_action: cranpose_ui::ImeAction::Done,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
            GlassAction("Open address", move || {
                let address = ui.address.text().trim().to_owned();
                if address.is_empty() {
                    return;
                }
                ui.notice.set(String::new());
                ui.sheet.set(Sheet::None);
                if folder {
                    open_network_folder(player, address);
                } else {
                    open_url(player, address);
                }
            });
        }
    }
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/library.rs"]
mod tests;
