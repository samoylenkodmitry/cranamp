//! Shared phone and tablet navigation. The skin remains a separate, unmodified surface.
use super::*;
use cranpose::liquid::prelude::*;
mod library;

const NAV_HEIGHT: f32 = 86.0;
const PLAY_ICON: &str = "M8 5v14l11-7z";

pub(super) fn touch_controls_enabled() -> bool {
    cfg!(any(target_os = "ios", target_os = "android"))
        || std::env::args().any(|arg| arg == "--touch-preview")
}

const TRANSPORT: [(SpriteRect, SpriteRect, f32, &str); 6] = [
    (
        PREV_BUTTON,
        PREV_BUTTON_ACTIVE,
        POS_CBUTTONS.0,
        "Previous track",
    ),
    (
        PLAY_BUTTON,
        PLAY_BUTTON_ACTIVE,
        POS_CBUTTONS.0 + 23.0,
        "Play",
    ),
    (
        PAUSE_BUTTON,
        PAUSE_BUTTON_ACTIVE,
        POS_CBUTTONS.0 + 46.0,
        "Pause",
    ),
    (
        STOP_BUTTON,
        STOP_BUTTON_ACTIVE,
        POS_CBUTTONS.0 + 69.0,
        "Stop",
    ),
    (
        NEXT_BUTTON,
        NEXT_BUTTON_ACTIVE,
        POS_CBUTTONS.0 + 92.0,
        "Next track",
    ),
    (
        EJECT_BUTTON,
        EJECT_BUTTON_ACTIVE,
        POS_EJECT.0,
        "Open audio files",
    ),
];

fn transport_action(player: MutableState<WinampState>, index: usize) {
    match index {
        0 => previous_track(player),
        1 => play_or_resume(player),
        2 => pause_playback(player),
        3 => stop_playback(player),
        4 => next_track(player),
        _ => open_audio_files(player),
    }
}

fn nearest_transport(x: f32, scale: f32) -> usize {
    TRANSPORT
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (x - (a.2 + a.0 .2 * 0.5) * scale)
                .abs()
                .total_cmp(&(x - (b.2 + b.0 .2 * 0.5) * scale).abs())
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

/// Expanded touch regions share one dispatcher. Nearest-center selection avoids
/// a later sibling stealing taps when the invisible 48-point targets overlap.
#[composable]
pub(super) fn TouchTransport(image: ImageBitmap, player: MutableState<WinampState>, scale: f32) {
    let pressed = cranpose_core::rememberMutableStateOf(|| None::<usize>);
    let height = (18.0 * scale).max(48.0);
    let y = POS_CBUTTONS.1 * scale - (height - 18.0 * scale) * 0.5;
    let left = ((POS_CBUTTONS.0 + PREV_BUTTON.2 * 0.5) * scale - 24.0).max(0.0);
    let right = (POS_EJECT.0 + EJECT_BUTTON.2 * 0.5) * scale + 24.0;
    for (index, (normal, active, x, _)) in TRANSPORT.into_iter().enumerate() {
        Sprite(
            image.clone(),
            if pressed.get() == Some(index) {
                active
            } else {
                normal
            },
            x,
            POS_CBUTTONS.1,
            scale,
        );
    }
    Box(
        Modifier::empty()
            .absolute_offset(left, y)
            .size_points(right - left, height)
            .pointer_input(
                scale.to_bits(),
                move |scope: PointerInputScope| async move {
                    scope
                        .await_pointer_event_scope(|events| async move {
                            loop {
                                let event = events.await_pointer_event().await;
                                match event.kind {
                                    PointerEventKind::Down => {
                                        pressed.set(Some(nearest_transport(
                                            event.position.x + left,
                                            scale,
                                        )));
                                        event.consume();
                                    }
                                    PointerEventKind::Up => {
                                        let previous = pressed.get_non_reactive();
                                        pressed.set(None);
                                        if event.position.x >= 0.0
                                            && event.position.x <= right - left
                                            && event.position.y >= 0.0
                                            && event.position.y <= height
                                        {
                                            let index =
                                                nearest_transport(event.position.x + left, scale);
                                            if previous == Some(index) {
                                                transport_action(player, index);
                                            }
                                        }
                                        event.consume();
                                    }
                                    PointerEventKind::Cancel => pressed.set(None),
                                    _ => {}
                                }
                            }
                        })
                        .await;
                },
            ),
        BoxSpec::default(),
        move || {
            for (index, (normal, _, x, label)) in TRANSPORT.into_iter().enumerate() {
                let width = (normal.2 * scale).max(48.0);
                Box(
                    Modifier::empty()
                        .size_points(width, height)
                        .absolute_offset((x + normal.2 * 0.5) * scale - left - width * 0.5, 0.0)
                        .stable_semantics(button_semantics(
                            label.into(),
                            Rc::new(move || transport_action(player, index)),
                        ))
                        .focusable(),
                    BoxSpec::default(),
                    || {},
                );
            }
        },
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Destination {
    Player,
    Library,
    Studio,
    Settings,
}

impl Destination {
    fn from_state(state: &WinampState) -> Self {
        if state.settings_open {
            Self::Settings
        } else if state.studio_open {
            Self::Studio
        } else if state.library_open {
            Self::Library
        } else {
            Self::Player
        }
    }

    fn select(self, state: &mut WinampState) {
        state.settings_open = self == Self::Settings;
        state.studio_open = self == Self::Studio;
        state.library_open = self == Self::Library;
    }

    fn index(self) -> usize {
        match self {
            Self::Player => 0,
            Self::Library => 1,
            Self::Studio => 2,
            Self::Settings => 3,
        }
    }
}

#[composable]
pub fn WinampMobileApp() {
    let tabs = remember_winamp_tab_state();
    let skin = remember_winamp_skin(tabs.player);
    WinampRuntimeEffects(tabs.player, tabs.peer_windows, skin);
    LiquidTheme(LiquidThemeSpec::default(), move || {
        MobileShell(tabs.player, skin)
    });
}

#[composable]
fn MobileShell(player: MutableState<WinampState>, skin: WinampSkinState) {
    let library = library::remember_library_ui();
    let focus = cranpose_ui::local_focus_manager().current();
    let insets = cranpose_ui::window_insets();
    // The mobile host already consumes the system safe area. Only add the
    // remaining keyboard obstruction here, so the tabs stay reachable.
    let keyboard_bottom = (insets.ime.bottom - insets.safe_area.bottom).max(0.0);
    cranpose_services::BackHandler(
        Destination::from_state(&player.get()) != Destination::Player,
        move || player.update(|state| Destination::Player.select(state)),
    );
    let colors = liquid_colors();
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(colors.background)
            .padding_each(0.0, 0.0, 0.0, keyboard_bottom),
        move |scope| {
            let width = scope.max_width().0;
            let height = scope.max_height().0;
            let nav_top = (height - NAV_HEIGHT).max(0.0);
            let destination = Destination::from_state(&player.get());
            let content_height = if destination == Destination::Player {
                nav_top.max(1.0)
            } else {
                height
            };
            Box(
                Modifier::empty()
                    .size_points(width, content_height)
                    .clip_to_bounds(),
                BoxSpec::default(),
                move || {
                    // This call remains mounted across tabs so its shared documents and
                    // undo histories survive a visit to Settings or the player.
                    WinampStudioSurfaceWithInset(player, skin, NAV_HEIGHT);
                    match destination {
                        Destination::Studio => {}
                        Destination::Settings => MobileSettings(player, skin),
                        Destination::Library => library::LibraryScreen(player, library),
                        Destination::Player => match skin.get() {
                            Ok(artwork) => MobilePlayer(artwork, player, skin),
                            Err(error) => WinampSkinError(error),
                        },
                    }
                },
            );
            let nav_width = (width - 24.0).clamp(1.0, 560.0);
            LiquidTabBar(
                Modifier::empty()
                    .absolute_offset((width - nav_width) * 0.5, nav_top + 10.0)
                    .width(nav_width),
                LiquidTabBarSpec::new((nav_width - 8.0) / 4.0),
                destination.index(),
                move |index| {
                    focus.clear_focus();
                    let destination = match index {
                        1 => Destination::Library,
                        2 => Destination::Studio,
                        3 => Destination::Settings,
                        _ => Destination::Player,
                    };
                    player.update(|state| destination.select(state));
                },
                |tabs| {
                    tabs.tab(PLAY_ICON, "Player");
                    tabs.tab(icons::LIST, "Library");
                    tabs.tab(icons::EDIT, "Skin Studio");
                    tabs.tab(icons::SETTINGS, "Settings");
                },
            );
        },
    );
}

#[composable]
fn MobilePlayer(artwork: WinampSkin, player: MutableState<WinampState>, skin: WinampSkinState) {
    BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
        let layout = resizable_stacked_layout(
            scope.max_width().0,
            scope.max_height().0,
            0.0,
            0.0,
            &player.get(),
            ui_scale(),
        );
        WinampStackedStage(artwork.clone(), player, skin, layout, StackedDrag::Fixed);
    });
}

#[composable]
fn MobileSettings(player: MutableState<WinampState>, skin: WinampSkinState) {
    BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
        let width = scope.max_width().0.min(720.0);
        Box(
            Modifier::empty()
                .size_points(width, scope.max_height().0)
                .absolute_offset((scope.max_width().0 - width) * 0.5, 0.0),
            BoxSpec::default(),
            move || MobileSettingsContent(player, skin),
        );
    });
}

#[composable]
fn MobileSettingsContent(player: MutableState<WinampState>, skin: WinampSkinState) {
    let choose_skin = cranpose_core::rememberMutableStateOf(|| false);
    let page = cranpose_core::rememberMutableStateOf(about::Page::default);
    let link_error = cranpose_core::rememberMutableStateOf(String::new);
    let library = cranpose_core::rememberMutableStateOf(bundled_library_skins);
    cranpose_core::LaunchedEffectAsync((choose_skin.get(), player.get().skin_path), move |_| {
        Box::pin(async move {
            if let Ok(entries) = cranpose_core::withBlocking(list_library_skins).await {
                library.set(entries);
            }
        })
    });
    cranpose_services::BackHandler(choose_skin.get(), move || choose_skin.set(false));
    cranpose_services::BackHandler(page.get() != about::Page::Settings, move || {
        page.set(page.get().parent());
    });
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    let reset_scroll = scroll;
    cranpose_core::LaunchedEffect((choose_skin.get(), page.get()), move |_| {
        reset_scroll.scroll_to(0.0);
    });
    let colors = liquid_colors();
    Column(
        Modifier::empty()
            .fill_max_size()
            .vertical_scroll(scroll, false)
            .padding_each(20.0, 20.0, 20.0, 20.0 + NAV_HEIGHT),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(20.0)),
        move || {
            if page.get() != about::Page::Settings {
                about::Content(page, true);
                return;
            }
            if choose_skin.get() {
                GlassAction("Back to Settings", move || choose_skin.set(false));
                Heading("Choose a skin");
                for entry in library.get() {
                    let active = player.get().skin_path
                        == entry
                            .path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned());
                    let title = entry
                        .label
                        .replace(" (Bundled)", "")
                        .trim_end_matches(".wsz")
                        .to_owned();
                    let label = if active {
                        format!("{title} · Selected")
                    } else {
                        title
                    };
                    GlassAction(label, move || apply_library_skin(player, skin, &entry));
                }
                GlassAction("Open skin file", move || {
                    player.update(|s| s.pending_skin_pick = true)
                });
                return;
            }
            Heading("Settings");
            Text(
                "Playback and appearance",
                Modifier::empty(),
                settings_text_style(16.0, colors.secondary_label),
            );
            SettingsCard("Appearance", move || {
                let current = player.get().skin_path;
                let title = library
                    .get()
                    .into_iter()
                    .find(|s| current == s.path.as_ref().map(|p| p.to_string_lossy().into_owned()))
                    .map(|s| {
                        s.label
                            .replace(" (Bundled)", "")
                            .trim_end_matches(".wsz")
                            .to_owned()
                    })
                    .unwrap_or_else(|| "Custom skin".into());
                Text(
                    title,
                    Modifier::empty(),
                    settings_text_style(18.0, colors.label),
                );
                GlassAction("Choose skin", move || choose_skin.set(true));
                SettingsToggle("Show equalizer", player.get().eq_visible, move |value| {
                    player.update(|s| s.eq_visible = value)
                });
                SettingsToggle(
                    "Show playlist",
                    player.get().playlist_visible,
                    move |value| player.update(|s| s.playlist_visible = value),
                );
            });
            SettingsCard("Playback", move || {
                let snapshot = player.get();
                SettingsToggle("Shuffle", snapshot.shuffle, move |value| {
                    player.update(|s| {
                        s.shuffle = value;
                        refresh_shuffle_order(s);
                    })
                });
                SettingsToggle("Repeat", snapshot.repeat, move |value| {
                    player.update(|s| s.repeat = value)
                });
                SettingsToggle("Equalizer enabled", snapshot.eq_enabled, move |value| {
                    player.update(|s| {
                        s.eq_enabled = value;
                        if let Err(error) = audio::set_equalizer(value, s.eq_values) {
                            s.status = error;
                        }
                    })
                });
                Text(
                    format!("Volume  {}%", (snapshot.volume * 100.0).round()),
                    Modifier::empty(),
                    settings_text_style(16.0, colors.label),
                );
                LiquidSlider(
                    Modifier::empty()
                        .fill_max_width()
                        .content_description("Player volume"),
                    snapshot.volume,
                    move |value| {
                        player.update(|s| {
                            if let Err(error) = audio::set_volume(value) {
                                s.status = error;
                            } else {
                                s.volume = value;
                            }
                        });
                    },
                );
                Text(
                    "Balance",
                    Modifier::empty(),
                    settings_text_style(16.0, colors.label),
                );
                LiquidSlider(
                    Modifier::empty()
                        .fill_max_width()
                        .content_description("Left and right balance"),
                    snapshot.balance,
                    move |value| {
                        player.update(|s| {
                            if let Err(error) = audio::set_balance(value) {
                                s.status = error;
                            } else {
                                s.balance = value;
                            }
                        });
                    },
                );
                GlassAction("Center balance", move || {
                    player.update(|s| {
                        if let Err(error) = audio::set_balance(0.5) {
                            s.status = error;
                        } else {
                            s.balance = 0.5;
                        }
                    })
                });
            });
            MobileSyncSettings(player);
            SettingsCard("Help and privacy", move || {
                GlassAction("Privacy policy", move || {
                    about::open_link(about::PRIVACY_URL, link_error)
                });
                GlassAction("Help with Cranamp", move || {
                    about::open_link(about::SUPPORT_URL, link_error)
                });
                about::LinkError(link_error, true);
                Text("Audio plays from its original location. Cranamp does not copy your music library.", Modifier::empty().fill_max_width(), settings_text_style(14.0, colors.secondary_label));
                GlassAction("About Cranamp", move || page.set(about::Page::About));
            });
        },
    );
}

#[composable]
fn MobileSyncSettings(player: MutableState<WinampState>) {
    use cranpose_coroflow::StateFlowCollect;
    let status = crate::sync::runtime::status_flow().collectAsState().get();
    let config = crate::sync::runtime::config_snapshot();
    let enabled = config.as_ref().is_some_and(|c| c.enabled);
    let folder = config.and_then(|c| c.folder);
    let colors = liquid_colors();
    SettingsCard("Folder sync", move || {
        Text("Shares track titles, play counts, listening positions and a device ID through the folder you choose. Anyone with folder access can read them. Audio files stay in place.", Modifier::empty().fill_max_width(), settings_text_style(14.0, colors.secondary_label));
        SettingsToggle("Sync listening progress", enabled, move |value| {
            let result = crate::sync::runtime::set_enabled(value);
            player.update(|s| s.status = result.summary());
        });
        Text(
            status.summary(),
            Modifier::empty().fill_max_width(),
            settings_text_style(14.0, colors.secondary_label),
        );
        if let Some(folder) = &folder {
            Text(
                shorten_sync_folder(folder),
                Modifier::empty().fill_max_width(),
                settings_text_style(14.0, colors.label),
            );
        }
        GlassAction(
            if folder.is_some() {
                "Change sync folder"
            } else {
                "Choose sync folder"
            },
            move || sync_pick_folder(player),
        );
    });
}

#[composable]
fn Heading(title: &'static str) {
    Text(
        title,
        Modifier::empty(),
        settings_text_style(32.0, liquid_colors().label),
    );
}

#[composable]
fn SettingsCard(title: &'static str, content: impl FnMut() + 'static) {
    let colors = liquid_colors();
    let content = Rc::new(RefCell::new(content));
    GlassSurface(
        Modifier::empty().fill_max_width(),
        Glass::regular().shape(LiquidShape::RoundedRect(24.0)),
        move || {
            let content = content.clone();
            Column(
                Modifier::empty().fill_max_width().padding(18.0),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(14.0)),
                move || {
                    Text(
                        title,
                        Modifier::empty(),
                        settings_text_style(20.0, colors.label),
                    );
                    (content.borrow_mut())();
                },
            );
        },
    );
}

#[composable]
pub(super) fn GlassAction(label: impl Into<String>, action: impl Fn() + 'static) {
    let label = label.into();
    GlassButton(
        Modifier::empty()
            .fill_max_width()
            .height(48.0)
            .content_description(label.clone()),
        GlassButtonSpec::glass(),
        action,
        move || {
            GlassButtonLabel(label.clone().into(), GlassButtonSpec::glass());
        },
    );
}

#[composable]
fn SettingsToggle(label: &'static str, value: bool, action: impl Fn(bool) + 'static) {
    let colors = liquid_colors();
    let action = Rc::new(action);
    Row(
        Modifier::empty().fill_max_width().height(52.0),
        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            Text(
                label,
                Modifier::empty().weight(1.0),
                settings_text_style(16.0, colors.label),
            );
            let action = action.clone();
            LiquidToggle(
                Modifier::empty().content_description(label),
                value,
                move |value| action(value),
            );
        },
    );
}

#[cfg(test)]
#[path = "../../test/unit/winamp/mobile.rs"]
mod tests;
