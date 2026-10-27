import Foundation
import Testing
import BibleKit

@Test func cornerLiftsBeforeTheBottomAndReverseIsMirrored() {
    let top = PageTurn.project(width: 800, height: 600, x: 800, y: 0, progress: 0.22, forward: true)
    let bottom = PageTurn.project(width: 800, height: 600, x: 800, y: 600, progress: 0.22, forward: true)
    #expect(top.z > 30 && top.x < 790 && top.y > 0)
    #expect(abs(bottom.x - 800) < 0.001 && abs(bottom.y - 600) < 0.001 && bottom.z == 0)
    for step in 0...100 {
        let t = Double(step) / 100
        let a = PageTurn.project(width: 800, height: 600, x: 725, y: 180, progress: t, forward: true)
        let b = PageTurn.project(width: 800, height: 600, x: 75, y: 180, progress: t, forward: false)
        #expect(abs(a.x + b.x - 800) < 0.001 && abs(a.y - b.y) < 0.001 && abs(a.z - b.z) < 0.001)
    }
}

@Test func diagonalPaperHasNoGapsAndLeavesTheViewport() {
    for (w, h) in [(800.0, 600.0), (420.0, 900.0), (1400.0, 400.0)] {
        for forward in [false, true] {
            for step in 0...100 {
                let f = PageTurn.frame(width: w, height: h, progress: Double(step) / 100, forward: forward)
                let l = f.layout, sign = forward ? 1.0 : -1.0
                #expect(f.strips.count == PageTurn.strips)
                for (a, b) in zip(f.strips, f.strips.dropFirst()) {
                    #expect(a.x.isFinite && a.y.isFinite && a.scaleX.isFinite)
                    #expect(abs(a.x + sign*l.nx*l.step*a.scaleX - b.x) < 0.001)
                    #expect(abs(a.y + l.ny*l.step*a.scaleX - b.y) < 0.001)
                    #expect(a.shadeStart >= 0 && a.shadeEnd <= 0.28)
                }
            }
            for x in [0.0, w/2, w] {
                for y in [0.0, h/2, h] {
                    let flat = PageTurn.project(width: w, height: h, x: x, y: y, progress: 0, forward: forward)
                    #expect(abs(flat.x - x) < 0.001 && abs(flat.y - y) < 0.001)
                    let end = PageTurn.project(width: w, height: h, x: x, y: y, progress: 1, forward: forward)
                    #expect(end.x < 0 || end.x > w || end.y < 0 || end.y > h)
                }
            }
        }
    }
    #expect(PageTurn.frame(width: 0, height: 600, progress: 0.5, forward: true).strips.isEmpty)
}
