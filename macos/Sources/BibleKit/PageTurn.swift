import Foundation

/// A developable sheet rolling around a diagonal cylinder. The top outside
/// corner lifts first, then the fold travels down the page. Affine bands share
/// their endpoints exactly, including on native software renderers.
public enum PageTurn {
    public static let strips = 64
    public static let samples = 60
    public static let duration = 0.62
    public static let angle = -0.42

    public struct Layout: Sendable {
        public let nx, ny, minU, spanU, minV, spanV: Double
        public var step: Double { spanU / Double(strips) }
    }
    public struct Strip: Sendable {
        public let source, x, y, scaleX, scaleY, front, shadeStart, shadeEnd: Double
    }
    public struct Shadow: Sendable {
        public let x, y, opacity: Double
    }
    public struct Sheet: Sendable {
        public let layout: Layout
        public let strips: [Strip]
        public let shadow: Shadow
    }
    public struct Point: Sendable {
        public let x, y, z: Double
    }

    private static func paper(_ width: Double, _ height: Double) -> Layout {
        Layout(nx: cos(angle), ny: sin(angle), minU: sin(angle) * height,
               spanU: cos(angle) * width - sin(angle) * height,
               minV: 0, spanV: -sin(angle) * width + cos(angle) * height)
    }
    private static func ease(_ progress: Double) -> Double {
        let t = min(1, max(0, progress.isFinite ? progress : 0))
        return t * t * (3 - 2 * t)
    }
    private static func bend(_ u: Double, _ fold: Double, _ radius: Double)
        -> (q: Double, z: Double, cosine: Double, shade: Double) {
        let d = u - fold
        guard d > 0 else { return (u, 0, 1, 0) }
        let a = min(.pi, d / radius)
        return (fold + radius * sin(a) - max(0, d - .pi * radius),
                radius * (1 - cos(a)), cos(a), 0.24 * abs(sin(a)) + (a > .pi / 2 ? 0.035 : 0))
    }
    public static func frame(width: Double, height: Double, progress: Double, forward: Bool) -> Sheet {
        guard width.isFinite, height.isFinite, width > 0, height > 0 else {
            return Sheet(layout: paper(0, 0), strips: [], shadow: Shadow(x: 0, y: 0, opacity: 0))
        }
        let l = paper(width, height), p = ease(progress)
        let radius = width * (0.075 + 0.065 * sin(.pi * p))
        let fold = l.minU + l.spanU - (l.spanU + width * 0.88) * p
        let bands = (0..<strips).map { i in
            let u = l.minU + Double(i) * l.step
            let a = bend(u, fold, radius), b = bend(u + l.step, fold, radius)
            let middle = bend(u + l.step / 2, fold, radius)
            let x = l.nx * a.q - l.ny * l.minV
            return Strip(source: u, x: forward ? x : width - x, y: l.ny * a.q + l.nx * l.minV,
                         scaleX: (b.q - a.q) / l.step, scaleY: forward ? 1 : -1,
                         front: middle.cosine >= 0 ? 1 : 0.06, shadeStart: a.shade, shadeEnd: b.shade)
        }
        let shadowU = fold - radius * 0.12
        let shadowX = l.nx * shadowU - l.ny * l.minV
        return Sheet(layout: l, strips: bands, shadow: Shadow(x: forward ? shadowX : width - shadowX,
            y: l.ny * shadowU + l.nx * l.minV, opacity: sin(.pi * p)))
    }
    public static func project(width: Double, height: Double, x: Double, y: Double,
                               progress: Double, forward: Bool) -> Point {
        let l = paper(width, height), p = ease(progress)
        let x = forward ? x : width - x
        let u = l.nx * x + l.ny * y, v = -l.ny * x + l.nx * y
        let b = bend(u, l.minU + l.spanU - (l.spanU + width * 0.88) * p,
                     width * (0.075 + 0.065 * sin(.pi * p)))
        let px = l.nx * b.q - l.ny * v
        return Point(x: forward ? px : width - px, y: l.ny * b.q + l.nx * v, z: b.z)
    }
}
