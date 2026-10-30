//! Persisted appearance for mobile chrome. The player's bitmap keeps its own colors.
use super::*;
use cranpose_core::{compositionLocalOf, CompositionLocal};
use std::cell::OnceCell;

const KEY: &str = "cranamp.appearance.v1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Mode {
    Day,
    Night,
    #[default]
    Auto,
    Skin,
}

impl Mode {
    pub(super) const ALL: [Self; 4] = [Self::Day, Self::Night, Self::Auto, Self::Skin];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Day => "Day",
            Self::Night => "Night",
            Self::Auto => "Auto",
            Self::Skin => "From skin background",
        }
    }

    pub(super) fn description(self) -> &'static str {
        match self {
            Self::Day => "Always use a light appearance.",
            Self::Night => "Always use a dark appearance.",
            Self::Auto => "Follow your device's appearance.",
            Self::Skin => "Use the player skin's corner color.",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Night => "night",
            Self::Auto => "auto",
            Self::Skin => "skin",
        }
    }

    fn parse(value: Option<&str>) -> Self {
        match value {
            Some("day") => Self::Day,
            Some("night") => Self::Night,
            Some("skin") => Self::Skin,
            _ => Self::Auto,
        }
    }

    fn scheme(self, background: Option<Color>) -> SchemeMode {
        match self {
            Self::Day => SchemeMode::Light,
            Self::Night => SchemeMode::Dark,
            Self::Auto => SchemeMode::Auto,
            Self::Skin => background.map_or(SchemeMode::Auto, |color| {
                if prefers_dark(color) {
                    SchemeMode::Dark
                } else {
                    SchemeMode::Light
                }
            }),
        }
    }
}

/// Choose the black/white label with the greater contrast against the sampled
/// sRGB background, including saturated skins whose RGB average is misleading.
pub(super) fn prefers_dark(color: Color) -> bool {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * linear(color.0) + 0.7152 * linear(color.1) + 0.0722 * linear(color.2);
    1.05 / (luminance + 0.05) > (luminance + 0.05) / 0.05
}

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Ui {
    pub mode: MutableState<Mode>,
    pub error: MutableState<String>,
    loaded: MutableState<bool>,
    revision: MutableState<u64>,
    attempted: MutableState<u64>,
}

impl Ui {
    pub(super) fn choose(self, mode: Mode) {
        self.mode.set(mode);
        self.error.set(String::new());
        self.revision.update(|revision| *revision += 1);
    }
}

#[composable]
pub(super) fn remember_theme() -> Ui {
    let ui = cranpose_core::remember(|| Ui {
        mode: cranpose_core::mutableStateOf(Mode::Auto),
        error: cranpose_core::mutableStateOf(String::new()),
        loaded: cranpose_core::mutableStateOf(false),
        revision: cranpose_core::mutableStateOf(0),
        attempted: cranpose_core::mutableStateOf(0),
    })
    .with(|ui| *ui);
    cranpose_core::LaunchedEffectAsync((), move |_| {
        let store = cranpose_services::preferences();
        Box::pin(async move {
            let stored = cranpose_core::withBlocking(move || store.get(KEY)).await;
            if ui.revision.get_non_reactive() == 0 {
                ui.mode.set(Mode::parse(stored.ok().flatten().as_deref()));
            }
            ui.loaded.set(true);
        })
    });
    let pending = ui.loaded.get() && ui.revision.get() != ui.attempted.get();
    cranpose_core::LaunchedEffectAsync(pending, move |_| {
        let store = cranpose_services::preferences();
        Box::pin(async move {
            if !pending {
                return;
            }
            while ui.revision.get_non_reactive() != ui.attempted.get_non_reactive() {
                let revision = ui.revision.get_non_reactive();
                let mode = ui.mode.get_non_reactive();
                let store = store.clone();
                let result = cranpose_core::withBlocking(move || store.set(KEY, mode.key()))
                    .await
                    .map_err(|error| error.to_string())
                    .and_then(|result| result.map_err(|error| error.to_string()));
                ui.error.set(result.err().map_or_else(String::new, |error| {
                    format!("Could not save your theme: {error}")
                }));
                ui.attempted.set(revision);
            }
        })
    });
    ui
}

fn skin_background() -> CompositionLocal<Option<Color>> {
    thread_local! {
        static LOCAL: OnceCell<CompositionLocal<Option<Color>>> = const { OnceCell::new() };
    }
    LOCAL.with(|local| local.get_or_init(|| compositionLocalOf(|| None)).clone())
}

/// Liquid controls retain their light/dark material palette. App surfaces and
/// Studio use the skin color behind that material in the skin-derived mode.
pub(in crate::winamp) fn colors() -> LiquidColors {
    let mut colors = cranpose::liquid::liquid_colors();
    if let Some(background) = skin_background().current() {
        colors.background = background;
    }
    colors
}

#[composable]
pub(super) fn Theme(ui: Ui, skin: WinampSkinState, content: impl FnOnce()) {
    let mode = ui.mode.get();
    let background = if mode == Mode::Skin {
        skin.get().ok().map(|skin| reflection::corner_color(&skin))
    } else {
        None
    };
    LiquidTheme(
        LiquidThemeSpec {
            scheme: mode.scheme(background),
            ..Default::default()
        },
        move || {
            cranpose_core::CompositionLocalProvider(
                [skin_background().provides(background)],
                content,
            );
        },
    );
}

#[cfg(test)]
#[path = "../../../test/unit/winamp/mobile_theme.rs"]
mod tests;
