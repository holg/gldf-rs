// GldfViewerApp.swift
// iOS GLDF Viewer Application

import SwiftUI
import GldfKit

@main
struct GldfViewerApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
        }
    }
}
