import Foundation

/// A cylindrical paper bend projected into narrow, contiguous strips. Keeping
/// the projection affine per strip also works on native software renderers.
public enum PageTurn {
    public static let strips = 32
    public static let duration = 0.46

    public struct Strip: Sendable {
        public let source: Double
        public let x: Double
        public let y: Double
        public let scaleX: Double
        public let scaleY: Double
        public let front: Double
        public let shade: Double
    }

    public static func frame(width: Double, height: Double, progress: Double, forward: Bool) -> [Strip] {
        guard width > 0, height > 0 else { return [] }
        let t = min(1, max(0, progress))
        let p = t * t * t * (t * (t * 6 - 15) + 10)
        let bend = 0.65 * sin(.pi * p)
        let step = width / Double(strips)
        let depth = width * 6
        var x = 0.0, z = 0.0
        var result: [Strip] = []
        for i in 0..<strips {
            let angle = .pi * p + bend * (2 * (Double(i) + 0.5) / Double(strips) - 1)
            let nextX = x + step * cos(angle)
            let nextZ = z + step * sin(angle)
            let a = depth / (depth - z), b = depth / (depth - nextZ)
            let left = width / 2 + (x - width / 2) * a
            let right = width / 2 + (nextX - width / 2) * b
            let sy = (a + b) / 2
            result.append(Strip(source: forward ? Double(i) * step : width - Double(i + 1) * step,
                x: forward ? left : width - right, y: height * (1 - sy) / 2,
                scaleX: (right - left) / step, scaleY: sy,
                front: cos(angle) >= 0 ? 1 : 0, shade: 0.18 * abs(sin(angle))))
            x = nextX; z = nextZ
        }
        return result
    }
}
