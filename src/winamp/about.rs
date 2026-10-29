//! About and legal pages shared by the mobile and desktop settings surfaces.
use super::*;

pub(super) const PRIVACY_URL: &str = "https://cranamp.dmitrysamoylenko.in/privacy/";
pub(super) const SUPPORT_URL: &str = "https://cranamp.dmitrysamoylenko.in/support/";
const CONTACT: &str = "trdmitriisamoilenko@gmail.com";

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
pub(super) enum Page {
    #[default]
    Settings,
    About,
    Licenses,
    License(usize),
}

impl Page {
    pub(super) fn parent(self) -> Self {
        match self {
            Self::Settings | Self::About => Self::Settings,
            Self::Licenses => Self::About,
            Self::License(_) => Self::Licenses,
        }
    }
}

#[derive(serde::Deserialize)]
struct Notice {
    title: String,
    text: String,
}

fn notices() -> &'static [Notice] {
    static NOTICES: std::sync::OnceLock<Vec<Notice>> = std::sync::OnceLock::new();
    NOTICES.get_or_init(|| {
        let mut notices = vec![
            Notice {
                title: "Cranamp — Apache License 2.0".into(),
                text: include_str!("../../LICENSE").into(),
            },
            Notice {
                title: "Liberation Sans — SIL Open Font License".into(),
                text: include_str!("../../docs/third-party/LiberationSans-OFL-1.1.txt").into(),
            },
        ];
        let dependencies: Vec<Notice> =
            serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/notices.json")))
                .expect("build-generated license catalog");
        notices.extend(dependencies);
        notices
    })
}

pub(super) fn open_link(url: &'static str, error: MutableState<String>) {
    error.set(
        match cranpose_services::default_uri_handler().open_uri(url) {
            Ok(()) => String::new(),
            Err(_) => format!("Could not open this link. You can use this address: {url}"),
        },
    );
}

#[composable]
pub(super) fn LinkError(error: MutableState<String>, mobile: bool) {
    if !error.get().is_empty() {
        AboutText(error.get(), mobile);
    }
}

#[composable]
fn AboutText(text: impl Into<String>, mobile: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    if mobile {
        Text(
            text.into(),
            Modifier::empty().fill_max_width(),
            settings_text_style(16.0, cranpose::liquid::prelude::liquid_colors().label),
        );
        return;
    }
    let _ = mobile;
    Text(
        text.into(),
        Modifier::empty().fill_max_width(),
        settings_text_style(12.0, SETTINGS_TEXT),
    );
}

#[composable]
fn AboutHeading(title: &'static str, mobile: bool) {
    #[cfg(not(target_arch = "wasm32"))]
    if mobile {
        Text(
            title.to_string(),
            Modifier::empty().fill_max_width(),
            settings_text_style(32.0, cranpose::liquid::prelude::liquid_colors().label),
        );
        return;
    }
    let _ = mobile;
    Text(
        title.to_string(),
        Modifier::empty().fill_max_width(),
        settings_text_style(17.0, SETTINGS_TEXT),
    );
}

#[composable]
fn AboutAction(label: impl Into<String>, mobile: bool, action: impl Fn() + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    if mobile {
        super::mobile::GlassAction(label, action);
        return;
    }
    let _ = mobile;
    SettingsActionButton(label.into(), SETTINGS_CARD, action);
}

#[composable]
fn AboutLink(label: &'static str, url: &'static str, mobile: bool) {
    let error = cranpose_core::rememberMutableStateOf(String::new);
    AboutAction(label, mobile, move || open_link(url, error));
    LinkError(error, mobile);
}

#[composable]
pub(super) fn Content(page: MutableState<Page>, mobile: bool) {
    let current = page.get();
    AboutAction(
        match current.parent() {
            Page::Settings => "Back to Settings",
            Page::About => "Back to About",
            _ => "Back to Licenses",
        },
        mobile,
        move || page.set(current.parent()),
    );
    match current {
        Page::Settings => {}
        Page::About => {
            AboutHeading("About Cranamp", mobile);
            AboutText(format!("Version {}", env!("CARGO_PKG_VERSION")), mobile);
            AboutText(
                "A music player with classic skins and Skin Studio. Built with Cranpose.",
                mobile,
            );
            AboutText(format!("Support contact\n{CONTACT}"), mobile);
            AboutLink(
                "Contact support",
                "mailto:trdmitriisamoilenko@gmail.com",
                mobile,
            );
            AboutText("Your mail app opens a draft. Include your app version and device model. Omit private files and secret URLs.", mobile);
            AboutLink("Help with Cranamp", SUPPORT_URL, mobile);
            AboutLink("Privacy policy", PRIVACY_URL, mobile);
            AboutLink(
                "Source code",
                "https://github.com/samoylenkodmitry/cranamp",
                mobile,
            );
            AboutAction("Open source licenses", mobile, move || {
                page.set(Page::Licenses)
            });
        }
        Page::Licenses => {
            AboutHeading("Open source licenses", mobile);
            for (index, notice) in notices().iter().enumerate() {
                AboutAction(notice.title.clone(), mobile, move || {
                    page.set(Page::License(index))
                });
            }
        }
        Page::License(index) => {
            if let Some(notice) = notices().get(index) {
                AboutHeading(&notice.title, mobile);
                // Keep individual text layouts bounded, including long dependency notices.
                for paragraph in notice.text.split("\n\n").filter(|text| !text.is_empty()) {
                    AboutText(paragraph.to_string(), mobile);
                }
            }
        }
    }
}
