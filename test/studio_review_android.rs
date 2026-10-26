use super::*;

const MODE: &str = "/data/user/0/com.cranamp.app.debug/files/studio-review-mode";
const RESULT: &str = "/data/user/0/com.cranamp.app.debug/files/studio-review-result";

pub(super) fn progress(value: &str) {
    std::fs::write(RESULT, value).expect("review progress");
    log::info!("Studio device review: {value}");
}

#[composable]
fn AndroidReviewRoot() {
    let enabled = cranpose_core::remember(|| std::path::Path::new(MODE).is_file());
    if !enabled.with(|enabled| *enabled) {
        crate::winamp::WinampStackedApp();
        return;
    }
    let stage = cranpose_core::rememberMutableStateOf(|| 0u8);
    cranpose_core::LaunchedEffectAsync((), move |_| {
        Box::pin(async move {
            progress("player");
            cranpose_core::delay(Duration::from_secs(1)).await;
            stage.set(1);
        })
    });
    match stage.get() {
        0 => crate::winamp::WinampStackedApp(),
        1 => {
            let document = cranpose_core::remember(|| open_document(None).expect("review skin"));
            SkinStudio(document.with(Clone::clone), None);
            let picker = cranpose_services::rememberOpenFileLauncher(
                "cranamp.review.cancel",
                move |result| {
                    assert!(
                        matches!(result, Ok(None)),
                        "the regression must cancel the picker"
                    );
                    progress("picker-cancelled");
                    stage.set(2);
                },
            );
            cranpose_core::LaunchedEffectAsync((), move |_| {
                let picker = picker.clone();
                Box::pin(async move {
                    cranpose_core::delay(Duration::from_millis(500)).await;
                    progress("picker-open");
                    picker.launch(studio_open_options());
                })
            });
        }
        _ => {
            BoxWithConstraints(Modifier::empty().fill_max_size(), move |scope| {
                ReviewCases(
                    scope.constraints().max_width,
                    scope.constraints().max_height - 40.,
                    0,
                    true,
                );
            });
        }
    }
}

cranpose::android_main! {
    launcher: crate::create_android_app(),
    content: AndroidReviewRoot,
}
