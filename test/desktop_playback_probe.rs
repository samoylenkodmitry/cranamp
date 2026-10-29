//! Exercises the desktop media registration and real output using an original file.
use cranpose::{composable, Modifier, Text, TextStyle};
use std::time::Duration;

fn main() {
    let path = std::env::args().nth(1).expect("path to original audio");
    cranamp::create_surface_app()
        .with_application_id("com.cranamp.playback-probe")
        .with_title("Cranamp playback probe")
        .with_size(360, 120)
        .run(move || Probe(path.clone()));
}

#[composable]
fn Probe(path: String) {
    cranpose::LaunchedEffect((), move |_| {
        cranpose_core::launchBlocking(
            move || probe(path),
            |result| {
                match result
                    .map_err(|error| error.to_string())
                    .and_then(|result| result)
                {
                    Ok(()) => {
                        println!("PLAYBACK PROBE: passed");
                    }
                    Err(error) => {
                        println!(
                            "PLAYBACK PROBE: failed: {}",
                            error
                                .rsplit_once(": ")
                                .map_or(error.as_str(), |(_, reason)| reason)
                        );
                        let _ = cranamp::audio::stop();
                        std::process::exit(1);
                    }
                }
                cranpose_services::request_exit();
            },
        )
        .expect("probe worker");
    });
    Text(
        "Checking original audio".to_string(),
        Modifier::empty(),
        TextStyle::default(),
    );
}

fn probe(path: String) -> Result<(), String> {
    let content = cranpose_services::IncomingContent::from_file(std::path::Path::new(&path))
        .content()
        .ok_or("cannot resolve incoming file")?;
    let track = cranamp::audio::track_from_picked_file(content)?.ok_or("unsupported name")?;
    println!("Media reference obtained");
    cranamp::audio::play_track(
        &track,
        cranamp::audio::PlaybackSettings {
            volume: 0.05,
            balance: 0.5,
            equalizer_enabled: false,
            equalizer_values: [0.5; 11],
        },
    )?;
    let mut peak = 0.0_f32;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(100));
        if let Some(samples) = cranpose_services::latest_media_samples() {
            peak = samples
                .samples
                .iter()
                .fold(peak, |peak, sample| peak.max(sample.abs()));
        }
    }
    println!(
        "Decoded sample peak: {peak}; state: {:?}; progress: {:?}",
        cranpose_services::playback_state(),
        cranpose_services::playback_progress()
    );
    cranamp::audio::stop()?;
    if peak == 0.0 {
        Err("no decoded samples reached the output".into())
    } else {
        Ok(())
    }
}
