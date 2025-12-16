// AppState.swift
// Main application state and ViewModel (iOS & macOS)

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
    case fileViewer
}

// MARK: - Open File Tab
struct OpenFileTab: Identifiable, Equatable {
    let id: String
    let fileName: String
    let contentType: String
    var data: Data?
    var textContent: String?
    var isLoading: Bool = true
    var error: String?
}

// MARK: - App State
@MainActor
class AppState: ObservableObject {
    // MARK: - Published Properties
    @Published var engine: GldfEngine?
    @Published var selectedNavItem: NavigationItem? = .overview
    @Published var isModified: Bool = false
    @Published var errorMessage: String?
    @Published var showError: Bool = false

    // Platform-specific file tracking
    #if os(macOS)
    @Published var currentFileURL: URL?
    #else
    @Published var currentFileName: String?
    @Published var showFilePicker: Bool = false
    #endif

    // Cached data from engine
    @Published var header: GldfHeader?
    @Published var files: [GldfFile] = []
    @Published var lightSources: [GldfLightSource] = []
    @Published var variants: [GldfVariant] = []
    @Published var stats: GldfStats?

    // Open file viewer tabs
    @Published var openFileTabs: [OpenFileTab] = []
    @Published var selectedFileTabId: String?
    @Published var showFileViewer: Bool = false

    // Library version
    let libraryVersion: String

    // MARK: - Init
    init() {
        self.libraryVersion = gldfLibraryVersion()
    }

    // MARK: - File Operations

    #if os(macOS)
    func showOpenPanel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [UTType(filenameExtension: "gldf")!]
        panel.allowsMultipleSelection = false
        panel.canChooseDirectories = false
        panel.message = "Select a GLDF file to open"

        if panel.runModal() == .OK, let url = panel.url {
            loadFile(from: url)
        }
    }

    func loadFile(from url: URL) {
        do {
            let data = try Data(contentsOf: url)
            engine = try GldfEngine.fromBytes(data: data)
            currentFileURL = url
            isModified = false
            refreshAllData()
        } catch {
            showError(message: "Failed to load GLDF: \(error.localizedDescription)")
        }
    }
    #endif

    func loadFromData(_ data: Data, fileName: String) {
        do {
            engine = try GldfEngine.fromBytes(data: data)
            #if os(macOS)
            currentFileURL = nil
            #else
            currentFileName = fileName
            selectedNavItem = .overview
            #endif
            isModified = false
            refreshAllData()
        } catch {
            showError(message: "Failed to load GLDF: \(error.localizedDescription)")
        }
    }

    func createNew() {
        engine = GldfEngine.newEmpty()
        #if os(macOS)
        currentFileURL = nil
        #else
        currentFileName = nil
        #endif
        isModified = false
        refreshAllData()
    }

    func loadDemo() {
        guard let demoURL = Bundle.main.url(forResource: "demo", withExtension: "gldf") else {
            showError(message: "Demo file not found in app bundle")
            return
        }
        do {
            let data = try Data(contentsOf: demoURL)
            engine = try GldfEngine.fromBytes(data: data)
            #if os(macOS)
            currentFileURL = nil
            #else
            currentFileName = "Demo GLDF"
            selectedNavItem = .overview
            #endif
            isModified = false
            refreshAllData()
        } catch {
            showError(message: "Failed to load demo: \(error.localizedDescription)")
        }
    }

    // MARK: - Export Operations (macOS only)

    #if os(macOS)
    func exportJSON() {
        guard let engine = engine else { return }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType.json]
        panel.nameFieldStringValue = currentFileURL?.deletingPathExtension().lastPathComponent ?? "gldf_export"
        panel.message = "Export GLDF as JSON"

        if panel.runModal() == .OK, let url = panel.url {
            do {
                let json = try engine.toPrettyJson()
                try json.write(to: url, atomically: true, encoding: .utf8)
            } catch {
                showError(message: "Failed to export JSON: \(error.localizedDescription)")
            }
        }
    }

    func exportXML() {
        guard let engine = engine else { return }

        let panel = NSSavePanel()
        panel.allowedContentTypes = [UTType.xml]
        panel.nameFieldStringValue = currentFileURL?.deletingPathExtension().lastPathComponent ?? "product"
        panel.message = "Export GLDF as XML"

        if panel.runModal() == .OK, let url = panel.url {
            do {
                let xml = try engine.toXml()
                try xml.write(to: url, atomically: true, encoding: .utf8)
            } catch {
                showError(message: "Failed to export XML: \(error.localizedDescription)")
            }
        }
    }
    #endif

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

    func updateCreatedWithApplication(_ app: String) {
        engine?.setCreatedWithApplication(app: app)
        markModified()
        header = engine?.getHeader()
    }

    func updateCreationTimeCode(_ timeCode: String) {
        engine?.setCreationTimeCode(timeCode: timeCode)
        markModified()
        header = engine?.getHeader()
    }

    func updateFormatVersion(_ version: String) {
        engine?.setFormatVersion(version: version)
        markModified()
        header = engine?.getHeader()
    }

    // MARK: - File Management

    func addFile(id: String, fileName: String, contentType: String, fileType: String) {
        engine?.addFile(id: id, fileName: fileName, contentType: contentType, fileType: fileType)
        markModified()
        files = engine?.getFiles() ?? []
        stats = engine?.getStats()
    }

    func removeFile(id: String) {
        engine?.removeFile(id: id)
        markModified()
        files = engine?.getFiles() ?? []
        stats = engine?.getStats()
    }

    func updateFile(id: String, fileName: String, contentType: String, fileType: String) {
        engine?.updateFile(id: id, fileName: fileName, contentType: contentType, fileType: fileType)
        markModified()
        files = engine?.getFiles() ?? []
    }

    // MARK: - File Viewer

    func openFileForViewing(fileId: String) {
        guard let engine = engine else { return }

        // Check if already open
        if let existingTab = openFileTabs.first(where: { $0.id == fileId }) {
            selectedFileTabId = existingTab.id
            selectedNavItem = .fileViewer
            return
        }

        // Find file info
        guard let fileInfo = files.first(where: { $0.id == fileId }) else { return }

        // Create new tab
        let newTab = OpenFileTab(
            id: fileId,
            fileName: fileInfo.fileName,
            contentType: fileInfo.contentType
        )
        openFileTabs.append(newTab)
        selectedFileTabId = fileId
        selectedNavItem = .fileViewer

        // Load content asynchronously
        Task {
            do {
                // Check if this is a URL-based file
                if fileInfo.fileType == "url" {
                    // Download from URL
                    guard let url = URL(string: fileInfo.fileName.trimmingCharacters(in: .whitespacesAndNewlines)) else {
                        throw NSError(domain: "GLDFViewer", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid URL: \(fileInfo.fileName)"])
                    }
                    let (data, _) = try await URLSession.shared.data(from: url)

                    await MainActor.run {
                        if let index = openFileTabs.firstIndex(where: { $0.id == fileId }) {
                            if fileInfo.contentType.starts(with: "ldc") {
                                openFileTabs[index].textContent = String(data: data, encoding: .utf8)
                            } else {
                                openFileTabs[index].data = data
                            }
                            openFileTabs[index].isLoading = false
                        }
                    }
                } else if fileInfo.contentType.starts(with: "ldc") {
                    // Text-based photometric files from archive
                    let content = try engine.getFileContentAsString(fileId: fileId)
                    await MainActor.run {
                        if let index = openFileTabs.firstIndex(where: { $0.id == fileId }) {
                            openFileTabs[index].textContent = content
                            openFileTabs[index].isLoading = false
                        }
                    }
                } else {
                    // Binary files (images, geometry, etc.) from archive
                    let fileContent = try engine.getFileContent(fileId: fileId)
                    await MainActor.run {
                        if let index = openFileTabs.firstIndex(where: { $0.id == fileId }) {
                            openFileTabs[index].data = Data(fileContent.data)
                            openFileTabs[index].isLoading = false
                        }
                    }
                }
            } catch {
                await MainActor.run {
                    if let index = openFileTabs.firstIndex(where: { $0.id == fileId }) {
                        openFileTabs[index].error = error.localizedDescription
                        openFileTabs[index].isLoading = false
                    }
                }
            }
        }
    }

    func closeFileTab(fileId: String) {
        openFileTabs.removeAll { $0.id == fileId }
        if selectedFileTabId == fileId {
            selectedFileTabId = openFileTabs.first?.id
        }
        if openFileTabs.isEmpty {
            showFileViewer = false
        }
    }

    func closeAllFileTabs() {
        openFileTabs.removeAll()
        selectedFileTabId = nil
        showFileViewer = false
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
