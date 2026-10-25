import SwiftUI
import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    weak var model: AppModel?
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { true }
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard let model else { return .terminateNow }
        Task { await model.flush(); sender.reply(toApplicationShouldTerminate: true) }
        return .terminateLater
    }
}

@main
struct BibbiaApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @StateObject private var model = AppModel()

    var body: some Scene {
        Window("Bibbia", id: "reader") {
            MainView(model: model)
                .frame(minWidth: 760, minHeight: 540)
                .task { delegate.model = model; await model.start() }
        }
        .defaultSize(width: 1100, height: 780)
        .windowStyle(.titleBar)
        .windowToolbarStyle(.unified)
        .commands {
            CommandGroup(replacing: .newItem) {
                Button(model.t("Importa traduzione…", "Import edition…")) { model.importEdition() }
                    .keyboardShortcut("i", modifiers: [.command, .shift])
            }
            CommandGroup(after: .textEditing) {
                Button(model.t("Cerca nella Bibbia", "Search the Bible")) { model.panel = .search }
                    .keyboardShortcut("f")
            }
            CommandMenu(model.t("Lettura", "Reading")) {
                Button(model.t("Capitolo precedente", "Previous chapter")) { model.step(-1) }
                    .keyboardShortcut(.leftArrow, modifiers: [.command, .option])
                Button(model.t("Capitolo successivo", "Next chapter")) { model.step(1) }
                    .keyboardShortcut(.rightArrow, modifiers: [.command, .option])
                Divider()
                Button(model.t("Testi e licenze…", "Texts and licenses…")) { model.showNotices = true }
            }
            CommandGroup(replacing: .appInfo) {
                Button(model.t("Informazioni su Bibbia", "About Bibbia")) {
                    NSApp.orderFrontStandardAboutPanel(options: [
                        .applicationName: "Bibbia", .applicationVersion: "0.5.0",
                        .credits: NSAttributedString(string: model.t("Lettore offline nativo per macOS. Swift · SwiftUI · AppKit\nTesti e licenze nel menu Lettura.", "Native offline reader for macOS. Swift · SwiftUI · AppKit\nTexts and licenses in the Reading menu."))
                    ])
                }
            }
        }
        Settings { PreferencesView(model: model) }
    }
}
