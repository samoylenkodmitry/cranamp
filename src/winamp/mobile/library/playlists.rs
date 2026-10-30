use super::*;
use crate::library::playlists::{self as model, Collection, Playlist};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq)]
enum Dialog {
    None,
    Overview,
    Name {
        id: Option<u64>,
        tracks: Rc<Vec<Track>>,
    },
    Options(u64),
    Delete(u64),
    Track(u64, usize),
    Choose(Rc<Vec<Track>>),
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Ui {
    data: MutableState<Rc<Collection>>,
    loaded: MutableState<bool>,
    load_attempt: MutableState<u64>,
    revision: MutableState<u64>,
    attempted: MutableState<u64>,
    error: MutableState<String>,
    message: MutableState<String>,
    dialog: MutableState<Dialog>,
    name: MutableState<String>,
    name_session: MutableState<u64>,
    selected: MutableState<BTreeSet<usize>>,
    candidates: MutableState<Rc<Vec<Track>>>,
}
impl Ui {
    pub(super) fn new() -> Self {
        Self {
            data: cranpose_core::mutableStateOf(Rc::new(Collection::default())),
            loaded: cranpose_core::mutableStateOf(false),
            load_attempt: cranpose_core::mutableStateOf(0),
            revision: cranpose_core::mutableStateOf(0),
            attempted: cranpose_core::mutableStateOf(0),
            error: cranpose_core::mutableStateOf(String::new()),
            message: cranpose_core::mutableStateOf(String::new()),
            dialog: cranpose_core::mutableStateOf(Dialog::None),
            name: cranpose_core::mutableStateOf(String::new()),
            name_session: cranpose_core::mutableStateOf(0),
            selected: cranpose_core::mutableStateOf(BTreeSet::new()),
            candidates: cranpose_core::mutableStateOf(Rc::new(Vec::new())),
        }
    }
    fn edit<T>(&self, action: impl FnOnce(&mut Collection) -> Result<T, String>) -> Option<T> {
        if !self.loaded.get_non_reactive() {
            self.message
                .set("Wait for your saved playlists to load.".into());
            return None;
        }
        let mut data = self.data.get_non_reactive();
        match action(Rc::make_mut(&mut data)) {
            Ok(result) => {
                self.data.set(data);
                self.revision.update(|value| *value += 1);
                self.message.set(String::new());
                Some(result)
            }
            Err(error) => {
                self.message.set(error);
                None
            }
        }
    }
    pub(super) fn name_playlist(&self, id: Option<u64>, tracks: Rc<Vec<Track>>) {
        let data = self.data.get_non_reactive();
        self.name.set(
            id.and_then(|id| data.get(id))
                .map_or_else(String::new, |list| list.name.clone()),
        );
        self.name_session.update(|value| *value += 1);
        self.message.set(String::new());
        self.dialog.set(Dialog::Name { id, tracks });
    }
    pub(super) fn choose(&self, tracks: Rc<Vec<Track>>) {
        self.message.set(String::new());
        self.dialog.set(Dialog::Choose(tracks));
    }
}

#[composable]
pub(super) fn Effects(player: MutableState<WinampState>, ui: Ui) {
    cranpose_core::LaunchedEffectAsync(ui.load_attempt.get(), move |_| {
        let store = cranpose_services::preferences();
        Box::pin(async move {
            let result = cranpose_core::withBlocking(move || model::load(store.as_ref()))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            match result {
                Ok(data) => {
                    ui.data.set(Rc::new(data));
                    ui.error.set(String::new());
                    ui.loaded.set(true);
                }
                Err(error) => ui.error.set(error),
            }
        })
    });
    let tracks = player.get().playlist;
    let observed = tracks.clone();
    cranpose_core::LaunchedEffect((ui.loaded.get(), tracks), move |_| {
        if !ui.loaded.get_non_reactive() {
            return;
        }
        let mut data = ui.data.get_non_reactive();
        if Rc::make_mut(&mut data).remember_tracks(&observed) {
            ui.data.set(data);
            ui.revision.update(|value| *value += 1);
        }
    });
    let pending = ui.loaded.get() && ui.revision.get() != ui.attempted.get();
    // One writer drains changes in order. New edits cannot race an older disk write.
    cranpose_core::LaunchedEffectAsync(pending, move |_| {
        let store = cranpose_services::preferences();
        Box::pin(async move {
            if !pending {
                return;
            }
            while ui.loaded.get_non_reactive()
                && ui.revision.get_non_reactive() != ui.attempted.get_non_reactive()
            {
                let revision = ui.revision.get_non_reactive();
                let data = ui.data.get_non_reactive().as_ref().clone();
                let store = store.clone();
                let result =
                    cranpose_core::withBlocking(move || model::save(store.as_ref(), &data))
                        .await
                        .map_err(|error| error.to_string())
                        .and_then(|result| result);
                ui.error.set(result.err().map_or_else(String::new, |error| {
                    format!("Could not save playlists: {error}")
                }));
                ui.attempted.set(revision);
            }
        })
    });
}

fn open(ui: LibraryUi, view: LibraryView) {
    ui.view.set(view);
    ui.list.scroll_to_item(0, 0.);
    set_text_field_text(&ui.query, "");
    ui.saved.message.set(String::new());
}
fn add_tracks(ui: LibraryUi, id: u64) {
    ui.saved.selected.set(BTreeSet::new());
    ui.saved
        .candidates
        .set(Rc::new(ui.saved.data.get_non_reactive().available.clone()));
    open(ui, LibraryView::AddToPlaylist(id));
}
fn play(player: MutableState<WinampState>, list: &Playlist, index: usize, shuffle: bool) {
    if list.tracks.is_empty() || index >= list.tracks.len() {
        return;
    }
    let index = if shuffle {
        (random_shuffle_seed() as usize) % list.tracks.len()
    } else {
        index
    };
    player.update(|state| {
        state.folder_scan_revision = state.folder_scan_revision.wrapping_add(1);
        state.folder_autoplay = None;
        state.pending_pick = None;
        state.shuffle = shuffle;
        replace_playlist_tracks(state, list.tracks.clone());
        state.current_index = Some(index);
        refresh_shuffle_order(state);
    });
    start_track(player, index);
}

#[composable]
pub(super) fn Screen(player: MutableState<WinampState>, ui: LibraryUi) {
    use cranpose_coroflow::StateFlowCollect;
    let _revision = crate::library::revision().collectAsState().get();
    let colors = liquid_colors();
    let focus = cranpose_ui::local_focus_manager().current();
    let view = ui.view.get();
    let data = ui.saved.data.get();
    let id = match view {
        LibraryView::Playlist(id) | LibraryView::AddToPlaylist(id) => Some(id),
        _ => None,
    };
    let picker = matches!(view, LibraryView::AddToPlaylist(_));
    let list = id.and_then(|id| data.get(id)).cloned();
    cranpose_services::BackHandler(id.is_some(), move || {
        open(
            ui,
            if picker {
                LibraryView::Playlist(id.unwrap())
            } else {
                LibraryView::Playlists
            },
        );
    });
    let query = ui.query.text();
    cranpose_core::LaunchedEffect(query.clone(), move |_| {
        ui.list.scroll_to_item(0, 0.);
    });
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(colors.background),
        move |scope| {
            let width = scope.max_width().0.min(760.);
            let compact = scope.max_height().0 < 560.;
            let data = data.clone();
            let list = list.clone();
            let query = query.clone();
            Column(
                Modifier::empty()
                    .size_points(width, scope.max_height().0)
                    .absolute_offset((scope.max_width().0 - width) * 0.5, 0.),
                ColumnSpec::default(),
                move || {
                    let list_header = list.clone();
                    let data_header = data.clone();
                    Column(
                        Modifier::empty().fill_max_width().padding_each(
                            20.,
                            if compact { 8. } else { 14. },
                            20.,
                            8.,
                        ),
                        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(8.)),
                        move || {
                            let list = list_header.clone();
                            let data = data_header.clone();
                            Row(
                                Modifier::empty().fill_max_width(),
                                RowSpec::default()
                                    .vertical_alignment(VerticalAlignment::CenterVertically)
                                    .horizontal_arrangement(LinearArrangement::SpacedBy(8.)),
                                move || {
                                    if id.is_some() {
                                        GlassIconButton(
                                            Modifier::empty().content_description(if picker {
                                                "Cancel adding tracks"
                                            } else {
                                                "Back to playlists"
                                            }),
                                            GlassButtonSpec::plain(),
                                            44.,
                                            move || {
                                                focus.clear_focus();
                                                open(
                                                    ui,
                                                    if picker {
                                                        LibraryView::Playlist(id.unwrap())
                                                    } else {
                                                        LibraryView::Playlists
                                                    },
                                                );
                                            },
                                            chrome::BACK,
                                        );
                                    }
                                    let list = list.clone();
                                    let data = data.clone();
                                    Column(
                                        Modifier::empty().weight(1.),
                                        ColumnSpec::default(),
                                        move || {
                                            chrome::Label(
                                                if picker {
                                                    "Add tracks".into()
                                                } else {
                                                    list.as_ref().map_or_else(
                                                        || "Library".into(),
                                                        |list| list.name.clone(),
                                                    )
                                                },
                                                Modifier::empty(),
                                                if compact { 24. } else { 28. },
                                                colors.label,
                                                true,
                                            );
                                            if !compact {
                                                chrome::Label(
                                                    if picker {
                                                        format!(
                                                            "{} selected",
                                                            ui.saved.selected.get().len()
                                                        )
                                                    } else {
                                                        list.as_ref().map_or_else(
                                                            || {
                                                                format!(
                                                                    "{} saved {}",
                                                                    data.lists.len(),
                                                                    if data.lists.len() == 1 {
                                                                        "playlist"
                                                                    } else {
                                                                        "playlists"
                                                                    }
                                                                )
                                                            },
                                                            |list| library_summary(&list.tracks),
                                                        )
                                                    },
                                                    Modifier::empty(),
                                                    14.,
                                                    colors.secondary_label,
                                                    false,
                                                );
                                            }
                                        },
                                    );
                                    if compact && width > 500. {
                                        LiquidSearchField(
                                            Modifier::empty().width(width * 0.32).on_focus_changed(
                                                move |state| {
                                                    ui.search_focused.set(state.has_focus())
                                                },
                                            ),
                                            ui.query,
                                            LiquidSearchFieldSpec {
                                                placeholder: Some(
                                                    if id.is_some() {
                                                        "Search tracks"
                                                    } else {
                                                        "Search playlists"
                                                    }
                                                    .into(),
                                                ),
                                                ..Default::default()
                                            },
                                        );
                                        if ui.search_focused.get() {
                                            TextButton(
                                                "Finish playlist search",
                                                "Done".into(),
                                                GlassButtonSpec::plain(),
                                                move || {
                                                    focus.clear_focus();
                                                },
                                            );
                                        }
                                    }
                                    if picker {
                                        let selected = ui.saved.selected.get();
                                        TextButton(
                                            "Add selected tracks",
                                            format!("Add ({})", selected.len()),
                                            if selected.is_empty() {
                                                GlassButtonSpec::plain()
                                                    .with_content_color(colors.tertiary_label)
                                            } else {
                                                GlassButtonSpec::prominent()
                                            },
                                            move || {
                                                let tracks = ui.saved.candidates.get_non_reactive();
                                                let chosen: Vec<_> = ui
                                                    .saved
                                                    .selected
                                                    .get_non_reactive()
                                                    .iter()
                                                    .filter_map(|&index| tracks.get(index).cloned())
                                                    .collect();
                                                if chosen.is_empty() {
                                                    return;
                                                }
                                                if ui
                                                    .saved
                                                    .edit(|data| data.add(id.unwrap(), &chosen))
                                                    .is_some()
                                                {
                                                    focus.clear_focus();
                                                    open(ui, LibraryView::Playlist(id.unwrap()));
                                                }
                                            },
                                        );
                                    } else if ui.saved.loaded.get() {
                                        GlassIconButton(
                                            Modifier::empty().content_description(
                                                if id.is_some() {
                                                    "Add tracks to playlist"
                                                } else {
                                                    "New playlist"
                                                },
                                            ),
                                            GlassButtonSpec::prominent(),
                                            48.,
                                            move || {
                                                if let Some(id) = id {
                                                    add_tracks(ui, id);
                                                } else {
                                                    ui.saved
                                                        .name_playlist(None, Rc::new(Vec::new()));
                                                }
                                            },
                                            icons::PLUS,
                                        );
                                        GlassIconButton(
                                            Modifier::empty().content_description(
                                                if id.is_some() {
                                                    "Saved playlist options"
                                                } else {
                                                    "Playlist library options"
                                                },
                                            ),
                                            GlassButtonSpec::glass(),
                                            48.,
                                            move || {
                                                ui.saved.dialog.set(
                                                    id.map_or(Dialog::Overview, Dialog::Options),
                                                )
                                            },
                                            icons::MORE_HORIZ,
                                        );
                                    }
                                },
                            );
                            if !(compact && width > 500.) {
                                Row(
                                    Modifier::empty().fill_max_width(),
                                    RowSpec::default()
                                        .vertical_alignment(VerticalAlignment::CenterVertically),
                                    move || {
                                        LiquidSearchField(
                                            Modifier::empty().weight(1.).on_focus_changed(
                                                move |state| {
                                                    ui.search_focused.set(state.has_focus())
                                                },
                                            ),
                                            ui.query,
                                            LiquidSearchFieldSpec {
                                                placeholder: Some(
                                                    if id.is_some() {
                                                        "Search tracks"
                                                    } else {
                                                        "Search playlists"
                                                    }
                                                    .into(),
                                                ),
                                                ..Default::default()
                                            },
                                        );
                                        if !ui.query.text().is_empty() {
                                            GlassIconButton(
                                                Modifier::empty()
                                                    .content_description("Clear playlist search"),
                                                GlassButtonSpec::plain(),
                                                44.,
                                                move || set_text_field_text(&ui.query, ""),
                                                icons::CLOSE,
                                            );
                                        }
                                        if ui.search_focused.get() {
                                            TextButton(
                                                "Finish playlist search",
                                                "Done".into(),
                                                GlassButtonSpec::plain(),
                                                move || {
                                                    focus.clear_focus();
                                                },
                                            );
                                        }
                                    },
                                );
                            }
                            if !picker {
                                LibraryTabs(ui);
                            }
                            if let Some(list) = list_header
                                .clone()
                                .filter(|list| !compact && !picker && !list.tracks.is_empty())
                            {
                                let shuffled = list.clone();
                                Row(
                                    Modifier::empty().fill_max_width(),
                                    RowSpec::default()
                                        .horizontal_arrangement(LinearArrangement::SpacedBy(12.)),
                                    move || {
                                        let list = list.clone();
                                        let shuffled = shuffled.clone();
                                        Pill("Play playlist", PLAY_ICON, move || {
                                            play(player, &list, 0, false)
                                        });
                                        Pill("Shuffle playlist", "M16 3h5v5h-2V6.4l-5.6 5.6 5.6 5.6V16h2v5h-5v-2h1.6l-5.6-5.6-6 6H2v-2h3.2l5.4-5.4L5.2 6.6H2v-2h4l6 6L17.6 5H16z", move || {
                                            play(player, &shuffled, 0, true)
                                        });
                                    },
                                );
                            }
                            Status(ui.saved);
                        },
                    );
                    let data = data.clone();
                    let list = list.clone();
                    let query = query.clone();
                    Box(
                        Modifier::empty()
                            .fill_max_width()
                            .weight(1.)
                            .clip_to_bounds(),
                        BoxSpec::default(),
                        move || {
                            if !ui.saved.loaded.get() && ui.saved.error.get().is_empty() {
                                Empty(
                                    "Loading playlists",
                                    "Your saved lists will appear here.",
                                    None,
                                    || {},
                                );
                            } else if !ui.saved.loaded.get() {
                                // Status above offers a retry without replacing saved data.
                            } else if picker {
                                TrackList(player, ui, list.clone(), true);
                            } else if let Some(list) = list.clone() {
                                TrackList(player, ui, Some(list), false);
                            } else {
                                Overview(player, ui, data.clone(), query.clone());
                            }
                        },
                    );
                },
            );
        },
    );
    Sheet(player, ui);
}

#[composable]
fn TextButton(
    label: &'static str,
    caption: String,
    spec: GlassButtonSpec,
    action: impl Fn() + 'static,
) {
    GlassButton(
        Modifier::empty().height(44.).content_description(label),
        spec.clone(),
        action,
        move || GlassButtonLabel(caption.clone().into(), spec.clone()),
    );
}

#[composable]
fn Status(ui: Ui) {
    let colors = liquid_colors();
    let error = ui.error.get();
    if !error.is_empty() {
        Text(
            error,
            Modifier::empty().fill_max_width(),
            settings_text_style(14., colors.destructive),
        );
        SmallAction(
            if ui.loaded.get() {
                "Retry saving playlists"
            } else {
                "Retry loading playlists"
            },
            GlassButtonSpec::plain(),
            move || {
                if ui.loaded.get_non_reactive() {
                    ui.revision.update(|value| *value += 1);
                } else {
                    ui.load_attempt.update(|value| *value += 1);
                }
            },
        );
    }
    let message = ui.message.get();
    if !message.is_empty() {
        Text(
            message,
            Modifier::empty().fill_max_width(),
            settings_text_style(14., colors.secondary_label),
        );
    }
}
#[composable]
fn Pill(label: &'static str, icon: &'static str, action: impl Fn() + 'static) {
    let colors = liquid_colors();
    GlassButton(
        Modifier::empty()
            .weight(1.)
            .height(46.)
            .content_description(label),
        GlassButtonSpec::glass(),
        action,
        move || {
            Row(
                Modifier::empty(),
                RowSpec::default()
                    .vertical_alignment(VerticalAlignment::CenterVertically)
                    .horizontal_arrangement(LinearArrangement::SpacedBy(8.)),
                move || {
                    icons::Icon(icon, None, 18., colors.accent);
                    chrome::Label(
                        if label == "Play playlist" {
                            "Play"
                        } else {
                            "Shuffle"
                        },
                        Modifier::empty(),
                        16.,
                        colors.accent,
                        true,
                    );
                },
            );
        },
    );
}
#[composable]
fn Empty(
    title: &'static str,
    body: &'static str,
    action: Option<&'static str>,
    on_click: impl Fn() + 'static,
) {
    let colors = liquid_colors();
    let on_click = Rc::new(on_click);
    Column(
        Modifier::empty().fill_max_width().padding(24.),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(14.)),
        move || {
            icons::Icon(icons::LIST, None, 42., colors.accent);
            chrome::Label(title, Modifier::empty(), 25., colors.label, true);
            Text(
                body,
                Modifier::empty().fill_max_width(),
                settings_text_style(16., colors.secondary_label),
            );
            if let Some(label) = action {
                let on_click = on_click.clone();
                SmallAction(label, GlassButtonSpec::glass(), move || on_click());
            }
        },
    );
}

#[composable]
fn Cover(list: Playlist) {
    use cranpose_coroflow::StateFlowCollect;
    let _revision = crate::library::revision().collectAsState().get();
    let colors = liquid_colors();
    let track = list
        .tracks
        .iter()
        .find(|track| crate::library::record(track).artwork.is_some())
        .or_else(|| list.tracks.first())
        .cloned();
    if let Some(track) = track {
        chrome::Artwork(track, 62.);
    } else {
        Box(
            Modifier::empty()
                .size_points(62., 62.)
                .rounded_corners(12.)
                .background(colors.fill),
            BoxSpec::new().content_alignment(Alignment::CENTER),
            move || icons::Icon(icons::LIST, None, 30., colors.accent),
        );
    }
}
#[composable]
fn Overview(player: MutableState<WinampState>, ui: LibraryUi, data: Rc<Collection>, query: String) {
    let terms: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    let lists: Rc<Vec<_>> = Rc::new(
        data.lists
            .iter()
            .rev()
            .filter(|list| {
                terms
                    .iter()
                    .all(|term| list.name.to_lowercase().contains(term))
            })
            .cloned()
            .collect(),
    );
    if lists.is_empty() {
        if data.lists.is_empty() {
            Empty("No playlists yet", "Create a playlist, or save the current queue. Your music stays in its original location.", Some("Create playlist"), move || ui.saved.name_playlist(None, Rc::new(Vec::new())));
        } else {
            Empty(
                "No matching playlists",
                "Try another name or clear the search.",
                Some("Clear playlist search"),
                move || set_text_field_text(&ui.query, ""),
            );
        }
        return;
    }
    let colors = liquid_colors();
    cranpose_ui::LazyColumn(
        Modifier::empty()
            .fill_max_size()
            .content_description("Saved playlists"),
        ui.list,
        cranpose_ui::LazyColumnSpec::default().content_padding(cranpose_ui::PaddingValues {
            start: 14.,
            end: 14.,
            bottom: chrome::bottom_inset(&player.get()) + 20.,
            ..Default::default()
        }),
        move |scope| {
            let lists = lists.clone();
            let keys = lists.clone();
            scope.items(
                LazyItems::new(lists.len()).key(move |i| keys[i].id),
                move |i| {
                    let Some(list) = ui.saved.data.get().get(lists[i].id).cloned() else {
                        return;
                    };
                    let id = list.id;
                    Row(
                        Modifier::empty().fill_max_width().height(88.),
                        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let list = list.clone();
                            GlassButton(
                                Modifier::empty()
                                    .weight(1.)
                                    .height(84.)
                                    .content_description(format!("Open playlist {}", list.name)),
                                GlassButtonSpec::plain(),
                                move || open(ui, LibraryView::Playlist(id)),
                                move || {
                                    let list = list.clone();
                                    Row(
                                        Modifier::empty().fill_max_size().padding(8.),
                                        RowSpec::default()
                                            .vertical_alignment(VerticalAlignment::CenterVertically)
                                            .horizontal_arrangement(LinearArrangement::SpacedBy(
                                                14.,
                                            )),
                                        move || {
                                            Cover(list.clone());
                                            let list = list.clone();
                                            Column(
                                                Modifier::empty().weight(1.),
                                                ColumnSpec::default().vertical_arrangement(
                                                    LinearArrangement::SpacedBy(4.),
                                                ),
                                                move || {
                                                    chrome::Label(
                                                        list.name.clone(),
                                                        Modifier::empty(),
                                                        17.,
                                                        colors.label,
                                                        true,
                                                    );
                                                    chrome::Label(
                                                        library_summary(&list.tracks),
                                                        Modifier::empty(),
                                                        13.,
                                                        colors.secondary_label,
                                                        false,
                                                    );
                                                },
                                            );
                                        },
                                    );
                                },
                            );
                            GlassIconButton(
                                Modifier::empty()
                                    .content_description(format!("Playlist {id} options")),
                                GlassButtonSpec::plain(),
                                44.,
                                move || ui.saved.dialog.set(Dialog::Options(id)),
                                icons::MORE_HORIZ,
                            );
                        },
                    );
                },
            );
        },
    );
}

#[composable]
fn TrackList(
    player: MutableState<WinampState>,
    ui: LibraryUi,
    list: Option<Playlist>,
    picker: bool,
) {
    let Some(list) = list else {
        return;
    };
    let id = list.id;
    let candidates = ui.saved.candidates.get();
    let tracks = cranpose_core::rememberKeyed(
        (ui.saved.data.get(), id, picker, candidates.clone()),
        move |_| {
            if picker {
                candidates
            } else {
                Rc::new(list.tracks.clone())
            }
        },
    );
    let cache = cranpose_core::remember(|| RefCell::new(FilterCache::default()));
    let rows = cache.with(|cell| cell.borrow_mut().project(&tracks, &ui.query.text()));
    if rows.is_empty() {
        if !ui.query.text().is_empty() {
            Empty(
                "No matching tracks",
                "Try another search.",
                Some("Clear playlist search"),
                move || set_text_field_text(&ui.query, ""),
            );
        } else if picker {
            Empty(
                "No tracks found",
                "Add music from the Tracks tab, or try another search.",
                None,
                || {},
            );
        } else {
            Empty(
                "Start with a song",
                "Choose tracks from your library to build this playlist.",
                Some("Add tracks to playlist"),
                move || add_tracks(ui, id),
            );
        }
        return;
    }
    let colors = liquid_colors();
    let existing: Rc<HashSet<String>> = Rc::new(
        ui.saved
            .data
            .get()
            .get(id)
            .into_iter()
            .flat_map(|list| list.tracks.iter())
            .map(|track| model::track_key(track).into())
            .collect(),
    );
    cranpose_ui::LazyColumn(
        Modifier::empty()
            .fill_max_size()
            .content_description(if picker {
                "Choose playlist tracks"
            } else {
                "Saved playlist tracks"
            }),
        ui.list,
        cranpose_ui::LazyColumnSpec::default().content_padding(cranpose_ui::PaddingValues {
            start: 14.,
            end: 14.,
            bottom: chrome::bottom_inset(&player.get()) + 20.,
            ..Default::default()
        }),
        move |scope| {
            let rows = rows.clone();
            let tracks = tracks.clone();
            let keys = rows.clone();
            let existing = existing.clone();
            scope.items(
                LazyItems::new(rows.len()).key(move |i| keys[i] as u64),
                move |row| {
                    let index = rows[row];
                    // Lazy rows keep their index key during moves; read current data in
                    // the row's composition so its title and artwork follow the edit.
                    use cranpose_coroflow::StateFlowCollect;
                    let _revision = crate::library::revision().collectAsState().get();
                    let current = ui.saved.data.get();
                    let track = if picker {
                        tracks.get(index)
                    } else {
                        current.get(id).and_then(|list| list.tracks.get(index))
                    };
                    let Some(track) = track.cloned() else {
                        return;
                    };
                    let present = picker && existing.contains(model::track_key(&track));
                    let selected = ui.saved.selected.get().contains(&index);
                    let title = crate::library::title(&track);
                    Row(
                        Modifier::empty().fill_max_width().height(72.),
                        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let track = track.clone();
                            GlassButton(
                                Modifier::empty()
                                    .weight(1.)
                                    .height(70.)
                                    .content_description(format!(
                                        "{} {}: {}",
                                        if picker {
                                            "Choose track"
                                        } else {
                                            "Play saved track"
                                        },
                                        index + 1,
                                        title
                                    )),
                                GlassButtonSpec::plain(),
                                move || {
                                    if picker {
                                        if !present {
                                            ui.saved.selected.update(|set| {
                                                if !set.remove(&index) {
                                                    set.insert(index);
                                                }
                                            });
                                        }
                                    } else if let Some(list) =
                                        ui.saved.data.get_non_reactive().get(id)
                                    {
                                        play(player, list, index, false);
                                    }
                                },
                                move || {
                                    let track = track.clone();
                                    Row(
                                        Modifier::empty().fill_max_size().padding(6.),
                                        RowSpec::default()
                                            .vertical_alignment(VerticalAlignment::CenterVertically)
                                            .horizontal_arrangement(LinearArrangement::SpacedBy(
                                                12.,
                                            )),
                                        move || {
                                            chrome::Artwork(track.clone(), 48.);
                                            let track = track.clone();
                                            Column(
                                                Modifier::empty().weight(1.),
                                                ColumnSpec::default().vertical_arrangement(
                                                    LinearArrangement::SpacedBy(3.),
                                                ),
                                                move || {
                                                    chrome::Label(
                                                        crate::library::title(&track),
                                                        Modifier::empty(),
                                                        16.,
                                                        colors.label,
                                                        false,
                                                    );
                                                    chrome::Label(
                                                        if present {
                                                            "Already added".into()
                                                        } else {
                                                            track_subtitle(&track)
                                                        },
                                                        Modifier::empty(),
                                                        13.,
                                                        colors.secondary_label,
                                                        false,
                                                    );
                                                },
                                            );
                                            if picker {
                                                icons::Icon(
                                                    if present || selected {
                                                        icons::CHECK
                                                    } else {
                                                        icons::PLUS
                                                    },
                                                    None,
                                                    22.,
                                                    if present {
                                                        colors.tertiary_label
                                                    } else {
                                                        colors.accent
                                                    },
                                                );
                                            }
                                        },
                                    );
                                },
                            );
                            if !picker {
                                GlassIconButton(
                                    Modifier::empty().content_description(format!(
                                        "Saved track {} options",
                                        index + 1
                                    )),
                                    GlassButtonSpec::plain(),
                                    44.,
                                    move || ui.saved.dialog.set(Dialog::Track(id, index)),
                                    icons::MORE_HORIZ,
                                );
                            }
                        },
                    );
                },
            );
        },
    );
}

#[composable]
pub(super) fn Sheet(player: MutableState<WinampState>, ui: LibraryUi) {
    if ui.saved.dialog.get() == Dialog::None {
        return;
    }
    let focus = cranpose_ui::local_focus_manager().current();
    cranpose_services::BackHandler(true, move || {
        focus.clear_focus();
        ui.saved.dialog.set(Dialog::None);
    });
    let state = rememberLiquidSheetState(true);
    LiquidModalBottomSheet(
        state,
        move || {
            focus.clear_focus();
            ui.saved.dialog.set(Dialog::None);
        },
        move || {
            let scroll = cranpose_ui::rememberScrollState!(0.);
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .vertical_scroll(scroll, false)
                    .padding_each(20., 8., 20., 32.),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(10.)),
                move || {
                    DialogContents(player, ui);
                    Status(ui.saved);
                    chrome::Divider();
                    SmallAction("Cancel", GlassButtonSpec::plain(), move || {
                        focus.clear_focus();
                        ui.saved.dialog.set(Dialog::None);
                    });
                },
            );
        },
    );
}
#[composable]
fn DialogContents(player: MutableState<WinampState>, ui: LibraryUi) {
    let colors = liquid_colors();
    let focus = cranpose_ui::local_focus_manager().current();
    match ui.saved.dialog.get() {
        Dialog::None => {}
        Dialog::Overview => {
            Heading("Your playlists");
            SheetAction("New playlist", move || {
                ui.saved.name_playlist(None, Rc::new(Vec::new()))
            });
            SheetAction("Save current queue", move || {
                ui.saved
                    .name_playlist(None, player.get_non_reactive().playlist)
            });
        }
        Dialog::Name { id, tracks } => {
            let name = cranpose_core::rememberKeyed(ui.saved.name_session.get(), move |_| {
                TextFieldState::new(ui.saved.name.get_non_reactive())
            });
            Heading(if id.is_some() {
                "Rename playlist"
            } else {
                "New playlist"
            });
            Text(
                "Name",
                Modifier::empty(),
                settings_text_style(14., colors.secondary_label),
            );
            cranpose_ui::BasicTextFieldWithOptions(
                name,
                Modifier::empty()
                    .fill_max_width()
                    .height(50.)
                    .glass_effect(Glass::regular().shape(LiquidShape::RoundedRect(14.)))
                    .padding_symmetric(12., 12.)
                    .content_description("Playlist name"),
                cranpose_ui::BasicTextFieldOptions {
                    text_style: settings_text_style(17., colors.label),
                    line_limits: cranpose_ui::TextFieldLineLimits::SingleLine,
                    ..Default::default()
                },
            );
            SmallAction("Save playlist", GlassButtonSpec::prominent(), move || {
                let name = name.text();
                if let Some(saved_id) = ui.saved.edit(|data| {
                    if let Some(id) = id {
                        data.rename(id, &name)?;
                        Ok(id)
                    } else {
                        data.create(&name, tracks.as_ref().clone())
                    }
                }) {
                    focus.clear_focus();
                    ui.saved.dialog.set(Dialog::None);
                    open(ui, LibraryView::Playlist(saved_id));
                }
            });
        }
        Dialog::Options(id) => {
            let Some(list) = ui.saved.data.get().get(id).cloned() else {
                return;
            };
            chrome::Label(
                list.name.clone(),
                Modifier::empty(),
                22.,
                colors.label,
                true,
            );
            SheetAction("Add tracks", move || {
                ui.saved.dialog.set(Dialog::None);
                add_tracks(ui, id);
            });
            if !list.tracks.is_empty() {
                let normal = list.clone();
                SheetAction("Play playlist", move || {
                    ui.saved.dialog.set(Dialog::None);
                    play(player, &normal, 0, false);
                });
                let shuffled = list.clone();
                SheetAction("Shuffle playlist", move || {
                    ui.saved.dialog.set(Dialog::None);
                    play(player, &shuffled, 0, true);
                });
            }
            SheetAction("Rename playlist", move || {
                ui.saved.name_playlist(Some(id), Rc::new(Vec::new()))
            });
            SheetAction("Duplicate playlist", move || {
                if let Some(id) = ui.saved.edit(|data| data.duplicate(id)) {
                    ui.saved.dialog.set(Dialog::None);
                    open(ui, LibraryView::Playlist(id));
                }
            });
            chrome::Divider();
            chrome::ActionRow("Delete playlist", chrome::TRASH, true, move || {
                ui.saved.dialog.set(Dialog::Delete(id))
            });
        }
        Dialog::Delete(id) => {
            Heading("Delete playlist?");
            Text("Only this saved list will be deleted. Your audio files and current playback will stay available.", Modifier::empty().fill_max_width(), settings_text_style(16., colors.secondary_label));
            SmallAction(
                "Confirm delete playlist",
                GlassButtonSpec::destructive(),
                move || {
                    if ui
                        .saved
                        .edit(|data| {
                            data.remove(id);
                            Ok(())
                        })
                        .is_some()
                    {
                        ui.saved.dialog.set(Dialog::None);
                        open(ui, LibraryView::Playlists);
                    }
                },
            );
        }
        Dialog::Track(id, index) => {
            let Some(list) = ui.saved.data.get().get(id).cloned() else {
                return;
            };
            let Some(track) = list.tracks.get(index) else {
                return;
            };
            chrome::Label(
                crate::library::title(track),
                Modifier::empty(),
                22.,
                colors.label,
                true,
            );
            for (label, to) in [
                ("Move up", index.checked_sub(1)),
                (
                    "Move down",
                    (index + 1 < list.tracks.len()).then_some(index + 1),
                ),
            ] {
                if let Some(to) = to {
                    SheetAction(label, move || {
                        if ui
                            .saved
                            .edit(|data| data.move_track(id, index, to))
                            .is_some()
                        {
                            ui.saved.dialog.set(Dialog::None);
                        }
                    });
                }
            }
            chrome::ActionRow(
                "Remove from saved playlist",
                chrome::TRASH,
                true,
                move || {
                    if ui
                        .saved
                        .edit(|data| {
                            let list = data.get_mut(id)?;
                            if index >= list.tracks.len() {
                                return Err("Choose the track again.".into());
                            }
                            list.tracks.remove(index);
                            Ok(())
                        })
                        .is_some()
                    {
                        ui.saved.dialog.set(Dialog::None);
                    }
                },
            );
        }
        Dialog::Choose(tracks) => {
            Heading("Add to playlist");
            let create = tracks.clone();
            SheetAction("New playlist", move || {
                ui.saved.name_playlist(None, create.clone())
            });
            for list in &ui.saved.data.get().lists {
                let id = list.id;
                let tracks = tracks.clone();
                SheetAction(list.name.clone(), move || {
                    if ui.saved.edit(|data| data.add(id, &tracks)).is_some() {
                        ui.saved.dialog.set(Dialog::None);
                        ui.notice.set("Added to saved playlist".into());
                    }
                });
            }
        }
    }
}
