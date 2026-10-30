# Library metadata fixtures

These files contain a generated 440 Hz tone, 0.12 seconds. No recorded music is included.

Generated with FFmpeg:

```sh
ffmpeg -f lavfi -i 'sine=frequency=440:duration=0.12' -metadata title='Tagged title' -metadata artist='Tagged artist' -metadata album='Tagged album' -metadata album_artist='Album artist' -metadata track='3/12' -metadata disc='2/2' tagged.flac
ffmpeg -i tagged.flac -c:a libmp3lame tagged.mp3
ffmpeg -i tagged.flac -c:a aac tagged.m4a
ffmpeg -i tagged.flac -c:a libopus tagged.opus
```

The M4A uses the default trailing `moov` placement. Tests insert a sparse `free` atom before it to check seeking without reading the intervening audio-sized payload. The FLAC picture test inserts a generated PNG into a picture metadata block.
