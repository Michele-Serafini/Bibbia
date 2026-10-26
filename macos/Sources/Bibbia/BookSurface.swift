import AppKit
import QuartzCore
import BibleKit

/// Core Animation owns the keyframes. The main thread has no per-frame timer,
/// and all strips share one bounded image of the outgoing visible page.
@MainActor
final class BookSurface: NSView {
    let scroll = NSScrollView()
    private var image: CGImage?
    private var overlay: CALayer?
    private var token = 0
    var generation: Int { token }
    private(set) var isTurning = false
    private(set) var turnsStarted = 0
    var position: Position?
    private var motionObserver: NSObjectProtocol?
    var allowsMotion = true { didSet { if !allowsMotion { cancelTurn() } } }

    override var isFlipped: Bool { true }
    static var paperColor: NSColor {
        NSColor(name: nil) { appearance in
            appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua
                ? NSColor(srgbRed: 0.141, green: 0.137, blue: 0.125, alpha: 1)
                : NSColor(srgbRed: 1, green: 0.992, blue: 0.969, alpha: 1)
        }
    }
    var motionAllowed: Bool {
        allowsMotion && (!NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ||
            (ProcessInfo.processInfo.arguments.contains("--ui-smoke") && ProcessInfo.processInfo.environment["BIBBIA_UI_TEST_MOTION"] == "1"))
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
        if scroll.frame.size != bounds.size { cancelTurn() }
        scroll.frame = bounds
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
        scroll.cacheDisplay(in: scroll.bounds, to: bitmap)
        return bitmap.cgImage
    }

    func turn(_ snapshot: CGImage?, forward: Bool) {
        cancelTurn()
        guard motionAllowed, let snapshot, let root = layer, window?.isVisible == true else { return }
        image = snapshot
        let epoch = token
        let canvas = CALayer()
        canvas.frame = bounds; canvas.masksToBounds = true
        root.addSublayer(canvas); overlay = canvas
        isTurning = true; turnsStarted += 1
        let width = bounds.width, height = bounds.height
        let samples = (0...32).map { PageTurn.frame(width: width, height: height,
                                                 progress: Double($0) / 32, forward: forward) }
        let flat = samples[0]
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        CATransaction.setCompletionBlock { [weak self] in
            MainActor.assumeIsolated { if self?.token == epoch { self?.cancelTurn() } }
        }
        let shadow = CAGradientLayer()
        shadow.frame = CGRect(x: forward ? 0 : width * 0.64, y: 0, width: width * 0.36, height: height)
        shadow.colors = [NSColor.black.withAlphaComponent(0.18).cgColor, NSColor.clear.cgColor]
        shadow.startPoint = CGPoint(x: forward ? 0 : 1, y: 0.5)
        shadow.endPoint = CGPoint(x: forward ? 1 : 0, y: 0.5)
        shadow.opacity = 0; canvas.addSublayer(shadow)
        animate(shadow, key: "opacity", values: (0...32).map { NSNumber(value: sin(.pi * Double($0) / 32)) })
        for i in 0..<PageTurn.strips {
            let s = flat[i]
            let slice = CALayer()
            slice.bounds = CGRect(x: 0, y: 0, width: width / Double(PageTurn.strips) + 0.7, height: height)
            slice.anchorPoint = .zero
            slice.position = CGPoint(x: s.x, y: s.y)
            slice.backgroundColor = Self.paperColor.cgColor
            slice.masksToBounds = true
            canvas.addSublayer(slice)
            let front = CALayer()
            front.frame = slice.bounds
            front.contents = snapshot
            front.contentsRect = CGRect(x: s.source / width, y: 0, width: slice.bounds.width / width, height: 1)
            front.contentsGravity = .resize
            front.magnificationFilter = .linear
            slice.addSublayer(front)
            let shade = CALayer()
            shade.frame = slice.bounds; shade.backgroundColor = NSColor.black.cgColor; shade.opacity = 0
            slice.addSublayer(shade)
            animate(slice, key: "position", values: samples.map { NSValue(point: NSPoint(x: $0[i].x, y: $0[i].y)) })
            animate(slice, key: "transform", values: samples.map {
                NSValue(caTransform3D: CATransform3DMakeScale($0[i].scaleX, $0[i].scaleY, 1))
            })
            animate(front, key: "opacity", values: samples.map { NSNumber(value: $0[i].front) })
            animate(shade, key: "opacity", values: samples.map { NSNumber(value: $0[i].shade) })
        }
        CATransaction.commit()
    }

    private func animate(_ layer: CALayer, key: String, values: [Any]) {
        let animation = CAKeyframeAnimation(keyPath: key)
        animation.values = values; animation.duration = PageTurn.duration
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
        guard let live = overlay?.presentation(), isTurning else { throw BibleError.invalid("No active paper animation") }
        let width = max(1, Int(bounds.width)), height = max(1, Int(bounds.height))
        guard let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: width, pixelsHigh: height,
            bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
            colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0),
              let context = NSGraphicsContext(bitmapImageRep: bitmap) else { throw BibleError.invalid("No frame context") }
        bitmap.size = bounds.size
        scroll.cacheDisplay(in: scroll.bounds, to: bitmap)
        context.cgContext.translateBy(x: 0, y: bounds.height)
        context.cgContext.scaleBy(x: 1, y: -1)
        live.render(in: context.cgContext)
        guard let png = bitmap.representation(using: .png, properties: [:]) else { throw BibleError.invalid("No motion frame") }
        try FileManager.default.createDirectory(at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        try png.write(to: url)
    }
}
