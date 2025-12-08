// SettingsView.swift
// App settings view

import SwiftUI
import GldfKit

struct SettingsView: View {
    @AppStorage("defaultExportFormat") private var defaultExportFormat = "json"
    @AppStorage("autoSaveEnabled") private var autoSaveEnabled = false

    var body: some View {
        Form {
            Section("Export") {
                Picker("Default Export Format", selection: $defaultExportFormat) {
                    Text("JSON").tag("json")
                    Text("XML").tag("xml")
                }
            }

            Section("Editor") {
                Toggle("Auto-save changes", isOn: $autoSaveEnabled)
            }

            Section("About") {
                HStack {
                    Text("GLDF Viewer")
                    Spacer()
                    Text("1.0.0")
                        .foregroundColor(.secondary)
                }

                HStack {
                    Text("Library Version")
                    Spacer()
                    Text(gldfLibraryVersion())
                        .foregroundColor(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .frame(width: 400, height: 300)
    }
}

#Preview {
    SettingsView()
}
