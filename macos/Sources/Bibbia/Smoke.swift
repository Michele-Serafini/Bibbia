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
            let readingID=try await store.createReadingPlan(title:"Native calendar",edition:position.edition,start:"2024-02-28",daily:2,book:0)
            showReadingPlans=true
            try await Task.sleep(for:.milliseconds(400))
            showReadingPlans=false
            try await Task.sleep(for:.milliseconds(200))
            try await store.deleteReadingPlan(readingID)
            try await Task.sleep(for: .seconds(2))
            guard let window = NSApp.windows.first(where: { $0.contentView != nil && $0.isVisible }),
                  let root = window.contentView,
                  let text = findReadingView(root) else { throw BibleError.invalid("Native reader window not found") }
            NSApp.activate(ignoringOtherApps: true)
            window.makeKeyAndOrderFront(nil)
            try await Task.sleep(for: .milliseconds(200))
            try check(window.frame.width >= 760, "Native window allocated")
            try check(text.string.contains(verses[0].text), "Chapter rendered in NSTextView")
            guard let book = text.enclosingScrollView?.superview as? BookSurface else { throw BibleError.invalid("Book surface missing") }
            bookMode = false; book.pager.reflow()
            if book.motionAllowed {
                step(1)
                try await waitUntil { book.isTurning && self.position.chapter == 1 }
                try await waitUntil { book.hasCurledPresentation }
                try await Task.sleep(for: .milliseconds(60))
                try book.saveMotionFrame(to: directory.appendingPathComponent("screenshots/macos-page-turn.png"))
                try await waitUntil { !book.isTurning }
                try check(book.turnsStarted > 0, "Curved page animation completes and releases the image")
                step(1); step(1)
                try await waitUntil { self.position.chapter == 3 && !self.loading }
                try await waitUntil { !book.isTurning }
                try check(position.chapter == 3, "Rapid page turns preserve the latest destination")
                let started = book.turnsStarted
                book.allowsMotion = false
                navigate(Position())
                try await waitUntil { self.position.chapter == 0 && !self.loading }
                try await Task.sleep(for: .milliseconds(80))
                try check(!book.isTurning && book.turnsStarted == started, "Reduced motion skips paper capture and animation")
                book.allowsMotion = true
            }
            try screenshot(window, name: "macos-initial.png")
            bookMode = true; book.pager.reflow()
            try await Task.sleep(for: .milliseconds(120))
            try check(book.pager.pageCount > 1, "Book mode paginates the chapter using native line fragments")
            let chapterBeforePage = position.chapter
            step(1)
            try check(position.chapter == chapterBeforePage && book.pager.index > 0, "Next page stays within the chapter")
            let savedPage = book.pager.index
            book.pager.lastChapter = ""; book.pager.restored = false
            book.pager.configure(book, text, self, target: position.verse)
            book.pager.configure(book, text, self, target: position.verse)
            try check(book.pager.index == savedPage, "Precise book resume survives deferred native layout")
            step(-1)
            try check(book.pager.index == 0, "Previous page restores the first spread")
            let originalFrame = window.frame
            window.setFrame(NSRect(x: originalFrame.minX, y: originalFrame.minY, width: 1500, height: 900), display: true)
            try await waitUntil { book.pager.columns == 2 }
            try await Task.sleep(for: .milliseconds(250))
            book.layoutSubtreeIfNeeded(); book.pager.reflow()
            try check(!book.pager.right.isHidden, "Wide book displays facing native pages")
            for (column, view) in [text, book.pager.second].compactMap({ $0 }).enumerated() {
                guard let manager = view.layoutManager else { throw BibleError.invalid("Missing page layout manager") }
                let page = book.pager.index + column
                let end = page+1 < book.pager.pages.count ? book.pager.pages[page+1].character : view.string.utf16.count
                let glyph = manager.glyphIndexForCharacter(at: max(0,end-1))
                let lastLine = manager.lineFragmentUsedRect(forGlyphAt: glyph, effectiveRange: nil)
                try check(lastLine.maxY + view.textContainerInset.height <= view.visibleRect.maxY + 1,
                          "Facing page \(column+1) contains every line before its next page break")
            }
            if let right = book.pager.second {
                let character = book.pager.pages[1].character
                let verse = text.readingLayout.spans.firstIndex { $0.location >= character } ?? 0
                try nativeDrag(right, range: NSRange(location: text.readingLayout.spans[verse].location, length: 8))
                try check(right.annotationPopover?.isShown == true && right.capturedSelection.first?.verse == verse,
                          "Right page native selection keeps original verse offsets")
                right.closePopover()
            }
            try screenshot(window, name: "macos-spread.png")
            window.setFrame(originalFrame, display: true)
            bookMode = false; book.pager.reflow(); book.pager.save()
            navigate(Position())
            try await waitUntil { !self.loading && self.position.chapter == 0 }
            try await Task.sleep(for: .milliseconds(160))
            text.scrollRangeToVisible(NSRange(location: 0, length: 1))
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
            let returnTo = position
            navigate(Position())
            try await waitUntil { !self.loading && self.position.edition == Position().edition }
            panel = .saved; loadSaved(reset: true)
            try await waitUntil { self.saved.count == 1 }
            let savedHit = saved[0]
            organizingNote = savedHit
            try await Task.sleep(for: .milliseconds(250))
            try await saveOrganization(savedHit,folder:"Studio",tags:"speranza, lettura")
            try await waitUntil { self.saved.first?.organization.folder == "Studio" }
            noteFilter.query = "unmatched-annotation-fixture"
            loadSaved(reset: true)
            try await waitUntil { self.saved.isEmpty }
            noteFilter.query = "SPERANZA"
            noteFilter.folder = "Studio"; noteFilter.tag = "lettura"; noteFilter.kind = "note"
            loadSaved(reset: true)
            try await waitUntil { self.saved.count == 1 }
            try check(saved[0].annotation.note == "Nota macOS 😀", "Organized note search combines folder, tag and note type without changing its content")
            try screenshot(window,name:"macos-notes.png")
            noteFilter = NoteFilter(); panel = .books; navigate(returnTo)
            try await waitUntil { !self.loading && self.position == returnTo }
            let ext = try await reopened.books(edition: "Raccolta estesa (83 sezioni)")
            try check(ext.count == 83 && ext.contains(where: { $0.chapters.count == 108 && $0.name.contains("Enoch") }), "Extended collection available")
            let beforeComparison = position
            preferences.comparisonEdition = "KJV"; comparing = true; comparisonChanged()
            try await waitUntil { !self.comparisonLoading && self.comparison != nil }
            try check(comparison?.rows[15].right.first?.book == 42
                      && comparison?.rows[15].right.first?.text.contains("God so loved") == true,
                      "Comparison aligns CEI and KJV John 3:16 across canonical book indices")
            try await Task.sleep(for: .milliseconds(300))
            guard let compared = findComparisonView(window.contentView!) else { throw BibleError.invalid("Native comparison documents missing") }
            let offsets = compared.alignedRowOffsets()
            try check(offsets[0].count == 36 && zip(offsets[0], offsets[1]).allSatisfy { abs($0-$1)<1 }, "Every compared row uses aligned native text layout")
            let phrase = (compared.readers[1].string as NSString).range(of: "For God so loved")
            try check(phrase.location != NSNotFound, "KJV comparison text is selectable")
            try nativeDrag(compared.readers[1], range: phrase)
            try check(compared.readers[1].selectedRange().length > 0, "Real mouse selection in comparison")
            try check(abs(compared.scrolls[0].contentView.bounds.minY-compared.scrolls[1].contentView.bounds.minY)<1, "Native comparison scroll synchronization")
            try screenshot(window,name: "macos-comparison.png")
            compared.open(1)
            try await waitUntil { !self.comparing && !self.loading && self.position.edition == "KJV" }
            try check(position.book == 42 && position.chapter == 2 && position.verse == 15,
                      "Compared verse opens at the original annotatable position")
            navigate(beforeComparison)
            try await waitUntil { !self.loading && self.position == beforeComparison }
            await flush()
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

    private func findComparisonView(_ view: NSView) -> ComparisonDocumentView? {
        if let compared = view as? ComparisonDocumentView { return compared }
        for child in view.subviews { if let compared = findComparisonView(child) { return compared } }
        return nil
    }

    private func nativeDrag(_ view: NSTextView, range: NSRange) throws {
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
