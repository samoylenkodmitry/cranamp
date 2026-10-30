//! Native GPU review at a requested logical size. Run only with studio-review.
fn main() {
    if std::env::args().nth(1).as_deref() == Some("--mobile") {
        let width = std::env::args()
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(393);
        let height = std::env::args()
            .nth(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(780);
        let launcher = cranamp::create_surface_app()
            .with_title("Cranamp · Mobile GPU review")
            .with_application_id("com.cranamp.mobile-review")
            .with_developer_inspector(false)
            .with_size(width, height);
        let insets = review_insets();
        let theme = std::env::var("CRANAMP_REVIEW_THEME").ok().map(|value| {
            if value == "dark" {
                cranpose_services::SystemTheme::Dark
            } else {
                cranpose_services::SystemTheme::Light
            }
        });
        with_capture(launcher, None).run(move || {
            let mut locals = vec![cranpose_ui::local_safe_area_insets().provides(insets)];
            if let Some(theme) = theme {
                locals.push(cranpose_services::local_system_theme().provides(theme));
            }
            cranpose_core::CompositionLocalProvider(locals, cranamp::winamp::WinampMobileApp);
        });
    }
    if std::env::args().nth(1).as_deref() == Some("--player") {
        let width = std::env::args()
            .nth(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1280);
        let height = std::env::args()
            .nth(3)
            .and_then(|s| s.parse().ok())
            .unwrap_or(800);
        let launcher = cranamp::create_surface_app()
            .with_title("Cranamp")
            .with_application_id("com.cranamp.store-capture")
            .with_developer_inspector(false)
            .with_size(width, height);
        with_capture(launcher, None).run(cranamp::winamp::WinampStackedApp);
        return;
    }
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
        let launcher = cranamp::create_surface_app()
            .with_title("Cranamp · Control review")
            .with_developer_inspector(false)
            .with_size(width, height + 40);
        with_capture(launcher, Some(height)).run(move || {
            cranamp::winamp::studio::ReviewGallery(width as f32, height as f32, start)
        });
    }
    let doc = cranamp::winamp::studio::open_document(None).expect("bundled skin");
    doc.lock()
        .unwrap()
        .state(serde_json::json!({"drawer":"none","panel":"canvas","zoom":1}))
        .unwrap();
    let launcher = cranamp::create_surface_app()
        .with_title("Cranamp · Layout review")
        .with_developer_inspector(false)
        .with_size(width, height);
    with_capture(launcher, None)
        .run(move || cranamp::winamp::studio::SkinStudio(doc.clone(), None));
}

fn review_insets() -> cranpose_ui::EdgeInsets {
    if std::env::var_os("CRANAMP_REVIEW_PHONE_INSETS").is_some() {
        cranpose_ui::EdgeInsets::from_components(0., 59., 0., 34.)
    } else {
        cranpose_ui::EdgeInsets::default()
    }
}

// Export actual GPU frames. When reviewing a case, omit only the separate
// review navigation footer below the product UI.
fn with_capture(
    launcher: cranpose::AppLauncher<cranpose::AppFonts>,
    product_height: Option<u32>,
) -> cranpose::AppLauncher<cranpose::AppFonts> {
    if let Some(path) = std::env::var_os("CRANAMP_REVIEW_CAPTURE") {
        let once = std::env::var_os("CRANAMP_REVIEW_CAPTURE_ONCE").is_some();
        let rgb = std::env::var_os("CRANAMP_REVIEW_CAPTURE_RGB").is_some();
        let settings = std::env::var_os("CRANAMP_REVIEW_OPEN_SETTINGS").is_some();
        let library = std::env::var_os("CRANAMP_REVIEW_OPEN_LIBRARY").is_some();
        let albums = std::env::var_os("CRANAMP_REVIEW_OPEN_ALBUMS").is_some();
        let studio = std::env::var_os("CRANAMP_REVIEW_OPEN_STUDIO").is_some();
        let insets = review_insets();
        let scale = std::env::var("CRANAMP_REVIEW_CAPTURE_SCALE")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .filter(|scale| scale.is_finite() && *scale > 0. && *scale <= 4.)
            .unwrap_or(1.);
        launcher.with_test_driver(move |robot| {
            std::thread::spawn(move || {
                if library || albums || studio || settings {
                    robot.wait_for_idle().expect("player ready");
                    let frame = robot.screenshot_with_scale(1.).expect("player frame");
                    let width = frame.width as f32;
                    let height = frame.height as f32;
                    let (tab, fraction) = if settings {
                        ("Settings", 0.875)
                    } else if studio {
                        ("Skin Studio", 0.625)
                    } else {
                        ("Library", 0.375)
                    };
                    robot
                        .click(12. + (width - 24.) * fraction, height - insets.bottom - 40.)
                        .expect("open destination");
                    robot.wait_for_idle().expect("destination ready");
                    robot.validate_content(tab).expect("destination visible");
                    if albums {
                        robot
                            .click(width * 0.75, insets.top + 147.)
                            .expect("open Albums");
                        robot.wait_for_idle().expect("Albums ready");
                    }
                }
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let Ok(shot) = robot.screenshot_with_scale(scale) else {
                        break;
                    };
                    if let Some(frame) =
                        image::RgbaImage::from_raw(shot.width, shot.height, shot.pixels)
                    {
                        let frame = if let Some(height) = product_height {
                            let height = (height as f32 * scale).round() as u32;
                            image::imageops::crop_imm(&frame, 0, 0, shot.width, height).to_image()
                        } else {
                            frame
                        };
                        let path = std::path::PathBuf::from(&path);
                        let pending = path.with_extension("pending.png");
                        let saved = if rgb {
                            assert!(
                                frame.pixels().all(|pixel| pixel[3] == 255),
                                "Store captures must be opaque"
                            );
                            image::DynamicImage::ImageRgba8(frame)
                                .into_rgb8()
                                .save(&pending)
                        } else {
                            frame.save(&pending)
                        };
                        if saved.is_ok() {
                            if let Err(error) = std::fs::rename(pending, path) {
                                eprintln!("Capture save failed: {error}");
                            } else if once {
                                cranpose_services::request_exit();
                                break;
                            }
                        }
                    }
                }
            });
        })
    } else {
        launcher
    }
}
