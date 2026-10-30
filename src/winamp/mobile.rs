//! Shared phone and tablet navigation. The skin remains a separate, unmodified surface.
use super::*;
use cranpose::liquid::prelude::*;
mod chrome;
mod library;
mod reflection;
mod settings;

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
    // Paint the whole viewport, including the status bar and home indicator.
    // Only content is inset; IME and safe-area obstruction overlap.
    let content_insets = insets.combined();
    cranpose_services::BackHandler(
        Destination::from_state(&player.get()) != Destination::Player,
        move || player.update(|state| Destination::Player.select(state)),
    );
    let colors = liquid_colors();
    let background = if Destination::from_state(&player.get()) == Destination::Player {
        skin.get()
            .map(|artwork| reflection::corner_color(&artwork))
            .unwrap_or(colors.background)
    } else {
        colors.background
    };
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(background)
            .padding_each(
                content_insets.left,
                content_insets.top,
                content_insets.right,
                content_insets.bottom,
            ),
        move |scope| {
            let width = scope.max_width().0;
            let height = scope.max_height().0;
            let nav_top = (height - NAV_HEIGHT).max(0.0);
            let destination = Destination::from_state(&player.get());
            Box(
                Modifier::empty()
                    .size_points(width, height)
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
            if matches!(destination, Destination::Library | Destination::Settings)
                && player.get().current_index.is_some()
            {
                chrome::MiniPlayer(
                    player,
                    Modifier::empty()
                        .absolute_offset(
                            (width - nav_width) * 0.5,
                            (nav_top - chrome::MINI_HEIGHT + 4.0).max(0.0),
                        )
                        .size_points(nav_width, 58.0),
                );
            }
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
                    tabs.push(LiquidTab::new(PLAY_ICON, "Player").with_icon_scale(0.72));
                    tabs.push(LiquidTab::new(icons::LIST, "Library").with_icon_scale(0.72));
                    tabs.push(LiquidTab::new(icons::EDIT, "Skin Studio").with_icon_scale(0.72));
                    tabs.push(LiquidTab::new(icons::SETTINGS, "Settings").with_icon_scale(0.72));
                },
            );
        },
    );
}

#[composable]
fn MobilePlayer(artwork: WinampSkin, player: MutableState<WinampState>, skin: WinampSkinState) {
    BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
        let snapshot = player.get();
        let layout =
            reflection::player_layout(scope.max_width().0, scope.max_height().0, &snapshot);
        WinampStackedStage(artwork.clone(), player, skin, layout, StackedDrag::Fixed);
        reflection::FloorReflection(
            reflection::corner_color(&artwork),
            snapshot,
            layout,
            scope.max_width().0,
            scope.max_height().0,
        );
    });
}

#[composable]
fn MobileSettings(player: MutableState<WinampState>, skin: WinampSkinState) {
    settings::SettingsScreen(player, skin);
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
