use super::*;

fn command(ui: &Ui, args: serde_json::Value) {
    let mut d = ui.shared.lock().unwrap();
    d.message = match d.paint_layer_command(&args, "Human") {
        Ok(_) => match args["action"].as_str().unwrap_or("") {
            "add" => "Paint layer added",
            "select" => "Paint destination changed",
            "move" => "Layer order changed",
            "delete" => "Layer deleted. Undo restores it.",
            "merge_down" => "Layer merged. Undo separates it again.",
            "set" if args.get("name").is_some() => "Layer renamed",
            "set" if args.get("opacity").is_some() => "Layer opacity changed",
            "set" if args.get("visible").is_some() => "Layer visibility changed",
            "set" if args.get("locked").is_some() => "Layer lock changed",
            _ => "Layer updated",
        }
        .into(),
        Err(error) => {
            let message = error.to_string();
            d.notify(message.clone());
            message
        }
    };
    d.revision += 1;
    ui.tick.set(d.revision);
}

#[composable]
pub(super) fn Destination(ui: Ui, area: Area) {
    let info = ui.shared.lock().unwrap().paint_layer_info();
    let name = info["layers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == info["active"])
        .and_then(|p| p["name"].as_str())
        .unwrap_or("Source artwork")
        .to_owned();
    let next = ui.clone();
    Control(
        Icon::Layers,
        "Painting layers".into(),
        "Choose where new paint goes".into(),
        Area::new(area.x + area.w - 44., area.y, 36., area.h),
        false,
        true,
        false,
        ui.hints,
        move || next.open_sheet(Sheet::Layers),
    );
    desktop::RowLabel(
        format!("Paint: {name}"),
        area.x + 12.,
        area.y + 10.,
        area.w - 72.,
        12.,
        studio_fg(),
    );
}

pub(super) fn height(info: &serde_json::Value) -> f32 {
    let planes = info["layers"].as_array();
    let selected =
        planes.is_some_and(|planes| planes.iter().any(|plane| plane["id"] == info["active"]));
    (if selected { 252. } else { 64. }) + 100. + planes.map_or(0, Vec::len) as f32 * 48.
}

#[composable]
pub(super) fn Panel(ui: Ui, area: Area) {
    let _ = ui.tick.get();
    let info = ui.shared.lock().unwrap().paint_layer_info();
    let planes = info["layers"].as_array().cloned().unwrap_or_default();
    let active = info["active"].as_str().map(str::to_owned);
    let selected = planes
        .iter()
        .position(|p| p["id"].as_str() == active.as_deref());
    let details_h = if selected.is_some() { 252. } else { 64. };
    let count = planes.len();
    ScrollArea(area, height(&info), move || {
        if let Some(index) = selected {
            let layer = &planes[index];
            let id = active.clone().unwrap();
            let name = layer["name"].as_str().unwrap_or("Paint layer").to_owned();
            let field = cranpose_core::remember(|| TextFieldState::new(name.clone())).with(|f| *f);
            cranpose_core::LaunchedEffect((id.clone(), name.clone()), move |_| {
                field.set_text(name);
            });
            Label("Name".into(), 12., 4., area.w - 24., 11., studio_dim());
            cranpose_ui::BasicTextField(
                field,
                Modifier::empty()
                    .absolute_offset(12., 24.)
                    .size_points(area.w - 72., 40.)
                    .then(design::surface(design::Surface::Field, false, false, false))
                    .content_description("Layer name")
                    .then(controls::feedback(String::new()))
                    .padding_symmetric(8., 10.),
                text_style(13., studio_fg()),
            );
            let next = ui.clone();
            let key = id.clone();
            Control(
                Icon::Check,
                "Rename layer".into(),
                "Apply the name in the field".into(),
                Area::new(area.w - 56., 22., 44., 44.),
                false,
                true,
                false,
                ui.hints,
                move || command(&next, json!({"action":"set","id":key,"name":field.text()})),
            );
            let opacity = layer["opacity"].as_u64().unwrap_or(255) as u8;
            Label(
                "Layer opacity (%)".into(),
                12.,
                84.,
                area.w - 96.,
                12.,
                studio_fg(),
            );
            let next = ui.clone();
            let key = id.clone();
            colour::ExactField(
                "Layer opacity value",
                ((opacity as f32 * 100. / 255.).round() as u32).to_string(),
                Area::new(area.w - 72., 72., 60., 40.),
                move |value| {
                    let Ok(value) = value.trim().parse::<u32>() else {
                        return false;
                    };
                    if value > 100 {
                        return false;
                    }
                    command(
                        &next,
                        json!({"action":"set","id":key,"opacity":(value as f32 * 255. / 100.).round() as u8}),
                    );
                    true
                },
            );
            let next = ui.clone();
            let key = id.clone();
            let end = ui.clone();
            cranpose_ui::Slider(
                Modifier::empty()
                    .absolute_offset(12., 116.)
                    .size_points(area.w - 24., 32.)
                    .content_description("Layer opacity")
                    .then(controls::feedback(String::new())),
                opacity as f32 / 255.,
                move |v| {
                    let mut d = next.shared.lock().unwrap();
                    let _ = d.opacity_stroke(&key, (v * 255.).round() as u8);
                    next.tick.set(d.revision);
                },
                move || {
                    end.shared.lock().unwrap().finish_opacity_stroke();
                    end.tick.set(end.shared.lock().unwrap().revision);
                },
                cranpose_ui::SliderSpec::default().thumb_extent(16.),
                move |scope| {
                    design::SliderTrack(area.w - 24., 32., scope.thumb_offset());
                },
            );
            for (i, (icon, label, action, enabled)) in [
                (Icon::Up, "Move layer up", "up", index + 1 < count),
                (Icon::Down, "Move layer down", "down", index > 0),
                (
                    Icon::Merge,
                    "Merge layer down",
                    "merge_down",
                    layer["visible"] == true,
                ),
                (Icon::Delete, "Delete layer", "delete", true),
            ]
            .into_iter()
            .enumerate()
            {
                let next = ui.clone();
                let key = id.clone();
                Control(
                    icon,
                    label.into(),
                    match action {
                        "up" => "Draw this layer above the next layer",
                        "down" => "Draw this layer below the previous layer",
                        "merge_down" => "Combine this layer with the layer below",
                        _ => "Undo restores this layer",
                    }
                    .into(),
                    Area::new(12. + i as f32 * (area.w - 24.) / 4., 156., 44., 44.),
                    false,
                    enabled,
                    false,
                    ui.hints,
                    move || {
                        command(
                            &next,
                            match action {
                                "up" => json!({"action":"move","id":key,"index":index+1}),
                                "down" => {
                                    json!({"action":"move","id":key,"index":index.saturating_sub(1)})
                                }
                                _ => json!({"action":action,"id":key}),
                            },
                        )
                    },
                );
            }
            let clipped = layer["clip_below"] == true;
            let next = ui.clone();
            desktop::ListRow(
                None,
                "Clip to layer below".into(),
                "Keep paint inside the visible pixels of the layer below".into(),
                if clipped { "On" } else { "Off" }.into(),
                Area::new(4., 204., area.w - 8., 44.),
                clipped,
                ui.hints,
                move || command(&next, json!({"action":"set","id":id,"clip_below":!clipped})),
            );
        } else {
            Label(
                "Source artwork".into(),
                12.,
                8.,
                area.w - 24.,
                14.,
                studio_fg(),
            );
            Label(
                "Add a layer to keep new paint separate.".into(),
                12.,
                32.,
                area.w - 24.,
                12.,
                studio_dim(),
            );
        }
        Label(
            "Layers, top first".into(),
            12.,
            details_h + 14.,
            area.w - 80.,
            12.,
            studio_dim(),
        );
        let next = ui.clone();
        Control(
            Icon::Add,
            "Add layer".into(),
            "Create a layer above the artwork".into(),
            Area::new(area.w - 56., details_h, 44., 44.),
            false,
            true,
            false,
            ui.hints,
            move || command(&next, json!({"action":"add"})),
        );
        for (row, layer) in planes.iter().rev().enumerate() {
            let id = layer["id"].as_str().unwrap().to_owned();
            let name = layer["name"].as_str().unwrap_or("Paint layer").to_owned();
            let y = details_h + 48. + row as f32 * 48.;
            let next = ui.clone();
            let key = id.clone();
            previews::LayerRow(
                ui.clone(),
                id.clone(),
                name.clone(),
                Area::new(4., y, area.w - 100., 48.),
                active.as_deref() == Some(&id),
                move || command(&next, json!({"action":"select","id":key})),
            );
            for (i, (icon, prop, on)) in [
                (Icon::Eye, "visible", layer["visible"] == true),
                (Icon::Lock, "locked", layer["locked"] == true),
            ]
            .into_iter()
            .enumerate()
            {
                let next = ui.clone();
                let key = id.clone();
                let label = if prop == "visible" {
                    if on {
                        "Hide"
                    } else {
                        "Show"
                    }
                } else if on {
                    "Unlock"
                } else {
                    "Lock"
                };
                Control(
                    icon,
                    format!("{label} {name}"),
                    String::new(),
                    Area::new(area.w - 96. + i as f32 * 44., y, 44., 44.),
                    on,
                    true,
                    false,
                    ui.hints,
                    move || command(&next, json!({"action":"set","id":key,prop:!on})),
                );
            }
        }
        let next = ui.clone();
        previews::LayerRow(
            ui.clone(),
            "base".into(),
            "Source artwork".into(),
            Area::new(4., details_h + 48. + count as f32 * 48., area.w - 8., 44.),
            active.is_none(),
            move || command(&next, json!({"action":"select","id":"base"})),
        );
    });
}
