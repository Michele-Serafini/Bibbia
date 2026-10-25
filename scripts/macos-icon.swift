// Vector source for the macOS icon; rendered at build time using only AppKit.
import AppKit

let output = CommandLine.arguments[1]
let bitmap = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: 1024, pixelsHigh: 1024,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: bitmap)
let tile = NSBezierPath(roundedRect: NSRect(x: 66, y: 66, width: 892, height: 892), xRadius: 188, yRadius: 188)
let shadow = NSShadow(); shadow.shadowColor = NSColor.black.withAlphaComponent(0.24)
shadow.shadowBlurRadius = 26; shadow.shadowOffset = NSSize(width: 0, height: -10)
NSGraphicsContext.saveGraphicsState(); shadow.set()
NSColor(calibratedRed: 0.08, green: 0.17, blue: 0.25, alpha: 1).setFill(); tile.fill()
NSGraphicsContext.restoreGraphicsState()
NSGradient(starting: NSColor(calibratedRed: 0.15, green: 0.30, blue: 0.38, alpha: 1),
           ending: NSColor(calibratedRed: 0.05, green: 0.12, blue: 0.21, alpha: 1))!.draw(in: tile, angle: -90)
let pages = NSBezierPath(roundedRect: NSRect(x: 286, y: 239, width: 466, height: 548), xRadius: 36, yRadius: 36)
NSColor(calibratedWhite: 0.95, alpha: 1).setFill(); pages.fill()
let cover = NSBezierPath(roundedRect: NSRect(x: 262, y: 266, width: 468, height: 542), xRadius: 34, yRadius: 34)
NSColor(calibratedRed: 0.25, green: 0.46, blue: 0.53, alpha: 1).setFill(); cover.fill()
let spine = NSBezierPath(roundedRect: NSRect(x: 262, y: 266, width: 64, height: 542), xRadius: 30, yRadius: 30)
NSColor(calibratedRed: 0.20, green: 0.37, blue: 0.45, alpha: 1).setFill(); spine.fill()
NSColor(calibratedRed: 0.94, green: 0.79, blue: 0.45, alpha: 1).setFill()
NSBezierPath(roundedRect: NSRect(x: 497, y: 438, width: 38, height: 242), xRadius: 10, yRadius: 10).fill()
NSBezierPath(roundedRect: NSRect(x: 426, y: 570, width: 180, height: 38), xRadius: 10, yRadius: 10).fill()
NSColor(calibratedWhite: 0.72, alpha: 1).setStroke()
for y in [252.0, 261.0] {
    let line = NSBezierPath(); line.move(to: NSPoint(x: 326, y: y)); line.line(to: NSPoint(x: 718, y: y))
    line.lineWidth = 2; line.stroke()
}
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using: .png, properties: [:])!.write(to: URL(fileURLWithPath: output))
