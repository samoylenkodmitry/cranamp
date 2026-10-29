use super::*;

#[composable]
pub(super) fn Panel(ui: Ui, area: Area) {
    let revision = ui.tick.get();
    let connection = agent_server::status();
    let clipboard = cranpose_ui::clipboard_session::local_clipboard().current();
    let copied = cranpose_core::rememberMutableStateOf(|| 0u8);
    let fallback = cranpose_core::rememberMutableStateOf(|| false);
    let copy_id = copied.get();
    cranpose_core::LaunchedEffectAsync(copy_id, move |_| {
        Box::pin(async move {
            if copy_id != 0 {
                cranpose_core::delay(Duration::from_millis(1800)).await;
                copied.set(0);
            }
        })
    });
    let expanded = fallback.get();
    let width = area.w - 32.;
    let access_y = if expanded { 436. } else { 344. };
    let shared = ui.shared.clone();
    let preview =
        cranpose_core::rememberKeyed((shared.clone(), revision, area.h >= 620.), move |_| {
            if area.h < 620. {
                return None;
            }
            let doc = shared.lock().unwrap();
            let name = doc
                .path
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_stem())
                .and_then(|name| name.to_str())
                .unwrap_or("Working copy")
                .to_owned();
            Some((previews::bitmap(doc.render()), name, doc.history_depth().0))
        });
    let preview_y = access_y + 148.;
    ScrollArea(
        area,
        access_y + 144. + if preview.is_some() { 152. } else { 0. },
        move || {
            Label(
                "Your agent can read this skin and local files.\nIts provider may receive the data it reads.".into(),
                16.,
                4.,
                width,
                13.,
                DIM,
            );
            let status = connection.clone();
            let actions = ui.clone();
            Box(
                Modifier::empty()
                    .absolute_offset(16., 56.)
                    .size_points(width, 136.)
                    .then(design::surface(design::Surface::Well, false, false, false)),
                BoxSpec::default(),
                move || {
                    let title = if status.stopping {
                        "Stopping server"
                    } else if status.running {
                        "Server running"
                    } else {
                        "Server off"
                    };
                    let tint = if status.running { ACCENT_LIT } else { DIM };
                    cranpose_ui::Canvas(
                        Modifier::empty()
                            .absolute_offset(14., 18.)
                            .size_points(24., 24.),
                        move |scope| controls::draw_icon(scope, Icon::Agent, tint),
                    );
                    desktop::RowLabel(title.into(), 50., 14., width - 118., 15., FG);
                    desktop::RowLabel(
                        if status.running {
                            "This computer only"
                        } else {
                            "No agent access"
                        }
                        .into(),
                        50.,
                        36.,
                        width - 118.,
                        11.,
                        DIM,
                    );
                    let shared = actions.shared.clone();
                    Control(
                        Icon::Power,
                        if status.running {
                            "Stop server"
                        } else {
                            "Start server"
                        }
                        .into(),
                        if status.running {
                            "Disconnect local agents"
                        } else {
                            "Allow local agents to edit this skin"
                        }
                        .into(),
                        Area::new(width - 54., 8., 44., 44.),
                        status.running,
                        !status.stopping,
                        false,
                        actions.hints,
                        move || {
                            if agent_server::status().running {
                                agent_server::stop_for(&shared);
                            } else {
                                let _ = agent_server::start(shared.clone());
                            }
                        },
                    );
                    let result = if status.testing {
                        "Checking connection...".into()
                    } else if status.error {
                        status.message.clone()
                    } else if let Some(count) = status.tools {
                        format!("Connection verified\n{count} tools available")
                    } else if status.running {
                        "Ready for your agent\nCheck the connection before you begin.".into()
                    } else {
                        "Start the server when you need it.\nIt stays off between sessions.".into()
                    };
                    cranpose_ui::BasicText(
                        result,
                        Modifier::empty()
                            .absolute_offset(14., 76.)
                            .width(width - 80.),
                        text_style(12., if status.error { design::ERROR } else { DIM }),
                        cranpose_ui::text::TextOverflow::Ellipsis,
                        true,
                        3,
                        1,
                    );
                    let shared = actions.shared.clone();
                    Control(
                        if status.tools.is_some() {
                            Icon::Check
                        } else {
                            Icon::Test
                        },
                        "Test connection".into(),
                        "Check the server identity and available tools".into(),
                        Area::new(width - 54., 76., 44., 44.),
                        status.tools.is_some(),
                        status.running && !status.testing && !status.stopping,
                        false,
                        actions.hints,
                        move || agent_server::test_connection(shared.clone()),
                    );
                },
            );
            desktop::RowLabel("HTTP server URL".into(), 16., 212., width, 12., FG);
            let url_clipboard = clipboard;
            Box(
                Modifier::empty()
                    .absolute_offset(16., 234.)
                    .size_points(width, 52.)
                    .then(design::surface(design::Surface::Field, false, false, false)),
                BoxSpec::default(),
                {
                    let hints = ui.hints;
                    move || {
                        desktop::RowLabel(
                            agent_server::ENDPOINT.into(),
                            12.,
                            18.,
                            width - 70.,
                            12.,
                            FG,
                        );
                        let clipboard = url_clipboard;
                        Control(
                            if copied.get() == 1 {
                                Icon::Check
                            } else {
                                Icon::Copy
                            },
                            "Copy server URL".into(),
                            "Paste into your agent's HTTP server settings".into(),
                            Area::new(width - 48., 4., 44., 44.),
                            false,
                            clipboard.has_system_clipboard() && connection.running,
                            false,
                            hints,
                            move || {
                                if let Some(url) = agent_server::connection_url() {
                                    clipboard.set_text(&url);
                                }
                                copied.set(1);
                            },
                        );
                    }
                },
            );
            Label(
                "The copied URL includes a secret for this session.\nShare it only with an agent you trust."
                    .into(),
                16.,
                296.,
                width,
                12.,
                DIM,
            );
            let hints = ui.hints;
            desktop::ListRow(
                Some(if expanded { Icon::Up } else { Icon::Down }),
                "Use a stdio connection".into(),
                "For agents that require a local command".into(),
                String::new(),
                Area::new(12., 332., width + 8., 40.),
                expanded,
                hints,
                move || fallback.set(!fallback.get_non_reactive()),
            );
            if expanded {
                Label("Keep the server running. Copy this command\ninto your agent's stdio server settings.".into(), 16., 382., width - 52., 12., DIM);
                Control(
                    if copied.get() == 2 {
                        Icon::Check
                    } else {
                        Icon::Copy
                    },
                    "Copy stdio command".into(),
                    "Copy the Cranamp executable path and --skin-studio-mcp".into(),
                    Area::new(area.w - 60., 376., 44., 44.),
                    false,
                    clipboard.has_system_clipboard() && connection.running,
                    false,
                    hints,
                    move || {
                        if let (Ok(path), Some(token)) =
                            (std::env::current_exe(), agent_server::connection_token())
                        {
                            clipboard.set_text(&format!(
                                "\"{}\" --skin-studio-mcp {token}",
                                path.display()
                            ));
                            copied.set(2);
                        }
                    },
                );
            }
            cranpose_ui::Canvas(
                Modifier::empty()
                    .absolute_offset(16., access_y + 48.)
                    .size_points(20., 20.),
                move |scope| controls::draw_icon(scope, Icon::Lock, DIM),
            );
            Label(
                "Local access".into(),
                46.,
                access_y + 48.,
                width - 30.,
                12.,
                FG,
            );
            if let Some((image, name, edits)) = &preview {
                previews::Divider(Area::new(16., preview_y, width, 1.));
                previews::Image(
                    image.clone(),
                    Area::new(20., preview_y + 16., 96., 120.),
                    "Shared document preview".into(),
                );
                Label(
                    "Shared document".into(),
                    132.,
                    preview_y + 24.,
                    area.w - 148.,
                    12.,
                    DIM,
                );
                desktop::RowLabel(name.clone(), 132., preview_y + 48., area.w - 148., 13., FG);
                Label(
                    format!("{edits} edits in Undo"),
                    132.,
                    preview_y + 74.,
                    area.w - 148.,
                    12.,
                    DIM,
                );
            }
            Label(
            "Agents with your session secret can use files\nCranamp can access. Stop revokes the secret.\nYour agent runs separately from Cranamp.".into(),
            16., access_y + 78., width, 12., DIM,
        );
        },
    );
}
