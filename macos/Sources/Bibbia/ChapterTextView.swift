import AppKit
import SwiftUI
import BibleKit

/// One TextKit view per chapter. AppKit performs layout, selection, accessibility,
/// scrolling and copying; there is no SwiftUI view allocated for every verse.
struct ChapterTextView: NSViewRepresentable {
    @ObservedObject var model: AppModel
    @Environment(\.colorScheme) private var colorScheme

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.hasVerticalScroller = true; scroll.autohidesScrollers = true
        scroll.borderType = .noBorder; scroll.drawsBackground = false
        let view = ReadingTextView(frame: .zero)
        view.isEditable = false; view.isSelectable = true; view.isRichText = true
        view.importsGraphics = false; view.allowsUndo = false
        view.isAutomaticLinkDetectionEnabled = false
        view.isAutomaticSpellingCorrectionEnabled = false
        view.isContinuousSpellCheckingEnabled = false
        view.drawsBackground = false; view.textContainerInset = NSSize(width: 28, height: 12)
        view.isVerticallyResizable = true; view.isHorizontallyResizable = false
        view.autoresizingMask = [.width]
        view.minSize = NSSize(width: 0, height: 0)
        view.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        view.textContainer?.widthTracksTextView = true
        view.textContainer?.containerSize = NSSize(width: 760, height: CGFloat.greatestFiniteMagnitude)
        view.setAccessibilityIdentifier("bibleText")
        view.setAccessibilityLabel(model.t("Testo del capitolo", "Chapter text"))
        scroll.documentView = view
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        guard let view = scroll.documentView as? ReadingTextView else { return }
        view.model = model
        view.render(revision: model.revision, size: model.preferences.fontSize, dark: colorScheme == .dark)
        if let target = model.scrollTarget, view.readingLayout.spans.indices.contains(target) {
            view.scrollRangeToVisible(view.readingLayout.spans[target])
            // Deferring avoids mutating observed state inside updateNSView.
            DispatchQueue.main.async { if model.scrollTarget == target { model.scrollTarget = nil } }
        }
    }

    static func dismantleNSView(_ scroll: NSScrollView, coordinator: ()) {
        (scroll.documentView as? ReadingTextView)?.closePopover()
    }
}

@MainActor
final class ReadingTextView: NSTextView {
    weak var model: AppModel?
    private(set) var readingLayout = ReadingLayout(verses: [])
    private var revision = -1
    private var size = 0
    private var dark = false
    private var renderedPosition: Position?
    private(set) var annotationPopover: NSPopover?
    private(set) var capturedSelection: [SelectedSpan] = []
    private var capturedPosition = Position()

    static func readingFont(_ size: CGFloat) -> NSFont {
        let descriptor = NSFont.systemFont(ofSize: size).fontDescriptor.withDesign(.serif)
        return descriptor.flatMap { NSFont(descriptor: $0, size: size) } ?? .systemFont(ofSize: size)
    }

    func render(revision: Int, size: Int, dark: Bool) {
        guard let model else { return }
        setAccessibilityLabel(model.t("Testo del capitolo", "Chapter text"))
        guard self.revision != revision || self.size != size || self.dark != dark else { return }
        let chapterChanged = renderedPosition?.edition != model.position.edition
            || renderedPosition?.book != model.position.book || renderedPosition?.chapter != model.position.chapter
        if chapterChanged { closePopover() }
        self.revision = revision; self.size = size; self.dark = dark
        let selected = selectedRange()
        let scrollOrigin = enclosingScrollView?.contentView.bounds.origin
        let oldString = string
        renderedPosition = model.position
        readingLayout = ReadingLayout(verses: model.verses)
        let paragraph = NSMutableParagraphStyle()
        let numberFont = NSFont.monospacedDigitSystemFont(ofSize: CGFloat(size) * 0.56, weight: .medium)
        let widestLabel = model.verses.map { "★ \($0.label)" as NSString }
            .map { $0.size(withAttributes: [.font: numberFont]).width }.max() ?? 24
        let gutter = ceil(widestLabel + 12)
        paragraph.headIndent = gutter; paragraph.firstLineHeadIndent = 0
        paragraph.tabStops = [NSTextTab(textAlignment: .left, location: gutter)]
        paragraph.defaultTabInterval = gutter
        paragraph.lineHeightMultiple = 1.22; paragraph.paragraphSpacing = 13
        let content = NSMutableAttributedString(string: readingLayout.text, attributes: [
            .font: Self.readingFont(CGFloat(size)), .foregroundColor: NSColor.textColor,
            .paragraphStyle: paragraph
        ])
        for (index, verse) in model.verses.enumerated() {
            let span = readingLayout.spans[index]
            let prefixStart = index == 0 ? 0 : NSMaxRange(readingLayout.spans[index - 1]) + 1
            content.addAttributes([.font: numberFont, .foregroundColor: NSColor.secondaryLabelColor],
                                  range: NSRange(location: prefixStart, length: span.location - prefixStart))
            let annotation = verse.annotation
            if annotation.underline { content.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: span) }
            if let color = annotation.highlight { addHighlight(color, to: content, range: span) }
            for mark in annotation.ranges {
                guard (try? Marking.validate(text: verse.text, start: mark.start, length: mark.length)) != nil else { continue }
                let range = NSRange(location: span.location + mark.start, length: mark.length)
                if mark.underline { content.addAttribute(.underlineStyle, value: NSUnderlineStyle.single.rawValue, range: range) }
                if let color = mark.highlight { addHighlight(color, to: content, range: range) }
            }
        }
        textStorage?.beginEditing()
        if oldString == readingLayout.text {
            // Font/theme/marks change attributes, keeping selection and scroll stable.
            content.enumerateAttributes(in: NSRange(location: 0, length: content.length)) { attributes, range, _ in
                textStorage?.setAttributes(attributes, range: range)
            }
        } else { textStorage?.setAttributedString(content) }
        textStorage?.endEditing()
        if !chapterChanged, NSMaxRange(selected) <= content.length {
            setSelectedRange(selected)
            if let scrollOrigin, let scroll = enclosingScrollView {
                scroll.contentView.scroll(to: scrollOrigin)
                scroll.reflectScrolledClipView(scroll.contentView)
            }
        }
        setAccessibilityLabel(model.t("Testo del capitolo", "Chapter text"))
    }

    private func addHighlight(_ name: String, to content: NSMutableAttributedString, range: NSRange) {
        guard let color = Self.highlightColor(name) else { return }
        content.addAttributes([.backgroundColor: color.withAlphaComponent(dark ? 0.28 : 0.35),
                               .foregroundColor: NSColor.textColor], range: range)
    }

    static func highlightColor(_ name: String) -> NSColor? {
        switch name {
        case "yellow": return .systemYellow
        case "green": return .systemGreen
        case "blue": return .systemBlue
        case "pink": return .systemPink
        default: return nil
        }
    }

    override func mouseDown(with event: NSEvent) {
        closePopover()
        super.mouseDown(with: event) // AppKit tracks the complete drag synchronously.
        showSelection()
    }

    override func keyDown(with event: NSEvent) {
        super.keyDown(with: event)
        if event.modifierFlags.contains(.shift) { showSelection() }
        else if event.keyCode == 53 { closePopover() }
    }

    override func menu(for event: NSEvent) -> NSMenu? {
        capture()
        let menu = super.menu(for: event) ?? NSMenu()
        guard !capturedSelection.isEmpty, let model else { return menu }
        menu.addItem(.separator())
        for (title, action) in [(model.t("Sottolinea", "Underline"), "underline"),
                                 (model.t("Evidenzia", "Highlight"), "yellow"),
                                 (model.t("Rimuovi segni", "Clear marks"), "clear")] {
            let item = NSMenuItem(title: title, action: #selector(menuMark(_:)), keyEquivalent: "")
            item.target = self; item.representedObject = action; menu.addItem(item)
        }
        return menu
    }

    @objc private func menuMark(_ sender: NSMenuItem) {
        if let action = sender.representedObject as? String { performAnnotation(action) }
    }

    private func capture() {
        guard let model else { return }
        capturedSelection = readingLayout.selection(selectedRange())
        capturedPosition = model.position
        if let first = capturedSelection.first {
            capturedPosition.verse = first.verse
            model.selectedVerse(first.verse)
        }
    }

    func showSelection() {
        guard let model, window != nil else { return }
        capture()
        guard !capturedSelection.isEmpty else { return }
        closePopover()
        let popover = NSPopover()
        popover.behavior = .semitransient; popover.animates = !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        let toolbar = SelectionToolbar(model: model, mark: { [weak self] in self?.performAnnotation($0) },
                                       bookmark: { [weak self] in self?.performBookmark() },
                                       note: { [weak self] in self?.showNote() },
                                       copy: { [weak self] in self?.copy(nil); self?.closePopover() })
        popover.contentViewController = NSHostingController(rootView: toolbar)
        popover.contentSize = NSSize(width: 380, height: 48)
        annotationPopover = popover
        popover.show(relativeTo: selectionAnchor(), of: self, preferredEdge: .maxY)
    }

    private func selectionAnchor() -> NSRect {
        let range = selectedRange()
        guard range.location != NSNotFound else { return visibleRect }
        let screen = firstRect(forCharacterRange: NSRange(location: range.location, length: min(range.length, 1)), actualRange: nil)
        guard let window else { return visibleRect }
        var rect = convert(window.convertFromScreen(screen), from: nil)
        rect.origin.x = max(visibleRect.minX + 16, min(rect.origin.x, visibleRect.maxX - 30))
        rect.origin.y = max(visibleRect.minY + 4, min(rect.origin.y, visibleRect.maxY - rect.height - 4))
        return rect
    }

    func performAnnotation(_ action: String) {
        model?.mark(capturedSelection, action: action, at: capturedPosition)
        closePopover()
    }

    func performBookmark() { model?.bookmark(at: capturedPosition); closePopover() }

    func showNote() {
        guard let model, model.verses.indices.contains(capturedPosition.verse) else { return }
        let target = capturedPosition
        let value = model.verses[target.verse].annotation.note
        closePopover()
        let popover = NSPopover()
        popover.behavior = .semitransient
        popover.animates = !NSWorkspace.shared.accessibilityDisplayShouldReduceMotion
        popover.contentViewController = NSHostingController(rootView: NoteEditor(
            text: value, title: model.t("Nota", "Note"), saveTitle: model.t("Salva", "Save"),
            cancelTitle: model.t("Annulla", "Cancel"),
            save: { [weak self, weak model] text in model?.note(text, at: target); self?.closePopover() },
            cancel: { [weak self] in self?.closePopover() }))
        popover.contentSize = NSSize(width: 350, height: 210)
        annotationPopover = popover
        popover.show(relativeTo: selectionAnchor(), of: self, preferredEdge: .maxY)
    }

    func closePopover() { annotationPopover?.close(); annotationPopover = nil }
}

private struct SelectionToolbar: View {
    @ObservedObject var model: AppModel
    let mark: (String) -> Void
    let bookmark: () -> Void
    let note: () -> Void
    let copy: () -> Void
    var body: some View {
        HStack(spacing: 8) {
            ForEach(Marking.colors, id: \.self) { color in
                Button { mark(color) } label: {
                    Circle().fill(Color(nsColor: ReadingTextView.highlightColor(color) ?? .systemYellow)).frame(width: 19, height: 19)
                        .overlay(Circle().strokeBorder(.primary.opacity(0.18), lineWidth: 1))
                }.help(colorName(color)).accessibilityLabel(colorName(color))
            }
            Divider().frame(height: 20)
            Button { mark("underline") } label: { Image(systemName: "underline") }.help(model.t("Sottolinea", "Underline"))
            Button(action: bookmark) { Image(systemName: "bookmark") }.help(model.t("Segnalibro", "Bookmark"))
            Button(action: note) { Image(systemName: "square.and.pencil") }.help(model.t("Nota", "Note"))
            Button(action: copy) { Image(systemName: "doc.on.doc") }.help(model.t("Copia", "Copy"))
            Button { mark("clear") } label: { Image(systemName: "eraser") }.help(model.t("Rimuovi segni", "Clear marks"))
        }.buttonStyle(.borderless).font(.system(size: 15)).padding(.horizontal, 14).frame(height: 48)
    }
    private func colorName(_ color: String) -> String {
        switch color {
        case "yellow": return model.t("Giallo", "Yellow")
        case "green": return model.t("Verde", "Green")
        case "blue": return model.t("Blu", "Blue")
        default: return model.t("Rosa", "Pink")
        }
    }
}

private struct NoteEditor: View {
    @State var text: String
    let title: String
    let saveTitle: String
    let cancelTitle: String
    let save: (String) -> Void
    let cancel: () -> Void
    @FocusState private var focused: Bool
    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(title).font(.headline)
            TextEditor(text: $text).font(.body).focused($focused)
                .border(Color.secondary.opacity(0.2))
            HStack {
                Button(cancelTitle, action: cancel).keyboardShortcut(.cancelAction)
                Spacer()
                Button(saveTitle) { save(text) }.keyboardShortcut(.defaultAction)
                    .disabled(text.utf8.count > 32768)
            }
        }.padding(14).frame(width: 350, height: 210).task { focused = true }
    }
}
