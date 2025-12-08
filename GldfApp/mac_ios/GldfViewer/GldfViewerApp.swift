// GldfViewerApp.swift
// GLDF Viewer - Cross-platform App (iOS & macOS)

import SwiftUI
import GldfKit

#if os(macOS)
// MARK: - macOS App Delegate
class AppDelegate: NSObject, NSApplicationDelegate {
    var appState: AppState?
    var pendingURL: URL?

    func application(_ application: NSApplication, open urls: [URL]) {
        for url in urls {
            if url.pathExtension.lowercased() == "gldf" {
                if let appState = appState {
                    Task { @MainActor in
                        appState.loadFile(from: url)
                    }
                } else {
                    pendingURL = url
                }
                break
            }
        }
    }

    func processPendingURL() {
        if let url = pendingURL, let appState = appState {
            Task { @MainActor in
                appState.loadFile(from: url)
            }
            pendingURL = nil
        }
    }
}
#endif

@main
struct GldfViewerApp: App {
    #if os(macOS)
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @Environment(\.openWindow) private var openWindow
    #endif

    @StateObject private var appState = AppState()

    var body: some Scene {
        #if os(macOS)
        WindowGroup(id: "main") {
            ContentView()
                .environmentObject(appState)
                .onAppear {
                    appDelegate.appState = appState
                    appDelegate.processPendingURL()
                }
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open GLDF...") {
                    ensureWindowOpen()
                    appState.showOpenPanel()
                }
                .keyboardShortcut("o", modifiers: .command)

                Divider()

                Button("New GLDF") {
                    ensureWindowOpen()
                    appState.createNew()
                }
                .keyboardShortcut("n", modifiers: .command)
            }

            CommandGroup(replacing: .saveItem) {
                Button("Export JSON...") {
                    appState.exportJSON()
                }
                .keyboardShortcut("e", modifiers: .command)
                .disabled(appState.engine == nil)

                Button("Export XML...") {
                    appState.exportXML()
                }
                .keyboardShortcut("e", modifiers: [.command, .shift])
                .disabled(appState.engine == nil)
            }
        }

        Settings {
            SettingsView()
        }
        #else
        // iOS
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .onOpenURL { url in
                    handleOpenURL(url)
                }
        }
        #endif
    }

    #if os(macOS)
    private func ensureWindowOpen() {
        if NSApplication.shared.windows.filter({ $0.isVisible && $0.title != "Settings" }).isEmpty {
            openWindow(id: "main")
        }
    }
    #endif

    #if os(iOS)
    private func handleOpenURL(_ url: URL) {
        guard url.startAccessingSecurityScopedResource() else {
            appState.errorMessage = "Cannot access file"
            appState.showError = true
            return
        }

        defer { url.stopAccessingSecurityScopedResource() }

        do {
            let data = try Data(contentsOf: url)
            appState.loadFromData(data, fileName: url.lastPathComponent)
        } catch {
            appState.errorMessage = "Failed to read file: \(error.localizedDescription)"
            appState.showError = true
        }
    }
    #endif
}
