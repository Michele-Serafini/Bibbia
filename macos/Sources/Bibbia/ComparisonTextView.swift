import AppKit
import SwiftUI
import BibleKit

/// Two selectable TextKit documents share aligned row positions. Their number
/// stays constant even for the longest chapters; no per-verse native controls.
struct ComparisonDocuments: NSViewRepresentable {
    @ObservedObject var model: AppModel
    let comparison: Comparison
    func makeNSView(context: Context) -> ComparisonDocumentView { ComparisonDocumentView(frame: .zero) }
    func updateNSView(_ view: ComparisonDocumentView, context: Context) { view.configure(comparison, model: model) }
}

@MainActor
private final class ComparedTextView: NSTextView {
    weak var owner: ComparisonDocumentView?
    var side = 0
    override func menu(for event: NSEvent) -> NSMenu? {
        let menu = super.menu(for: event) ?? NSMenu()
        let item = NSMenuItem(title: owner?.model?.t("Apri e annota", "Open and annotate") ?? "", action: #selector(ComparisonDocumentView.openCompared(_:)), keyEquivalent: "")
        item.target = owner; item.tag = side
        item.isEnabled = owner?.selectedVerse(side) != nil
        menu.insertItem(.separator(), at: 0); menu.insertItem(item, at: 0)
        return menu
    }
}

@MainActor
final class ComparisonDocumentView: NSView, NSTextViewDelegate {
    weak var model: AppModel?
    private struct Segment { let range: NSRange; let verse: ComparedVerse }
    private let documents = [ComparedTextView(frame: .zero), ComparedTextView(frame: .zero)]
    var readers: [NSTextView] { documents }
    let scrolls = [NSScrollView(), NSScrollView()]
    private let buttons = [NSButton(), NSButton()]
    private var segments = [[Segment](), [Segment]()]
    private var rows = [[NSRange](), [NSRange]()]
    private var rowY = [CGFloat]()
    private var key = ""
    private var width: CGFloat = 0
    private var syncing = false, rebuilding = false
    private var focus: Int?
    private var bodyStyle = NSMutableParagraphStyle()
    override var isFlipped: Bool { true }

    override init(frame: NSRect) {
        super.init(frame: frame)
        setAccessibilityIdentifier("comparisonList")
        for i in 0..<2 {
            let view = documents[i], scroll = scrolls[i], button = buttons[i]
            view.owner = self; view.side = i; view.delegate = self
            view.isEditable = false; view.isSelectable = true; view.isRichText = true
            view.importsGraphics = false; view.allowsUndo = false
            view.isAutomaticLinkDetectionEnabled = false; view.isAutomaticSpellingCorrectionEnabled = false
            view.isContinuousSpellCheckingEnabled = false; view.drawsBackground = false
            view.textContainerInset = NSSize(width: 12, height: 8)
            view.isVerticallyResizable = true; view.isHorizontallyResizable = false
            view.minSize = .zero; view.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
            view.textContainer?.widthTracksTextView = false
            view.setAccessibilityIdentifier(i == 0 ? "comparisonLeft" : "comparisonRight")
            scroll.documentView = view; scroll.drawsBackground = false; scroll.borderType = .noBorder
            scroll.hasVerticalScroller = i == 1; scroll.autohidesScrollers = true; scroll.scrollerStyle = .overlay
            scroll.contentView.postsBoundsChangedNotifications = true
            NotificationCenter.default.addObserver(self, selector: #selector(scrolled(_:)), name: NSView.boundsDidChangeNotification, object: scroll.contentView)
            button.image = NSImage(systemSymbolName: "arrow.up.right", accessibilityDescription: nil)
            button.bezelStyle = .rounded; button.target = self; button.action = #selector(openCompared(_:)); button.tag = i
            addSubview(scroll); addSubview(button)
        }
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    func configure(_ comparison: Comparison, model: AppModel) {
        self.model = model
        for i in 0..<2 {
            let label = model.t("Apri e annota il versetto selezionato", "Open and annotate the selected verse")
            buttons[i].toolTip = label; buttons[i].setAccessibilityLabel(label)
        }
        let next = "\(comparison.edition)/\(comparison.other)/\(model.position.book)/\(model.position.chapter)/\(model.preferences.fontSize)/\(model.preferences.language)"
        guard key != next else { return }
        let changedChapter = key.split(separator: "/").prefix(4) != next.split(separator: "/").prefix(4)
        rebuilding = true; defer { rebuilding = false }
        bodyStyle = NSMutableParagraphStyle(); bodyStyle.lineHeightMultiple = 1.2
        let attributes: [NSAttributedString.Key: Any] = [.font: ReadingTextView.readingFont(CGFloat(model.preferences.fontSize)), .foregroundColor: NSColor.textColor, .paragraphStyle: bodyStyle]
        for side in 0..<2 {
            let content = NSMutableAttributedString(string: "")
            segments[side].removeAll(keepingCapacity: true); rows[side].removeAll(keepingCapacity: true)
            for row in comparison.rows {
                let start = content.length, verses = side == 0 ? row.left : row.right
                if verses.isEmpty {
                    content.append(NSAttributedString(string: model.t("Nessun versetto corrispondente verificato.", "No verified matching verse.") + "\n", attributes: attributes.merging([.foregroundColor: NSColor.secondaryLabelColor]) { _, value in value }))
                }
                for verse in verses {
                    let a = content.length
                    content.append(NSAttributedString(string: verse.reference + "\n", attributes: [.font: NSFont.systemFont(ofSize: 12), .foregroundColor: NSColor.secondaryLabelColor, .paragraphStyle: bodyStyle]))
                    content.append(NSAttributedString(string: verse.text + "\n", attributes: attributes))
                    segments[side].append(Segment(range: NSRange(location: a, length: content.length-a), verse: verse))
                }
                content.append(NSAttributedString(string: "\n", attributes: attributes))
                rows[side].append(NSRange(location: start, length: content.length-start))
            }
            // A final character provides a native line position after the last
            // row, so the same measurement also covers the bottom of the chapter.
            content.append(NSAttributedString(string: "\u{200B}", attributes: attributes))
            documents[side].textStorage?.setAttributedString(content)
        }
        if changedChapter {
            focus = comparison.rows.firstIndex { $0.left.contains { $0.verse == model.position.verse } }
            rowY.removeAll()
        }
        key = next; width = 0; needsLayout = true
        updateButtons()
    }

    override func layout() {
        super.layout()
        let column = max(20, (bounds.width-24)/2), height = max(20, bounds.height-38)
        for side in 0..<2 {
            scrolls[side].frame = NSRect(x: CGFloat(side)*(column+24), y: 0, width: column, height: height)
            buttons[side].frame = NSRect(x: CGFloat(side)*(column+24)+12, y: height+6, width: 34, height: 26)
        }
        guard bounds.width > 80, !rows[0].isEmpty, width != column else { return }
        align(column: column, height: height)
    }
    private func y(_ side: Int, _ character: Int) -> CGFloat {
        guard let manager = documents[side].layoutManager else { return 0 }
        let glyph = manager.glyphIndexForCharacter(at: character)
        return manager.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil).minY
    }
    private func gap(_ side: Int, _ index: Int, _ extra: CGFloat) {
        let style = bodyStyle.mutableCopy() as! NSMutableParagraphStyle
        style.paragraphSpacing = 16+extra
        documents[side].textStorage?.addAttribute(.paragraphStyle, value: style, range: NSRange(location: NSMaxRange(rows[side][index])-1, length: 1))
    }
    private func ensureLayout() {
        for view in documents { if let container = view.textContainer { view.layoutManager?.ensureLayout(for: container) } }
    }
    private func align(column: CGFloat, height: CGFloat) {
        rebuilding = true; syncing = true
        defer { rebuilding = false; syncing = false }
        let old = scrolls[0].contentView.bounds.minY
        let anchor = rowY.lastIndex { $0 <= old } ?? 0
        let within = old-(rowY.indices.contains(anchor) ? rowY[anchor] : 0)
        for side in 0..<2 {
            let view = documents[side]
            view.setFrameSize(NSSize(width: column, height: max(height, view.frame.height)))
            view.textContainer?.containerSize = NSSize(width: max(20, column-24), height: CGFloat.greatestFiniteMagnitude)
            view.textStorage?.beginEditing()
            for i in rows[side].indices { gap(side, i, 0) }
            view.textStorage?.endEditing()
        }
        ensureLayout()
        let heights = (0..<2).map { side in rows[side].map { y(side, NSMaxRange($0))-y(side, $0.location) } }
        for side in 0..<2 {
            documents[side].textStorage?.beginEditing()
            for i in rows[side].indices { gap(side, i, max(0, heights[1-side][i]-heights[side][i])) }
            documents[side].textStorage?.endEditing()
        }
        ensureLayout()
        rowY = rows[0].map { y(0, $0.location)+8 }
        for view in documents {
            if let manager = view.layoutManager, let container = view.textContainer {
                view.setFrameSize(NSSize(width: column, height: max(height, manager.usedRect(for: container).maxY+16)))
            }
        }
        let row = min(focus ?? anchor, max(0, rowY.count-1))
        let target = (rowY.indices.contains(row) ? rowY[row] : 0) + (focus == nil ? max(0, within) : 0)
        for scroll in scrolls {
            scroll.contentView.scroll(to: NSPoint(x: 0, y: target)); scroll.reflectScrolledClipView(scroll.contentView)
        }
        if let focus {
            for side in 0..<2 { documents[side].setSelectedRange(NSRange(location: rows[side][focus].location, length: 0)) }
        }
        focus = nil; width = column; updateButtons()
    }
    @objc private func scrolled(_ note: Notification) {
        guard !syncing, let clip = note.object as? NSClipView else { return }
        syncing = true; defer { syncing = false }
        let other = scrolls[clip === scrolls[0].contentView ? 1 : 0]
        other.contentView.scroll(to: NSPoint(x: 0, y: clip.bounds.minY)); other.reflectScrolledClipView(other.contentView)
    }
    func textViewDidChangeSelection(_ notification: Notification) { if !rebuilding { updateButtons() } }
    private func updateButtons() { for i in 0..<2 { buttons[i].isEnabled = selectedVerse(i) != nil } }
    func selectedVerse(_ side: Int) -> ComparedVerse? {
        let at = documents[side].selectedRange().location
        return segments[side].first { NSLocationInRange(at, $0.range) }?.verse
    }
    @objc func openCompared(_ sender: Any?) {
        let side = (sender as? NSControl)?.tag ?? (sender as? NSMenuItem)?.tag ?? 0
        open(side)
    }
    func open(_ side: Int) { if let verse = selectedVerse(side) { model?.openComparedVerse(verse) } }
    func alignedRowOffsets() -> [[CGFloat]] { (0..<2).map { side in rows[side].map { y(side, $0.location) } } }
}
