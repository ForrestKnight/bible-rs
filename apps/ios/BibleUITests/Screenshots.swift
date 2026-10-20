import XCTest

/// The screenshots in docs/screenshots/ios. Skipped unless BIBLE_SCREENSHOTS=1
/// (`TEST_RUNNER_BIBLE_SCREENSHOTS=1` through xcodebuild); scripts/ios-screenshots.sh runs it.
final class Screenshots: XCTestCase {
    private var app: XCUIApplication!

    func testCapture() throws {
        try XCTSkipUnless(ProcessInfo.processInfo.environment["BIBLE_SCREENSHOTS"] == "1")
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchArguments = ["--uitest"]
        app.launch()
        XCTAssertTrue(app.buttons["reference"].waitForExistence(timeout: 10))

        // A little study history to show.
        go("jn 3")
        verse("John 3:3").tap()
        app.selectionAction("Highlight Yellow").tap()
        verse("John 3:5").tap()
        app.selectionAction("Note").tap()
        app.textFields["Write a note"].typeText("Born of water and of the Spirit: compare Ezekiel 36:25–27.")
        app.buttons["Save"].tap()
        verse("John 3:7").tap()
        app.selectionAction("Highlight Blue").tap()

        for theme in ["Light", "Dark", "Sepia"] {
            setTheme(theme)
            let name = theme.lowercased()
            go("jn 3")
            snap("reader-\(name)")
            if theme == "Sepia" {
                go("ps 23")
                snap("psalm-\(name)")
                continue
            }
            verse("John 3:2").tap()
            snap("selection-\(name)")
            app.selectionAction("Clear selection").tap()

            go("ps 23")
            snap("psalm-\(name)")

            app.buttons["search"].tap()
            let field = app.searchFields.firstMatch
            XCTAssertTrue(field.waitForExistence(timeout: 3))
            field.typeText("love one another")
            XCTAssertTrue(
                app.buttons.matching(NSPredicate(format: "label BEGINSWITH 'John 13:34'")).firstMatch
                    .waitForExistence(timeout: 5))
            app.typeText("\n")
            snap("search-\(name)")
            // iPhone: the search field's Close button. iPad shows search in a sheet with Done.
            let close = app.buttons["Close"].firstMatch
            (close.exists ? close : app.buttons["Done"].firstMatch).tap()

            app.buttons["reference"].tap()
            XCTAssertTrue(app.buttons["Psalms 23"].waitForExistence(timeout: 3))
            snap("books-\(name)")
            app.buttons["Done"].tap()
        }
    }

    private func verse(_ id: String) -> XCUIElement {
        let e = app.staticTexts[id]
        XCTAssertTrue(e.waitForExistence(timeout: 5), id)
        return e
    }

    private func go(_ reference: String) {
        app.buttons["reference"].tap()
        let field = app.searchFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 3))
        field.tap()
        field.typeText(reference + "\n")
        sleep(1)
    }

    private func setTheme(_ theme: String) {
        app.buttons["settings"].tap()
        app.buttons["\(theme) theme"].tap()
        app.buttons["Done"].tap()
        sleep(1)
    }

    private func snap(_ name: String) {
        sleep(1)
        let shot = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        shot.name = "ios-\(name)"
        shot.lifetime = .keepAlways
        add(shot)
    }
}
