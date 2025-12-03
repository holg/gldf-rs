// GldfViewerApp.swift
// GLDF Viewer - macOS App

import SwiftUI

@main
struct GldfViewerApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
        .commands {
            CommandGroup(replacing: .newItem) {
                Button("Open GLDF...") {
                    appState.showOpenPanel()
                }
                .keyboardShortcut("o", modifiers: .command)

                Divider()

                Button("New GLDF") {
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
    }
}
