//! The mobile controls hosted beside the classic desktop player.
use super::*;

#[composable]
pub(in crate::winamp) fn DesktopNavigation(
    player: MutableState<WinampState>,
    width: f32,
    top: f32,
) {
    let nav_width = (width - 8.0).clamp(1.0, 560.0);
    Box(
        Modifier::empty()
            .absolute_offset(0.0, top)
            .size_points(width, NAV_HEIGHT)
            .background(liquid_colors().background),
        BoxSpec::default(),
        move || {
            NavigationBar(
                player,
                Modifier::empty().absolute_offset((width - nav_width) * 0.5, NAV_TOP_INSET),
                nav_width,
            )
        },
    );
}

#[composable]
pub(in crate::winamp) fn DesktopWorkspace(stack: WinampWindowStackSpec) {
    let player = stack.state;
    let skin = stack.skin_state;
    let appearance = theme::remember_theme();
    let library = library::remember_library_ui();
    library::PlaylistEffects(player, library);
    let library_window = rememberWindowState(760.0, 780.0);
    let studio_window = rememberWindowState(1140.0, 820.0);
    let settings_window = rememberWindowState(600.0, 780.0);
    theme::Theme(appearance, skin, move || {
        let main_window = stack.peer_windows.main;
        WinampWindowStack(stack.clone());
        let destination = Destination::from_state(&player.get());
        let (title, window) = match destination {
            Destination::Studio => ("Cranamp Skin Studio", studio_window),
            Destination::Settings => ("Cranamp Settings", settings_window),
            _ => ("Cranamp Library", library_window),
        };
        let origin = main_window.position().unwrap_or(default_main_position());
        let beside = origin.x + main_window.size().width + 16.0;
        let size = window.size();
        let colors = liquid_colors();
        // Removing the window modifier releases the OS window and its closed key.
        // The host stays mounted so Studio documents and undo history survive.
        let modifier = if destination == Destination::Player {
            Modifier::empty().size_points(0.0, 0.0).clip_to_bounds()
        } else {
            Modifier::empty()
                .window(
                    WindowConfig::new(title, size.width, size.height)
                        .with_position(beside, origin.y)
                        .with_state(window)
                        .with_min_size(360.0, 420.0)
                        .with_focus(cranpose::WindowFocus::Always)
                        .on_close_requested(move || {
                            player.update(|state| Destination::Player.select(state));
                        }),
                )
                .fill_max_size()
                .background(colors.background)
        };
        Box(modifier, BoxSpec::default(), move || {
            WinampStudioSurface(player, skin);
            match destination {
                Destination::Library => library::LibraryScreen(player, library),
                Destination::Settings => settings::SettingsScreen(player, skin, appearance),
                Destination::Player | Destination::Studio => {}
            }
        });
    });
}
