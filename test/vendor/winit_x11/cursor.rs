use super::*;

/// Run with `xvfb-run -a cargo test -p winit-x11 --lib -- --include-ignored`.
/// A checked X11 request detects a freed handle.
#[test]
#[ignore = "requires a disposable X11 display"]
fn cloned_cursor_keeps_the_server_resource_alive() {
    let connection = Arc::new(XConnection::new(None).expect("X11 display"));
    let cursor = connection
        .create_cursor_from_image(2, 2, 0, 0, &[255; 16])
        .expect("cursor image");
    let original = CustomCursor {
        resource: Arc::new(CursorResource {
            xconn: connection.clone(),
            cursor,
        }),
    };
    let held_by_window = original.clone();
    drop(held_by_window);
    // Freeing one clone must not invalidate the cached image.
    connection
        .xcb_connection()
        .change_window_attributes(
            connection.default_root().root,
            &xproto::ChangeWindowAttributesAux::new().cursor(cursor),
        )
        .unwrap()
        .check()
        .expect("cached cursor remains usable");
    drop(original);
    assert!(
        connection
            .xcb_connection()
            .change_window_attributes(
                connection.default_root().root,
                &xproto::ChangeWindowAttributesAux::new().cursor(cursor)
            )
            .unwrap()
            .check()
            .is_err(),
        "the last owner frees the cursor"
    );
}
