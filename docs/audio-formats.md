# Audio format support

Playback requires a decoder for the audio inside a file. Native Cranamp uses
Symphonia 0.6.1 with all bundled codecs and an Opus decoder. Selected files stay
in their original location. Cranamp does not copy or convert them during import.

| Audio | Native files and direct streams | Browser files and URLs |
| --- | --- | --- |
| MP1, MP2, MP3 | Shared decoder | Browser support |
| AAC-LC, ALAC | Shared decoder | Browser support |
| FLAC, Vorbis | Shared decoder | Browser support |
| PCM, ADPCM in WAV, AIFF/AIFC or CAF | Shared decoder | Browser support |
| Opus in Ogg or WebM | Shared decoder | Browser support |
| Audio in MP4 or MKV | Requires a supported audio codec | Browser support |

Native Opus playback uses opus-pure with a pinned portability fix. It decodes
packets during playback and applies header gain and packet trimming. Standard
mono, stereo and family-1 surround layouts are accepted. Other channel mappings
are rejected. Chained streams are not certified.

Android references selected documents through their provider URIs. Apple uses
bookmarks so the decoder retains access to selected files. Seekable sources use
the original file descriptor. A provider pipe uses a 256 KiB memory buffer with
backpressure and does not support seeking. A provider can impose its own caching
or availability rules. Some containers require random access and cannot play
through a provider that exposes only a forward stream.

Folder discovery publishes the first audio track immediately and continues in
batches. It does not open every provider track to discover its duration. Browser
picks create object URLs for the original files; these references last for the
page session. Browser playback depends on the browser's codecs. iOS HLS uses
AVPlayer and does not support the shared equalizer or visualizer.

WMA, Monkey's Audio, WavPack, DSD and tracker modules are outside the native
decoder set. HE-AAC requires a platform decoder that supports it. Protected files
are unsupported.

Framework integration tests play generated WAV, AIFF, FLAC, ALAC, MP2 and Opus
fixtures, plus Opus audio from a WebM video. A provider-pipe test checks that
paused playback does not download the full file. Cranamp tests verify that
selection keeps the source reference and that a delayed folder scan publishes
its first track. The Galaxy Tab test uses the real RoundSync folder and checks
playback progress and audio-cache growth.

References: [Symphonia 0.6.1](https://docs.rs/symphonia/0.6.1/symphonia/),
[opus-pure](https://github.com/samoylenkodmitry/opus-pure/tree/cc886594b6b1a4bd11f66a69e42d9c5ed9a8ecce),
[Web audio codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs).
