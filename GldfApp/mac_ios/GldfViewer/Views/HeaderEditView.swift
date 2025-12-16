// HeaderEditView.swift
// View for editing GLDF header information

import SwiftUI
import GldfKit

struct HeaderEditView: View {
    @EnvironmentObject var appState: AppState

    @State private var author: String = ""
    @State private var manufacturer: String = ""
    @State private var createdWithApplication: String = ""
    @State private var creationTimeCode: String = ""
    @State private var versionMajor: String = ""
    @State private var versionMinor: String = ""
    @State private var versionPreRelease: String = ""

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                // Header Info
                GroupBox("Header Information") {
                    VStack(alignment: .leading, spacing: 16) {
                        FormField(label: "Manufacturer", text: $manufacturer) {
                            appState.updateManufacturer(manufacturer)
                        }

                        FormField(label: "Author", text: $author) {
                            appState.updateAuthor(author)
                        }

                        FormField(label: "Created With Application", text: $createdWithApplication) {
                            appState.updateCreatedWithApplication(createdWithApplication)
                        }

                        FormField(label: "Creation Time Code", text: $creationTimeCode) {
                            appState.updateCreationTimeCode(creationTimeCode)
                        }
                    }
                    .padding(.vertical, 8)
                }

                // Format Version
                GroupBox("Format Version") {
                    HStack(spacing: 12) {
                        VStack(alignment: .leading) {
                            Text("Major")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            TextField("1", text: $versionMajor)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .onSubmit { updateVersion() }
                        }

                        VStack(alignment: .leading) {
                            Text("Minor")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            TextField("0", text: $versionMinor)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .onSubmit { updateVersion() }
                        }

                        VStack(alignment: .leading) {
                            Text("Pre-release")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            TextField("3", text: $versionPreRelease)
                                .textFieldStyle(.roundedBorder)
                                .frame(width: 60)
                                .onSubmit { updateVersion() }
                        }

                        Spacer()

                        Text("v\(versionMajor).\(versionMinor).\(versionPreRelease)")
                            .font(.title2)
                            .foregroundColor(.secondary)
                            .padding(.horizontal)
                    }
                    .padding(.vertical, 8)
                }

                Spacer()
            }
            .padding()
        }
        .navigationTitle("Header")
        .onAppear { loadValues() }
        .onChange(of: appState.header) { _ in loadValues() }
    }

    private func loadValues() {
        guard let header = appState.header else { return }
        author = header.author
        manufacturer = header.manufacturer
        createdWithApplication = header.createdWithApplication
        creationTimeCode = header.creationTimeCode

        // Parse version string "x.y.z"
        let parts = header.formatVersion.split(separator: ".")
        versionMajor = parts.count > 0 ? String(parts[0]) : "1"
        versionMinor = parts.count > 1 ? String(parts[1]) : "0"
        versionPreRelease = parts.count > 2 ? String(parts[2]) : "0"
    }

    private func updateVersion() {
        let major = Int(versionMajor) ?? 1
        let minor = Int(versionMinor) ?? 0
        let preRelease = Int(versionPreRelease) ?? 0
        let versionString = preRelease > 0 ? "\(major).\(minor).\(preRelease)" : "\(major).\(minor)"
        appState.updateFormatVersion(versionString)
    }
}

// MARK: - Form Field Component
struct FormField: View {
    let label: String
    @Binding var text: String
    let onCommit: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            TextField(label, text: $text)
                .textFieldStyle(.roundedBorder)
                .onSubmit { onCommit() }
        }
    }
}

#Preview {
    HeaderEditView()
        .environmentObject(AppState())
}
