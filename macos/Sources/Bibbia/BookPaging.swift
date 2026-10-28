import AppKit
import BibleKit

struct ReaderPlace: Codable {
    var bookMode = true
    var edition = ""
    var book = 0, chapter = 0, verse = 0, offset = 0
}

@MainActor
final class BookPager {
    struct Page { let y: CGFloat; let character: Int }
    weak var model: AppModel?
    weak var surface: BookSurface?
    var second: ReadingTextView?
    let right = NSScrollView()
    var pages = [Page(y: 0, character: 0)]
    var index = 0, columns = 1
    var lastChapter: String = ""
    private var seekTarget: Int?
    private struct LayoutStamp:Equatable {let size:CGSize;let revision:Int;let book:Bool}
    private var stamp:LayoutStamp?
    var anchor = 0
    var place = ReaderPlace()
    var loaded = false, restored = false, updating = false, lastPage = false
    var pageCount: Int { pages.count }
    var mode: Bool { model?.bookMode ?? false }

    func configure(_ surface: BookSurface, _ primary: ReadingTextView, _ model: AppModel, target: Int?) {
        self.surface = surface; self.model = model; model.readingSurface = surface
        if !loaded {
            if let data = try? Data(contentsOf: model.directory.appendingPathComponent("reader.json")),
               let value = try? JSONDecoder().decode(ReaderPlace.self, from: data) { place = value }
            loaded = true
        }
        second?.adoptContext(primary)
        let key = "\(model.position.edition)/\(model.position.book)/\(model.position.chapter)"
        let seek=lastChapter != key || (target != nil && target != seekTarget)
        if seek {
            let verse = min(target ?? model.position.verse, max(0, primary.readingLayout.spans.count-1))
            anchor = primary.readingLayout.spans.indices.contains(verse) ? primary.readingLayout.spans[verse].location : 0
            if !restored, !primary.readingLayout.spans.isEmpty, place.edition == model.position.edition, place.book == model.position.book,
               place.chapter == model.position.chapter, place.verse == verse,
               primary.readingLayout.spans.indices.contains(verse) {
                anchor += min(place.offset, primary.readingLayout.spans[verse].length)
            }
            if !primary.readingLayout.spans.isEmpty { restored = true }
            lastChapter = key
        }
        seekTarget = target
        reflow(seek:seek)
    }

    func reflow(seek:Bool=false) {
        guard !updating, let surface, let model, let primary = surface.scroll.documentView as? ReadingTextView,
              surface.bounds.width > 60, surface.bounds.height > 60 else { return }
        updating = true; defer { updating = false }
        let next=LayoutStamp(size:surface.bounds.size,revision:primary.layoutRevision,book:mode)
        if stamp==next {
            if seek { index=max(0,(pages.lastIndex{$0.character<=anchor} ?? 0)/columns*columns);if lastPage{index=(pages.count-1)/columns*columns};show() }
            return
        }
        stamp=next
        let height = surface.bounds.height
        columns = mode && surface.bounds.width >= 1040 ? 2 : 1
        if columns==2,second==nil,let storage=primary.textStorage {
            let manager=NSLayoutManager(),container=NSTextContainer(size:NSSize(width:600,height:CGFloat.greatestFiniteMagnitude))
            storage.addLayoutManager(manager);manager.addTextContainer(container)
            let view=ReadingTextView(frame:.zero,textContainer:container)
            view.configureForReading();view.adoptContext(primary);view.setAccessibilityIdentifier("bibleTextRight")
            right.documentView=view;right.drawsBackground=false;right.borderType = .noBorder
            surface.addSubview(right);second=view
        }else if columns==1,let view=second {
            view.closePopover()
            if let manager=view.layoutManager {view.textStorage?.removeLayoutManager(manager)}
            right.documentView=nil;right.removeFromSuperview();second=nil
        }
        let width = (surface.bounds.width-CGFloat(columns-1)*24)/CGFloat(columns)
        let views = [primary, second].compactMap { $0 }
        for (n, view) in views.enumerated() {
            let scroll = n == 0 ? surface.scroll : right
            scroll.isHidden = n >= columns
            scroll.hasVerticalScroller = !mode
            scroll.frame = NSRect(x: CGFloat(n)*(width+24), y: 0, width: width, height: height)
            // In book mode the pager owns the column width. A later document
            // height change must not let NSTextView change line wrapping after
            // we have measured the page breaks.
            view.textContainer?.widthTracksTextView = !mode
            view.setFrameSize(NSSize(width: width, height: max(height, view.frame.height)))
            view.textContainer?.containerSize = NSSize(width: max(20,width-56), height: CGFloat.greatestFiniteMagnitude)
            view.layoutManager?.ensureLayout(for: view.textContainer!)
        }
        guard mode, let manager = primary.layoutManager, let container = primary.textContainer else {
            right.isHidden = true; primary.sizeToFit(); publish(""); return
        }
        pages = [Page(y: 0, character: 0)]
        let range = manager.glyphRange(for: container)
        var start: CGFloat = 0
        manager.enumerateLineFragments(forGlyphRange: range) { rect, used, _, glyphs, _ in
            let character = manager.characterIndexForGlyph(at: glyphs.location)
            guard character < max(0,primary.string.utf16.count-1) else { return }
            let y = rect.minY + primary.textContainerInset.height
            if y + max(rect.height,used.height) > start+height-6, y > start+1 {
                self.pages.append(Page(y: y, character: character)); start = y
            }
        }
        let textHeight = manager.usedRect(for: container).maxY + primary.textContainerInset.height
        for view in views { view.setFrameSize(NSSize(width: width, height: max(height,textHeight+height))) }
        index = max(0, (pages.lastIndex { $0.character <= anchor } ?? 0)/columns*columns)
        if lastPage { index = (pages.count-1)/columns*columns }
        show()
        _ = model
    }

    func show() {
        guard let surface, mode else { return }
        let height = surface.bounds.height
        for (n, scroll) in [surface.scroll, right].enumerated() {
            let p = index+n
            scroll.isHidden = n >= columns || p >= pages.count
            guard p < pages.count else { continue }
            let visible = p+1 < pages.count ? min(height,pages[p+1].y-pages[p].y) : height
            scroll.setFrameSize(NSSize(width: scroll.frame.width, height: max(1,visible)))
            scroll.contentView.scroll(to: NSPoint(x: 0, y: pages[p].y))
            scroll.reflectScrolledClipView(scroll.contentView)
        }
        anchor = pages[index].character
        publish("\(index+1)\(columns == 2 && index+1 < pages.count ? "–\(index+2)" : "") / \(pages.count)")
    }
    private func publish(_ text: String) {
        guard let model, model.pageInfo != text else { return }
        DispatchQueue.main.async { [weak model] in model?.pageInfo = text }
    }
    func turn(_ delta: Int) -> Bool {
        guard mode, let surface, let model, !updating else { return false }
        let target = index+delta*columns
        guard target >= 0, target < pages.count else { lastPage = delta < 0; return false }
        (surface.scroll.documentView as? ReadingTextView)?.closePopover(); second?.closePopover()
        let image = surface.capture()
        index = target; show(); save()
        surface.turn(image, forward: delta > 0)
        _ = model
        return true
    }
    func save() {
        guard let model, let view = surface?.scroll.documentView as? ReadingTextView else { return }
        let verse = view.readingLayout.spans.firstIndex { NSMaxRange($0) > anchor }
            ?? max(0, view.readingLayout.spans.count-1)
        place = ReaderPlace(bookMode: model.bookMode, edition: model.position.edition, book: model.position.book,
                            chapter: model.position.chapter, verse: verse,
                            offset: max(0,anchor-(view.readingLayout.spans.indices.contains(verse) ? view.readingLayout.spans[verse].location : 0)))
        if mode { model.selectedVerse(verse) }
        if let data = try? JSONEncoder().encode(place) { try? data.write(to: model.directory.appendingPathComponent("reader.json"), options: .atomic) }
    }
}
