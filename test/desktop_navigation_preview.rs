//! Native window review without the inspector covering the navigation.
fn main() {
    cranamp::create_desktop_app()
        .with_application_id("com.cranamp.desktop-navigation-review")
        .with_developer_inspector(false)
        .run(cranamp::winamp::WinampStandaloneApp);
}
