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
