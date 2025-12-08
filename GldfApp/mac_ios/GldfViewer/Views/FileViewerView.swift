// FileViewerView.swift
// Tab-based viewer for GLDF embedded files

import SwiftUI
import GldfKit
import UniformTypeIdentifiers
import SceneKit
import ModelIO
import PDFKit

#if os(iOS)
// iOS placeholder - full file viewer is macOS only for now
struct FileViewerView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "eye.slash")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("File Viewer")
                .font(.title2)
                .fontWeight(.semibold)

            Text("Detailed file viewing is available on macOS.\nOn iOS, you can view files from the Files list.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            if !appState.openFileTabs.isEmpty {
                Divider()
                    .frame(width: 200)

                Text("Open files: \(appState.openFileTabs.count)")
                    .font(.caption)

                ForEach(appState.openFileTabs) { tab in
                    HStack {
                        Image(systemName: "doc")
                        Text(tab.fileName)
                            .lineLimit(1)
                        Spacer()
                        Button {
                            appState.closeFileTab(fileId: tab.id)
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(.secondary)
                        }
                    }
                    .padding(.horizontal)
                }
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
#else
// macOS full implementation
struct FileViewerView: View {
    @EnvironmentObject var appState: AppState
    @State private var isDragging = false

    var body: some View {
        VStack(spacing: 0) {
            // Tab bar with add button
            HStack(spacing: 0) {
                if !appState.openFileTabs.isEmpty {
                    TabBarView()
                }
                Spacer()
                // Add file button
                Button {
                    openFilePanel()
                } label: {
                    Image(systemName: "plus")
                        .frame(width: 28, height: 28)
                }
                .buttonStyle(.borderless)
                .help("Open file (.ldt, .ies, .l3d)")
                .padding(.trailing, 8)
            }
            .frame(height: 32)
            .background(Color.platformSecondaryBackground)

            Divider()

            // Content area with drop support
            ZStack {
                if let selectedId = appState.selectedFileTabId,
                   let tab = appState.openFileTabs.first(where: { $0.id == selectedId }) {
                    FileContentView(tab: tab)
                } else {
                    DropZoneView(isDragging: $isDragging)
                }

                // Drop overlay
                if isDragging {
                    RoundedRectangle(cornerRadius: 12)
                        .strokeBorder(Color.accentColor, style: StrokeStyle(lineWidth: 3, dash: [10]))
                        .background(Color.accentColor.opacity(0.1))
                        .padding(20)
                }
            }
            .onDrop(of: [.fileURL], isTargeted: $isDragging) { providers in
                handleFileDrop(providers: providers)
            }
        }
        .frame(minWidth: 600, minHeight: 400)
    }

    private func openFilePanel() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [
            UTType(filenameExtension: "ldt")!,
            UTType(filenameExtension: "ies")!,
            UTType(filenameExtension: "l3d")!
        ]
        panel.allowsMultipleSelection = true
        panel.message = "Select photometric or geometry files"

        if panel.runModal() == .OK {
            for url in panel.urls {
                openExternalFile(url: url)
            }
        }
    }

    private func handleFileDrop(providers: [NSItemProvider]) -> Bool {
        for provider in providers {
            provider.loadItem(forTypeIdentifier: UTType.fileURL.identifier, options: nil) { item, _ in
                guard let data = item as? Data,
                      let url = URL(dataRepresentation: data, relativeTo: nil) else { return }

                let ext = url.pathExtension.lowercased()
                if ["ldt", "ies", "l3d"].contains(ext) {
                    DispatchQueue.main.async {
                        openExternalFile(url: url)
                    }
                }
            }
        }
        return true
    }

    private func openExternalFile(url: URL) {
        let fileId = UUID().uuidString
        let fileName = url.lastPathComponent
        let ext = url.pathExtension.lowercased()

        let contentType: String
        switch ext {
        case "ldt": contentType = "ldc/eulumdat"
        case "ies": contentType = "ldc/ies"
        case "l3d": contentType = "geo/l3d"
        default: contentType = "unknown"
        }

        var newTab = OpenFileTab(
            id: fileId,
            fileName: fileName,
            contentType: contentType
        )

        // Load file content
        do {
            let data = try Data(contentsOf: url)
            if contentType.starts(with: "ldc") {
                newTab.textContent = String(data: data, encoding: .utf8) ?? String(data: data, encoding: .isoLatin1)
            } else {
                newTab.data = data
            }
            newTab.isLoading = false
        } catch {
            newTab.error = error.localizedDescription
            newTab.isLoading = false
        }

        appState.openFileTabs.append(newTab)
        appState.selectedFileTabId = fileId
    }
}

// MARK: - Drop Zone View
struct DropZoneView: View {
    @Binding var isDragging: Bool

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "arrow.down.doc")
                .font(.system(size: 48))
                .foregroundColor(isDragging ? .accentColor : .secondary)

            Text("Drop files here")
                .font(.headline)
                .foregroundColor(isDragging ? .accentColor : .secondary)

            Text("or click + to open .ldt, .ies, .l3d files")
                .font(.subheadline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Tab Bar
struct TabBarView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 0) {
                ForEach(appState.openFileTabs) { tab in
                    TabItemView(tab: tab, isSelected: tab.id == appState.selectedFileTabId)
                        .onTapGesture {
                            appState.selectedFileTabId = tab.id
                        }
                }
                Spacer()
            }
        }
        .frame(height: 32)
        .background(Color.platformSecondaryBackground)
    }
}

struct TabItemView: View {
    let tab: OpenFileTab
    let isSelected: Bool
    @EnvironmentObject var appState: AppState

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: iconForContentType(tab.contentType))
                .font(.system(size: 12))
                .foregroundColor(colorForContentType(tab.contentType))

            Text(tab.fileName)
                .font(.system(size: 12))
                .lineLimit(1)

            Button {
                appState.closeFileTab(fileId: tab.id)
            } label: {
                Image(systemName: "xmark")
                    .font(.system(size: 9, weight: .bold))
                    .foregroundColor(.secondary)
            }
            .buttonStyle(.plain)
            .opacity(isSelected ? 1 : 0.5)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 6)
        .background(isSelected ? Color.platformSelectedBackground : Color.clear)
        .cornerRadius(4)
    }

    private func iconForContentType(_ type: String) -> String {
        if type.starts(with: "ldc") { return "rays" }
        if type.starts(with: "image") { return "photo" }
        if type == "geo/l3d" { return "cube" }
        if type.starts(with: "document") { return "doc" }
        return "doc.questionmark"
    }

    private func colorForContentType(_ type: String) -> Color {
        if type.starts(with: "ldc") { return .orange }
        if type.starts(with: "image") { return .blue }
        if type == "geo/l3d" { return .green }
        return .secondary
    }
}

// MARK: - File Content View
struct FileContentView: View {
    let tab: OpenFileTab

    var body: some View {
        Group {
            if tab.isLoading {
                ProgressView("Loading \(tab.fileName)...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error = tab.error {
                ErrorContentView(fileName: tab.fileName, error: error)
            } else if tab.contentType.starts(with: "image") {
                ImageContentView(tab: tab)
            } else if tab.contentType.starts(with: "ldc") {
                PhotometryContentView(tab: tab)
            } else if tab.contentType == "geo/l3d" {
                GeometryContentView(tab: tab)
            } else if tab.contentType == "document/pdf" || tab.fileName.lowercased().hasSuffix(".pdf") {
                PDFContentView(tab: tab)
            } else {
                GenericContentView(tab: tab)
            }
        }
    }
}

// MARK: - PDF Content View
struct PDFContentView: View {
    let tab: OpenFileTab

    var body: some View {
        if let data = tab.data, let document = PDFDocument(data: data) {
            PDFKitView(document: document)
        } else {
            VStack(spacing: 16) {
                Image(systemName: "doc.text")
                    .font(.system(size: 48))
                    .foregroundColor(.secondary)
                Text("Unable to load PDF")
                    .font(.headline)
                    .foregroundColor(.secondary)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

// MARK: - PDFKit View (Platform-specific)
#if os(macOS)
struct PDFKitView: NSViewRepresentable {
    let document: PDFDocument

    func makeNSView(context: Context) -> PDFView {
        let pdfView = PDFView()
        pdfView.document = document
        pdfView.autoScales = true
        pdfView.displayMode = .singlePageContinuous
        pdfView.displayDirection = .vertical
        return pdfView
    }

    func updateNSView(_ pdfView: PDFView, context: Context) {
        pdfView.document = document
    }
}
#else
struct PDFKitView: UIViewRepresentable {
    let document: PDFDocument

    func makeUIView(context: Context) -> PDFView {
        let pdfView = PDFView()
        pdfView.document = document
        pdfView.autoScales = true
        pdfView.displayMode = .singlePageContinuous
        pdfView.displayDirection = .vertical
        return pdfView
    }

    func updateUIView(_ pdfView: PDFView, context: Context) {
        pdfView.document = document
    }
}
#endif

// MARK: - Image Content View
struct ImageContentView: View {
    let tab: OpenFileTab
    @State private var scale: CGFloat = 1.0

    var body: some View {
        VStack {
            #if os(macOS)
            if let data = tab.data, let nsImage = NSImage(data: data) {
                ScrollView([.horizontal, .vertical]) {
                    Image(nsImage: nsImage)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .scaleEffect(scale)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }

                // Zoom controls
                HStack {
                    Button {
                        scale = max(0.1, scale - 0.25)
                    } label: {
                        Image(systemName: "minus.magnifyingglass")
                    }

                    Text("\(Int(scale * 100))%")
                        .frame(width: 50)

                    Button {
                        scale = min(5.0, scale + 0.25)
                    } label: {
                        Image(systemName: "plus.magnifyingglass")
                    }

                    Button {
                        scale = 1.0
                    } label: {
                        Text("Fit")
                    }
                }
                .padding()
            } else {
                Text("Unable to load image")
                    .foregroundColor(.secondary)
            }
            #else
            if let data = tab.data, let uiImage = UIImage(data: data) {
                ScrollView([.horizontal, .vertical]) {
                    Image(uiImage: uiImage)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .scaleEffect(scale)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }

                // Zoom controls
                HStack {
                    Button {
                        scale = max(0.1, scale - 0.25)
                    } label: {
                        Image(systemName: "minus.magnifyingglass")
                    }

                    Text("\(Int(scale * 100))%")
                        .frame(width: 50)

                    Button {
                        scale = min(5.0, scale + 0.25)
                    } label: {
                        Image(systemName: "plus.magnifyingglass")
                    }

                    Button {
                        scale = 1.0
                    } label: {
                        Text("Fit")
                    }
                }
                .padding()
            } else {
                Text("Unable to load image")
                    .foregroundColor(.secondary)
            }
            #endif
        }
    }
}

// MARK: - Photometry Content View (LDT/IES)
struct PhotometryContentView: View {
    let tab: OpenFileTab
    @State private var showRawData = false
    @State private var diagramType: DiagramType = .polar
    @State private var parsedData: GldfKit.EulumdatData?

    enum DiagramType: String, CaseIterable {
        case polar = "Polar"
        case cartesian = "Cartesian"
    }

    var body: some View {
        HSplitView {
            // Diagram area
            VStack(spacing: 0) {
                // Toolbar
                HStack {
                    Text("Light Distribution")
                        .font(.headline)

                    Spacer()

                    Picker("Diagram", selection: $diagramType) {
                        ForEach(DiagramType.allCases, id: \.self) { type in
                            Text(type.rawValue).tag(type)
                        }
                    }
                    .pickerStyle(.segmented)
                    .frame(width: 180)
                }
                .padding()

                // Diagram
                if let content = tab.textContent {
                    Group {
                        switch diagramType {
                        case .polar:
                            PolarDiagramView(ldtContent: content, parsedData: $parsedData)
                        case .cartesian:
                            CartesianDiagramView(ldtContent: content, parsedData: $parsedData)
                        }
                    }
                    .frame(minWidth: 300, minHeight: 300)
                    .padding()
                } else {
                    Text("No photometric data")
                        .foregroundColor(.secondary)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                }

                // Info panel
                if let data = parsedData {
                    PhotometryInfoPanel(data: data)
                }
            }
            .frame(minWidth: 400)

            // Raw data view
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Text("Raw Data")
                        .font(.headline)
                    Spacer()
                    Toggle("Show", isOn: $showRawData)
                        .toggleStyle(.switch)
                        .controlSize(.small)
                }
                .padding()

                if showRawData, let content = tab.textContent {
                    ScrollView {
                        Text(content)
                            .font(.system(size: 11, design: .monospaced))
                            .textSelection(.enabled)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding()
                    }
                    .background(Color.platformTertiaryBackground)
                } else {
                    Spacer()
                }
            }
            .frame(minWidth: 200)
        }
    }
}

// MARK: - Photometry Info Panel
struct PhotometryInfoPanel: View {
    let data: GldfKit.EulumdatData

    var body: some View {
        let numCPlanes = actualCPlaneCount(from: data)
        HStack(spacing: 20) {
            InfoItem(label: "Manufacturer", value: data.manufacturer)
            InfoItem(label: "Luminaire", value: data.luminaireName)
            InfoItem(label: "Lamp", value: data.lampType)
            InfoItem(label: "Lumens", value: String(format: "%.0f lm", data.totalLumens))
            InfoItem(label: "LORL", value: String(format: "%.1f%%", data.lorl))
            InfoItem(label: "C Planes", value: "\(data.cPlaneCount) (\(numCPlanes) with data)")
            InfoItem(label: "Gamma", value: "\(data.gammaCount) angles")
            InfoItem(label: "Max I", value: String(format: "%.0f cd", data.maxIntensity))
        }
        .padding()
        .background(Color.platformSecondaryBackground)
    }
}

struct InfoItem: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value.isEmpty ? "—" : value)
                .font(.caption)
                .lineLimit(1)
        }
    }
}

// MARK: - EULUMDAT Parsing (via FFI)
// Note: EulumdatData is provided by GldfKit FFI (cross-platform Rust parser)
// Use parseEulumdat(content:) from GldfKit to parse LDT files

/// Helper to get intensity for a specific C plane and gamma index from flat array
func getIntensity(from data: GldfKit.EulumdatData, cPlaneIndex: Int, gammaIndex: Int) -> Double {
    let index = cPlaneIndex * Int(data.gammaCount) + gammaIndex
    guard index >= 0, index < data.intensities.count else { return 0 }
    return data.intensities[index]
}

/// Get intensities for a specific C plane
func getPlaneIntensities(from data: GldfKit.EulumdatData, cPlaneIndex: Int) -> [Double] {
    let start = cPlaneIndex * Int(data.gammaCount)
    let end = start + Int(data.gammaCount)
    guard start >= 0, end <= data.intensities.count else { return [] }
    return Array(data.intensities[start..<end])
}

/// Calculate actual number of C planes with data based on symmetry
func actualCPlaneCount(from data: GldfKit.EulumdatData) -> Int {
    let (mc1, mc2) = calculateMcRange(symmetry: Int(data.symmetry), nCPlanes: Int(data.cPlaneCount))
    return mc2 - mc1 + 1
}

/// Calculate mc1 and mc2 based on symmetry (mirrors Rust implementation)
func calculateMcRange(symmetry: Int, nCPlanes: Int) -> (Int, Int) {
    switch symmetry {
    case 0: return (1, nCPlanes)                    // No symmetry
    case 1: return (1, 1)                           // Symmetry about vertical axis
    case 2: return (1, nCPlanes / 2 + 1)            // C0-C180 plane symmetry
    case 3:                                         // C90-C270 plane symmetry
        let mc1 = 3 * (nCPlanes / 4) + 1
        return (mc1, mc1 + nCPlanes / 2)
    case 4: return (1, nCPlanes / 4 + 1)            // C0-C180 and C90-C270 symmetry
    default: return (1, max(1, nCPlanes))
    }
}

// MARK: - Polar Diagram View
struct PolarDiagramView: View {
    let ldtContent: String
    @Binding var parsedData: GldfKit.EulumdatData?

    var body: some View {
        GeometryReader { geometry in
            let size = min(geometry.size.width, geometry.size.height)
            let center = CGPoint(x: geometry.size.width / 2, y: geometry.size.height / 2)
            let radius = size / 2 - 50

            ZStack {
                Canvas { context, _ in
                    // Background circles with intensity labels
                    for fraction in [0.25, 0.5, 0.75, 1.0] {
                        let circleRadius = radius * fraction
                        let rect = CGRect(
                            x: center.x - circleRadius,
                            y: center.y - circleRadius,
                            width: circleRadius * 2,
                            height: circleRadius * 2
                        )
                        context.stroke(Circle().path(in: rect), with: .color(.gray.opacity(0.3)), lineWidth: 1)
                    }

                    // Angle lines (every 30 degrees)
                    for i in 0..<12 {
                        let angle = Double(i) * 30 * .pi / 180
                        var path = Path()
                        path.move(to: center)
                        path.addLine(to: CGPoint(
                            x: center.x + radius * sin(angle),
                            y: center.y - radius * cos(angle)
                        ))
                        context.stroke(path, with: .color(.gray.opacity(0.3)), lineWidth: 1)
                    }

                    // Draw intensity curves
                    if let data = parsedData, !data.intensities.isEmpty, !data.gammaAngles.isEmpty {
                        let numCPlanes = actualCPlaneCount(from: data)

                        // Draw C0 plane (first plane)
                        let firstPlane = getPlaneIntensities(from: data, cPlaneIndex: 0)
                        if !firstPlane.isEmpty {
                            drawIntensityCurve(context: context, center: center, radius: radius,
                                              intensities: firstPlane, maxIntensity: data.maxIntensity,
                                              color: .orange, gammaAngles: data.gammaAngles)
                        }

                        // Draw C90 plane if available
                        if numCPlanes > 1 {
                            // Find index for C90 plane based on C angles
                            var c90Index = 0
                            if let idx = data.cAngles.firstIndex(where: { abs($0 - 90) < 1 }) {
                                c90Index = idx
                            } else if !data.cAngles.isEmpty {
                                // Estimate: for half-sphere data, C90 is roughly at 1/4 point
                                c90Index = min(numCPlanes / 4, numCPlanes - 1)
                            }

                            if c90Index > 0 && c90Index < numCPlanes {
                                let c90Plane = getPlaneIntensities(from: data, cPlaneIndex: c90Index)
                                if !c90Plane.isEmpty {
                                    drawIntensityCurve(context: context, center: center, radius: radius,
                                                      intensities: c90Plane, maxIntensity: data.maxIntensity,
                                                      color: .blue, gammaAngles: data.gammaAngles)
                                }
                            }
                        }
                    }

                    // Draw angle labels
                    for angle in [0, 30, 60, 90, 120, 150, 180] {
                        let radians = Double(angle) * .pi / 180
                        let labelPoint = CGPoint(
                            x: center.x + (radius + 25) * sin(radians),
                            y: center.y - (radius + 25) * cos(radians)
                        )
                        context.draw(Text("\(angle)°").font(.caption2).foregroundColor(.secondary), at: labelPoint)
                    }
                }

                // Show "No data" message if parsing failed
                if let data = parsedData, data.intensities.isEmpty {
                    VStack {
                        Text("No intensity data found")
                            .foregroundColor(.secondary)
                        Text("C planes: \(data.cPlaneCount), Gamma angles: \(data.gammaCount)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } else if parsedData == nil {
                    Text("Parsing LDT data...")
                        .foregroundColor(.secondary)
                }

                // Legend
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 4) {
                        Circle().fill(Color.orange).frame(width: 8, height: 8)
                        Text("C0-C180").font(.caption2)
                    }
                    HStack(spacing: 4) {
                        Circle().fill(Color.blue).frame(width: 8, height: 8)
                        Text("C90-C270").font(.caption2)
                    }
                    if let data = parsedData {
                        Divider()
                        Text("Max: \(Int(data.maxIntensity)) cd").font(.caption2).foregroundColor(.secondary)
                    }
                }
                .padding(8)
                .background(Color.platformSecondaryBackground.opacity(0.9))
                .cornerRadius(6)
                .position(x: 70, y: 50)
            }
        }
        .onAppear {
            // Use FFI parser (cross-platform Rust implementation)
            parsedData = GldfKit.parseEulumdat(content: ldtContent)
        }
    }

    private func drawIntensityCurve(context: GraphicsContext, center: CGPoint, radius: CGFloat,
                                    intensities: [Double], maxIntensity: Double, color: Color,
                                    gammaAngles: [Double]) {
        guard !intensities.isEmpty, maxIntensity > 0 else { return }

        var path = Path()

        // Draw the curve from 0 to max gamma angle
        for (index, intensity) in intensities.enumerated() {
            let gamma: Double
            if index < gammaAngles.count {
                gamma = gammaAngles[index]
            } else {
                // Fallback: distribute evenly
                gamma = Double(index) * 180.0 / Double(max(1, intensities.count - 1))
            }

            let radians = gamma * .pi / 180
            let normalizedIntensity = min(1.0, intensity / maxIntensity)
            let r = radius * CGFloat(normalizedIntensity)

            let point = CGPoint(
                x: center.x + r * Foundation.sin(radians),
                y: center.y - r * Foundation.cos(radians)
            )

            if index == 0 {
                path.move(to: point)
            } else {
                path.addLine(to: point)
            }
        }

        // Mirror for full polar (from max gamma to 360-max gamma)
        // This creates the symmetric lower half
        for (index, intensity) in intensities.reversed().enumerated() {
            let originalIndex = intensities.count - 1 - index
            let gamma: Double
            if originalIndex < gammaAngles.count {
                gamma = gammaAngles[originalIndex]
            } else {
                gamma = Double(originalIndex) * 180.0 / Double(max(1, intensities.count - 1))
            }

            let radians = (360 - gamma) * .pi / 180
            let normalizedIntensity = min(1.0, intensity / maxIntensity)
            let r = radius * CGFloat(normalizedIntensity)

            let point = CGPoint(
                x: center.x + r * Foundation.sin(radians),
                y: center.y - r * Foundation.cos(radians)
            )
            path.addLine(to: point)
        }

        path.closeSubpath()

        context.fill(path, with: .color(color.opacity(0.15)))
        context.stroke(path, with: .color(color), lineWidth: 2)
    }
}

// MARK: - Cartesian Diagram View
struct CartesianDiagramView: View {
    let ldtContent: String
    @Binding var parsedData: GldfKit.EulumdatData?

    var body: some View {
        GeometryReader { geometry in
            let margin: CGFloat = 60
            let width = geometry.size.width - margin * 2
            let height = geometry.size.height - margin * 2

            ZStack {
                Canvas { context, _ in
                    let origin = CGPoint(x: margin, y: geometry.size.height - margin)

                    // Draw axes
                    var axisPath = Path()
                    axisPath.move(to: CGPoint(x: margin, y: margin))
                    axisPath.addLine(to: origin)
                    axisPath.addLine(to: CGPoint(x: geometry.size.width - margin, y: origin.y))
                    context.stroke(axisPath, with: .color(.gray), lineWidth: 1)

                    // Horizontal grid lines with Y axis labels
                    for i in 0...4 {
                        let y = origin.y - height * CGFloat(i) / 4
                        var gridPath = Path()
                        gridPath.move(to: CGPoint(x: margin, y: y))
                        gridPath.addLine(to: CGPoint(x: geometry.size.width - margin, y: y))
                        context.stroke(gridPath, with: .color(.gray.opacity(0.3)), lineWidth: 1)

                        // Y axis percentage labels
                        let percentage = i * 25
                        context.draw(Text("\(percentage)%").font(.caption2).foregroundColor(.secondary),
                                    at: CGPoint(x: margin - 25, y: y))
                    }

                    // Vertical grid lines with X axis labels
                    let maxGamma = parsedData?.gammaAngles.last ?? 90
                    let angleSteps = maxGamma <= 90 ? [0, 15, 30, 45, 60, 75, 90] : [0, 30, 60, 90, 120, 150, 180]

                    for angle in angleSteps {
                        if Double(angle) > maxGamma { continue }
                        let xFraction = maxGamma > 0 ? Double(angle) / maxGamma : 0
                        let x = margin + width * CGFloat(xFraction)

                        var gridPath = Path()
                        gridPath.move(to: CGPoint(x: x, y: margin))
                        gridPath.addLine(to: CGPoint(x: x, y: origin.y))
                        context.stroke(gridPath, with: .color(.gray.opacity(0.3)), lineWidth: 1)

                        // X axis labels
                        context.draw(Text("\(angle)°").font(.caption2).foregroundColor(.secondary),
                                    at: CGPoint(x: x, y: origin.y + 15))
                    }

                    // Draw intensity curves
                    if let data = parsedData, !data.intensities.isEmpty, !data.gammaAngles.isEmpty {
                        let maxAngle = data.gammaAngles.last ?? 90
                        let numCPlanes = actualCPlaneCount(from: data)

                        // C0 plane
                        let firstPlane = getPlaneIntensities(from: data, cPlaneIndex: 0)
                        if !firstPlane.isEmpty {
                            var curvePath = Path()

                            for (index, intensity) in firstPlane.enumerated() {
                                let gamma = index < data.gammaAngles.count ? data.gammaAngles[index] : Double(index)
                                let xFraction = maxAngle > 0 ? gamma / maxAngle : 0
                                let x = margin + width * CGFloat(xFraction)
                                let normalizedIntensity = data.maxIntensity > 0 ? intensity / data.maxIntensity : 0
                                let y = origin.y - height * CGFloat(normalizedIntensity)

                                if index == 0 {
                                    curvePath.move(to: CGPoint(x: x, y: y))
                                } else {
                                    curvePath.addLine(to: CGPoint(x: x, y: y))
                                }
                            }

                            context.stroke(curvePath, with: .color(.orange), lineWidth: 2)
                        }

                        // C90 plane
                        if numCPlanes > 1 {
                            var c90Index = 0
                            if let idx = data.cAngles.firstIndex(where: { abs($0 - 90) < 1 }) {
                                c90Index = idx
                            } else if !data.cAngles.isEmpty {
                                c90Index = min(numCPlanes / 4, numCPlanes - 1)
                            }

                            if c90Index > 0 && c90Index < numCPlanes {
                                var c90Path = Path()
                                let c90Plane = getPlaneIntensities(from: data, cPlaneIndex: c90Index)

                                for (index, intensity) in c90Plane.enumerated() {
                                    let gamma = index < data.gammaAngles.count ? data.gammaAngles[index] : Double(index)
                                    let xFraction = maxAngle > 0 ? gamma / maxAngle : 0
                                    let x = margin + width * CGFloat(xFraction)
                                    let normalizedIntensity = data.maxIntensity > 0 ? intensity / data.maxIntensity : 0
                                    let y = origin.y - height * CGFloat(normalizedIntensity)

                                    if index == 0 {
                                        c90Path.move(to: CGPoint(x: x, y: y))
                                    } else {
                                        c90Path.addLine(to: CGPoint(x: x, y: y))
                                    }
                                }
                                context.stroke(c90Path, with: .color(.blue), lineWidth: 2)
                            }
                        }
                    }

                    // Axis labels
                    context.draw(Text("Intensity (cd)").font(.caption2).foregroundColor(.secondary),
                                at: CGPoint(x: margin - 35, y: margin - 15))
                    context.draw(Text("γ (°)").font(.caption2).foregroundColor(.secondary),
                                at: CGPoint(x: geometry.size.width - margin + 15, y: origin.y))
                }

                // Show "No data" message if parsing failed
                if let data = parsedData, data.intensities.isEmpty {
                    Text("No intensity data found")
                        .foregroundColor(.secondary)
                }

                // Legend
                VStack(alignment: .leading, spacing: 4) {
                    HStack(spacing: 4) {
                        Rectangle().fill(Color.orange).frame(width: 12, height: 3)
                        Text("C0-C180").font(.caption2)
                    }
                    HStack(spacing: 4) {
                        Rectangle().fill(Color.blue).frame(width: 12, height: 3)
                        Text("C90-C270").font(.caption2)
                    }
                    if let data = parsedData {
                        Divider()
                        Text("Max: \(Int(data.maxIntensity)) cd").font(.caption2).foregroundColor(.secondary)
                    }
                }
                .padding(8)
                .background(Color.platformSecondaryBackground.opacity(0.9))
                .cornerRadius(6)
                .position(x: geometry.size.width - 80, y: 50)
            }
        }
        .onAppear {
            if parsedData == nil {
                // Use FFI parser (cross-platform Rust implementation)
                parsedData = GldfKit.parseEulumdat(content: ldtContent)
            }
        }
    }
}

// MARK: - Geometry Content View (3D)
struct GeometryContentView: View {
    let tab: OpenFileTab
    @State private var l3dFile: GldfKit.L3dFile?
    @State private var error: String?
    @State private var isLoading = true

    var body: some View {
        Group {
            if isLoading {
                ProgressView("Loading L3D...")
            } else if let error = error {
                VStack(spacing: 16) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 48))
                        .foregroundColor(.orange)
                    Text("Failed to load L3D")
                        .font(.headline)
                    Text(error)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            } else if let l3d = l3dFile {
                L3dSceneView(l3dFile: l3d)
            } else {
                Text("No L3D data available")
                    .foregroundColor(.secondary)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .onAppear {
            loadL3d()
        }
    }

    private func loadL3d() {
        guard let data = tab.data else {
            error = "No data available"
            isLoading = false
            return
        }

        // Parse L3D using FFI (cross-platform Rust parser)
        do {
            l3dFile = try GldfKit.parseL3d(data: data)
            isLoading = false
        } catch {
            self.error = error.localizedDescription
            isLoading = false
        }
    }
}

// MARK: - L3D SceneKit View
struct L3dSceneView: View {
    let l3dFile: GldfKit.L3dFile
    @State private var scene: SCNScene?
    @State private var loadError: String?

    var body: some View {
        VStack(spacing: 0) {
            // Info bar
            HStack {
                Text("L3D Scene")
                    .font(.headline)
                Spacer()
                Text("\(l3dFile.scene.parts.count) parts")
                    .foregroundColor(.secondary)
                Text("•")
                    .foregroundColor(.secondary)
                Text("\(l3dFile.scene.geometryDefinitions.count) geometries")
                    .foregroundColor(.secondary)
                if !l3dFile.scene.joints.isEmpty {
                    Text("•")
                        .foregroundColor(.secondary)
                    Text("\(l3dFile.scene.joints.count) joints")
                        .foregroundColor(.secondary)
                }
            }
            .padding()

            Divider()

            // 3D View
            if let scene = scene {
                SceneView(
                    scene: scene,
                    options: [.allowsCameraControl]
                )
            } else if let error = loadError {
                VStack {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.system(size: 48))
                        .foregroundColor(.orange)
                    Text(error)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                ProgressView("Building scene...")
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }

            Divider()

            // Parts list
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(l3dFile.scene.parts, id: \.partName) { part in
                        L3dPartBadge(part: part)
                    }
                }
                .padding(.horizontal)
            }
            .frame(height: 60)
        }
        .onAppear {
            buildScene()
        }
    }

    private func buildScene() {
        // Build scene on main thread to avoid threading issues with SceneKit
        let newScene = SCNScene()

        // Set background color to match WASM version (light gray: 0.8, 0.8, 0.8)
        newScene.background.contents = NSColor(calibratedRed: 0.8, green: 0.8, blue: 0.8, alpha: 1.0)

        // Ambient light - matches WASM: intensity 0.4
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.color = NSColor.white
        ambientLight.light?.intensity = 400  // SceneKit uses 0-1000 scale, WASM uses 0-1
        newScene.rootNode.addChildNode(ambientLight)

        // Main directional light - matches WASM: intensity 2.0 from (-1, -1, -1)
        let mainLight = SCNNode()
        mainLight.light = SCNLight()
        mainLight.light?.type = .directional
        mainLight.light?.color = NSColor.white
        mainLight.light?.intensity = 2000  // Brighter main light
        mainLight.position = SCNVector3(x: -1, y: -1, z: -1)
        mainLight.look(at: SCNVector3(0, 0, 0))
        newScene.rootNode.addChildNode(mainLight)

        // Fill light - matches WASM: intensity 0.2 from (-0.1, 0.5, 0.5)
        let fillLight = SCNNode()
        fillLight.light = SCNLight()
        fillLight.light?.type = .directional
        fillLight.light?.color = NSColor.white
        fillLight.light?.intensity = 200  // Softer fill light
        fillLight.position = SCNVector3(x: -0.1, y: 0.5, z: 0.5)
        fillLight.look(at: SCNVector3(0, 0, 0))
        newScene.rootNode.addChildNode(fillLight)

        // Cache for loaded OBJ geometries
        var geometryCache: [String: SCNNode] = [:]

        // Load each part
        for part in l3dFile.scene.parts {
            // Find geometry definition
            guard let geoDef = l3dFile.scene.geometryDefinitions.first(where: { $0.id == part.geometryId }) else {
                continue
            }

            // Load OBJ if not cached
            if geometryCache[geoDef.id] == nil {
                if let objNode = loadObjFromL3d(geoDef: geoDef) {
                    geometryCache[geoDef.id] = objNode
                }
            }

            // Clone cached geometry and apply transform
            if let templateNode = geometryCache[geoDef.id] {
                let partNode = templateNode.clone()
                partNode.name = part.partName

                // Apply world transform from FFI (column-major 4x4 matrix)
                let matrix = part.worldTransform.values
                if matrix.count == 16 {
                    let transform = SCNMatrix4(
                        m11: CGFloat(matrix[0]), m12: CGFloat(matrix[1]), m13: CGFloat(matrix[2]), m14: CGFloat(matrix[3]),
                        m21: CGFloat(matrix[4]), m22: CGFloat(matrix[5]), m23: CGFloat(matrix[6]), m24: CGFloat(matrix[7]),
                        m31: CGFloat(matrix[8]), m32: CGFloat(matrix[9]), m33: CGFloat(matrix[10]), m34: CGFloat(matrix[11]),
                        m41: CGFloat(matrix[12]), m42: CGFloat(matrix[13]), m43: CGFloat(matrix[14]), m44: CGFloat(matrix[15])
                    )
                    partNode.transform = transform
                }

                newScene.rootNode.addChildNode(partNode)
            }
        }

        // Add light emitting objects as yellow indicators
        for part in l3dFile.scene.parts {
            for leo in part.lightEmittingObjects {
                let leoNode = SCNNode()
                let size: CGFloat = leo.shapeDimensions.first.map { CGFloat($0) } ?? 0.05

                if leo.shapeType == "circle" {
                    leoNode.geometry = SCNSphere(radius: size / 2)
                } else {
                    let w = leo.shapeDimensions.count > 0 ? CGFloat(leo.shapeDimensions[0]) : 0.05
                    let h = leo.shapeDimensions.count > 1 ? CGFloat(leo.shapeDimensions[1]) : 0.05
                    leoNode.geometry = SCNBox(width: w, height: h, length: 0.01, chamferRadius: 0)
                }

                leoNode.geometry?.firstMaterial?.diffuse.contents = NSColor.yellow
                leoNode.geometry?.firstMaterial?.emission.contents = NSColor.yellow
                leoNode.name = "LEO: \(leo.partName)"
                leoNode.position = SCNVector3(Float(leo.position.x), Float(leo.position.y), Float(leo.position.z))

                newScene.rootNode.addChildNode(leoNode)
            }
        }

        self.scene = newScene
    }

    private func loadObjFromL3d(geoDef: GldfKit.L3dGeometryDef) -> SCNNode? {
        // Find asset in L3D file
        guard let assetData = GldfKit.getL3dAsset(l3dFile: l3dFile, filename: geoDef.filename) else {
            return nil
        }

        // Write to temp file (SceneKit needs file URL)
        let tempDir = FileManager.default.temporaryDirectory
        let tempFile = tempDir.appendingPathComponent(geoDef.filename)

        do {
            try assetData.write(to: tempFile)

            // Also look for MTL file
            let mtlFilename = geoDef.filename.replacingOccurrences(of: ".obj", with: ".mtl")
            if let mtlData = GldfKit.getL3dAsset(l3dFile: l3dFile, filename: mtlFilename) {
                let mtlFile = tempDir.appendingPathComponent(mtlFilename)
                try mtlData.write(to: mtlFile)
            }

            // Load with SceneKit's OBJ loader
            let loadedScene = try SCNScene(url: tempFile, options: nil)

            // Clean up temp files
            try? FileManager.default.removeItem(at: tempFile)

            return loadedScene.rootNode.clone()
        } catch {
            print("Failed to load OBJ \(geoDef.filename): \(error)")
            return nil
        }
    }
}

// MARK: - L3D Part Badge
struct L3dPartBadge: View {
    let part: GldfKit.L3dScenePart

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(part.partName)
                .font(.caption)
                .fontWeight(.medium)
            Text(part.geometryId)
                .font(.caption2)
                .foregroundColor(.secondary)
            if !part.lightEmittingObjects.isEmpty {
                HStack(spacing: 2) {
                    Image(systemName: "lightbulb.fill")
                        .font(.system(size: 8))
                        .foregroundColor(.yellow)
                    Text("\(part.lightEmittingObjects.count) LEO")
                        .font(.caption2)
                        .foregroundColor(.yellow)
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 4)
        .background(Color.platformSecondaryBackground)
        .cornerRadius(6)
    }
}

// MARK: - Generic Content View
struct GenericContentView: View {
    let tab: OpenFileTab

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.questionmark")
                .font(.system(size: 48))
                .foregroundColor(.secondary)

            Text(tab.fileName)
                .font(.headline)

            Text("Content Type: \(tab.contentType)")
                .foregroundColor(.secondary)

            if let data = tab.data {
                Text("Size: \(ByteCountFormatter.string(fromByteCount: Int64(data.count), countStyle: .file))")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            if let textContent = tab.textContent {
                ScrollView {
                    Text(textContent)
                        .font(.system(size: 11, design: .monospaced))
                        .textSelection(.enabled)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding()
                }
                .background(Color.platformTertiaryBackground)
                .frame(maxHeight: 300)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

// MARK: - Error Content View
struct ErrorContentView: View {
    let fileName: String
    let error: String

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "exclamationmark.triangle")
                .font(.system(size: 48))
                .foregroundColor(.red)

            Text("Failed to load \(fileName)")
                .font(.headline)

            Text(error)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .padding()
    }
}

// MARK: - Empty Viewer
struct EmptyFileViewerView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "eye.slash")
                .font(.system(size: 48))
                .foregroundColor(.secondary)

            Text("No file selected")
                .font(.headline)
                .foregroundColor(.secondary)

            Text("Select a file from the Files list to view its contents")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
#endif

#Preview {
    FileViewerView()
        .environmentObject(AppState())
}
