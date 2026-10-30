# Library albums and artwork

Audio plays from its original URI. Folder discovery publishes the first track before the rest of the tree is known. A separate coroutine indexes metadata through Cranpose's blocking dispatcher. It does not write audio files or sidecars.

Embedded title, artist, album, album artist, track number and disc number take priority. Missing titles and artists use the filename. A missing album uses the containing folder; `Disc 1`, `Disk 2` and `CD1` folders share their parent album. Album groups retain the source folder identity so unrelated releases with the same name remain separate.

A valid embedded picture takes priority over folder artwork. Front-cover pictures take priority over other embedded pictures. Folder pictures are selected deterministically: `cover`, `folder`, `front` or `album`, other names containing `front` or `cover`, other images, then back covers and booklet images. Names and extensions are case insensitive. JPEG, PNG, WebP and BMP are supported. Newly discovered covers update existing tracks. Track actions continue to use original playlist indices and source URIs.

The Albums tab opens each album's tracks in disc and track order. Search includes embedded metadata and inferred album names. Indexing never rewrites the player's playlist titles or source locations.

Long Library lists have a scrollbar on the right. Dragging its thumb jumps through tracks, albums or saved playlists while rows remain lazy. The thumb follows normal swipe scrolling and supports accessibility progress actions. Its touch area is 44 points wide and stays clear of row actions and the player dock. Short lists hide the scrollbar. Long playlist chooser sheets have their own scrollbar.

Native editions store `index.json` and SHA-256-addressed PNG thumbnails under the application's `cache/library-v1` directory. Identical pictures share one thumbnail. Thumbnail dimensions fit within 320 by 320 pixels without stretching. Encoded source pictures are limited to 8 MiB and decoded pictures to 4096 pixels per side and 64 MiB. The metadata reader has a 12 MiB read budget per audio file. HTTP reads use validated 64 KiB ranges with an eight-second timeout per request. Metadata parsing does not decode audio packets.

Known file size and modification time invalidate cached records. Sources without a modification time are rechecked in the background when reopened. The index appears immediately after startup. Restored local and WebDAV playlists also check their containing folders for cover images. Corrupt caches are ignored. Missing thumbnails are rebuilt from the original source. Orphan thumbnails are removed after indexing; the native thumbnail cache is limited to 256 MiB. The index is limited to 100,000 records and 32 MiB. Cache eviction does not remove music.

The browser keeps its metadata and thumbnails in site storage. Its provider API offers sequential reads, so metadata reads stop after 8 MiB; metadata beyond that limit falls back to filenames. Native sources support seeking, including Android document descriptors and HTTP ranges.

Recognition uses the selected files and folders. There is no online album lookup.

## Saved playlists

The mobile Library has Tracks, Albums and Playlists in its Liquid Glass segmented control. Use the plus button in Playlists to create a named list, or use **Save current queue** in Library options to preserve the current order. A track's menu and the selection toolbar can add music to a saved list. Playlist details provide a searchable track picker, Play and Shuffle buttons, and track menus for moving or removing entries. Playlist options include rename, duplicate and delete. Deletion requires confirmation.

Play and Shuffle replace the current playing queue. Editing a saved list leaves current playback unchanged. The track picker retains references to music already added to the app, so switching queues does not lose the other available tracks. Saved lists and their order survive restarts. Artwork uses the same track and album fallback as the rest of the Library.

Named playlists live in the `cranamp.playlists.v1` application preference, separate from the metadata cache. It contains source URIs and display metadata, with a 32 MiB serialized limit. Reads and writes use Cranpose's blocking dispatcher. One writer saves changes in order; a failed save remains visible with a retry control. Unreadable saved data is preserved. Creating, duplicating or deleting a playlist never copies or deletes an audio file.
