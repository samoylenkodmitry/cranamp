use super::*;
use std::io::Cursor;
fn ready<T>(future: impl std::future::Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    let waker = std::task::Waker::noop();
    match future
        .as_mut()
        .poll(&mut std::task::Context::from_waker(waker))
    {
        std::task::Poll::Ready(value) => value,
        _ => panic!("short fixtures should finish before the cooperative yield"),
    }
}
#[test]
fn decodes_real_files_to_playable_pcm() {
    for name in [
        "tone.wav",
        "tone.flac",
        "tone.m4a",
        "tone.aiff",
        "tone.opus",
        "tone.mp2",
    ] {
        let bytes = std::fs::read(format!(
            "{}/test/fixtures/audio/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        let decoder = Decoder::open(
            Box::new(Cursor::new(bytes)),
            name.rsplit('.').next().unwrap(),
        )
        .unwrap();
        let mut output = Cursor::new(Vec::new());
        ready(write_wav(decoder, &mut output, WEB_PCM_LIMIT))
            .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let wav = output.into_inner();
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 48_000);
        assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 2);
        assert_eq!(
            u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize,
            wav.len() - 44
        );
        let samples: Vec<i16> = wav[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|v| i16::from_le_bytes([v[0], v[1]]))
            .collect();
        assert!(
            samples.iter().any(|s| s.abs() > 1000),
            "{name} decoded silence"
        );
        if name == "tone.opus" {
            assert_eq!(samples.len(), 5760 * 2, "Opus pre-skip and end trim");
        }
        let mut decoded_again = Decoder::open(Box::new(Cursor::new(wav)), "wav").unwrap();
        assert!(decoded_again.next(&mut Vec::new()).unwrap().is_some());
    }
}
#[test]
fn rejects_broken_input_and_limits_expansion() {
    assert!(Decoder::open(Box::new(Cursor::new(vec![0u8; 128])), "opus").is_err());
    let bytes = include_bytes!("../fixtures/audio/tone.flac").to_vec();
    let decoder = Decoder::open(Box::new(Cursor::new(bytes)), "flac").unwrap();
    let mut output = Cursor::new(Vec::new());
    assert!(ready(write_wav(decoder, &mut output, 1024))
        .unwrap_err()
        .to_string()
        .contains("limit"));
    assert!(output.get_ref().len() <= 1024);
}
#[test]
fn recognizes_opus_by_container_bytes() {
    assert!(is_ogg_opus(include_bytes!("../fixtures/audio/tone.opus")));
    assert!(!is_ogg_opus(b"OggS"));
    assert!(!is_ogg_opus(include_bytes!("../fixtures/audio/tone.flac")));
}
