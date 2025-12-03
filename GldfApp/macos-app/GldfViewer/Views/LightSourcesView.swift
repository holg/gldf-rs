// LightSourcesView.swift
// View for displaying light sources

import SwiftUI
import GldfKit

struct LightSourcesView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedId: String?
    @State private var filterType: LightSourceFilter = .all

    enum LightSourceFilter: String, CaseIterable {
        case all = "All"
        case fixed = "Fixed"
        case changeable = "Changeable"
    }

    var filteredSources: [GldfLightSource] {
        switch filterType {
        case .all:
            return appState.lightSources
        case .fixed:
            return appState.lightSources.filter { $0.lightSourceType == "fixed" }
        case .changeable:
            return appState.lightSources.filter { $0.lightSourceType == "changeable" }
        }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Picker("Filter", selection: $filterType) {
                    ForEach(LightSourceFilter.allCases, id: \.self) { type in
                        Text(type.rawValue).tag(type)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 300)

                Spacer()

                Text("\(filteredSources.count) light source(s)")
                    .foregroundColor(.secondary)
            }
            .padding()

            Divider()

            // Light Sources Table
            if filteredSources.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "lightbulb.slash")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No light sources defined")
                        .font(.headline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Table(filteredSources, selection: $selectedId) {
                    TableColumn("ID", value: \.id)
                        .width(min: 100, ideal: 150)

                    TableColumn("Name", value: \.name)
                        .width(min: 150, ideal: 250)

                    TableColumn("Type") { source in
                        HStack {
                            Image(systemName: source.lightSourceType == "fixed" ? "lightbulb.fill" : "lightbulb")
                                .foregroundColor(source.lightSourceType == "fixed" ? .orange : .blue)
                            Text(source.lightSourceType.capitalized)
                        }
                    }
                    .width(min: 100, ideal: 120)
                }
            }
        }
        .navigationTitle("Light Sources")
    }
}

#Preview {
    LightSourcesView()
        .environmentObject(AppState())
}
