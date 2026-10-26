//! Portable import decoding for formats the platform player cannot open.
use anyhow::{Context, Result};
use std::io::{Read, Seek, SeekFrom, Write};
use symphonia::core::{
    codecs::{
        audio::{AudioDecoder, AudioDecoderOptions},
        CodecParameters,
    },
    formats::{probe::Hint, FormatOptions, FormatReader, TrackType},
    io::{MediaSource, MediaSourceStream},
    meta::MetadataOptions,
};

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) const WEB_PCM_LIMIT: usize = 256 * 1024 * 1024;

pub(crate) fn is_ogg_opus(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"OggS") || bytes.len() < 28 {
        return false;
    }
    let payload = 27 + bytes[26] as usize;
    bytes.get(payload..payload + 8) == Some(b"OpusHead")
}

pub(crate) enum Decoder {
    Audio {
        format: Box<dyn FormatReader>,
        decoder: Box<dyn AudioDecoder>,
        track: u32,
    },
    Opus {
        reader: Box<opus_pure::OggOpusReader<Box<dyn MediaSource>>>,
        decoder: Box<opus_pure::OpusDecoder>,
        trim: opus_pure::Trim,
        channels: usize,
    },
}
impl Decoder {
    pub(crate) fn open(mut source: Box<dyn MediaSource>, extension: &str) -> Result<Self> {
        let mut header = [0; 512];
        let count = source.read(&mut header)?;
        source.seek(SeekFrom::Start(0))?;
        if is_ogg_opus(&header[..count]) {
            let reader = opus_pure::OggOpusReader::new(source)?;
            let head = reader.head().clone();
            let channels = head.channel_count as usize;
            anyhow::ensure!(
                channels == 1 || channels == 2,
                "Opus import currently supports mono and stereo"
            );
            let decoder = head.decoder(48_000)?;
            let trim = opus_pure::Trim::new(&head, 48_000, channels)?;
            return Ok(Self::Opus {
                reader: Box::new(reader),
                decoder: Box::new(decoder),
                trim,
                channels,
            });
        }
        let mut hint = Hint::new();
        hint.with_extension(extension);
        let format = symphonia::default::get_probe().probe(
            &hint,
            MediaSourceStream::new(source, Default::default()),
            FormatOptions::default(),
            MetadataOptions::default(),
        )?;
        let track = format
            .first_track(TrackType::Audio)
            .context("File has no audio track")?;
        let id = track.id;
        let Some(CodecParameters::Audio(params)) = &track.codec_params else {
            anyhow::bail!("File has no audio codec");
        };
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(params, &AudioDecoderOptions::default())?;
        Ok(Self::Audio {
            format,
            decoder,
            track: id,
        })
    }

    fn next(&mut self, samples: &mut Vec<f32>) -> Result<Option<(u32, u16)>> {
        samples.clear();
        match self {
            Self::Audio {
                format,
                decoder,
                track,
            } => loop {
                let Some(packet) = format.next_packet()? else {
                    return Ok(None);
                };
                if packet.track_id != *track {
                    continue;
                }
                let decoded = decoder.decode(&packet)?;
                let spec = decoded.spec();
                let rate = spec.rate();
                let channels = spec.channels().count() as u16;
                decoded.copy_to_vec_interleaved(samples);
                return Ok(Some((rate, channels)));
            },
            Self::Opus {
                reader,
                decoder,
                trim,
                channels,
            } => {
                let Some(packet) = reader.packets().next() else {
                    return Ok(None);
                };
                let packet = packet?;
                samples.resize(opus_pure::MAX_PACKET_SAMPLES * *channels, 0.);
                let frames =
                    decoder.decode(&packet.data, opus_pure::MAX_PACKET_SAMPLES, samples)?;
                let kept = trim.keep(&packet, &samples[..frames * *channels]).to_vec();
                *samples = kept;
                Ok(Some((48_000, *channels as u16)))
            }
        }
    }
}

/// Writes a bounded PCM WAV incrementally. Yield between blocks so imports do
/// not prevent touch input or picker callbacks on the UI executor.
pub(crate) async fn write_wav(
    mut decoder: Decoder,
    output: &mut (impl Write + Seek),
    limit: usize,
) -> Result<()> {
    output.write_all(&[0; 44])?;
    let mut samples = Vec::new();
    let mut format = None;
    let mut size = 0usize;
    let mut blocks = 0usize;
    while let Some(current) = decoder.next(&mut samples)? {
        if let Some(previous) = format {
            anyhow::ensure!(previous == current, "Audio format changes during this file");
        }
        format = Some(current);
        let added = samples
            .len()
            .checked_mul(2)
            .context("Audio size overflow")?;
        anyhow::ensure!(
            added <= limit.saturating_sub(size + 44),
            "Decoded audio exceeds the {} MiB import limit",
            limit / 1024 / 1024
        );
        let pcm: Vec<u8> = samples
            .iter()
            .flat_map(|sample| {
                let value = (sample.clamp(-1., 1.) * 32768.)
                    .round()
                    .clamp(-32768., 32767.) as i16;
                value.to_le_bytes()
            })
            .collect();
        output.write_all(&pcm)?;
        size += added;
        blocks += 1;
        if blocks.is_multiple_of(32) {
            coroflow::delay(std::time::Duration::from_millis(1)).await;
        }
    }
    let (rate, channels) = format.context("File contains no decoded audio")?;
    anyhow::ensure!(
        size > 0 && rate > 0 && channels > 0,
        "File contains no decoded audio"
    );
    let align = channels.checked_mul(2).context("Too many audio channels")?;
    let byte_rate = rate
        .checked_mul(align as u32)
        .context("Audio rate overflow")?;
    let data_size = u32::try_from(size).context("WAV size overflow")?;
    let riff_size = data_size.checked_add(36).context("WAV size overflow")?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(b"RIFF")?;
    output.write_all(&riff_size.to_le_bytes())?;
    output.write_all(b"WAVEfmt ")?;
    output.write_all(&16u32.to_le_bytes())?;
    output.write_all(&1u16.to_le_bytes())?;
    output.write_all(&channels.to_le_bytes())?;
    output.write_all(&rate.to_le_bytes())?;
    output.write_all(&byte_rate.to_le_bytes())?;
    output.write_all(&align.to_le_bytes())?;
    output.write_all(&16u16.to_le_bytes())?;
    output.write_all(b"data")?;
    output.write_all(&data_size.to_le_bytes())?;
    output.flush()?;
    Ok(())
}

#[cfg(test)]
#[path = "../test/unit/audio_import.rs"]
mod tests;
