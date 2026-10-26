import Foundation
import Testing
import BibleKit

@Test func paperBendsContinuouslyInBothDirections() {
    for forward in [false, true] {
        let flat = PageTurn.frame(width: 800, height: 600, progress: 0, forward: forward)
        for strip in flat {
            #expect(abs(strip.x - strip.source) < 0.001)
            #expect(abs(strip.scaleX - 1) < 0.001 && strip.front == 1)
        }
        for step in 0...100 {
            let frame = PageTurn.frame(width: 800, height: 600, progress: Double(step) / 100, forward: forward)
            #expect(frame.allSatisfy { $0.x.isFinite && $0.scaleY.isFinite && $0.scaleY > 0 })
            for i in 0..<(frame.count - 1) {
                let gap = forward ? frame[i].x + 25 * frame[i].scaleX - frame[i + 1].x
                    : frame[i + 1].x + 25 * frame[i + 1].scaleX - frame[i].x
                #expect(abs(gap) < 0.001)
            }
        }
    }
    let half = PageTurn.frame(width: 800, height: 600, progress: 0.5, forward: true)
    #expect(half.contains { $0.front == 1 } && half.contains { $0.front == 0 })
}
