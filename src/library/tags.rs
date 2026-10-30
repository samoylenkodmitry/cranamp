use super::Record;
use symphonia::core::{
    common::Limit,
    formats::{probe::Hint, FormatOptions, TrackType},
    io::{MediaSource, MediaSourceStream},
    meta::{MetadataOptions, StandardTag, StandardVisualKey},
};

pub(super) fn read(
    source: Box<dyn MediaSource>,
    name: &str,
) -> anyhow::Result<(Record, Option<Vec<u8>>)> {
    let mut hint = Hint::new();
    if let Some((_, extension)) = name.rsplit_once('.') {
        hint.with_extension(extension);
    }
    let options = MetadataOptions::default()
        .limit_tag_bytes(Limit::Maximum(64 * 1024))
        .limit_visual_bytes(Limit::Maximum(super::MAX_IMAGE_BYTES));
    let mut format = symphonia::default::get_probe().probe(
        &hint,
        MediaSourceStream::new(source, Default::default()),
        FormatOptions::default(),
        options,
    )?;
    let mut record = Record::default();
    if let Some(track) = format.first_track(TrackType::Audio) {
        if let (Some(base), Some(duration)) = (track.time_base, track.duration) {
            let seconds = base.calc_duration(duration).map(|t| t.as_secs_f64());
            record.duration = seconds
                .filter(|s| s.is_finite() && *s > 0.0)
                .map(|s| s as f32);
        }
    }
    let mut artwork = None;
    let mut front = false;
    let mut metadata = format.metadata();
    loop {
        if let Some(revision) = metadata.current() {
            for container in std::iter::once(&revision.media)
                .chain(revision.per_track.iter().map(|t| &t.metadata))
            {
                for tag in &container.tags {
                    let value = match &tag.std {
                        Some(StandardTag::TrackTitle(v)) => Some((&mut record.title, v)),
                        Some(StandardTag::Artist(v)) => Some((&mut record.artist, v)),
                        Some(StandardTag::Album(v)) => Some((&mut record.album, v)),
                        Some(StandardTag::AlbumArtist(v)) => Some((&mut record.album_artist, v)),
                        Some(StandardTag::TrackNumber(n)) => {
                            record.track_number = Some(*n);
                            None
                        }
                        Some(StandardTag::DiscNumber(n)) => {
                            record.disc_number = Some(*n);
                            None
                        }
                        _ => None,
                    };
                    if let Some((slot, value)) = value {
                        let value =
                            value.trim_matches(|c: char| c.is_control() || c.is_whitespace());
                        if !value.is_empty() {
                            *slot = Some(value.chars().take(1024).collect());
                        }
                    }
                }
                for visual in &container.visuals {
                    let is_front = visual.usage == Some(StandardVisualKey::FrontCover);
                    if visual.data.len() <= super::MAX_IMAGE_BYTES
                        && (artwork.is_none() || (is_front && !front))
                    {
                        if let Ok(thumb) = super::thumbnail(&visual.data) {
                            artwork = Some(thumb);
                            front = is_front;
                        }
                    }
                }
            }
        }
        if metadata.pop().is_none() {
            break;
        }
    }
    Ok((record, artwork))
}
