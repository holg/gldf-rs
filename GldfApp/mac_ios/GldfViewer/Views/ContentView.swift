// ContentView.swift
// Main content view with sidebar navigation (iOS & macOS)

import SwiftUI
import UniformTypeIdentifiers
import GldfKit

struct ContentView: View {
    @EnvironmentObject var appState: AppState

    #if os(macOS)
    @State private var isDragging = false
    #else
    @State private var showingFilePicker = false
    #endif

    var body: some View {
        #if os(macOS)
        // macOS: NavigationSplitView with drag & drop
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
        #else
        // iOS: NavigationView with file importer
        NavigationView {
            List {
                if appState.engine != nil {
                    NavigationLink(tag: NavigationItem.overview, selection: $appState.selectedNavItem) {
                        OverviewView()
                    } label: {
                        Label("Overview", systemImage: "doc.text")
                    }

                    NavigationLink(tag: NavigationItem.header, selection: $appState.selectedNavItem) {
                        HeaderEditView()
                    } label: {
                        Label("Header", systemImage: "info.circle")
                    }

                    NavigationLink(tag: NavigationItem.files, selection: $appState.selectedNavItem) {
                        FilesListView()
                    } label: {
                        Label("Files", systemImage: "folder")
                    }

                    NavigationLink(tag: NavigationItem.lightSources, selection: $appState.selectedNavItem) {
                        LightSourcesView()
                    } label: {
                        Label("Light Sources", systemImage: "lightbulb")
                    }

                    NavigationLink(tag: NavigationItem.variants, selection: $appState.selectedNavItem) {
                        VariantsView()
                    } label: {
                        Label("Variants", systemImage: "square.stack.3d.up")
                    }

                    NavigationLink(tag: NavigationItem.statistics, selection: $appState.selectedNavItem) {
                        StatisticsView()
                    } label: {
                        Label("Statistics", systemImage: "chart.bar")
                    }

                    NavigationLink(tag: NavigationItem.rawData, selection: $appState.selectedNavItem) {
                        RawDataView()
                    } label: {
                        Label("Raw Data", systemImage: "doc.plaintext")
                    }

                    if !appState.openFileTabs.isEmpty {
                        NavigationLink(tag: NavigationItem.fileViewer, selection: $appState.selectedNavItem) {
                            FileViewerView()
                        } label: {
                            Label("File Viewer", systemImage: "eye")
                        }
                    }
                } else {
                    WelcomeView(showingFilePicker: $showingFilePicker)
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

            // Default detail view for iPad
            if appState.engine != nil {
                OverviewView()
            } else {
                WelcomeView(showingFilePicker: $showingFilePicker)
            }
        }
        .navigationViewStyle(.automatic)
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
        #endif
    }

    #if os(macOS)
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
    #endif

    #if os(iOS)
    private func handleFileImport(result: Result<[URL], Error>) {
        switch result {
        case .success(let urls):
            guard let url = urls.first else { return }

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
    #endif
}

// MARK: - Sidebar View (macOS)
#if os(macOS)
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

                NavigationLink(value: NavigationItem.electrical) {
                    Label("Electrical", systemImage: "bolt.circle")
                }

                NavigationLink(value: NavigationItem.applications) {
                    Label("Applications", systemImage: "tag")
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

// MARK: - Detail View (macOS)
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
            case .electrical:
                ElectricalEditView()
            case .applications:
                ApplicationsEditView()
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
#endif

// MARK: - Welcome View
struct WelcomeView: View {
    @EnvironmentObject var appState: AppState

    #if os(macOS)
    @Binding var isDragging: Bool
    #else
    @Binding var showingFilePicker: Bool
    #endif

    var body: some View {
        VStack(spacing: 24) {
            Image(systemName: "lightbulb.circle")
                .font(.system(size: 80))
                #if os(macOS)
                .foregroundColor(isDragging ? .accentColor : .secondary)
                #else
                .foregroundColor(.accentColor)
                #endif

            Text("GLDF Viewer")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("Library Version: \(appState.libraryVersion)")
                .font(.subheadline)
                .foregroundColor(.secondary)

            Divider()
                .frame(width: 200)

            VStack(spacing: 12) {
                #if os(macOS)
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

                    Button("Load Demo") {
                        appState.loadDemo()
                    }
                    .buttonStyle(.bordered)
                }
                #else
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

                Button {
                    appState.loadDemo()
                } label: {
                    Label("Load Demo", systemImage: "sparkles")
                        .frame(minWidth: 200)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
                #endif
            }
            .padding(.top, 20)

            Spacer()
        }
        .padding(40)
        #if os(macOS)
        .background(
            RoundedRectangle(cornerRadius: 16)
                .strokeBorder(
                    isDragging ? Color.accentColor : Color.clear,
                    style: StrokeStyle(lineWidth: 3, dash: [10])
                )
        )
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        #endif
    }
}

// MARK: - Preview
#Preview {
    ContentView()
        .environmentObject(AppState())
}
