// ContentView.swift
// iOS Main Content View

import SwiftUI
import UniformTypeIdentifiers
import GldfKit

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var showingFilePicker = false

    var body: some View {
        NavigationView {
            // Sidebar
            List(selection: $appState.selectedNavItem) {
                if appState.engine != nil {
                    NavigationLink(value: NavigationItem.overview) {
                        Label("Overview", systemImage: "doc.text")
                    }

                    NavigationLink(value: NavigationItem.header) {
                        Label("Header", systemImage: "info.circle")
                    }

                    NavigationLink(value: NavigationItem.files) {
                        Label("Files", systemImage: "folder")
                    }

                    NavigationLink(value: NavigationItem.lightSources) {
                        Label("Light Sources", systemImage: "lightbulb")
                    }

                    NavigationLink(value: NavigationItem.variants) {
                        Label("Variants", systemImage: "square.stack.3d.up")
                    }

                    NavigationLink(value: NavigationItem.statistics) {
                        Label("Statistics", systemImage: "chart.bar")
                    }

                    NavigationLink(value: NavigationItem.rawData) {
                        Label("Raw Data", systemImage: "doc.plaintext")
                    }
                }
            }
            .listStyle(.sidebar)
            .navigationTitle("GLDF Viewer")
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button {
                        showingFilePicker = true
                    } label: {
                        Image(systemName: "folder.badge.plus")
                    }
                }
            }

            // Detail View
            if appState.engine != nil {
                detailView
            } else {
                WelcomeView(showingFilePicker: $showingFilePicker)
            }
        }
        .fileImporter(
            isPresented: $showingFilePicker,
            allowedContentTypes: [UTType(filenameExtension: "gldf")!],
            allowsMultipleSelection: false
        ) { result in
            handleFileImport(result: result)
        }
        .alert("Error", isPresented: $appState.showError) {
            Button("OK") { }
        } message: {
            Text(appState.errorMessage ?? "Unknown error")
        }
    }

    @ViewBuilder
    private var detailView: some View {
        switch appState.selectedNavItem {
        case .overview:
            OverviewView()
        case .header:
            HeaderView()
        case .files:
            FilesListView()
        case .lightSources:
            LightSourcesView()
        case .variants:
            VariantsView()
        case .statistics:
            StatisticsView()
        case .rawData:
            RawDataView()
        case .none:
            OverviewView()
        }
    }

    private func handleFileImport(result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            guard let url = urls.first else { return }

            // Start accessing security-scoped resource
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

        case .failure(let error):
            appState.errorMessage = "File selection failed: \(error.localizedDescription)"
            appState.showError = true
        }
    }
}

// MARK: - Welcome View
struct WelcomeView: View {
    @EnvironmentObject var appState: AppState
    @Binding var showingFilePicker: Bool

    var body: some View {
        VStack(spacing: 24) {
            Image(systemName: "lightbulb.circle")
                .font(.system(size: 80))
                .foregroundColor(.accentColor)

            Text("GLDF Viewer")
                .font(.largeTitle)
                .fontWeight(.bold)

            Text("Global Lighting Data Format")
                .font(.title3)
                .foregroundColor(.secondary)

            VStack(spacing: 12) {
                Button {
                    showingFilePicker = true
                } label: {
                    Label("Open GLDF File", systemImage: "folder")
                        .frame(minWidth: 200)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button {
                    appState.createNew()
                } label: {
                    Label("Create New", systemImage: "plus")
                        .frame(minWidth: 200)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
            }
            .padding(.top, 20)

            Spacer()

            Text("Library v\(appState.libraryVersion)")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
    }
}

// MARK: - Overview View
struct OverviewView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 20) {
                if let header = appState.header {
                    GroupBox("Product Information") {
                        VStack(alignment: .leading, spacing: 8) {
                            InfoRow(label: "Manufacturer", value: header.manufacturer)
                            InfoRow(label: "Author", value: header.author)
                            InfoRow(label: "Format Version", value: header.formatVersion)
                            InfoRow(label: "Created With", value: header.createdWithApplication)
                        }
                    }
                }

                if let stats = appState.stats {
                    GroupBox("Statistics") {
                        LazyVGrid(columns: [
                            GridItem(.flexible()),
                            GridItem(.flexible())
                        ], spacing: 12) {
                            StatCard(title: "Files", value: "\(stats.filesCount)", icon: "folder")
                            StatCard(title: "Light Sources", value: "\(stats.fixedLightSourcesCount + stats.changeableLightSourcesCount)", icon: "lightbulb")
                            StatCard(title: "Variants", value: "\(stats.variantsCount)", icon: "square.stack.3d.up")
                            StatCard(title: "Photometries", value: "\(stats.photometriesCount)", icon: "rays")
                        }
                    }
                }
            }
            .padding()
        }
        .navigationTitle("Overview")
    }
}

struct InfoRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(.secondary)
            Spacer()
            Text(value.isEmpty ? "—" : value)
        }
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let icon: String

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title2)
                .foregroundColor(.accentColor)
            Text(value)
                .font(.title)
                .fontWeight(.bold)
            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(Color(uiColor: .secondarySystemBackground))
        .cornerRadius(12)
    }
}

// MARK: - Header View
struct HeaderView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        Form {
            if let header = appState.header {
                Section("Manufacturer Information") {
                    LabeledContent("Manufacturer", value: header.manufacturer)
                    LabeledContent("Author", value: header.author)
                }

                Section("Format Information") {
                    LabeledContent("Format Version", value: header.formatVersion)
                    LabeledContent("Created With", value: header.createdWithApplication)
                    LabeledContent("Creation Time", value: header.creationTimeCode)
                }
            }
        }
        .navigationTitle("Header")
    }
}

// MARK: - Files List View
struct FilesListView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(appState.files, id: \.id) { file in
            VStack(alignment: .leading, spacing: 4) {
                Text(file.fileName)
                    .font(.headline)
                HStack {
                    Text(file.contentType)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(file.fileType)
                        .font(.caption)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 2)
                        .background(Color.accentColor.opacity(0.2))
                        .cornerRadius(4)
                }
            }
            .padding(.vertical, 4)
        }
        .navigationTitle("Files (\(appState.files.count))")
    }
}

// MARK: - Light Sources View
struct LightSourcesView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(appState.lightSources, id: \.id) { source in
            VStack(alignment: .leading, spacing: 4) {
                Text(source.name)
                    .font(.headline)
                Text(source.id)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 4)
        }
        .navigationTitle("Light Sources (\(appState.lightSources.count))")
    }
}

// MARK: - Variants View
struct VariantsView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(appState.variants, id: \.id) { variant in
            VStack(alignment: .leading, spacing: 4) {
                Text(variant.name)
                    .font(.headline)
                Text(variant.id)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.vertical, 4)
        }
        .navigationTitle("Variants (\(appState.variants.count))")
    }
}

// MARK: - Statistics View
struct StatisticsView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List {
            if let stats = appState.stats {
                Section("Content Counts") {
                    LabeledContent("Files", value: "\(stats.filesCount)")
                    LabeledContent("Fixed Light Sources", value: "\(stats.fixedLightSourcesCount)")
                    LabeledContent("Changeable Light Sources", value: "\(stats.changeableLightSourcesCount)")
                    LabeledContent("Variants", value: "\(stats.variantsCount)")
                    LabeledContent("Simple Geometries", value: "\(stats.simpleGeometriesCount)")
                    LabeledContent("Model Geometries", value: "\(stats.modelGeometriesCount)")
                    LabeledContent("Photometries", value: "\(stats.photometriesCount)")
                }
            }
        }
        .navigationTitle("Statistics")
    }
}

// MARK: - Raw Data View
struct RawDataView: View {
    @EnvironmentObject var appState: AppState
    @State private var jsonContent: String = ""

    var body: some View {
        ScrollView {
            Text(jsonContent)
                .font(.system(size: 11, design: .monospaced))
                .textSelection(.enabled)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding()
        }
        .navigationTitle("Raw Data")
        .onAppear {
            if let engine = appState.engine {
                jsonContent = (try? engine.toPrettyJson()) ?? "Failed to serialize"
            }
        }
    }
}

#Preview {
    ContentView()
        .environmentObject(AppState())
}
