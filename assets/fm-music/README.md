# Cranamp FM

A streaming set Cranamp can load without touching local files. The playlist in
this folder, `cranamp-fm-playlist.m3u`, is compiled into the binary and reached
from the playlist window's **LIST -> CRANAMP FM** entry; the tracks themselves
are streamed from <https://fm.dmitrysamoylenko.in> and are never bundled.

This is the counterpart to `assets/demo-music/`, which ships local files for
offline use. Keep the audio out of this folder: only the playlist belongs here.

## Tracks

The set was generated with YuE2-3B and is licensed CC-BY-NC-4.0, so it is for
personal, non-commercial listening. Attribution for every track is carried in
the `#EXTINF` artist field.

## Regenerating

The playlist is plain extended M3U with absolute `https://` URLs and real
durations. To add a track, upload the `.mp3` to the host and append an
`#EXTINF:<seconds>,<artist> - <title>` line plus its URL; `ffprobe -v error
-show_entries format=duration -of csv=p=0 <file>` gives the duration.
