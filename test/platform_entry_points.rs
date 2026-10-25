const STACKED_APP: &str = "WinampStackedApp";
#[test]
fn android_and_ios_compose_the_same_player() {
    let lib = include_str!("../src/lib.rs");
    let shared = lib
        .split("fn mobile_root()")
        .nth(1)
        .expect("mobile platforms share one root")
        .split("\n}")
        .next()
        .expect("mobile_root has a body");
    assert!(
        shared.contains(STACKED_APP) && shared.contains(".safe_area_padding()"),
        "mobile root must compose {STACKED_APP} inside the safe area: {shared}"
    );
    let ios = lib
        .split("fn ios_entry_point()")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    assert!(ios.contains("run(mobile_root)"));
    let android = lib
        .split("cranpose::android_main! {")
        .nth(1)
        .expect("Android enters through the android_main! macro")
        .split('}')
        .next()
        .expect("android_main! has a body");
    assert!(
        android.contains("content: mobile_root"),
        "Android must use the shared mobile root: {android}"
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
