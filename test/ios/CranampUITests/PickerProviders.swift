// What the iOS document picker enables, read back as a test assertion.
//
// The picker is a system view. Which providers it offers is decided by the
// content type the app asked for, and the app cannot read that decision back --
// there is no list of what got greyed out. Looking at the dialog is the only
// check there is, so this looks at it.
//
// The app under test is not built by Xcode: it is a Rust binary assembled into
// a bundle by `platform/ios/build-app.sh`, already installed on the device.
// `XCUIApplication(bundleIdentifier:)` attaches to it by identity, and
// `--open-picker=folder` makes it open the folder picker on launch rather than
// asking this test to drive a wgpu-rendered UI that publishes no controls.

import XCTest

final class PickerProviders: XCTestCase {
    private static let appBundleId = "io.cranamp.app"

    override func setUp() {
        super.setUp()
        continueAfterFailure = true
    }

    /// Opens the folder picker and prints every provider row with whether it is
    /// enabled. The point is the transcript, not a pass/fail: which providers a
    /// given phone offers depends on what is installed on it, so pinning names
    /// here would assert the tester's phone rather than the app.
    func testFolderPickerProviders() {
        report(argument: "--open-picker=folder", label: "FOLDER")
    }

    /// The same, for a file pick. This is the control: a file pick asks for
    /// `public.item`, which everything conforms to, so every provider should be
    /// enabled here. A provider disabled in the folder run and enabled in this
    /// one is the difference the requested type makes.
    func testFilePickerProviders() {
        report(argument: "--open-picker=files", label: "FILES")
    }

    func testRemoteMusicFolder() {
        let app = XCUIApplication(bundleIdentifier: Self.appBundleId)
        app.launch()
        XCTAssertTrue(app.images.firstMatch.waitForExistence(timeout: 20))
        let scale = app.frame.width / 275.0
        let clock = app.images["0:00"].firstMatch
        XCTAssertTrue(clock.waitForExistence(timeout: 10))
        let footerY = clock.frame.midY
        let origin = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
        origin.withOffset(CGVector(dx: 23 * scale, dy: footerY - 9 * scale)).tap()
        origin.withOffset(CGVector(dx: 45 * scale, dy: footerY - 33 * scale)).tap()
        let browse = app.buttons["Browse"]
        if browse.waitForExistence(timeout: 20) && browse.isHittable { browse.tap() }
        for _ in 0..<8 {
            if app.cells["DOC.sidebar.item.Tailscale"].exists { break }
            let back = app.navigationBars.buttons.element(boundBy: 0)
            guard back.exists && back.isHittable && back.label != "Cancel" else { break }
            back.tap()
        }
        let provider = app.cells["DOC.sidebar.item.Tailscale"]
        guard provider.waitForExistence(timeout: 10) else {
            XCTFail("Install and enable the Tailscale Files provider before this device test.")
            return
        }
        provider.tap()
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "folder-provider-root"
        shot.lifetime = .keepAlways
        add(shot)
        let account = app.cells.matching(NSPredicate(format: "label CONTAINS %@", "@")).firstMatch
        guard account.waitForExistence(timeout: 15) else {
            XCTFail("The Tailscale provider did not open for folder selection; see folder-provider-root.")
            return
        }
        account.tap()
        for name in ["samarch", "huge", "music"] {
            let entry = app.staticTexts[name].firstMatch
            for _ in 0..<8 {
                if entry.exists && entry.isHittable { break }
                app.collectionViews.firstMatch.swipeUp()
            }
            XCTAssertTrue(entry.waitForExistence(timeout: 10), "Missing \(name): " + app.debugDescription)
            guard entry.exists else { return }
            entry.tap()
        }
        let done = app.buttons["Done"].firstMatch
        let open = done.exists ? done : app.buttons["Open"].firstMatch
        guard open.waitForExistence(timeout: 10) else {
            XCTFail("The provider did not offer a folder selection button: " + app.debugDescription)
            return
        }
        open.tap()
        let times = app.images.matching(NSPredicate(format: "label MATCHES %@", "[0-9]+:[0-9]{2}"))
        let deadline = Date().addingTimeInterval(30)
        var elapsed = "0:00"
        var elapsedSeconds = 0
        while Date() < deadline {
            if times.count > 0 {
                elapsed = times.element(boundBy: times.count - 1).label
                let parts = elapsed.split(separator: ":").compactMap { Int($0) }
                if parts.count == 2 { elapsedSeconds = parts[0] * 60 + parts[1] }
                if elapsedSeconds >= 2 { break }
            }
            Thread.sleep(forTimeInterval: 0.25)
        }
        XCTAssertGreaterThanOrEqual(elapsedSeconds, 2, "Playback did not advance: " + app.debugDescription)
        print("REMOTE PLAYBACK elapsed: " + elapsed)
    }

    private func report(argument: String, label: String) {
        let app = XCUIApplication(bundleIdentifier: Self.appBundleId)
        app.launchArguments = [argument]
        app.launch()

        // The picker is a remote view served by another process, so it is not
        // reliably under the app's element tree. Waiting on any cell anywhere
        // is what actually settles.
        let deadline = Date().addingTimeInterval(25)
        while Date() < deadline {
            if app.cells.count > 0 || app.tables.count > 0 || app.collectionViews.count > 0 {
                break
            }
            Thread.sleep(forTimeInterval: 0.5)
        }
        Thread.sleep(forTimeInterval: 2.0)

        // The picker reopens wherever it was last used, which is a folder
        // somebody was browsing, not the list of providers. `Browse` is the
        // tab that shows locations; from there, walking back out of any
        // remembered subfolder reaches the root, which is the only screen that
        // names the providers at all.
        let browse = app.buttons["Browse"]
        if browse.exists && browse.isHittable {
            browse.tap()
            Thread.sleep(forTimeInterval: 1.5)
        }
        for _ in 0..<6 {
            let back = app.navigationBars.buttons.element(boundBy: 0)
            guard back.exists, back.isHittable, back.label != "Cancel" else { break }
            back.tap()
            Thread.sleep(forTimeInterval: 1.0)
        }
        Thread.sleep(forTimeInterval: 1.5)

        print("=== \(label) PICKER: element tree ===")
        print(app.debugDescription)

        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "\(label)-picker"
        shot.lifetime = .keepAlways
        add(shot)
    }
}
