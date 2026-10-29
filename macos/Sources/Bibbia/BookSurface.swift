import AppKit
import QuartzCore
import BibleKit

/// Core Animation owns the keyframes. The main thread has no per-frame timer,
/// and all strips share one bounded image of the outgoing visible page.
@MainActor
final class BookSurface: NSView {
    let scroll = NSScrollView()
    let pager = BookPager()
    private var wheelDistance: CGFloat = 0
    private var wheelTime: TimeInterval = 0
    func pageWheel(_ delta: CGFloat) {
        wheelDistance += delta
        if abs(wheelDistance) > 35, ProcessInfo.processInfo.systemUptime-wheelTime > 0.35 {
            pager.model?.step(wheelDistance < 0 ? 1 : -1)
            wheelDistance = 0; wheelTime = ProcessInfo.processInfo.systemUptime
        }
    }
    private var allocatedSize = CGSize.zero
    private var image: CGImage?
    private var overlay: CALayer?
    private var restingCornerX: CGFloat = 0
    private var token = 0
    var generation: Int { token }
    private(set) var isTurning = false
    private(set) var turnsStarted = 0
    var position: Position?
    private var motionObserver: NSObjectProtocol?
    var allowsMotion = true { didSet { if !allowsMotion { cancelTurn() } } }
    var hasCurledPresentation: Bool {
        guard isTurning, let corner = overlay?.sublayers?.last?.presentation() else { return false }
        return abs(corner.transform.m11-restingCornerX) > 0.02
    }

    override var isFlipped: Bool { true }
    var sepia = false { didSet { if sepia != oldValue { cancelTurn() } } }
    static let sepiaPaper = NSColor(srgbRed: 244/255, green: 231/255, blue: 205/255, alpha: 1)
    static func paperColor(sepia: Bool = false) -> NSColor {
        if sepia { return sepiaPaper }
        return NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                ? NSColor(srgbRed: 0.141, green: 0.137, blue: 0.125, alpha: 1)
                : NSColor(srgbRed: 1, green: 0.992, blue: 0.969, alpha: 1)
        }
    }
    var motionAllowed: Bool {
        allowsMotion && (pager.model?.performanceMotion ?? true) && (!NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ||
            ((ProcessInfo.processInfo.arguments.contains("--ui-smoke") || AppModel.performanceProfile() != nil) && ProcessInfo.processInfo.environment["BIBBIA_UI_TEST_MOTION"] == "1"))
    }
    private var paperCGColor: CGColor {
        var color = CGColor(gray: 1, alpha: 1)
        effectiveAppearance.performAsCurrentDrawingAppearance { color = Self.paperColor(sepia: sepia).cgColor }
        return color
    }

    override init(frame: NSRect) {
        super.init(frame: frame)
        wantsLayer = true
        layer?.masksToBounds = true
        scroll.hasVerticalScroller = true; scroll.autohidesScrollers = true
        scroll.borderType = .noBorder; scroll.drawsBackground = false
        addSubview(scroll)
    }
    required init?(coder: NSCoder) { fatalError("init(coder:) is not used") }

    override func layout() {
        super.layout()
        if allocatedSize != bounds.size { cancelTurn(); allocatedSize = bounds.size }
        if pager.model != nil { pager.reflow() } else { scroll.frame = bounds }
    }
    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            cancelTurn()
            if let observer = motionObserver { NSWorkspace.shared.notificationCenter.removeObserver(observer); motionObserver = nil }
        } else if motionObserver == nil {
            motionObserver = NSWorkspace.shared.notificationCenter.addObserver(
                forName: NSWorkspace.accessibilityDisplayOptionsDidChangeNotification, object: nil, queue: .main
            ) { [weak self] _ in
                MainActor.assumeIsolated { if self?.motionAllowed == false { self?.cancelTurn() } }
            }
        }
    }
    override func viewDidChangeEffectiveAppearance() {
        super.viewDidChangeEffectiveAppearance()
        cancelTurn()
    }

    func capture() -> CGImage? {
        cancelTurn()
        guard motionAllowed, window?.isVisible == true, bounds.width > 20, bounds.height > 20 else { return nil }
        let scale = min(window?.backingScaleFactor ?? 1, 1600 / bounds.width,
                        sqrt(2_000_000 / (bounds.width * bounds.height)))
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil,
            pixelsWide: max(1, Int(bounds.width * scale)), pixelsHigh: max(1, Int(bounds.height * scale)),
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0) else { return nil }
        bitmap.size = bounds.size
        cacheDisplay(in: bounds, to: bitmap)
        return bitmap.cgImage
    }

    func turn(_ snapshot: CGImage?, forward: Bool) {
        cancelTurn()
        guard motionAllowed, let snapshot, let root = layer, window?.isVisible == true else { return }
        image = snapshot
        let epoch = token
        let canvas = CALayer()
        canvas.frame = bounds; canvas.masksToBounds = true
        // AppKit manages the NSView's backing layer. Our standalone containers
        // must explicitly use top-left coordinates for the corner and bitmap.
        canvas.isGeometryFlipped = true
        root.addSublayer(canvas); overlay = canvas
        isTurning = true; turnsStarted += 1
        let width = bounds.width, height = bounds.height
        let samples = (0...PageTurn.samples).map { PageTurn.frame(width: width, height: height,
            progress: Double($0) / Double(PageTurn.samples), forward: forward) }
        let l = samples[0].layout
        let sign = forward ? 1.0 : -1.0
        restingCornerX = sign*l.nx
        func transform(_ sx: Double, _ sy: Double) -> CATransform3D {
            // The cached AppKit bitmap and standalone CALayer geometry have
            // opposite vertical origins. Reflect both source and destination;
            // flat text stays unchanged and the physical upper corner lifts.
            CATransform3DMakeAffineTransform(CGAffineTransform(
                a: sign*l.nx*sx, b: -l.ny*sx, c: -sign*l.ny*sy, d: -l.nx*sy, tx: 0, ty: 0))
        }
        let paper = paperCGColor
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        CATransaction.setCompletionBlock { [weak self] in
            MainActor.assumeIsolated { if self?.token == epoch { self?.cancelTurn() } }
        }
        let shadow = CAGradientLayer()
        shadow.bounds = CGRect(x: 0, y: 0, width: width*0.30, height: l.spanV)
        shadow.anchorPoint = .zero
        shadow.position = CGPoint(x: samples[0].shadow.x, y: height-samples[0].shadow.y)
        shadow.transform = transform(1, 1)
        shadow.colors = [NSColor.clear.cgColor, NSColor.black.withAlphaComponent(0.10).cgColor,
                         NSColor.black.withAlphaComponent(0.22).cgColor, NSColor.clear.cgColor]
        shadow.locations = [0, 0.40, 0.68, 1]
        shadow.startPoint = CGPoint(x: 0, y: 0.5); shadow.endPoint = CGPoint(x: 1, y: 0.5)
        shadow.opacity = 0; canvas.addSublayer(shadow)
        animate(shadow, key: "position", values: samples.map { NSValue(point: NSPoint(x: $0.shadow.x, y: height-$0.shadow.y)) })
        animate(shadow, key: "opacity", values: samples.map { NSNumber(value: $0.shadow.opacity) })
        for i in 0..<PageTurn.strips {
            let s = samples[0].strips[i]
            let slice = CALayer()
            slice.bounds = CGRect(x: 0, y: 0, width: l.step+0.65, height: l.spanV)
            slice.anchorPoint = .zero
            slice.position = CGPoint(x: s.x, y: height-s.y)
            slice.transform = transform(s.scaleX, 1)
            slice.masksToBounds = true
            canvas.addSublayer(slice)
            let source = CALayer()
            source.bounds = CGRect(x: 0, y: 0, width: width, height: height)
            source.anchorPoint = .zero
            source.position = CGPoint(x: (forward ? 0 : l.nx*width)-s.source+l.ny*height,
                                      y: (forward ? 0 : -l.ny*width)-l.minV+l.nx*height)
            source.setAffineTransform(CGAffineTransform(a: sign*l.nx, b: -sign*l.ny, c: -l.ny, d: -l.nx, tx: 0, ty: 0))
            source.backgroundColor = paper; source.masksToBounds = true
            slice.addSublayer(source)
            let front = CALayer()
            front.frame = source.bounds; front.contents = snapshot; front.contentsGravity = .resize
            front.magnificationFilter = .linear
            source.addSublayer(front)
            let sx = l.nx*s.source-l.ny*l.minV, sy = l.ny*s.source+l.nx*l.minV
            let ex = sx+l.nx*l.step, ey = sy+l.ny*l.step
            func shade(_ reverse: Bool) -> CAGradientLayer {
                let gradient = CAGradientLayer()
                gradient.frame = source.bounds
                gradient.startPoint = CGPoint(x: (forward ? sx : width-sx)/width, y: 1-sy/height)
                gradient.endPoint = CGPoint(x: (forward ? ex : width-ex)/width, y: 1-ey/height)
                gradient.colors = reverse ? [NSColor.clear.cgColor, NSColor.black.cgColor]
                                          : [NSColor.black.cgColor, NSColor.clear.cgColor]
                gradient.opacity = 0; source.addSublayer(gradient)
                return gradient
            }
            let shadeStart = shade(false), shadeEnd = shade(true)
            animate(slice, key: "position", values: samples.map { NSValue(point: NSPoint(x: $0.strips[i].x, y: height-$0.strips[i].y)) })
            animate(slice, key: "transform", values: samples.map { NSValue(caTransform3D: transform($0.strips[i].scaleX, 1)) })
            animate(front, key: "opacity", values: samples.map { NSNumber(value: $0.strips[i].front) })
            animate(shadeStart, key: "opacity", values: samples.map { NSNumber(value: $0.strips[i].shadeStart) })
            animate(shadeEnd, key: "opacity", values: samples.map { NSNumber(value: $0.strips[i].shadeEnd) })
        }
        CATransaction.commit()
    }

    private func animate(_ layer: CALayer, key: String, values: [Any]) {
        let animation = CAKeyframeAnimation(keyPath: key)
        animation.values = values
        // The disposable UI fixture slows the same compositor keyframes so
        // WindowServer capture can show the early fold on loaded CI machines.
        // Ordinary launches always use the shared 1100 ms duration.
        let recording = ProcessInfo.processInfo.arguments.contains("--ui-smoke") &&
            ProcessInfo.processInfo.environment["BIBBIA_UI_TEST_MOTION"] == "1"
        animation.duration = recording ? 1.8 : PageTurn.duration
        animation.calculationMode = .linear
        animation.timingFunction = CAMediaTimingFunction(name: .linear)
        animation.isRemovedOnCompletion = false; animation.fillMode = .forwards
        layer.add(animation, forKey: key)
    }

    func cancelTurn() {
        token += 1
        overlay?.removeAllAnimations()
        overlay?.removeFromSuperlayer(); overlay = nil; image = nil; isTurning = false
    }

    func saveMotionFrame(to url: URL) throws {
        guard let overlay, isTurning else {
            throw BibleError.invalid("No active paper animation")
        }
        // Verify the compositor is actually deforming the corner, rather than
        // merely waiting for the completion callback with a static bitmap.
        guard hasCurledPresentation else {
            throw BibleError.invalid("Corner presentation has not curled")
        }
        // Freeze each presentation layer separately. Rendering only the parent
        // presentation layer can otherwise capture its children's model poses.
        func freeze(_ model: CALayer) -> CALayer {
            let live = model.presentation() ?? model
            let copy: CALayer
            if let gradient = live as? CAGradientLayer {
                let clone = CAGradientLayer()
                clone.colors = gradient.colors; clone.locations = gradient.locations
                clone.startPoint = gradient.startPoint; clone.endPoint = gradient.endPoint
                copy = clone
            } else { copy = CALayer() }
            copy.bounds = live.bounds; copy.anchorPoint = live.anchorPoint
            copy.position = live.position; copy.transform = live.transform
            copy.opacity = live.opacity; copy.isHidden = live.isHidden
            copy.masksToBounds = live.masksToBounds; copy.isGeometryFlipped = live.isGeometryFlipped
            copy.contents = live.contents; copy.contentsRect = live.contentsRect
            copy.contentsGravity = live.contentsGravity; copy.contentsScale = live.contentsScale
            copy.backgroundColor = live.backgroundColor
            copy.sublayers = model.sublayers?.map { freeze($0) }
            return copy
        }
        let live = freeze(overlay)
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        // Also retain the actual WindowServer frame when screen capture is
        // available in the disposable native CI session.
        if let window {
            let capture = Process()
            capture.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
            capture.arguments = ["-x", "-l", String(window.windowNumber),
                url.deletingLastPathComponent().appendingPathComponent("macos-page-turn-desktop.png").path]
            try? capture.run()
            if capture.isRunning { capture.waitUntilExit() }
        }
        let width = max(1, Int(bounds.width)), height = max(1, Int(bounds.height))
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
              let context = NSGraphicsContext(bitmapImageRep: bitmap) else { throw BibleError.invalid("No frame context") }
        bitmap.size = bounds.size
        scroll.cacheDisplay(in: scroll.bounds, to: bitmap)
        context.cgContext.saveGState()
        context.cgContext.setBlendMode(.destinationOver)
        context.cgContext.setFillColor(paperCGColor)
        context.cgContext.fill(bounds)
        context.cgContext.restoreGState()
        context.cgContext.translateBy(x: 0, y: bounds.height)
        context.cgContext.scaleBy(x: 1, y: -1)
        live.render(in: context.cgContext)
        guard let png = bitmap.representation(using: .png, properties: [:]) else { throw BibleError.invalid("No motion frame") }
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try png.write(to: url)
    }
}
