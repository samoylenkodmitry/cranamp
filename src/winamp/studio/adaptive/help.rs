use super::*;

struct Topic {
    title: &'static str,
    steps: &'static [&'static str],
    note: &'static str,
}

const TOPICS: &[Topic] = &[
    Topic {
        title: "Open a skin and start drawing",
        steps: &[
            "Go back to the project screen and choose Open skin or project. Select a WSZ, ZIP or CSTUDIO file.",
            "Choose Joined canvas to edit the assembled skin. Choose a window if you want a closer view.",
            "Choose Pencil, set Size and Opacity, then choose a colour. Draw on the canvas. Undo removes the last stroke.",
        ],
        note: "A CSTUDIO project keeps your paint layers. A WSZ contains the finished skin.",
    },
    Topic {
        title: "Change what a stroke affects",
        steps: &[
            "Choose the window or sprite that you want to paint. A selected sprite limits the destination of new paint.",
            "Open Canvas state. Active and Released choose the appearance that you see.",
            "Under Paint into, choose This state to change only the displayed version, or All states to change every version.",
        ],
        note: "Viewing a pressed button and painting every button state are separate choices. Check both before a stroke.",
    },
    Topic {
        title: "Copy, flip and stamp pixels",
        steps: &[
            "Choose Lift pixels from Drawing tools. Drag a rectangle around the pixels to copy. The original pixels stay in place.",
            "Choose Use copied pixels to switch to Stamp. Its properties include horizontal flip, vertical flip and rotation.",
            "Drag to place the copy. Repeat places several copies along that drag. Undo removes the last placement.",
        ],
        note: "Stamp needs a captured rectangle. A rectangle may be up to 512 pixels wide and high.",
    },
    Topic {
        title: "Put text on the artwork",
        steps: &[
            "Choose Text from Drawing tools. Enter the words in Text to paint and set Text size.",
            "Open Tool settings to choose the font and spacing. Choose the ink colour with Brush colour.",
            "Drag on the canvas to position the text, then release to keep it. Undo removes that placement; changing the input affects the next placement.",
        ],
        note: "Text becomes painted pixels. Save a project and keep text on its own paint layer if you want to replace it later.",
    },
    Topic {
        title: "Choose an exact colour",
        steps: &[
            "Open Brush colour, or use the colour editor beside the canvas on a wide window.",
            "Enter six HEX digits such as #4a80c2, or enter Red, Green and Blue values from 0 to 255.",
            "Use Save colour to add the current colour. To update an existing swatch, select it, choose the new colour, then use Replace swatch.",
        ],
        note: "Incomplete or invalid input leaves the paint colour unchanged. Saved colours are stored in CSTUDIO projects.",
    },
    Topic {
        title: "Work with paint layers",
        steps: &[
            "Open Painting layers and add a layer before drawing. Select a row to make it the destination for new paint.",
            "Use the eye to hide or show a layer. Use the lock to stop changes to that layer.",
            "Select a layer to show its name, opacity and order controls at the top of the panel. Merge and Delete affect that layer; Undo restores the previous result.",
        ],
        note: "Source artwork edits the original sprites directly. Paint layers keep new work separate until export.",
    },
    Topic {
        title: "Move around without painting",
        steps: &[
            "Choose Pan canvas to move with one finger, a pen or a mouse. Select a drawing tool to paint again.",
            "Use two fingers to pan or pinch to zoom. A second finger cancels the tentative one-finger stroke.",
            "Use Fit canvas to see the whole image. Zoom buttons and scrollbars work outside the painting area.",
        ],
        note: "A mouse wheel or trackpad scroll moves the canvas. Pen hover does not draw.",
    },
    Topic {
        title: "Find why a stroke changes nothing",
        steps: &[
            "Check the selected paint layer. A locked layer rejects painting; a hidden layer can contain changes that you cannot see.",
            "Check the selected sprite and Canvas state. Your stroke may be outside that sprite or on a different version.",
            "Check Tool settings for Lock transparency and Paint only this colour. In Project actions, open Sprite rectangles and clear an unwanted paint clip.",
        ],
        note: "The message below the canvas reports an error. Use Undo if a stroke changed an unexpected destination.",
    },
    Topic {
        title: "Save work or use it in the player",
        steps: &[
            "Open Save and share. Choose Save project to keep paint layers and editable sources in a CSTUDIO file.",
            "Choose Export WSZ skin when you need a skin for a player. Export combines the visible artwork.",
            "Choose Apply to player to use the current skin in Cranamp. Save a project as well if you want to continue editing later.",
        ],
        note: "Save draft keeps a recovery copy on this device. Exporting a WSZ does not replace a saved project.",
    },
];

#[composable]
pub(super) fn HelpScreen(ui: Ui, layout: Layout) {
    let selected = cranpose_core::rememberMutableStateOf(|| None::<usize>);
    let wide = layout.width >= 700.;
    let list_w = if wide { 260. } else { layout.width - 24. };
    let current = selected.get().or(wide.then_some(0));
    if wide || current.is_none() {
        ScrollArea(
            Area::new(12., 72., list_w, layout.height - 108.),
            TOPICS.len() as f32 * 64.,
            move || {
                for (i, topic) in TOPICS.iter().enumerate() {
                    desktop::ListRow(
                        None,
                        topic.title.into(),
                        String::new(),
                        String::new(),
                        Area::new(0., i as f32 * 64., list_w, 60.),
                        current == Some(i),
                        ui.hints,
                        move || selected.set(Some(i)),
                    );
                }
            },
        );
    }
    let Some(index) = current.or(wide.then_some(0)) else {
        return;
    };
    let x = if wide { 292. } else { 20. };
    let width = (layout.width - x - 24.).min(680.);
    let scroll = cranpose_ui::rememberScrollState!(0.);
    cranpose_ui::Column(
        Modifier::empty()
            .absolute_offset(x, 72.)
            .size_points(width, layout.height - 108.)
            .clip_to_bounds()
            .vertical_scroll(scroll, false),
        cranpose_ui::ColumnSpec::default(),
        move || {
            if !wide {
                Button(
                    Modifier::empty()
                        .width(width)
                        .height(44.)
                        .content_description("All help topics")
                        .then(controls::feedback("All help topics".into())),
                    ButtonSpec::default(),
                    move || selected.set(None),
                    || {
                        Text(
                            "All help topics",
                            Modifier::empty(),
                            text_style(14., studio_accent_lit()),
                        );
                    },
                );
            }
            Text(
                TOPICS[index].title,
                Modifier::empty().width(width).padding_symmetric(0., 12.),
                text_style(20., studio_fg()),
            );
            for (i, step) in TOPICS[index].steps.iter().enumerate() {
                Text(
                    format!("{}. {step}", i + 1),
                    Modifier::empty().width(width).padding_symmetric(0., 10.),
                    text_style(15., studio_fg()),
                );
            }
            Text(
                TOPICS[index].note,
                Modifier::empty().width(width).padding_symmetric(0., 16.),
                text_style(14., studio_dim()),
            );
        },
    );
}
