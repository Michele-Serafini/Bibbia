import AppKit
import Foundation
import BibleKit

extension AppModel {
    func runReopenSmoke() async {
        do {
            guard directory.lastPathComponent.hasPrefix("bibbia-macos-smoke-"),
                  FileManager.default.fileExists(atPath: directory.appendingPathComponent(".bibbia-test-fixture").path),
                  let store else { throw BibleError.invalid("Reopen test requires an isolated marked profile") }
            try await Task.sleep(for: .milliseconds(500))
            try check(position.edition == "CEI 2008" && position.chapter == 2 && preferences.fontSize == 28
                      && preferences.language == "en" && preferences.theme == "Dark", "Second process restores reading preferences")
            let notes = try await store.saved(edition: Position().edition)
            try check(notes.first?.annotation.note == "Nota macOS 😀" && notes.first?.annotation.bookmark == true,
                      "Second process restores notes and bookmarks")
            try Data("passed\n".utf8).write(to: directory.appendingPathComponent("reopen-passed.txt"))
            NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0)
        } catch {
            FileHandle.standardError.write(Data("macOS reopen smoke failed: \(error)\n".utf8))
            Darwin.exit(1)
        }
    }
    /// Integration mode requires an explicitly marked disposable directory.
    /// It is never run as part of an ordinary launch.
    func runSmoke() async {
        do {
            guard directory.lastPathComponent.hasPrefix("bibbia-macos-smoke-"),
                  FileManager.default.fileExists(atPath: directory.appendingPathComponent(".bibbia-test-fixture").path),
                  let store else { throw BibleError.invalid("UI tests require a marked isolated --data-dir") }
            try await Task.sleep(for: .seconds(2))
            guard let window = NSApp.windows.first(where: { $0.contentView != nil && $0.isVisible }),
                  let root = window.contentView,
                  let text = findReadingView(root) else { throw BibleError.invalid("Native reader window not found") }
            NSApp.activate(ignoringOtherApps: true)
            window.makeKeyAndOrderFront(nil)
            try await Task.sleep(for: .milliseconds(200))
            try check(window.frame.width >= 760, "Native window allocated")
            try check(text.string.contains(verses[0].text), "Chapter rendered in NSTextView")
            try screenshot(window, name: "macos-initial.png")
            let font = text.textStorage?.attribute(.font, at: text.readingLayout.spans[0].location, effectiveRange: nil) as? NSFont
            try check(font != nil && font?.fontName != NSFont.systemFont(ofSize: 22).fontName, "Native serif reading font")
            FileHandle.standardOutput.write(Data("Reading font: \(font?.fontName ?? "missing")\n".utf8))
            let selection = NSRange(location: text.readingLayout.spans[0].location, length: 8)
            try nativeDrag(text, range: selection)
            try check(text.annotationPopover?.isShown == true && !text.capturedSelection.isEmpty,
                      "AppKit mouse drag automatically opens the selection toolbar")
            text.closePopover()
            text.setSelectedRange(selection)
            text.showSelection()
            try check(text.annotationPopover?.isShown == true, "Native selection popover shown")
            try check((text.annotationPopover?.contentSize.width ?? 1000) <= 400, "Compact selection toolbar")
            try check(text.capturedSelection == [SelectedSpan(verse: 0, start: 0, length: 8)], "Exact UTF-16 selection")
            text.performAnnotation("underline")
            try await waitUntil { self.verses.first?.annotation.ranges.first?.underline == true }
            text.setSelectedRange(selection); text.showSelection(); text.performAnnotation("yellow")
            try await waitUntil { self.verses.first?.annotation.ranges.first?.highlight == "yellow" }
            text.setSelectedRange(selection); text.showSelection(); text.performBookmark()
            try await waitUntil { self.verses.first?.annotation.bookmark == true }
            // Bookmark star has the same UTF-16 length as the unmarked prefix.
            text.setSelectedRange(selection); text.showSelection(); text.showNote()
            try check(text.annotationPopover?.isShown == true, "Native note popover shown")
            try check((text.annotationPopover?.contentSize.height ?? 1000) <= 240, "Compact note editor")
            text.closePopover()
            note("Nota macOS 😀", at: position)
            try await waitUntil { self.verses.first?.annotation.note == "Nota macOS 😀" }
            let saved = try await store.saved(edition: position.edition)
            try check(saved.count == 1 && saved[0].annotation.bookmark, "Saved collection populated")
            let cei = try await store.books(edition: "CEI 2008")
            try check(cei.count == 73, "CEI Catholic 73 books")
            let result = try await store.search(edition: "CEI 2008", query: "ha tanto amato il mondo")
            guard let john = result.hits.first(where: { $0.position.chapter == 2 && $0.label == "16" }) else {
                throw BibleError.invalid("CEI John 3:16 not found")
            }
            navigate(john.position)
            try await waitUntil { self.position == john.position && !self.loading }
            try check(verses.contains { $0.label == "16" && $0.text.contains("amato") }, "Search navigates to native reader")
            panel = .search; query = "ha tanto amato il mondo"; search()
            try await waitUntil { !self.searching && !self.hits.isEmpty }
            let previousHits = hits.map(\.position)
            navigate(john.position)
            try await waitUntil { !self.loading }
            try check(!searching && hits.map(\.position) == previousHits, "Opening a result reuses the search results")
            panel = .books
            preferences.theme = "Light"; preferencesChanged()
            try await Task.sleep(for: .milliseconds(500))
            try screenshot(window, name: "macos-light.png")
            preferences.theme = "Dark"; preferencesChanged()
            try await Task.sleep(for: .milliseconds(500))
            try screenshot(window, name: "macos-dark.png")
            let oldHeight = text.layoutManager?.usedRect(for: text.textContainer!).height ?? 0
            preferences.fontSize = 28; preferences.language = "en"; preferencesChanged()
            try await Task.sleep(for: .milliseconds(500))
            text.layoutManager?.ensureLayout(for: text.textContainer!)
            let newHeight = text.layoutManager?.usedRect(for: text.textContainer!).height ?? 0
            try check(newHeight > oldHeight, "Text size updates real TextKit layout")
            try check(position.edition == "CEI 2008" && t("it", "en") == "en", "Independent interface and Bible language")
            await flush()
            let reopened = try BibleStore(directory: directory)
            let snapshot = try await reopened.bootstrap()
            try check(snapshot.position == position && snapshot.preferences.fontSize == 28 && snapshot.preferences.theme == "Dark", "Position/theme/font survive reopen")
            let notes = try await reopened.saved(edition: Position().edition)
            try check(notes.first?.annotation.note == "Nota macOS 😀", "Notes survive reopen")
            let ext = try await reopened.books(edition: "Raccolta estesa (83 sezioni)")
            try check(ext.count == 83 && ext.contains(where: { $0.chapters.count == 108 && $0.name.contains("Enoch") }), "Extended collection available")
            let marker = directory.appendingPathComponent("ui-passed.txt")
            try Data("Native macOS UI checks passed\n".utf8).write(to: marker)
            print("Native macOS UI checks passed")
            // A run-loop selector follows the same route as the native Quit
            // command. Calling terminate inside a main-dispatch block prevents
            // that queue from servicing the asynchronous terminateLater flush.
            NSApp.perform(#selector(NSApplication.terminate(_:)), with: nil, afterDelay: 0)
        } catch {
            FileHandle.standardError.write(Data("macOS UI smoke failed: \(error)\n".utf8))
            // This path is only reachable in the explicit integration-test mode.
            Darwin.exit(1)
        }
    }

    private func check(_ condition: Bool, _ message: String) throws {
        guard condition else { throw BibleError.invalid(message) }
        FileHandle.standardOutput.write(Data("PASS: \(message)\n".utf8))
    }

    private func waitUntil(_ condition: () -> Bool) async throws {
        for _ in 0..<200 {
            if condition() { return }
            try await Task.sleep(for: .milliseconds(25))
        }
        throw BibleError.invalid("Timed out waiting for UI update")
    }

    private func findReadingView(_ view: NSView) -> ReadingTextView? {
        if let reader = view as? ReadingTextView { return reader }
        for child in view.subviews { if let reader = findReadingView(child) { return reader } }
        return nil
    }

    private func nativeDrag(_ view: ReadingTextView, range: NSRange) throws {
        guard let window = view.window else { throw BibleError.invalid("Reader has no window") }
        window.makeKeyAndOrderFront(nil)
        window.makeFirstResponder(view)
        view.scrollRangeToVisible(range)
        let first = view.firstRect(forCharacterRange: NSRange(location: range.location, length: 1), actualRange: nil)
        let last = view.firstRect(forCharacterRange: NSRange(location: NSMaxRange(range), length: 1), actualRange: nil)
        let start = window.convertPoint(fromScreen: NSPoint(x: first.minX + 0.2, y: first.midY))
        let end = window.convertPoint(fromScreen: NSPoint(x: last.minX + 0.2, y: last.midY))
        func event(_ type: NSEvent.EventType, _ point: NSPoint, _ number: Int) throws -> NSEvent {
            guard let result = NSEvent.mouseEvent(with: type, location: point, modifierFlags: [],
                timestamp: ProcessInfo.processInfo.systemUptime, windowNumber: window.windowNumber,
                context: nil, eventNumber: number, clickCount: 1, pressure: 1) else {
                throw BibleError.invalid("Unable to create native mouse event")
            }
            return result
        }
        // AppKit's real mouseDown tracking loop consumes these queued drag/up
        // events. This needs no Accessibility permission or external click tool.
        let down = try event(.leftMouseDown, start, 1)
        let drag = try event(.leftMouseDragged, end, 2)
        let up = try event(.leftMouseUp, end, 3)
        NSApp.postEvent(up, atStart: true)
        NSApp.postEvent(drag, atStart: true)
        view.mouseDown(with: down)
    }

    private func screenshot(_ window: NSWindow, name: String) throws {
        guard let view = window.contentView?.superview,
              let rep = view.bitmapImageRepForCachingDisplay(in: view.bounds) else {
            throw BibleError.invalid("Screenshot unavailable")
        }
        view.cacheDisplay(in: view.bounds, to: rep)
        guard let png = rep.representation(using: .png, properties: [:]) else { throw BibleError.invalid("PNG unavailable") }
        let folder = directory.appendingPathComponent("screenshots", isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        try png.write(to: folder.appendingPathComponent(name))
    }
}
