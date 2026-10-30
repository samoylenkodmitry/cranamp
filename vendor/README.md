# X11 cursor ownership patch

`winit-x11` is the crates.io source of version `0.31.0-beta.3`, under its
original Apache-2.0 license. Cranamp pins the other winit crates through Cranpose.

The upstream `CustomCursor` clones the raw X11 cursor handle. Each clone frees
that handle when dropped. A window drops its previous clone whenever the cursor
changes, which invalidates the image retained in Cranpose's cache. Hovering back
over that region then leaves the previous cursor visible.

The patch in `src/util/cursor.rs` shares ownership of the X11 resource through
`Arc` and frees it when the last owner drops. No public API changes. Remove this
patch after the pinned winit release contains the ownership fix. The upstream
unit tests are moved to `/test/vendor/winit_x11/` to follow this repository's
test placement rule.

On X11, `test/linux_cursor_probe.py` reads the current cursor dimensions and
pixel hash without moving the pointer. Hover the player body, seek bar and title
bar, then revisit them. Each region must keep its own hash on repeated visits.
Run the server resource regression in a disposable X11 session with
`xvfb-run -a cargo test -p winit-x11 --lib -- --include-ignored`.
