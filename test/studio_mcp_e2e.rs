#![cfg(not(any(target_arch = "wasm32", target_os = "android", target_os = "ios")))]
mod common;
use common::{pump, visible_texts, HitGraphRenderer};
use cranpose_app_shell::AppShell;
use std::time::{Duration, Instant};

fn click(shell: &mut AppShell<HitGraphRenderer>, label: &str) {
    fn find(node: &cranpose_ui::SemanticsNode, label: &str) -> Option<(f32, f32)> {
        if node.hidden {
            return None;
        }
        if node.description.as_deref() == Some(label) && node.enabled {
            return Some((
                node.bounds.x + node.bounds.width / 2.,
                node.bounds.y + node.bounds.height / 2.,
            ));
        }
        node.children.iter().find_map(|child| find(child, label))
    }
    visible_texts(shell);
    let (x, y) = find(shell.semantics_tree().unwrap().root(), label).expect("enabled control");
    shell.set_cursor(x, y);
    shell.pointer_pressed();
    pump(shell);
    shell.pointer_released();
    pump(shell);
}

fn wait_for(shell: &mut AppShell<HitGraphRenderer>, text: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        pump(shell);
        let labels = visible_texts(shell);
        if labels.iter().any(|label| label.contains(text)) {
            return;
        }
        assert!(Instant::now() < deadline, "Missing {text}: {labels:?}");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn desktop_connection_controls_refresh_and_closing_studio_disconnects_agents() {
    let document = cranamp::winamp::studio::open_document(None).unwrap();
    document
        .lock()
        .unwrap()
        .state(serde_json::json!({"drawer":"agent"}))
        .unwrap();
    let root = document.clone();
    let mut shell = AppShell::new(
        HitGraphRenderer::default(),
        cranpose_core::location_key(file!(), line!(), column!()),
        move || cranamp::winamp::studio::SkinStudio(root.clone(), None),
    );
    shell.set_buffer_size(1440, 900);
    shell.set_viewport(1440., 900.);
    wait_for(&mut shell, "Server off");
    let history = document.lock().unwrap().history();
    let notice = document.lock().unwrap().notice.clone();
    click(&mut shell, "Start server");
    wait_for(&mut shell, "Server running");
    click(&mut shell, "Test connection");
    wait_for(&mut shell, "Connection verified");
    assert!(std::net::TcpStream::connect("127.0.0.1:18765").is_ok());
    assert_eq!(document.lock().unwrap().history(), history);
    assert_eq!(document.lock().unwrap().notice, notice);
    click(&mut shell, "Stop server");
    wait_for(&mut shell, "Server off");
    assert!(std::net::TcpStream::connect("127.0.0.1:18765").is_err());
    click(&mut shell, "Start server");
    wait_for(&mut shell, "Server running");
    drop(shell);
    let deadline = Instant::now() + Duration::from_secs(5);
    while std::net::TcpStream::connect("127.0.0.1:18765").is_ok() {
        assert!(
            Instant::now() < deadline,
            "Closing Studio must stop the server"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
