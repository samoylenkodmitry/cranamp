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

        print("=== \(label) PICKER: element tree ===")
        print(app.debugDescription)

        print("=== \(label) PICKER: rows ===")
        for kind in [app.cells, app.buttons, app.staticTexts] {
            for element in kind.allElementsBoundByIndex {
                let title = element.label.trimmingCharacters(in: .whitespacesAndNewlines)
                guard !title.isEmpty else { continue }
                print("row: \(title) | enabled=\(element.isEnabled) | hittable=\(element.isHittable)")
            }
        }

        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "\(label)-picker"
        shot.lifetime = .keepAlways
        add(shot)
    }
}
