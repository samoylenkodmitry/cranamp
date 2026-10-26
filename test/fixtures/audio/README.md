# Audio import fixtures

These files contain a generated 440 Hz tone, 0.12 seconds, 48 kHz stereo.
No recorded music is included. The Opus file uses libopus so the portable decoder
is checked against an independently encoded stream.

Generated with FFmpeg:

```
ffmpeg -f lavfi -i sine=frequency=440:sample_rate=48000:duration=0.12 -ac 2 -c:a pcm_s16le tone.wav
ffmpeg -i tone.wav -c:a flac tone.flac
ffmpeg -i tone.wav -c:a alac tone.m4a
ffmpeg -i tone.wav -c:a pcm_s16be tone.aiff
ffmpeg -i tone.wav -c:a libopus tone.opus
ffmpeg -i tone.wav -c:a mp2 tone.mp2
```
