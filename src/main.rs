#![forbid(unsafe_code)]
fn main() {
    #[cfg(feature = "logging")]
    let _ = env_logger::try_init();
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--touch-preview") {
        cranamp::winamp::studio::run_touch_preview(args.get(2).is_some_and(|s| s == "--studio"));
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--skin-studio") {
        cranamp::winamp::studio::run(args.get(2).map(String::as_str));
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--skin-studio-mcp") {
        cranamp::winamp::studio::stdio_bridge();
        return;
    }
    cranamp::create_desktop_app().run(cranamp::winamp::WinampStandaloneApp);
}
