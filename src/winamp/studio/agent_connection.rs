use super::*;

#[composable]
pub fn AgentConnection(shared: SharedDocument, _revision: u64, room: (f32, f32)) {
    let connection = agent_server::status();
    let clipboard = cranpose_ui::clipboard_session::local_clipboard().current();
    let scroll = cranpose_ui::rememberScrollState!(0.0);
    cranpose_ui::Column(
        Modifier::empty()
            .size_points(room.0 + 24., room.1)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            let shared = shared.clone();
            let connection = connection.clone();
            Box(
                Modifier::empty().size_points(room.0 + 24., 560.),
                BoxSpec::default(),
                move || {
                    let width = room.0;
                    Label("AGENT CONNECTION".into(), 12., 17., width - 86., 14., FG);
                    Label("Let a local agent edit this canvas through MCP.\nIts changes share your undo history.".into(), 12., 58., width, 12., DIM);
                    Label(
                        if connection.stopping {
                            "Stopping…"
                        } else if connection.running {
                            "Running · this computer only"
                        } else {
                            "Stopped"
                        }
                        .into(),
                        12.,
                        112.,
                        width,
                        14.,
                        if connection.running { ACCENT_LIT } else { FG },
                    );
                    Label(agent_server::ENDPOINT.into(), 12., 148., width, 12., FG);
                    let d = shared.clone();
                    MaybeAction(
                        if connection.running {
                            "Stop server"
                        } else {
                            "Start server"
                        }
                        .into(),
                        12.,
                        184.,
                        132.,
                        !connection.stopping,
                        move || {
                            if agent_server::status().running {
                                agent_server::stop_for(&d);
                            } else {
                                let _ = agent_server::start(d.clone());
                            }
                        },
                    );
                    let d = shared.clone();
                    MaybeAction(
                        if connection.testing {
                            "Testing…"
                        } else {
                            "Test connection"
                        }
                        .into(),
                        152.,
                        184.,
                        (width - 140.).min(160.),
                        connection.running && !connection.testing,
                        move || agent_server::test_connection(d.clone()),
                    );
                    let d = shared.clone();
                    MaybeAction(
                        "Copy connection details".into(),
                        12.,
                        224.,
                        width.min(236.),
                        clipboard.has_system_clipboard(),
                        move || {
                            clipboard.set_text(&format!("Cranamp Skin Studio\nMCP URL: {}\nTransport: HTTP (local computer only)\nStdio: cranamp --skin-studio-mcp\nKeep Skin Studio open and its agent server running.\n", agent_server::ENDPOINT));
                            note(&d, "Agent connection details copied".into());
                        },
                    );
                    Label(connection.message.clone(), 12., 274., width, 12., FG);
                    Label("Test connection checks the server identity and\navailable tools. It does not change your skin.".into(), 12., 354., width, 12., DIM);
                    Label("ACCESS\nLocal programs can read and edit the open skin\nand use files this app can access. Stop the server\nto disconnect agents. Closing Studio stops it too.\n\nThe server is unavailable on mobile and web.".into(), 12., 414., width, 11., DIM);
                },
            );
        },
    );
}
