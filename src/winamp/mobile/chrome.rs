use super::*;

#[cfg(test)]
#[path = "../../../test/unit/winamp/mobile_chrome.rs"]
mod tests;

pub(super) const MUSIC: &str = "M9 3v12.2A3.5 3.5 0 1 0 11 18V7h8v6.2A3.5 3.5 0 1 0 21 16V3H9z";
pub(super) const PAUSE: &str = "M6 5h4v14H6zM14 5h4v14h-4z";
pub(super) const NEXT: &str = "M5 5v14l10-7zM17 5h3v14h-3z";
pub(super) const CHEVRON: &str = "M9 5l7 7-7 7-1.4-1.4L13.2 12 7.6 6.4z";
pub(super) const BACK: &str = "M15 5l-7 7 7 7 1.4-1.4L10.8 12l5.6-5.6z";
pub(super) const TRASH: &str =
    "M8 3h8v2h5v2H3V5h5V3zm-3 6h14l-1 12H6L5 9zm4 2v8h2v-8H9zm4 0v8h2v-8h-2z";
pub(super) const MINI_HEIGHT: f32 = 66.0;

pub(super) fn bottom_inset(state: &WinampState) -> f32 {
    NAV_HEIGHT
        + if state.current_index.is_some() {
            MINI_HEIGHT
        } else {
            0.0
        }
}

#[composable]
pub(super) fn Label(
    text: impl Into<String>,
    modifier: Modifier,
    size: f32,
    color: Color,
    bold: bool,
) {
    let mut style = settings_text_style(size, color);
    if bold {
        style.span_style.font_weight = Some(cranpose_ui::text::FontWeight(600));
    }
    BasicText(
        text.into(),
        modifier,
        style,
        TextOverflow::Ellipsis,
        false,
        1,
        1,
    );
}

#[composable]
pub(super) fn Divider() {
    Box(
        Modifier::empty()
            .fill_max_width()
            .height(0.5)
            .background(liquid_colors().separator),
        BoxSpec::default(),
        || {},
    );
}

#[composable]
pub(super) fn Section(title: &'static str, content: impl FnMut() + 'static) {
    let colors = liquid_colors();
    let content = Rc::new(RefCell::new(content));
    Column(
        Modifier::empty().fill_max_width(),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::SpacedBy(8.0)),
        move || {
            Label(
                title.to_uppercase(),
                Modifier::empty().padding_symmetric(3.0, 0.0),
                12.0,
                colors.secondary_label,
                false,
            );
            let content = content.clone();
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .rounded_corners(18.0)
                    .background(colors.surface)
                    .padding_symmetric(14.0, 0.0),
                ColumnSpec::default(),
                move || (content.borrow_mut())(),
            );
        },
    );
}

#[composable]
pub(super) fn ActionRow(
    label: impl Into<String>,
    icon: &'static str,
    destructive: bool,
    action: impl Fn() + 'static,
) {
    let label = label.into();
    let colors = liquid_colors();
    let color = if destructive {
        colors.destructive
    } else if label == "Play now" {
        colors.accent
    } else {
        colors.label
    };
    GlassButton(
        Modifier::empty()
            .fill_max_width()
            .height(52.0)
            .content_description(label.clone()),
        GlassButtonSpec::plain().with_content_color(color),
        action,
        move || {
            let label = label.clone();
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default()
                    .vertical_alignment(VerticalAlignment::CenterVertically)
                    .horizontal_arrangement(LinearArrangement::SpacedBy(16.0)),
                move || {
                    icons::Icon(icon, None, 22.0, color);
                    Label(
                        label.clone(),
                        Modifier::empty().weight(1.0),
                        16.0,
                        color,
                        false,
                    );
                },
            );
        },
    );
}

// Album art is read only for visible local tracks. The bounded memory cache also
// remembers missing covers, so playback ticks and scrolling do not repeat I/O.
type CoverCache = std::collections::VecDeque<(std::path::PathBuf, Option<ImageBitmap>)>;
fn cover_cache() -> &'static std::sync::Mutex<CoverCache> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<CoverCache>> = std::sync::OnceLock::new();
    CACHE.get_or_init(Default::default)
}

fn cover_folder(track: &Track) -> Option<std::path::PathBuf> {
    let path = track.path.as_deref()?;
    let path = cranpose_services::path_from_uri(path)?;
    path.parent().map(std::path::Path::to_path_buf)
}

fn read_cover(folder: std::path::PathBuf) -> Option<ImageBitmap> {
    use std::io::Read;
    if let Some((_, cover)) = cover_cache()
        .lock()
        .ok()?
        .iter()
        .find(|(key, _)| key == &folder)
    {
        return cover.clone();
    }
    let cover = [
        "cover.jpg",
        "folder.jpg",
        "cover.png",
        "folder.png",
        "Cover.jpg",
        "Folder.jpg",
    ]
    .into_iter()
    .find_map(|name| {
        let file = std::fs::File::open(folder.join(name)).ok()?;
        if file.metadata().ok()?.len() > 4 * 1024 * 1024 {
            return None;
        }
        let mut bytes = Vec::new();
        file.take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() > 4 * 1024 * 1024 {
            return None;
        }
        let rgba = crate::content_io::decode_image(&bytes).ok()?;
        let thumb = image::imageops::thumbnail(&rgba, 160, 160);
        ImageBitmap::from_rgba8(thumb.width(), thumb.height(), thumb.into_raw()).ok()
    });
    if let Ok(mut cache) = cover_cache().lock() {
        if cache.len() >= 128 {
            cache.pop_front();
        }
        cache.push_back((folder, cover.clone()));
    }
    cover
}

#[composable]
pub(super) fn Artwork(track: Track, size: f32) {
    let folder = cover_folder(&track);
    let picture = cranpose_core::rememberKeyed(folder.clone(), |_| {
        cranpose_core::mutableStateOf(None::<ImageBitmap>)
    });
    cranpose_core::LaunchedEffectAsync(folder.clone(), move |_| {
        Box::pin(async move {
            if let Some(folder) = folder {
                if let Ok(cover) = cranpose_core::withBlocking(move || read_cover(folder)).await {
                    picture.set(cover);
                }
            }
        })
    });
    let artist = parsed_track_artist(&track.title).unwrap_or(&track.title);
    let hash = artist.bytes().fold(0u32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte as u32)
    });
    let palette = [
        Color(0.15, 0.31, 0.48, 1.),
        Color(0.32, 0.23, 0.48, 1.),
        Color(0.13, 0.37, 0.37, 1.),
        Color(0.45, 0.26, 0.23, 1.),
        Color(0.26, 0.33, 0.48, 1.),
    ];
    let base = palette[hash as usize % palette.len()];
    Box(
        Modifier::empty()
            .size_points(size, size)
            .rounded_corners(8.0)
            .clip_to_bounds()
            .background(base),
        BoxSpec::new().content_alignment(Alignment::CENTER),
        move || {
            if let Some(bitmap) = picture.get() {
                cranpose_ui::Image(
                    bitmap,
                    None,
                    Modifier::empty().fill_max_size(),
                    Alignment::CENTER,
                    cranpose_ui::ContentScale::Crop,
                    1.0,
                    None,
                );
            } else {
                Box(
                    Modifier::empty()
                        .size_points(size * 0.68, size * 0.68)
                        .rounded_corners(size)
                        .background(Color::WHITE.with_alpha(0.07)),
                    BoxSpec::new().content_alignment(Alignment::CENTER),
                    move || {
                        icons::Icon(MUSIC, None, size * 0.42, Color::WHITE.with_alpha(0.82));
                    },
                );
            }
        },
    );
}

#[composable]
pub(super) fn MiniPlayer(player: MutableState<WinampState>, modifier: Modifier) {
    let snapshot = player.get();
    let Some(track) = snapshot
        .current_index
        .and_then(|index| snapshot.playlist.get(index))
        .cloned()
    else {
        return;
    };
    let colors = liquid_colors();
    let playing = snapshot.playback == PlaybackState::Playing;
    GlassSurface(
        modifier,
        Glass::regular().shape(LiquidShape::RoundedRect(18.0)),
        move || {
            let track = track.clone();
            Column(
                Modifier::empty()
                    .fill_max_size()
                    .padding_each(8., 6., 6., 5.),
                ColumnSpec::default(),
                move || {
                    let track = track.clone();
                    Row(
                        Modifier::empty().fill_max_width().height(44.),
                        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let track = track.clone();
                            Row(
                                Modifier::empty()
                                    .weight(1.)
                                    .fill_max_height()
                                    .content_description("Show current track in Player")
                                    .clickable(move |_| {
                                        player.update(|s| Destination::Player.select(s))
                                    }),
                                RowSpec::default()
                                    .vertical_alignment(VerticalAlignment::CenterVertically)
                                    .horizontal_arrangement(LinearArrangement::SpacedBy(10.)),
                                move || {
                                    Artwork(track.clone(), 42.);
                                    let track = track.clone();
                                    Column(
                                        Modifier::empty().weight(1.),
                                        ColumnSpec::default()
                                            .vertical_arrangement(LinearArrangement::SpacedBy(3.)),
                                        move || {
                                            Label(
                                                parsed_track_title(&track.title),
                                                Modifier::empty().fill_max_width(),
                                                13.,
                                                colors.label,
                                                true,
                                            );
                                            Label(
                                                library::track_subtitle(&track),
                                                Modifier::empty().fill_max_width(),
                                                11.,
                                                colors.secondary_label,
                                                false,
                                            );
                                        },
                                    );
                                },
                            );
                            GlassIconButton(
                                Modifier::empty().content_description(if playing {
                                    "Pause library playback"
                                } else {
                                    "Play library selection"
                                }),
                                GlassButtonSpec::plain(),
                                44.,
                                move || {
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
                                44.,
                                move || next_track(player),
                                NEXT,
                            );
                        },
                    );
                    BoxWithConstraints(
                        Modifier::empty()
                            .fill_max_width()
                            .height(3.)
                            .padding_each(52., 1., 4., 0.),
                        move |scope| {
                            Box(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(2.)
                                    .background(colors.separator),
                                BoxSpec::default(),
                                || {},
                            );
                            Box(
                                Modifier::empty()
                                    .width(scope.max_width().0 * clamp01(snapshot.position))
                                    .height(2.)
                                    .background(colors.accent),
                                BoxSpec::default(),
                                || {},
                            );
                        },
                    );
                },
            );
        },
    );
}
