# Library albums and artwork

Audio plays from its original URI. Folder discovery publishes the first track before the rest of the tree is known. A separate coroutine indexes metadata through Cranpose's blocking dispatcher. It does not write audio files or sidecars.

Embedded title, artist, album, album artist, track number and disc number take priority. Missing titles and artists use the filename. A missing album uses the containing folder; `Disc 1`, `Disk 2` and `CD1` folders share their parent album. Album groups retain the source folder identity so unrelated releases with the same name remain separate.

A valid embedded picture takes priority over folder artwork. Front-cover pictures take priority over other embedded pictures. Folder pictures are selected deterministically: `cover`, `folder`, `front` or `album`, other names containing `front` or `cover`, other images, then back covers and booklet images. Names and extensions are case insensitive. JPEG, PNG, WebP and BMP are supported. Newly discovered covers update existing tracks. Track actions continue to use original playlist indices and source URIs.

The Albums tab opens each album's tracks in disc and track order. Search includes embedded metadata and inferred album names. Indexing never rewrites the player's playlist titles or source locations.

Native editions store `index.json` and SHA-256-addressed PNG thumbnails under the application's `cache/library-v1` directory. Identical pictures share one thumbnail. Thumbnail dimensions fit within 320 by 320 pixels without stretching. Encoded source pictures are limited to 8 MiB and decoded pictures to 4096 pixels per side and 64 MiB. The metadata reader has a 12 MiB read budget per audio file. HTTP reads use validated 64 KiB ranges with an eight-second timeout per request. Metadata parsing does not decode audio packets.

Known file size and modification time invalidate cached records. Sources without a modification time are rechecked in the background when reopened. The index appears immediately after startup. Restored local and WebDAV playlists also check their containing folders for cover images. Corrupt caches are ignored. Missing thumbnails are rebuilt from the original source. Orphan thumbnails are removed after indexing; the native thumbnail cache is limited to 256 MiB. The index is limited to 100,000 records and 32 MiB. Cache eviction does not remove music.

The browser keeps its metadata and thumbnails in site storage. Its provider API offers sequential reads, so metadata reads stop after 8 MiB; metadata beyond that limit falls back to filenames. Native sources support seeking, including Android document descriptors and HTTP ranges.

Recognition uses the selected files and folders. There is no online album lookup.
