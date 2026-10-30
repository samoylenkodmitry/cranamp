//! Exercises the desktop media registration and real output using an original file.
use cranpose::{composable, Modifier, Text, TextStyle};
use std::time::Duration;

fn main() {
    let mut args = std::env::args().skip(1);
    let first = args
        .next()
        .expect("path to original audio, or --folder PATH");
    let folder = first == "--folder" || first == "--webdav";
    let webdav = first == "--webdav";
    let path = if folder {
        args.next().expect("folder path")
    } else {
        first
    };
    cranamp::create_surface_app()
        .with_application_id("com.cranamp.playback-probe")
        .with_title("Cranamp playback probe")
        .with_size(360, 120)
        .run(move || Probe(path.clone(), folder, webdav));
}

#[composable]
fn Probe(path: String, folder: bool, webdav: bool) {
    cranpose::LaunchedEffect((), move |_| {
        cranpose_core::spawn_ui_task(async move {
            let started = std::time::Instant::now();
            let track = if folder {
                let selected = if webdav {
                    cranpose_services::WebDavFolder::new(
                        &path,
                        cranpose_services::default_http_client(),
                    )
                    .expect("WebDAV URL")
                    .handle()
                } else {
                    cranpose_services::file_folder(&path)
                };
                let stream = cranpose_services::folder_files(selected);
                loop {
                    match stream.next().await {
                        Ok(Some(entry)) => {
                            match cranamp::audio::track_from_picked_file(entry).await {
                                Ok(Some(track)) => {
                                    println!(
                                        "Candidate after {:?}; complete tree not enumerated",
                                        started.elapsed()
                                    );
                                    let played = cranpose_core::withBlocking(move || probe(track))
                                        .await
                                        .expect("probe worker");
                                    match played {
                                        Ok(()) => {
                                            println!("PLAYBACK PROBE: passed");
                                            cranpose_services::request_exit();
                                            return;
                                        }
                                        Err(error) => {
                                            println!("Skipped unplayable candidate: {error}")
                                        }
                                    }
                                }
                                Ok(None) => continue,
                                Err(error) => panic!("folder entry: {error}"),
                            }
                        }
                        Ok(None) => panic!("folder has no supported audio"),
                        Err(error) => panic!("folder scan: {error}"),
                    }
                }
            } else {
                let content =
                    cranpose_services::IncomingContent::from_file(std::path::Path::new(&path))
                        .content()
                        .expect("cannot resolve incoming file");
                cranamp::audio::track_from_picked_file(content)
                    .await
                    .expect("content reference")
                    .expect("supported name")
            };
            println!(
                "First track available after {:?}; complete tree not enumerated: {folder}",
                started.elapsed()
            );
            cranpose_core::launchBlocking(
                move || probe(track),
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
    });
    Text(
        "Checking original audio".to_string(),
        Modifier::empty(),
        TextStyle::default(),
    );
}

fn probe(track: cranamp::audio::Track) -> Result<(), String> {
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
