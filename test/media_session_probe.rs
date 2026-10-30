//! Exercises the real player, library cover lookup and desktop media adapter.
//! The silent test backend keeps playback stable while the OS session is read.
//! Pass an output directory; wait for `ready`, then write `next`, read the
//! cleared session after `cleared`, and write `exit` to finish the probe.
use cranpose_services::{MediaError, MediaItem, MediaPlayer, PlaybackState};
use std::{sync::Arc, time::Duration};

struct SilentPlayer;
impl MediaPlayer for SilentPlayer {
    fn capabilities(&self) -> cranpose_services::MediaCapabilities {
        cranpose_services::MediaCapabilities {
            session: true,
            ..Default::default()
        }
    }
    fn prepare(&self, _: &MediaItem) -> Result<(), MediaError> {
        cranpose_services::publish_playback_state(PlaybackState::Paused);
        Ok(())
    }
    fn play(&self) -> Result<(), MediaError> {
        cranpose_services::publish_playback_state(PlaybackState::Playing);
        Ok(())
    }
    fn pause(&self) {
        cranpose_services::publish_playback_state(PlaybackState::Paused);
    }
    fn stop(&self) {}
    fn set_volume(&self, _: f32) {}
}

fn main() {
    let directory = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&directory).unwrap();
    let directory = std::fs::canonicalize(directory).unwrap();
    let mut saved = "version=1\ncurrent_index=0\n".to_owned();
    for (album, title) in [
        ("Blue Album", "Alice - Blue Song"),
        ("Bare Album", "Bob - Bare Song"),
    ] {
        let folder = directory.join(album);
        std::fs::create_dir_all(&folder).unwrap();
        let track = folder.join(format!("{title}.wav"));
        std::fs::write(&track, include_bytes!("fixtures/audio/tone.wav")).unwrap();
        let hex = |text: &str| {
            text.bytes()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        saved.push_str(&format!(
            "track={}\t{}\t90\n",
            hex(title),
            hex(&track.to_string_lossy())
        ));
    }
    image::RgbaImage::from_pixel(64, 64, image::Rgba([10, 60, 180, 255]))
        .save(directory.join("Blue Album/cover.png"))
        .unwrap();
    cranamp::create_surface_app()
        .with_application_id("com.cranamp.media-session-probe")
        .with_title("Cranamp media session test")
        .with_developer_inspector(false)
        .with_size(393, 780)
        .with_test_driver(move |robot| {
            std::thread::spawn(move || {
                robot.wait_for_idle().unwrap();
                click(&robot, "Play");
                wait(|| {
                    cranpose_services::current_media_item()
                        .is_some_and(|item| item.metadata.artwork.is_some())
                });
                std::fs::write(directory.join("ready"), "Blue Song\nAlice\nBlue Album\n").unwrap();
                wait(|| directory.join("next").exists());
                click(&robot, "Next track");
                wait(|| {
                    cranpose_services::current_media_item().is_some_and(|item| {
                        item.metadata.title == "Bare Song" && item.metadata.artwork.is_none()
                    })
                });
                std::fs::write(directory.join("cleared"), "Bare Song\nBob\nBare Album\n").unwrap();
                wait(|| directory.join("exit").exists());
                cranpose_services::request_exit();
            });
        })
        .run(move || {
            let saved = saved.clone();
            cranpose_core::remember(move || {
                cranpose_services::set_platform_media_player(Arc::new(SilentPlayer));
                cranpose_services::preferences()
                    .set("cranamp.player", &saved)
                    .unwrap();
            });
            cranamp::winamp::WinampMobileApp();
        });
}

fn wait(condition: impl Fn() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(120);
    while !condition() {
        assert!(
            std::time::Instant::now() < deadline,
            "media probe timed out"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn click(robot: &cranpose::Robot, label: &str) {
    fn find<'a>(
        nodes: &'a [cranpose::SemanticElement],
        label: &str,
    ) -> Option<&'a cranpose::SemanticElement> {
        nodes.iter().find_map(|node| {
            if node.clickable && node.text.as_deref() == Some(label) {
                Some(node)
            } else {
                find(&node.children, label)
            }
        })
    }
    let semantics = robot.get_semantics().unwrap();
    let bounds = find(&semantics, label).expect("player control").bounds;
    robot
        .click(bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.)
        .unwrap();
}
