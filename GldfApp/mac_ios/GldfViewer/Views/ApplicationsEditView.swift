// ApplicationsEditView.swift
// View for editing applications (where the luminaire can be used)

import SwiftUI
import GldfKit

struct ApplicationsEditView: View {
    @EnvironmentObject var appState: AppState
    @State private var applications: [String] = []
    @State private var newApplication: String = ""
    @State private var selectedCommonApp: String = ""

    let commonApplications = [
        "Office",
        "Industrial",
        "Retail",
        "Hospitality",
        "Healthcare",
        "Education",
        "Residential",
        "Outdoor",
        "Street",
        "Sports",
        "Museum",
        "Warehouse",
        "Parking",
        "Emergency",
        "Accent",
        "Architectural",
        "Facade",
        "Landscape"
    ]

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Text("Define where this luminaire can be used")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(applications.count) application(s)")
                    .foregroundColor(.secondary)
            }
            .padding()

            Divider()

            List {
                // Current Applications Section
                Section("Current Applications") {
                    if applications.isEmpty {
                        Text("No applications defined")
                            .foregroundColor(.secondary)
                            .italic()
                    } else {
                        ForEach(Array(applications.enumerated()), id: \.offset) { index, app in
                            HStack {
                                Image(systemName: "checkmark.circle.fill")
                                    .foregroundColor(.green)
                                Text(app)
                                Spacer()
                                Button(action: {
                                    removeApplication(at: index)
                                }) {
                                    Image(systemName: "xmark.circle.fill")
                                        .foregroundColor(.red)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }
                }

                // Add Common Application Section
                Section("Add Common Application") {
                    HStack {
                        Picker("Select", selection: $selectedCommonApp) {
                            Text("-- Select --").tag("")
                            ForEach(availableCommonApps, id: \.self) { app in
                                Text(app).tag(app)
                            }
                        }

                        Button("Add") {
                            if !selectedCommonApp.isEmpty {
                                addApplication(selectedCommonApp)
                                selectedCommonApp = ""
                            }
                        }
                        .disabled(selectedCommonApp.isEmpty)
                    }
                }

                // Add Custom Application Section
                Section("Add Custom Application") {
                    HStack {
                        TextField("Enter custom application", text: $newApplication)
                            .textFieldStyle(.roundedBorder)
                            .onSubmit {
                                addCustomApplication()
                            }

                        Button(action: addCustomApplication) {
                            Image(systemName: "plus.circle.fill")
                                .foregroundColor(.blue)
                        }
                        .disabled(newApplication.trimmingCharacters(in: .whitespaces).isEmpty)
                        .buttonStyle(.plain)
                    }
                }
            }
        }
        .navigationTitle("Applications")
        .onAppear {
            loadApplications()
        }
    }

    var availableCommonApps: [String] {
        commonApplications.filter { !applications.contains($0) }
    }

    private func loadApplications() {
        guard let engine = appState.engine else { return }
        applications = engine.getApplications()
    }

    private func addApplication(_ app: String) {
        guard let engine = appState.engine else { return }
        guard !applications.contains(app) else { return }

        engine.addApplication(application: app)
        applications.append(app)
        appState.markModified()
    }

    private func addCustomApplication() {
        let trimmed = newApplication.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        addApplication(trimmed)
        newApplication = ""
    }

    private func removeApplication(at index: Int) {
        guard let engine = appState.engine else { return }
        engine.removeApplication(index: UInt32(index))
        applications.remove(at: index)
        appState.markModified()
    }
}

#Preview {
    ApplicationsEditView()
        .environmentObject(AppState())
}
