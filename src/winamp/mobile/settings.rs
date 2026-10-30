use super::chrome::{self, Label};
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Main,
    Skins,
    Playback,
    Sync,
}

#[derive(Clone, PartialEq)]
struct SkinPreview {
    entry: LibrarySkin,
    bitmap: Option<ImageBitmap>,
}

fn skin_name(entry: &LibrarySkin) -> String {
    entry
        .label
        .replace(" (Bundled)", "")
        .trim_end_matches(".wsz")
        .trim_start_matches("Catamp ")
        .to_owned()
}

fn preview_png(path: Option<&str>) -> Option<&'static [u8]> {
    Some(match path.unwrap_or("bundled:silverplay") {
        "bundled:silverplay" => include_bytes!("../../../assets/skins/Catamp Silverplay.png"),
        "bundled:feral-night" => include_bytes!("../../../assets/skins/Catamp Feral Night.png"),
        "bundled:cat-scan" => include_bytes!("../../../assets/skins/Catamp Cat Scan.png"),
        "bundled:seance" => include_bytes!("../../../assets/skins/Catamp Seance.png"),
        "bundled:salvage" => include_bytes!("../../../assets/skins/Catamp Salvage.png"),
        "bundled:freefall" => include_bytes!("../../../assets/skins/Catamp Freefall.png"),
        "bundled:moonlit" => include_bytes!("../../../assets/skins/Catamp Catnip.png"),
        "bundled:moon-garden" => include_bytes!("../../../assets/skins/Catamp Moon Garden.png"),
        "bundled:midnight-snack" => {
            include_bytes!("../../../assets/skins/Catamp Midnight Snack.png")
        }
        "bundled:purr-chaos" => {
            include_bytes!("../../../assets/skins/Catamp Purr Chaos Font Fixed.png")
        }
        "bundled:aqua-mischief" => include_bytes!("../../../assets/skins/Catamp Aqua Mischief.png"),
        _ => return None,
    })
}

fn load_previews() -> Vec<SkinPreview> {
    list_library_skins()
        .into_iter()
        .map(|entry| {
            let bitmap = if let Some(bytes) =
                preview_png(entry.path.as_ref().and_then(|path| path.to_str()))
            {
                crate::content_io::decode_image(bytes)
                    .ok()
                    .and_then(|rgba| {
                        let height = (rgba.width() * 116 / 275).min(rgba.height());
                        let main =
                            image::imageops::crop_imm(&rgba, 0, 0, rgba.width(), height).to_image();
                        let main = image::imageops::thumbnail(&main, 550, 232);
                        ImageBitmap::from_rgba8(main.width(), main.height(), main.into_raw()).ok()
                    })
            } else {
                entry
                    .path
                    .as_ref()
                    .and_then(|path| load_skin_file(path).ok())
                    .map(|skin| skin.main)
            };
            SkinPreview { entry, bitmap }
        })
        .collect()
}

#[composable]
pub(super) fn SettingsScreen(player: MutableState<WinampState>, skin: WinampSkinState) {
    let page = cranpose_core::rememberMutableStateOf(|| Page::Main);
    let legal = cranpose_core::rememberMutableStateOf(about::Page::default);
    let error = cranpose_core::rememberMutableStateOf(String::new);
    let query = cranpose_core::remember(|| TextFieldState::new("")).with(|state| *state);
    let searching = cranpose_core::rememberMutableStateOf(|| false);
    let focus = cranpose_ui::local_focus_manager().current();
    let previews = cranpose_core::rememberMutableStateOf(|| {
        bundled_library_skins()
            .into_iter()
            .map(|entry| SkinPreview {
                entry,
                bitmap: None,
            })
            .collect::<Vec<_>>()
    });
    cranpose_core::LaunchedEffectAsync(player.get().skin_path, move |_| {
        Box::pin(async move {
            if let Ok(loaded) = cranpose_core::withBlocking(load_previews).await {
                previews.set(loaded);
            }
        })
    });
    cranpose_services::BackHandler(page.get() != Page::Main, move || page.set(Page::Main));
    cranpose_services::BackHandler(legal.get() != about::Page::Settings, move || {
        legal.set(legal.get().parent())
    });
    let scroll = cranpose_ui::rememberScrollState!(0.);
    cranpose_core::LaunchedEffect((page.get(), legal.get()), move |_| scroll.scroll_to(0.));
    let colors = liquid_colors();
    BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
        let width = scope.max_width().0.min(720.);
        let inset = chrome::bottom_inset(&player.get());
        let compact = scope.max_height().0 < 560.;
        Column(
            Modifier::empty()
                .size_points(width, scope.max_height().0)
                .absolute_offset((scope.max_width().0 - width) * 0.5, 0.)
                .vertical_scroll(scroll, false)
                .padding_each(16., if compact { 8. } else { 14. }, 16., inset + 20.),
            ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(if compact {
                12.
            } else {
                22.
            })),
            move || {
                if legal.get() != about::Page::Settings {
                    about::Content(legal, true);
                    return;
                }
                if page.get() != Page::Main {
                    let title = match page.get() {
                        Page::Skins => "Skins",
                        Page::Playback => "Playback",
                        Page::Sync => "Folder sync",
                        Page::Main => "Settings",
                    };
                    Row(
                        Modifier::empty().fill_max_width(),
                        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            GlassIconButton(
                                Modifier::empty().content_description("Back to Settings"),
                                GlassButtonSpec::plain(),
                                44.,
                                move || {
                                    focus.clear_focus();
                                    page.set(Page::Main);
                                },
                                chrome::BACK,
                            );
                            Label(title, Modifier::empty().weight(1.), 28., colors.label, true);
                            if page.get() == Page::Skins {
                                GlassIconButton(
                                    Modifier::empty().content_description("Open skin file"),
                                    GlassButtonSpec::plain(),
                                    44.,
                                    move || player.update(|s| s.pending_skin_pick = true),
                                    icons::PLUS,
                                );
                            }
                        },
                    );
                } else {
                    Label("Settings", Modifier::empty(), 28., colors.label, true);
                }
                match page.get() {
                    Page::Skins => {
                        if !compact {
                            Text(
                                "Choose a skin. Make the player yours.",
                                Modifier::empty(),
                                settings_text_style(14., colors.secondary_label),
                            );
                        }
                        Row(
                            Modifier::empty().fill_max_width(),
                            RowSpec::default()
                                .vertical_alignment(VerticalAlignment::CenterVertically),
                            move || {
                                LiquidSearchField(
                                    Modifier::empty().weight(1.).on_focus_changed(move |state| {
                                        searching.set(state.has_focus())
                                    }),
                                    query,
                                    LiquidSearchFieldSpec {
                                        placeholder: Some("Find a skin".into()),
                                        on_glass: false,
                                        ..Default::default()
                                    },
                                );
                                if !query.text().is_empty() {
                                    GlassIconButton(
                                        Modifier::empty().content_description("Clear skin search"),
                                        GlassButtonSpec::plain(),
                                        44.,
                                        move || set_text_field_text(&query, ""),
                                        icons::CLOSE,
                                    );
                                }
                                if searching.get() {
                                    GlassButton(
                                        Modifier::empty()
                                            .height(44.)
                                            .content_description("Finish skin search"),
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
                        SkinGrid(player, skin, previews.get(), query.text(), width - 32.);
                    }
                    Page::Playback => PlaybackSettings(player),
                    Page::Sync => MobileSyncSettings(player),
                    Page::Main => {
                        chrome::Section("Appearance", move || {
                            let current = player.get().skin_path;
                            let selected = previews.get().into_iter().find(|preview| {
                                current
                                    == preview
                                        .entry
                                        .path
                                        .as_ref()
                                        .map(|p| p.to_string_lossy().into_owned())
                            });
                            GlassButton(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(78.)
                                    .content_description("Choose skin"),
                                GlassButtonSpec::plain(),
                                move || page.set(Page::Skins),
                                move || {
                                    let selected = selected.clone();
                                    Row(
                                        Modifier::empty().fill_max_width(),
                                        RowSpec::default()
                                            .vertical_alignment(VerticalAlignment::CenterVertically)
                                            .horizontal_arrangement(LinearArrangement::SpacedBy(
                                                12.,
                                            )),
                                        move || {
                                            PreviewImage(
                                                selected.as_ref().and_then(|s| s.bitmap.clone()),
                                                96.,
                                                42.,
                                            );
                                            let title = selected
                                                .as_ref()
                                                .map(|s| skin_name(&s.entry))
                                                .unwrap_or_else(|| "Custom skin".into());
                                            Column(
                                                Modifier::empty().weight(1.),
                                                ColumnSpec::default().vertical_arrangement(
                                                    LinearArrangement::SpacedBy(4.),
                                                ),
                                                move || {
                                                    Label(
                                                        "Player skin",
                                                        Modifier::empty(),
                                                        11.,
                                                        colors.secondary_label,
                                                        false,
                                                    );
                                                    Label(
                                                        title.clone(),
                                                        Modifier::empty().fill_max_width(),
                                                        15.,
                                                        colors.label,
                                                        true,
                                                    );
                                                },
                                            );
                                            icons::Icon(
                                                chrome::CHEVRON,
                                                None,
                                                17.,
                                                colors.tertiary_label,
                                            );
                                        },
                                    );
                                },
                            );
                            chrome::Divider();
                            SettingsToggle("Show equalizer", player.get().eq_visible, move |v| {
                                player.update(|s| s.eq_visible = v)
                            });
                            chrome::Divider();
                            SettingsToggle(
                                "Show playlist",
                                player.get().playlist_visible,
                                move |v| player.update(|s| s.playlist_visible = v),
                            );
                        });
                        chrome::Section("Sound", move || {
                            SettingsToggle(
                                "Equalizer enabled",
                                player.get().eq_enabled,
                                move |value| {
                                    player.update(|s| {
                                        s.eq_enabled = value;
                                        if let Err(e) = audio::set_equalizer(value, s.eq_values) {
                                            s.status = e;
                                        }
                                    })
                                },
                            );
                            chrome::Divider();
                            BalanceControl(player);
                            chrome::Divider();
                            NavigationRow(
                                "Playback controls",
                                "Volume, shuffle and repeat",
                                move || page.set(Page::Playback),
                            );
                        });
                        chrome::Section("Library and sync", move || {
                            NavigationRow(
                                "Manage playlist",
                                "Search and organize your tracks",
                                move || player.update(|s| Destination::Library.select(s)),
                            );
                            chrome::Divider();
                            NavigationRow(
                                "Folder sync",
                                "Share listening progress between devices",
                                move || page.set(Page::Sync),
                            );
                        });
                        chrome::Section("Help and privacy", move || {
                            NavigationRow(
                                "About Cranamp",
                                "Version, contact and licenses",
                                move || legal.set(about::Page::About),
                            );
                            chrome::Divider();
                            NavigationRow(
                                "Privacy policy",
                                "How Cranamp handles your data",
                                move || about::open_link(about::PRIVACY_URL, error),
                            );
                            chrome::Divider();
                            NavigationRow(
                                "Help with Cranamp",
                                "Support and common questions",
                                move || about::open_link(about::SUPPORT_URL, error),
                            );
                            about::LinkError(error, true);
                        });
                        Text(
                            "Your audio files stay in their original location.",
                            Modifier::empty().fill_max_width(),
                            settings_text_style(12., colors.secondary_label),
                        );
                    }
                }
            },
        );
    });
}

#[composable]
fn NavigationRow(title: &'static str, subtitle: &'static str, action: impl Fn() + 'static) {
    let colors = liquid_colors();
    GlassButton(
        Modifier::empty()
            .fill_max_width()
            .height(64.)
            .content_description(title),
        GlassButtonSpec::plain(),
        action,
        move || {
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Column(
                        Modifier::empty().weight(1.),
                        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(4.)),
                        move || {
                            Label(
                                title,
                                Modifier::empty().fill_max_width(),
                                15.,
                                colors.label,
                                false,
                            );
                            Label(
                                subtitle,
                                Modifier::empty().fill_max_width(),
                                11.,
                                colors.secondary_label,
                                false,
                            );
                        },
                    );
                    icons::Icon(chrome::CHEVRON, None, 17., colors.tertiary_label);
                },
            );
        },
    );
}

#[composable]
fn BalanceControl(player: MutableState<WinampState>) {
    let colors = liquid_colors();
    let balance = player.get().balance;
    Column(
        Modifier::empty()
            .fill_max_width()
            .padding_each(0., 14., 0., 8.),
        ColumnSpec::default(),
        move || {
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Label(
                        "Balance",
                        Modifier::empty().weight(1.),
                        15.,
                        colors.label,
                        false,
                    );
                    Label(
                        if (balance - 0.5).abs() < 0.01 {
                            "Centered".into()
                        } else {
                            format!(
                                "{} {}%",
                                if balance < 0.5 { "Left" } else { "Right" },
                                ((balance - 0.5).abs() * 200.).round()
                            )
                        },
                        Modifier::empty(),
                        11.,
                        colors.secondary_label,
                        false,
                    );
                },
            );
            Row(
                Modifier::empty().fill_max_width().height(48.),
                RowSpec::default()
                    .vertical_alignment(VerticalAlignment::CenterVertically)
                    .horizontal_arrangement(LinearArrangement::SpacedBy(8.)),
                move || {
                    Label("L", Modifier::empty(), 11., colors.secondary_label, false);
                    LiquidSlider(
                        Modifier::empty()
                            .weight(1.)
                            .content_description("Left and right balance"),
                        balance,
                        move |value| {
                            player.update(|s| match audio::set_balance(value) {
                                Ok(()) => s.balance = value,
                                Err(e) => s.status = e,
                            })
                        },
                    );
                    Label("R", Modifier::empty(), 11., colors.secondary_label, false);
                    GlassButton(
                        Modifier::empty()
                            .height(44.)
                            .content_description("Center balance"),
                        GlassButtonSpec::plain(),
                        move || {
                            player.update(|s| match audio::set_balance(0.5) {
                                Ok(()) => s.balance = 0.5,
                                Err(e) => s.status = e,
                            })
                        },
                        || GlassButtonLabel("Reset".into(), GlassButtonSpec::plain()),
                    );
                },
            );
        },
    );
}

#[composable]
fn PlaybackSettings(player: MutableState<WinampState>) {
    let colors = liquid_colors();
    chrome::Section("Playback", move || {
        SettingsToggle("Shuffle", player.get().shuffle, move |value| {
            player.update(|s| {
                s.shuffle = value;
                refresh_shuffle_order(s);
            })
        });
        chrome::Divider();
        SettingsToggle("Repeat", player.get().repeat, move |value| {
            player.update(|s| s.repeat = value)
        });
        chrome::Divider();
        Label(
            format!("Volume  {}%", (player.get().volume * 100.).round()),
            Modifier::empty().padding_symmetric(0., 14.),
            15.,
            colors.label,
            false,
        );
        LiquidSlider(
            Modifier::empty()
                .fill_max_width()
                .height(48.)
                .content_description("Player volume"),
            player.get().volume,
            move |value| {
                player.update(|s| match audio::set_volume(value) {
                    Ok(()) => s.volume = value,
                    Err(e) => s.status = e,
                })
            },
        );
    });
}

#[composable]
fn PreviewImage(bitmap: Option<ImageBitmap>, width: f32, height: f32) {
    Box(
        Modifier::empty()
            .size_points(width, height)
            .rounded_corners(6.)
            .clip_to_bounds()
            .background(liquid_colors().fill),
        BoxSpec::new().content_alignment(Alignment::CENTER),
        move || {
            if let Some(bitmap) = bitmap.clone() {
                cranpose_ui::Image(
                    bitmap,
                    None,
                    Modifier::empty().fill_max_size(),
                    Alignment::CENTER,
                    cranpose_ui::ContentScale::Fit,
                    1.,
                    None,
                );
            } else {
                icons::Icon(icons::EYE, None, 24., liquid_colors().secondary_label);
            }
        },
    );
}

#[composable]
fn SkinGrid(
    player: MutableState<WinampState>,
    skin: WinampSkinState,
    entries: Vec<SkinPreview>,
    query: String,
    width: f32,
) {
    let colors = liquid_colors();
    let query = query.trim().to_lowercase();
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|p| p.entry.label.to_lowercase().contains(&query))
        .collect();
    let columns = if width >= 600. { 3 } else { 2 };
    let cell = (width - 12. * (columns - 1) as f32) / columns as f32;
    if entries.is_empty() {
        Text(
            "No matching skins",
            Modifier::empty(),
            settings_text_style(16., colors.secondary_label),
        );
    }
    for pair in entries.chunks(columns) {
        let pair = pair.to_vec();
        Row(
            Modifier::empty().fill_max_width(),
            RowSpec::default().horizontal_arrangement(LinearArrangement::SpacedBy(12.)),
            move || {
                for preview in &pair {
                    let selected = player.get().skin_path
                        == preview
                            .entry
                            .path
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned());
                    let entry = preview.entry.clone();
                    let bitmap = preview.bitmap.clone();
                    let name = skin_name(&entry);
                    let label = format!("{}{}", name, if selected { " · Selected" } else { "" });
                    Column(
                        Modifier::empty()
                            .width(cell)
                            .rounded_corners(12.)
                            .background(if selected {
                                colors.accent
                            } else {
                                colors.surface
                            })
                            .padding(2.)
                            .content_description(label)
                            .clickable(move |_| apply_library_skin(player, skin, &entry)),
                        ColumnSpec::default(),
                        move || {
                            let name = name.clone();
                            let bitmap = bitmap.clone();
                            Column(
                                Modifier::empty()
                                    .fill_max_width()
                                    .rounded_corners(10.)
                                    .background(colors.surface)
                                    .padding(6.),
                                ColumnSpec::default()
                                    .vertical_arrangement(LinearArrangement::SpacedBy(7.)),
                                move || {
                                    PreviewImage(
                                        bitmap.clone(),
                                        cell - 16.,
                                        (cell - 16.) * 116. / 275.,
                                    );
                                    let name = name.clone();
                                    Row(
                                        Modifier::empty().fill_max_width(),
                                        RowSpec::default().vertical_alignment(
                                            VerticalAlignment::CenterVertically,
                                        ),
                                        move || {
                                            Label(
                                                name.clone(),
                                                Modifier::empty().weight(1.),
                                                13.,
                                                colors.label,
                                                true,
                                            );
                                            if selected {
                                                icons::Icon(icons::CHECK, None, 17., colors.accent);
                                            }
                                        },
                                    );
                                    Label(
                                        if selected { "Current" } else { "Tap to apply" },
                                        Modifier::empty(),
                                        11.,
                                        if selected {
                                            colors.accent
                                        } else {
                                            colors.secondary_label
                                        },
                                        false,
                                    );
                                },
                            );
                        },
                    );
                }
            },
        );
    }
}
