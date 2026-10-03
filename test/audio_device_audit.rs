use cranpose::{composable, Modifier, Text, TextStyle};
use cranpose_services::{EqualizerSettings, MediaItem};
use serde_json::json;
use std::time::{Duration, Instant};

fn main() {
    cranamp::create_surface_app().run(screen);
}

#[composable]
fn screen() {
    let status = cranpose::rememberMutableStateOf(|| "Testing audio output…".to_string());
    cranpose::LaunchedEffect((), move |_| {
        cranpose_core::launchBlocking(run, move |result| {
            status.set(match result {
                Ok(()) => "Audio audit finished. Read audio-audit.json.".into(),
                Err(error) => format!("Audio audit failed: {error:#}"),
            });
        });
    });
    Text(
        status.get(),
        Modifier::empty().fill_max_size().padding(30.0),
        TextStyle::default(),
    );
}

fn run() -> anyhow::Result<()> {
    let directory = cranpose::application_directories()?.data;
    std::fs::create_dir_all(&directory)?;
    let source = directory.join("audio-audit.wav");
    std::fs::write(&source, wave())?;
    let output = directory.join("audio-audit.json");
    let result = measure(&source, &output);
    cranpose_services::stop_media();
    std::fs::remove_file(source)?;
    if let Err(error) = &result {
        std::fs::write(&output, json!({"error": format!("{error:#}")}).to_string())?;
    }
    result
}

fn measure(source: &std::path::Path, output: &std::path::Path) -> anyhow::Result<()> {
    cranpose_services::open_media(MediaItem::new(cranpose_services::uri_for_path(source)))?;
    cranpose_services::set_media_volume(0.05);
    anyhow::ensure!(
        cranpose_services::set_media_balance(0.0),
        "balance unavailable"
    );
    anyhow::ensure!(
        cranpose_services::set_media_analysis_enabled(true),
        "analysis unavailable"
    );
    let mut eq = EqualizerSettings {
        enabled: true,
        preamp_db: 0.0,
        gains_db: vec![0.0; 10],
    };
    anyhow::ensure!(
        cranpose_services::set_media_equalizer(eq.clone()),
        "EQ unavailable"
    );
    cranpose_services::play_media()?;
    let start = Instant::now();
    let mut observations = Vec::new();
    let mut baseline = 0.0_f32;
    let mut boosted = 0.0_f32;
    for step in 0..80 {
        std::thread::sleep(Duration::from_millis(250));
        if step == 12 {
            eq.gains_db[5] = 6.0;
            cranpose_services::set_media_equalizer(eq.clone());
        }
        if step == 28 {
            anyhow::ensure!(
                cranpose_services::set_media_balance(1.0),
                "live balance unavailable"
            );
        }
        if step == 36 {
            cranpose_services::pause_media();
        }
        if step == 40 {
            cranpose_services::seek_media(Duration::from_secs(1))?;
            cranpose_services::play_media()?;
        }
        let samples = cranpose_services::latest_media_samples();
        let peak = samples
            .as_ref()
            .map(|samples| {
                samples
                    .samples
                    .iter()
                    .fold(0.0_f32, |peak, value| peak.max(value.abs()))
            })
            .unwrap_or(0.0);
        if (5..11).contains(&step) {
            baseline = baseline.max(peak);
        }
        if (18..27).contains(&step) {
            boosted = boosted.max(peak);
        }
        observations.push(json!({"wall_seconds": start.elapsed().as_secs_f64(), "position_seconds": cranpose_services::playback_progress().position.as_secs_f64(), "state": format!("{:?}", cranpose_services::playback_state()), "analysis_peak": peak, "sample_rate": samples.as_ref().map(|s|s.sample_rate), "channels": samples.as_ref().map(|s|s.channels)}));
        std::fs::write(
            output,
            json!({"observations": observations, "baseline":baseline,"boosted":boosted})
                .to_string(),
        )?;
    }
    anyhow::ensure!(
        baseline > 0.09 && boosted > baseline * 1.8 && boosted < baseline * 2.2,
        "Unexpected EQ levels: {baseline} → {boosted}"
    );
    anyhow::ensure!(
        cranpose_services::playback_progress().position > Duration::from_secs(3),
        "Output callbacks did not advance playback"
    );
    std::fs::write(
        output,
        json!({"passed": true, "baseline":baseline,"boosted":boosted,"observations":observations})
            .to_string(),
    )?;
    Ok(())
}

fn wave() -> Vec<u8> {
    let frames = 48_000 * 60;
    let size = frames * 4;
    let mut bytes = Vec::with_capacity(44 + size);
    bytes.extend(b"RIFF");
    bytes.extend(((36 + size) as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(2_u16.to_le_bytes());
    bytes.extend(48_000_u32.to_le_bytes());
    bytes.extend(192_000_u32.to_le_bytes());
    bytes.extend(4_u16.to_le_bytes());
    bytes.extend(16_u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((size as u32).to_le_bytes());
    for frame in 0..frames {
        let sample = (0.1
            * (std::f32::consts::TAU * 1_000.0 * frame as f32 / 48_000.0).sin()
            * 32767.0) as i16;
        bytes.extend(sample.to_le_bytes());
        bytes.extend(sample.to_le_bytes());
    }
    bytes
}
