// OverviewView.swift
// Comprehensive overview of GLDF file content

import SwiftUI
import GldfKit

struct OverviewView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 24) {
                // Product Info Card
                if let header = appState.header {
                    ProductInfoCard(header: header)
                }

                // Statistics Grid
                if let stats = appState.stats {
                    StatisticsGrid(stats: stats)
                }

                // Content Sections
                HStack(alignment: .top, spacing: 20) {
                    // Left Column
                    VStack(alignment: .leading, spacing: 20) {
                        FilesOverviewCard(files: appState.files)
                        LightSourcesOverviewCard(lightSources: appState.lightSources)
                    }

                    // Right Column
                    VStack(alignment: .leading, spacing: 20) {
                        VariantsOverviewCard(variants: appState.variants)
                    }
                }
            }
            .padding(24)
        }
        .navigationTitle("Overview")
        .background(Color(nsColor: .windowBackgroundColor))
    }
}

// MARK: - Product Info Card
struct ProductInfoCard: View {
    let header: GldfHeader

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack {
                Image(systemName: "lightbulb.circle.fill")
                    .font(.system(size: 32))
                    .foregroundColor(.orange)

                VStack(alignment: .leading, spacing: 4) {
                    Text(header.manufacturer.isEmpty ? "Unknown Manufacturer" : header.manufacturer)
                        .font(.title)
                        .fontWeight(.bold)

                    Text("GLDF Format \(header.formatVersion)")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }

                Spacer()
            }

            Divider()

            LazyVGrid(columns: [
                GridItem(.flexible()),
                GridItem(.flexible())
            ], spacing: 12) {
                InfoRow(label: "Author", value: header.author.isEmpty ? "—" : header.author)
                InfoRow(label: "Created With", value: header.createdWithApplication.isEmpty ? "—" : header.createdWithApplication)
                InfoRow(label: "Creation Date", value: formatTimeCode(header.creationTimeCode))
                InfoRow(label: "Format Version", value: header.formatVersion)
            }
        }
        .padding(20)
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(12)
    }

    private func formatTimeCode(_ timeCode: String) -> String {
        guard !timeCode.isEmpty else { return "—" }
        // Try to format ISO 8601 date
        let formatter = ISO8601DateFormatter()
        if let date = formatter.date(from: timeCode) {
            let displayFormatter = DateFormatter()
            displayFormatter.dateStyle = .medium
            displayFormatter.timeStyle = .short
            return displayFormatter.string(from: date)
        }
        return timeCode
    }
}

struct InfoRow: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value)
                .font(.body)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

// MARK: - Statistics Grid
struct StatisticsGrid: View {
    let stats: GldfStats

    var body: some View {
        LazyVGrid(columns: [
            GridItem(.flexible()),
            GridItem(.flexible()),
            GridItem(.flexible()),
            GridItem(.flexible())
        ], spacing: 16) {
            StatCard(
                icon: "folder.fill",
                value: "\(stats.filesCount)",
                label: "Files",
                color: .blue
            )
            StatCard(
                icon: "lightbulb.fill",
                value: "\(stats.fixedLightSourcesCount + stats.changeableLightSourcesCount)",
                label: "Light Sources",
                color: .yellow
            )
            StatCard(
                icon: "square.stack.3d.up.fill",
                value: "\(stats.variantsCount)",
                label: "Variants",
                color: .purple
            )
            StatCard(
                icon: "rays",
                value: "\(stats.photometriesCount)",
                label: "Photometries",
                color: .orange
            )
        }
    }
}

struct StatCard: View {
    let icon: String
    let value: String
    let label: String
    let color: Color

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.system(size: 24))
                .foregroundColor(color)

            Text(value)
                .font(.title2)
                .fontWeight(.bold)

            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding(16)
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(10)
    }
}

// MARK: - Files Overview Card
struct FilesOverviewCard: View {
    let files: [GldfFile]
    @State private var isExpanded = true

    var groupedFiles: [(String, [GldfFile])] {
        var groups: [String: [GldfFile]] = [:]
        for file in files {
            let category = categoryFor(contentType: file.contentType)
            groups[category, default: []].append(file)
        }
        return groups.sorted { $0.key < $1.key }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            DisclosureGroup(isExpanded: $isExpanded) {
                VStack(alignment: .leading, spacing: 8) {
                    ForEach(groupedFiles, id: \.0) { category, categoryFiles in
                        VStack(alignment: .leading, spacing: 4) {
                            Text(category)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .textCase(.uppercase)

                            ForEach(categoryFiles.prefix(5)) { file in
                                HStack {
                                    Image(systemName: iconFor(contentType: file.contentType))
                                        .foregroundColor(colorFor(contentType: file.contentType))
                                        .frame(width: 20)

                                    Text(file.fileName)
                                        .font(.system(.body, design: .monospaced))
                                        .lineLimit(1)

                                    Spacer()

                                    Text(file.contentType)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                            }

                            if categoryFiles.count > 5 {
                                Text("+ \(categoryFiles.count - 5) more...")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                    .padding(.leading, 24)
                            }
                        }
                        .padding(.vertical, 4)
                    }
                }
                .padding(.top, 8)
            } label: {
                HStack {
                    Image(systemName: "folder.fill")
                        .foregroundColor(.blue)
                    Text("Files")
                        .font(.headline)
                    Spacer()
                    Text("\(files.count)")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(16)
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(10)
    }

    private func categoryFor(contentType: String) -> String {
        if contentType.starts(with: "ldc") { return "Photometric" }
        if contentType.starts(with: "image") { return "Images" }
        if contentType == "geo/l3d" { return "Geometry" }
        if contentType.starts(with: "document") { return "Documents" }
        return "Other"
    }

    private func iconFor(contentType: String) -> String {
        if contentType.starts(with: "ldc") { return "rays" }
        if contentType.starts(with: "image") { return "photo" }
        if contentType == "geo/l3d" { return "cube" }
        if contentType.starts(with: "document") { return "doc" }
        return "doc.questionmark"
    }

    private func colorFor(contentType: String) -> Color {
        if contentType.starts(with: "ldc") { return .orange }
        if contentType.starts(with: "image") { return .blue }
        if contentType == "geo/l3d" { return .green }
        if contentType.starts(with: "document") { return .gray }
        return .secondary
    }
}

// MARK: - Light Sources Overview Card
struct LightSourcesOverviewCard: View {
    let lightSources: [GldfLightSource]
    @State private var isExpanded = true

    var fixedSources: [GldfLightSource] {
        lightSources.filter { $0.lightSourceType == "fixed" }
    }

    var changeableSources: [GldfLightSource] {
        lightSources.filter { $0.lightSourceType == "changeable" }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            DisclosureGroup(isExpanded: $isExpanded) {
                VStack(alignment: .leading, spacing: 8) {
                    if !fixedSources.isEmpty {
                        Text("Fixed")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .textCase(.uppercase)

                        ForEach(fixedSources.prefix(5)) { source in
                            LightSourceRow(source: source)
                        }

                        if fixedSources.count > 5 {
                            Text("+ \(fixedSources.count - 5) more...")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.leading, 24)
                        }
                    }

                    if !changeableSources.isEmpty {
                        Text("Changeable")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .textCase(.uppercase)
                            .padding(.top, 8)

                        ForEach(changeableSources.prefix(5)) { source in
                            LightSourceRow(source: source)
                        }

                        if changeableSources.count > 5 {
                            Text("+ \(changeableSources.count - 5) more...")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.leading, 24)
                        }
                    }

                    if lightSources.isEmpty {
                        Text("No light sources defined")
                            .font(.body)
                            .foregroundColor(.secondary)
                    }
                }
                .padding(.top, 8)
            } label: {
                HStack {
                    Image(systemName: "lightbulb.fill")
                        .foregroundColor(.yellow)
                    Text("Light Sources")
                        .font(.headline)
                    Spacer()
                    Text("\(lightSources.count)")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(16)
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(10)
    }
}

struct LightSourceRow: View {
    let source: GldfLightSource

    var body: some View {
        HStack {
            Image(systemName: source.lightSourceType == "fixed" ? "lightbulb.fill" : "lightbulb.slash")
                .foregroundColor(source.lightSourceType == "fixed" ? .yellow : .orange)
                .frame(width: 20)

            VStack(alignment: .leading, spacing: 2) {
                Text(source.name.isEmpty ? source.id : source.name)
                    .font(.body)

                Text("ID: \(source.id)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()
        }
    }
}

// MARK: - Variants Overview Card
struct VariantsOverviewCard: View {
    let variants: [GldfVariant]
    @State private var isExpanded = true

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            DisclosureGroup(isExpanded: $isExpanded) {
                VStack(alignment: .leading, spacing: 8) {
                    if variants.isEmpty {
                        Text("No variants defined")
                            .font(.body)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(variants.prefix(10)) { variant in
                            VStack(alignment: .leading, spacing: 4) {
                                HStack {
                                    Image(systemName: "square.stack.3d.up")
                                        .foregroundColor(.purple)
                                        .frame(width: 20)

                                    Text(variant.name.isEmpty ? variant.id : variant.name)
                                        .font(.body)
                                        .fontWeight(.medium)

                                    Spacer()
                                }

                                if !variant.description.isEmpty {
                                    Text(variant.description)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                        .lineLimit(2)
                                        .padding(.leading, 24)
                                }

                                Text("ID: \(variant.id)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                                    .padding(.leading, 24)
                            }
                            .padding(.vertical, 4)

                            if variant.id != variants.prefix(10).last?.id {
                                Divider()
                            }
                        }

                        if variants.count > 10 {
                            Text("+ \(variants.count - 10) more...")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.leading, 24)
                        }
                    }
                }
                .padding(.top, 8)
            } label: {
                HStack {
                    Image(systemName: "square.stack.3d.up.fill")
                        .foregroundColor(.purple)
                    Text("Variants")
                        .font(.headline)
                    Spacer()
                    Text("\(variants.count)")
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(16)
        .background(Color(nsColor: .controlBackgroundColor))
        .cornerRadius(10)
    }
}

#Preview {
    OverviewView()
        .environmentObject(AppState())
}
