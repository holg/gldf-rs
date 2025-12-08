// FilesListView.swift
// View for managing GLDF file definitions

import SwiftUI
import GldfKit

struct FilesListView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedFileId: String?
    @State private var showAddSheet = false
    @State private var filterType: FileFilterType = .all

    enum FileFilterType: String, CaseIterable {
        case all = "All"
        case photometric = "Photometric"
        case image = "Images"
        case geometry = "Geometry"
        case other = "Other"
    }

    var filteredFiles: [GldfFile] {
        switch filterType {
        case .all:
            return appState.files
        case .photometric:
            return appState.files.filter { $0.contentType.starts(with: "ldc") }
        case .image:
            return appState.files.filter { $0.contentType.starts(with: "image") }
        case .geometry:
            return appState.files.filter { $0.contentType == "geo/l3d" }
        case .other:
            return appState.files.filter { file in
                !file.contentType.starts(with: "ldc") &&
                !file.contentType.starts(with: "image") &&
                file.contentType != "geo/l3d"
            }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Picker("Filter", selection: $filterType) {
                    ForEach(FileFilterType.allCases, id: \.self) { type in
                        Text(type.rawValue).tag(type)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 400)

                Spacer()

                Button {
                    showAddSheet = true
                } label: {
                    Label("Add File", systemImage: "plus")
                }
            }
            .padding()

            Divider()

            // Files Table
            if filteredFiles.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "folder.badge.questionmark")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No \(filterType == .all ? "" : filterType.rawValue.lowercased() + " ")files defined")
                        .font(.headline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Table(filteredFiles, selection: $selectedFileId) {
                    TableColumn("ID", value: \.id)
                        .width(min: 80, ideal: 120)

                    TableColumn("File Name", value: \.fileName)
                        .width(min: 150, ideal: 200)

                    TableColumn("Content Type") { file in
                        HStack {
                            fileTypeIcon(for: file.contentType)
                            Text(file.contentType)
                        }
                    }
                    .width(min: 100, ideal: 150)

                    TableColumn("Type", value: \.fileType)
                        .width(min: 80, ideal: 100)

                    TableColumn("Actions") { file in
                        HStack(spacing: 8) {
                            Button {
                                appState.openFileForViewing(fileId: file.id)
                            } label: {
                                Image(systemName: "eye")
                            }
                            .buttonStyle(.borderless)
                            .help("View file contents")

                            Button(role: .destructive) {
                                appState.removeFile(id: file.id)
                            } label: {
                                Image(systemName: "trash")
                            }
                            .buttonStyle(.borderless)
                            .help("Remove file")
                        }
                    }
                    .width(70)
                }
            }
        }
        .navigationTitle("Files")
        .sheet(isPresented: $showAddSheet) {
            AddFileSheet(isPresented: $showAddSheet)
        }
    }

    @ViewBuilder
    private func fileTypeIcon(for contentType: String) -> some View {
        let (icon, color) = iconForContentType(contentType)
        Image(systemName: icon)
            .foregroundColor(color)
    }

    private func iconForContentType(_ contentType: String) -> (String, Color) {
        switch contentType {
        case let type where type.starts(with: "ldc"):
            return ("rays", .orange)
        case let type where type.starts(with: "image"):
            return ("photo", .blue)
        case "geo/l3d":
            return ("cube", .green)
        case let type where type.starts(with: "document"):
            return ("doc", .gray)
        default:
            return ("doc.questionmark", .secondary)
        }
    }
}

// MARK: - Add File Sheet
struct AddFileSheet: View {
    @EnvironmentObject var appState: AppState
    @Binding var isPresented: Bool

    @State private var id = ""
    @State private var fileName = ""
    @State private var contentType = "image/png"
    @State private var fileType = "localFileName"

    let contentTypes = [
        "image/png",
        "image/jpg",
        "image/svg",
        "ldc/eulumdat",
        "ldc/ies",
        "geo/l3d",
        "document/pdf",
        "other"
    ]

    let fileTypes = ["localFileName", "url"]

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Add File Definition")
                    .font(.headline)
                Spacer()
                Button("Cancel") {
                    isPresented = false
                }
                .keyboardShortcut(.cancelAction)
            }
            .padding()

            Divider()

            // Form
            Form {
                TextField("ID", text: $id)
                TextField("File Name", text: $fileName)

                Picker("Content Type", selection: $contentType) {
                    ForEach(contentTypes, id: \.self) { type in
                        Text(type).tag(type)
                    }
                }

                Picker("File Type", selection: $fileType) {
                    ForEach(fileTypes, id: \.self) { type in
                        Text(type).tag(type)
                    }
                }
            }
            .padding()

            Divider()

            // Footer
            HStack {
                Spacer()
                Button("Add") {
                    appState.addFile(
                        id: id,
                        fileName: fileName,
                        contentType: contentType,
                        fileType: fileType
                    )
                    isPresented = false
                }
                .keyboardShortcut(.defaultAction)
                .disabled(id.isEmpty || fileName.isEmpty)
            }
            .padding()
        }
        .frame(width: 400, height: 300)
    }
}

#Preview {
    FilesListView()
        .environmentObject(AppState())
}
