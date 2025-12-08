// StatisticsView.swift
// View for displaying GLDF statistics

import SwiftUI
import GldfKit

struct StatisticsView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                if let stats = appState.stats {
                    // Files Section
                    GroupBox("Files") {
                        StatRow(label: "Total Files", value: "\(stats.filesCount)")
                    }

                    // Light Sources Section
                    GroupBox("Light Sources") {
                        VStack(alignment: .leading, spacing: 12) {
                            StatRow(label: "Fixed Light Sources", value: "\(stats.fixedLightSourcesCount)")
                            StatRow(label: "Changeable Light Sources", value: "\(stats.changeableLightSourcesCount)")
                            Divider()
                            StatRow(label: "Total Light Sources",
                                   value: "\(stats.fixedLightSourcesCount + stats.changeableLightSourcesCount)",
                                   isTotal: true)
                        }
                    }

                    // Product Section
                    GroupBox("Product Definitions") {
                        StatRow(label: "Variants", value: "\(stats.variantsCount)")
                    }

                    // Photometry Section
                    GroupBox("Photometry & Geometry") {
                        VStack(alignment: .leading, spacing: 12) {
                            StatRow(label: "Photometries", value: "\(stats.photometriesCount)")
                            StatRow(label: "Simple Geometries", value: "\(stats.simpleGeometriesCount)")
                            StatRow(label: "Model Geometries", value: "\(stats.modelGeometriesCount)")
                        }
                    }
                } else {
                    VStack(spacing: 16) {
                        Image(systemName: "chart.bar.xaxis")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text("No GLDF loaded")
                            .font(.headline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }

                Spacer()
            }
            .padding()
        }
        .navigationTitle("Statistics")
    }
}

// MARK: - Stat Row
struct StatRow: View {
    let label: String
    let value: String
    var isTotal: Bool = false

    var body: some View {
        HStack {
            Text(label)
                .foregroundColor(isTotal ? .primary : .secondary)
                .fontWeight(isTotal ? .semibold : .regular)
            Spacer()
            Text(value)
                .font(.system(.body, design: .monospaced))
                .fontWeight(isTotal ? .bold : .medium)
        }
    }
}

#Preview {
    StatisticsView()
        .environmentObject(AppState())
}
