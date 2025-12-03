// AppState.swift
// iOS Application State

import SwiftUI
import Combine
import UniformTypeIdentifiers
import GldfKit

// MARK: - Navigation
enum NavigationItem: Hashable {
    case overview
    case header
    case files
    case lightSources
    case variants
    case statistics
    case rawData
}

// MARK: - App State
@MainActor
class AppState: ObservableObject {
    // MARK: - Published Properties
    @Published var engine: GldfEngine?
    @Published var selectedNavItem: NavigationItem? = .overview
    @Published var isModified: Bool = false
    @Published var currentFileName: String?
    @Published var errorMessage: String?
    @Published var showError: Bool = false
    @Published var showFilePicker: Bool = false

    // Cached data from engine
    @Published var header: GldfHeader?
    @Published var files: [GldfFile] = []
    @Published var lightSources: [GldfLightSource] = []
    @Published var variants: [GldfVariant] = []
    @Published var stats: GldfStats?

    // Library version
    let libraryVersion: String

    // MARK: - Init
    init() {
        self.libraryVersion = gldfLibraryVersion()
    }

    // MARK: - File Operations

    func loadFromData(_ data: Data, fileName: String) {
        do {
            engine = try GldfEngine.fromBytes(data: data)
            currentFileName = fileName
            isModified = false
            refreshAllData()
        } catch {
            showError(message: "Failed to load GLDF: \(error.localizedDescription)")
        }
    }

    func createNew() {
        engine = GldfEngine.newEmpty()
        currentFileName = nil
        isModified = false
        refreshAllData()
    }

    // MARK: - Data Refresh

    func refreshAllData() {
        guard let engine = engine else {
            header = nil
            files = []
            lightSources = []
            variants = []
            stats = nil
            return
        }

        header = engine.getHeader()
        files = engine.getFiles()
        lightSources = engine.getLightSources()
        variants = engine.getVariants()
        stats = engine.getStats()
    }

    // MARK: - Header Editing

    func updateAuthor(_ author: String) {
        engine?.setAuthor(author: author)
        markModified()
        header = engine?.getHeader()
    }

    func updateManufacturer(_ manufacturer: String) {
        engine?.setManufacturer(manufacturer: manufacturer)
        markModified()
        header = engine?.getHeader()
    }

    // MARK: - Helper Methods

    private func markModified() {
        isModified = engine?.isModified() ?? false
    }

    private func showError(message: String) {
        errorMessage = message
        showError = true
    }
}
