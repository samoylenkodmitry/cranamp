# Playback coverage target

Status: engineering scope, not a claim of implemented support. See
[current format support](../../audio-formats.md) for the code on this branch.
The target applies to desktop, iOS, Android and browsers. The same file should
play on each platform through a shared decoder when the platform cannot decode it.

## Format set

| Use | Required codecs | Containers and examples |
| --- | --- | --- |
| Music and podcasts | MP3, AAC-LC, HE-AAC v1/v2, Vorbis, Opus | MP3, AAC/ADTS, M4A/M4B, Ogg, WebM |
| Lossless music and recordings | FLAC, ALAC, PCM integer/float, common ADPCM variants | FLAC, Ogg FLAC, M4A, WAV/RF64, AIFF/AIFC, CAF |
| Existing music libraries | WMA including Pro/Lossless, WavPack, Monkey's Audio, Musepack | ASF/WMA, WV, APE, MPC |
| Audio in video | Above codecs plus AC-3, E-AC-3, DTS and MP2 | MP4/M4V, MOV, MKV, WebM, AVI, MPEG-PS/TS, 3GP |
| Voice recordings | AMR-NB/WB, Speex, G.711 | AMR, 3GP, Ogg, WAV |

Tracker formats (MOD/XM/IT/S3M), MIDI synthesis and DSD need their own playback
contracts. They belong in the follow-up coverage inventory. They must not be
advertised as supported merely because their extensions pass a picker filter.

## Video audio flow

The normal Add files action accepts audio and video. Probe the file contents.
When there is one supported audio stream, add it directly to the playlist.
When there are several, open an Audio tracks sheet with language, title and channel
count. Select the container's default supported track initially. Offer a preview
and an Add action; retain the selection for later playback. Use neutral names such
as “Audio track 2” when metadata is absent.

Store source identity and stream identity together. Playback decodes the selected
audio stream; it does not render video or rewrite the source file. The item menu
offers Change audio track. Switching tracks preserves playback time when possible.
A file with no audio gets a specific message. Encrypted or unsupported streams
get a reason and remain available for a different selection.

The track chooser follows Studio's size rules: full-screen list on a small phone,
sheet on a large phone, and a bounded dialog on a tablet. This is a player flow;
it does not belong among Skin Studio's painting tools.

## Decoder approach

Use a common probe/decode/seek interface with stream metadata and explicit codec
capabilities. Retain the current decoder where it meets that interface. FFmpeg is
the leading candidate for missing codecs and containers: its published tables
include WMA, WavPack, Monkey's Audio and the audio codecs used in common video
containers. Build capabilities still depend on the chosen configuration.
[FFmpeg format and codec documentation](https://www.ffmpeg.org/general.html).

Native integration belongs in the shared audio backend, with packaged libraries
for each target. A host-installed executable is not a mobile solution. Browser
fallback should decode incrementally in a worker into bounded PCM buffers, with
an audio worklet for output. Account for builds without shared memory. Keep native
browser decoding when it provides the required format and stream selection.

Preserve decoded precision through floating-point playback buffers. Resample only
for the output device. Define channel mapping and stereo downmix explicitly.
The current full-file 16-bit WAV conversion is an interim import path; it cannot
serve as the final implementation for long files or high-resolution libraries.
Temporary conversion, if needed, has a storage limit, progress and cancellation.

Before choosing the final dependency, verify reproducible builds for each target,
the enabled codec list, distribution terms and startup/memory cost. Package only
required demuxers and audio decoders. Video decoding and encoding are unnecessary
for this task. Pin the build configuration in the repository.

## Completion evidence

Maintain a codec-by-container fixture matrix under root `test/`, including a video
with two named audio streams and a video without audio. Record actual playback
results for desktop, physical iOS/Android devices and the supported browsers.
Cross-compilation alone does not certify playback.

Each required combination must start, seek, finish and replay. Verify gapless trim
where applicable, channel mapping and duration. A long source must start without
decoding the whole file into memory. Cancel must release temporary storage.
Mobile checks include background playback and audio interruption recovery.
Corrupt packets and unsupported streams must produce useful errors without
breaking the next playlist item.

Direct URLs need the same coverage decision as files. Test seeking with HTTP range
support and clear behavior without it. Keep HLS on its established platform path
until an alternative has equivalent behavior. Browser CORS restrictions remain
visible to the user; they are not codec failures.
