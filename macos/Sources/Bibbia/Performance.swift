import AppKit
import BibleKit

@MainActor
private final class PerformanceGate {
    private let source: DispatchSourceSignal
    private var continuation: CheckedContinuation<Void,Never>?
    init() {
        Darwin.signal(SIGUSR1,SIG_IGN)
        source=DispatchSource.makeSignalSource(signal:SIGUSR1,queue:.main)
        source.setEventHandler { [weak self] in MainActor.assumeIsolated {
            self?.continuation?.resume();self?.continuation=nil
        }}
        source.resume()
    }
    func wait(_ publish:()->Void) async {
        await withCheckedContinuation { continuation in self.continuation=continuation;publish() }
    }
    func cancel(){source.cancel()}
}

extension AppModel {
    static func performanceProfile()->URL? {
        guard let path=ProcessInfo.processInfo.environment["BIBBIA_PERFORMANCE_PROFILE"] else{return nil}
        let url=URL(fileURLWithPath:path,isDirectory:true)
        return url.lastPathComponent.hasPrefix("bibbia-perf-") && FileManager.default.fileExists(atPath:url.appendingPathComponent(".bibbia-perf-fixture").path) ? url:nil
    }
    func runPerformance(_ profile:URL) async throws {
        let gate=PerformanceGate();defer{gate.cancel()}
        performanceMotion=false
        var clock=Date()
        func ready() async throws {
            for _ in 0..<400 {
                if !loading && (!comparing || comparison != nil) {break}
                try await Task.sleep(for:.milliseconds(20))
            }
            try await Task.sleep(for:.milliseconds(120))
            for window in NSApp.windows {window.contentView?.layoutSubtreeIfNeeded();window.displayIfNeeded()}
        }
        func checkpoint(_ name:String) async throws {
            try await ready()
            let managers=(readingSurface?.scroll.documentView as? ReadingTextView)?.textStorage?.layoutManagers.count ?? 0
            let data=try JSONSerialization.data(withJSONObject:["elapsed_ms":Date().timeIntervalSince(clock)*1000,"layout_managers":managers])
            await gate.wait {try? data.write(to:profile.appendingPathComponent(name+".json"),options:.atomic)}
            clock=Date()
        }
        guard let window=NSApp.windows.first(where:{$0.isVisible}) else{throw BibleError.invalid("Performance window missing")}
        window.setFrame(NSRect(x:20,y:20,width:1000,height:720),display:true)
        try await checkpoint("initial")
        navigate(Position(edition:"CEI 2008",book:22,chapter:118))
        try await checkpoint("long_chapter")
        preferences.comparisonEdition="KJV";comparing=true;comparisonChanged()
        try await checkpoint("comparison")
        comparing=false;comparisonChanged()
        try await checkpoint("returned")
        window.setFrame(NSRect(x:20,y:20,width:1500,height:900),display:true)
        try await checkpoint("wide")
        window.setFrame(NSRect(x:20,y:20,width:1000,height:720),display:true)
        try await checkpoint("narrow")
        for i in 0..<8 {
            navigate(Position(edition:"CEI 2008",book:22,chapter:117+i%2));try await ready()
            comparing=true;comparisonChanged();try await ready();comparing=false;comparisonChanged();try await ready()
        }
        try await checkpoint("cycled")
        performanceMotion=true;step(1)
        for _ in 0..<100 {if readingSurface?.isTurning == true {break};try await Task.sleep(for:.milliseconds(10))}
        try await checkpoint("animation")
        try await Task.sleep(for:.milliseconds(1300))
        try await checkpoint("settled")
        NSApp.perform(#selector(NSApplication.terminate(_:)),with:nil,afterDelay:0)
    }
}
