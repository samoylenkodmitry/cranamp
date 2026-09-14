//! Every touch platform enters the player through the same composable.
//!
//! It did not. Android entered `WinampAndroidApp`, which scales the stacked
//! windows to the surface, and iOS entered `WinampSurfaceApp`, which is the
//! desktop's floating windows at a fixed scale. On a phone that is a small
//! Winamp adrift on a black screen, and it shipped, because nothing compared
//! the two entry points and no test could see a layout.
//!
//! Reading the source is the point. What broke was not a value any assertion
//! could reach at run time -- it was two lines naming two different
//! composables, one per operating system.

/// The name every touch platform's entry point must call.
const STACKED_APP: &str = "WinampStackedApp";

#[test]
fn android_and_ios_compose_the_same_player() {
    let lib = include_str!("../src/lib.rs");

    let ios = lib
        .split("fn ios_root()")
        .nth(1)
        .expect("iOS enters through ios_root")
        .split("\n}")
        .next()
        .expect("ios_root has a body");
    assert!(
        ios.contains(STACKED_APP),
        "iOS composes something other than {STACKED_APP}: {ios}"
    );

    let android = lib
        .split("cranpose::android_main! {")
        .nth(1)
        .expect("Android enters through the android_main! macro")
        .split('}')
        .next()
        .expect("android_main! has a body");
    assert!(
        android.contains(STACKED_APP),
        "Android composes something other than {STACKED_APP}: {android}"
    );
}

/// The stacked stage must not name an operating system.
///
/// Enumerating them inside it -- `#[cfg(target_os = ...)]` blocks each
/// defining the same drag variables -- is what made the stage itself
/// per-platform, and left every platform outside the list unable to use it at
/// all. What differs is how a window drags, and that arrives as an argument.
#[test]
fn the_stacked_stage_is_not_written_per_operating_system() {
    let winamp = include_str!("../src/winamp/mod.rs");
    let stage = winamp
        .split("\nfn WinampStackedStage(")
        .nth(1)
        .expect("the stacked stage is what every touch platform draws")
        .split("\n}\n")
        .next()
        .expect("the stage has a body");

    assert!(
        !stage.contains("target_os"),
        "the stacked stage names an operating system again:\n{stage}"
    );
}
