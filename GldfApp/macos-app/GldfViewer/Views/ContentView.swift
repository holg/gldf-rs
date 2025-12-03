// ContentView.swift
// Main content view with sidebar navigation

import SwiftUI
import UniformTypeIdentifiers

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var isDragging = false

    var body: some View {
        NavigationSplitView {
            SidebarView()
        } detail: {
            if appState.engine != nil {
                DetailView()
            } else {
                WelcomeView(isDragging: $isDragging)
            }
        }
        .frame(minWidth: 800, minHeight: 500)
        .navigationTitle(windowTitle)
        .onDrop(of: [UTType.fileURL], isTargeted: $isDragging) { providers in
            handleDrop(providers: providers)
        }
        .alert("Error", isPresented: $appState.showError) {
            Button("OK", role: .cancel) { }
        } message: {
            Text(appState.errorMessage ?? "Unknown error")
        }
    }

    private var windowTitle: String {
        if let url = appState.currentFileURL {
            let modified = appState.isModified ? " (modified)" : ""
            return url.lastPathComponent + modified
        } else if appState.engine != nil {
            return "Untitled GLDF" + (appState.isModified ? " (modified)" : "")
        }
        return "GLDF Viewer"
    }

    private func handleDrop(providers: [NSItemProvider]) -> Bool {
        guard let provider = providers.first else { return false }

        provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, error in
            guard let data = item as? Data,
                  let url = URL(dataRepresentation: data, relativeTo: nil),
                  url.pathExtension.lowercased() == "gldf" else {
                return
            }

            DispatchQueue.main.async {
                appState.loadFile(from: url)
            }
        }

        return true
    }
}

// MARK: - Sidebar View
struct SidebarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        List(selection: $appState.selectedNavItem) {
            Section("Viewer") {
                NavigationLink(value: NavigationItem.overview) {
                    Label("Overview", systemImage: "doc.richtext")
                }

                NavigationLink(value: NavigationItem.rawData) {
                    Label("Raw Data", systemImage: "curlybraces")
                }

                if !appState.openFileTabs.isEmpty {
                    NavigationLink(value: NavigationItem.fileViewer) {
                        Label("File Viewer (\(appState.openFileTabs.count))", systemImage: "eye")
                    }
                }
            }

            Section("Document") {
                NavigationLink(value: NavigationItem.header) {
                    Label("Header", systemImage: "doc.text")
                }

                NavigationLink(value: NavigationItem.statistics) {
                    Label("Statistics", systemImage: "chart.bar")
                }
            }

            Section("Definitions") {
                NavigationLink(value: NavigationItem.files) {
                    Label("Files (\(appState.files.count))", systemImage: "folder")
                }

                NavigationLink(value: NavigationItem.lightSources) {
                    Label("Light Sources (\(appState.lightSources.count))", systemImage: "lightbulb")
                }

                NavigationLink(value: NavigationItem.variants) {
                    Label("Variants (\(appState.variants.count))", systemImage: "square.stack.3d.up")
                }
            }
        }
        .listStyle(.sidebar)
        .navigationSplitViewColumnWidth(min: 180, ideal: 200)
        .disabled(appState.engine == nil)
    }
}

// MARK: - Detail View
struct DetailView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        Group {
            switch appState.selectedNavItem {
            case .overview:
                OverviewView()
            case .rawData:
                RawDataView()
            case .fileViewer:
                FileViewerView()
            case .header:
                HeaderEditView()
            case .files:
                FilesListView()
            case .lightSources:
                LightSourcesView()
            case .variants:
                VariantsView()
            case .statistics:
                StatisticsView()
            case .none:
                Text("Select an item from the sidebar")
                    .foregroundColor(.secondary)
            }
        }
    }
}

// MARK: - Welcome View
struct WelcomeView: View {
    @EnvironmentObject var appState: AppState
    @Binding var isDragging: Bool

    var body: some View {
        VStack(spacing: 24) {
            Image(systemName: "lightbulb.circle")
                .font(.system(size: 80))
                .foregroundColor(isDragging ? .accentColor : .secondary)

            Text("GLDF Viewer")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("Library Version: \(appState.libraryVersion)")
                .font(.subheadline)
                .foregroundColor(.secondary)

            Divider()
                .frame(width: 200)

            VStack(spacing: 12) {
                Text("Drop a GLDF file here")
                    .font(.headline)

                Text("or")
                    .foregroundColor(.secondary)

                HStack(spacing: 16) {
                    Button("Open File...") {
                        appState.showOpenPanel()
                    }
                    .buttonStyle(.borderedProminent)

                    Button("Create New") {
                        appState.createNew()
                    }
                    .buttonStyle(.bordered)
                }
            }
        }
        .padding(40)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .strokeBorder(
                    isDragging ? Color.accentColor : Color.clear,
                    style: StrokeStyle(lineWidth: 3, dash: [10])
                )
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Preview
#Preview {
    ContentView()
        .environmentObject(AppState())
}
