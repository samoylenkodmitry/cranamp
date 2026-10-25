# Audio format support

A filename identifies a container; playback also requires a decoder for the audio
inside it. Cranamp enables every codec and container in Symphonia 0.6.1 on native
platforms. Browser imports now have the same Rust decoder as a fallback.

| Audio | Native local files | Browser local files |
| --- | --- | --- |
| MP1, MP2, MP3 | Shared decoder | Browser decoder or PCM import copy |
| AAC-LC, ALAC | Shared decoder | Browser decoder or PCM import copy |
| FLAC, Vorbis | Shared decoder | Browser decoder or PCM import copy |
| PCM, ADPCM in WAV, AIFF/AIFC or CAF | Shared decoder | Browser decoder or PCM import copy |
| Mono/stereo Opus in Ogg (`.opus`, `.ogg`, `.oga`) | PCM import copy | Browser decoder or PCM import copy |
| Audio in MP4, MKV or WebM | Requires a supported codec | Browser decoder or PCM import copy if the Rust decoder supports its codec |

Native Opus import uses opus-pure 0.2.2. It preserves the source file and creates a
16-bit PCM WAV playback copy in app storage. It applies Opus header gain, pre-skip
and end trim. Multichannel Opus and chained streams are not certified. PCM copies
have the decoded sample rate and channel count. Source imports and native PCM
copies are each limited to 2 GiB. Failed conversions remove temporary files.

Browser import first tries the actual file through Web Audio. If the browser
cannot decode it, Cranamp decodes it in Rust to a 16-bit PCM WAV and creates a
local object URL. Files remain on the device. Browser source files are limited to
64 MiB and fallback PCM copies to 256 MiB. Conversion yields between blocks so
input remains responsive. A conversion error appears in the player status.

This fallback applies to files selected through the picker, including files from
folder imports. Direct audio URLs still use the platform playback backend.
Native Opus URLs therefore remain unsupported. Browser URL playback depends on
the browser. iOS HLS continues to use AVPlayer.

WMA, Monkey's Audio (APE), WavPack, DSD and tracker modules are not supported by
this decoder set. HE-AAC requires a platform decoder that supports it. APE metadata
support in Symphonia does not mean Monkey's Audio decoding support.

The repository tests decode generated WAV, AIFF, FLAC, ALAC, MP2 and Opus fixtures.
They check audible sample data and WAV headers, including Opus trim length.
Cross-compilation verifies the shared implementation for web and mobile; physical
device playback and interruptions need separate platform testing.

References: [Symphonia 0.6.1](https://docs.rs/symphonia/0.6.1/symphonia/),
[opus-pure](https://docs.rs/opus-pure/0.2.2/opus_pure/),
[Web audio codec guide](https://developer.mozilla.org/en-US/docs/Web/Media/Guides/Formats/Audio_codecs).
