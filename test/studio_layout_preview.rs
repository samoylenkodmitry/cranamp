//! Native GPU review at a requested logical size. Run only with studio-review.
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--design") {
        let width = std::env::args()
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1280);
        let height = std::env::args()
            .nth(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(860);
        cranamp::create_surface_app()
            .with_title("Studio controls")
            .with_developer_inspector(false)
            .with_size(width, height)
            .run(move || cranamp::winamp::studio::DesignSystem(width as f32, height as f32));
    }
    if std::env::args().nth(1).as_deref() == Some("--list") {
        let names: Vec<_> = cranamp::winamp::studio::review_cases()
            .into_iter()
            .map(|case| case.name)
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&names).expect("case names")
        );
        return;
    }
    let mut args = std::env::args().skip(1);
    let width = args
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(430);
    let height = args
        .next()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(820);
    let start = args.next().and_then(|s| s.parse::<usize>().ok());
    if let Some(start) = start {
        cranpose_services::set_platform_accessibility_options(
            cranpose_services::AccessibilityOptions {
                reduce_motion: true,
                ..Default::default()
            },
        );
        cranamp::create_surface_app()
            .with_title("Cranamp · Control review")
            .with_developer_inspector(false)
            .with_size(width, height + 40)
            .run(move || {
                cranamp::winamp::studio::ReviewGallery(width as f32, height as f32, start)
            });
    }
    let doc = cranamp::winamp::studio::open_document(None).expect("bundled skin");
    doc.lock()
        .unwrap()
        .state(serde_json::json!({"drawer":"none","panel":"canvas","zoom":1}))
        .unwrap();
    cranamp::create_surface_app()
        .with_title("Cranamp · Layout review")
        .with_developer_inspector(false)
        .with_size(width, height)
        .run(move || cranamp::winamp::studio::SkinStudio(doc.clone(), None));
}
