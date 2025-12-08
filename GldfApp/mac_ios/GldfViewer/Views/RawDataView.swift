// RawDataView.swift
// Raw XML/JSON viewer for GLDF content

import SwiftUI
import GldfKit
#if os(iOS)
import UIKit
#endif

struct RawDataView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedFormat: DataFormat = .xml
    @State private var rawContent: String = ""
    @State private var isLoading = false
    @State private var searchText = ""
    @State private var fontSize: CGFloat = 12

    enum DataFormat: String, CaseIterable {
        case xml = "XML"
        case json = "JSON"
    }

    var filteredContent: String {
        guard !searchText.isEmpty else { return rawContent }
        let lines = rawContent.components(separatedBy: "\n")
        let matchingLines = lines.enumerated().filter { _, line in
            line.localizedCaseInsensitiveContains(searchText)
        }
        if matchingLines.isEmpty {
            return "No matches found for '\(searchText)'"
        }
        return matchingLines.map { index, line in
            "\(index + 1): \(line)"
        }.joined(separator: "\n")
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                Picker("Format", selection: $selectedFormat) {
                    ForEach(DataFormat.allCases, id: \.self) { format in
                        Text(format.rawValue).tag(format)
                    }
                }
                .pickerStyle(.segmented)
                .frame(width: 160)

                Spacer()

                // Search field
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundColor(.secondary)
                    TextField("Search...", text: $searchText)
                        .textFieldStyle(.plain)
                        .frame(width: 150)
                    if !searchText.isEmpty {
                        Button {
                            searchText = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(.secondary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(6)
                .background(Color.platformSecondaryBackground)
                .cornerRadius(6)

                Spacer()

                // Font size controls
                HStack(spacing: 4) {
                    Button {
                        fontSize = max(8, fontSize - 1)
                    } label: {
                        Image(systemName: "textformat.size.smaller")
                    }
                    .buttonStyle(.borderless)

                    Text("\(Int(fontSize))pt")
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .frame(width: 32)

                    Button {
                        fontSize = min(24, fontSize + 1)
                    } label: {
                        Image(systemName: "textformat.size.larger")
                    }
                    .buttonStyle(.borderless)
                }

                Button {
                    copyToClipboard()
                } label: {
                    Label("Copy", systemImage: "doc.on.doc")
                }
                .buttonStyle(.bordered)
            }
            .padding()

            Divider()

            // Content area
            if isLoading {
                ProgressView("Generating \(selectedFormat.rawValue)...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if rawContent.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "doc.text")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("No content to display")
                        .font(.headline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ScrollView([.horizontal, .vertical]) {
                    Text(filteredContent)
                        .font(.system(size: fontSize, design: .monospaced))
                        .textSelection(.enabled)
                        .padding()
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                .background(Color.platformTertiaryBackground)
            }

            // Status bar
            Divider()
            HStack {
                Text(statusText)
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                if !searchText.isEmpty {
                    Text(matchCountText)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.horizontal)
            .padding(.vertical, 6)
            .background(Color.platformBackground)
        }
        .navigationTitle("Raw Data")
        .onChange(of: selectedFormat) { _ in
            loadContent()
        }
        .onAppear {
            loadContent()
        }
    }

    private var statusText: String {
        let lines = rawContent.components(separatedBy: "\n").count
        let chars = rawContent.count
        return "\(lines) lines, \(formatNumber(chars)) characters"
    }

    private var matchCountText: String {
        guard !searchText.isEmpty else { return "" }
        let lines = rawContent.components(separatedBy: "\n")
        let matchCount = lines.filter { $0.localizedCaseInsensitiveContains(searchText) }.count
        return "\(matchCount) matching lines"
    }

    private func formatNumber(_ n: Int) -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        return formatter.string(from: NSNumber(value: n)) ?? "\(n)"
    }

    private func loadContent() {
        guard let engine = appState.engine else {
            rawContent = ""
            return
        }

        isLoading = true

        // Run in background to not block UI
        Task {
            do {
                let content: String
                switch selectedFormat {
                case .xml:
                    let xml = try engine.toXml()
                    content = formatXML(xml)
                case .json:
                    content = try engine.toPrettyJson()
                }

                await MainActor.run {
                    rawContent = content
                    isLoading = false
                }
            } catch {
                await MainActor.run {
                    rawContent = "Error generating \(selectedFormat.rawValue): \(error.localizedDescription)"
                    isLoading = false
                }
            }
        }
    }

    private func formatXML(_ xml: String) -> String {
        // Pretty print XML with indentation
        var result = ""
        var indentLevel = 0
        let indentString = "  "
        var currentTag = ""
        var i = xml.startIndex

        while i < xml.endIndex {
            let char = xml[i]

            if char == "<" {
                // Check if it's a closing tag
                let nextIndex = xml.index(after: i)
                if nextIndex < xml.endIndex && xml[nextIndex] == "/" {
                    indentLevel = max(0, indentLevel - 1)
                }

                // Add text content before tag if present
                let textContent = currentTag.trimmingCharacters(in: .whitespacesAndNewlines)
                if !textContent.isEmpty {
                    result += textContent
                }
                currentTag = ""

                if !result.isEmpty && !result.hasSuffix("\n") {
                    result += "\n"
                }
                result += String(repeating: indentString, count: indentLevel)
                currentTag = String(char)
            } else if char == ">" {
                currentTag += String(char)
                result += currentTag

                // Check if self-closing or opening tag
                if !currentTag.contains("</") && !currentTag.contains("/>") && !currentTag.contains("<?") && !currentTag.contains("<!") {
                    indentLevel += 1
                }

                currentTag = ""
            } else {
                currentTag += String(char)
            }

            i = xml.index(after: i)
        }

        // Add any remaining content
        if !currentTag.isEmpty {
            result += currentTag
        }

        return result
    }

    private func copyToClipboard() {
        #if os(macOS)
        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.setString(rawContent, forType: .string)
        #else
        UIPasteboard.general.string = rawContent
        #endif
    }
}

#Preview {
    RawDataView()
        .environmentObject(AppState())
}
