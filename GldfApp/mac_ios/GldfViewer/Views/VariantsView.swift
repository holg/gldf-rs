// VariantsView.swift
// View for displaying product variants with emitter/geometry details and 3D preview

import SwiftUI
import GldfKit
import SceneKit

struct VariantsView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedId: String?
    @State private var searchText = ""
    @State private var showDetailSheet = false
    @State private var show3DPreview = false

    var filteredVariants: [GldfVariant] {
        if searchText.isEmpty {
            return appState.variants
        }
        return appState.variants.filter { variant in
            variant.id.localizedCaseInsensitiveContains(searchText) ||
            variant.name.localizedCaseInsensitiveContains(searchText) ||
            variant.description.localizedCaseInsensitiveContains(searchText)
        }
    }

    var selectedVariant: GldfVariant? {
        guard let id = selectedId else { return nil }
        return appState.variants.first { $0.id == id }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                TextField("Search variants...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)

                Spacer()

                if let variant = selectedVariant {
                    if variant.geometryId != nil || !variant.emitterRefs.isEmpty {
                        Button(action: { show3DPreview = true }) {
                            Label("3D Preview", systemImage: "cube")
                        }
                    }
                    Button(action: { showDetailSheet = true }) {
                        Label("Details", systemImage: "info.circle")
                    }
                }

                Text("\(filteredVariants.count) variant(s)")
                    .foregroundColor(.secondary)
            }
            .padding()

            Divider()

            // Variants Table
            if filteredVariants.isEmpty {
                VStack(spacing: 16) {
                    Image(systemName: "square.stack.3d.up.slash")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text(searchText.isEmpty ? "No variants defined" : "No matching variants")
                        .font(.headline)
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                Table(filteredVariants, selection: $selectedId) {
                    TableColumn("ID", value: \.id)
                        .width(min: 100, ideal: 150)

                    TableColumn("Name", value: \.name)
                        .width(min: 150, ideal: 200)

                    TableColumn("Description", value: \.description)
                        .width(min: 150, ideal: 200)

                    TableColumn("Geometry") { variant in
                        if let geomId = variant.geometryId {
                            HStack {
                                Image(systemName: "cube")
                                    .foregroundColor(.blue)
                                Text(geomId)
                            }
                        } else {
                            Text("-")
                                .foregroundColor(.secondary)
                        }
                    }
                    .width(min: 100, ideal: 150)

                    TableColumn("Emitters") { variant in
                        if variant.emitterRefs.isEmpty {
                            Text("-")
                                .foregroundColor(.secondary)
                        } else {
                            HStack {
                                Image(systemName: "lightbulb.fill")
                                    .foregroundColor(.orange)
                                Text("\(variant.emitterRefs.count)")
                            }
                        }
                    }
                    .width(min: 80, ideal: 100)
                }
            }
        }
        .navigationTitle("Variants")
        .sheet(isPresented: $showDetailSheet) {
            if let variant = selectedVariant {
                VariantDetailView(variant: variant)
                    .environmentObject(appState)
            }
        }
        .sheet(isPresented: $show3DPreview) {
            if let variant = selectedVariant {
                Variant3DPreviewSheet(variant: variant)
                    .environmentObject(appState)
            }
        }
    }
}

// MARK: - 3D Preview Sheet

struct Variant3DPreviewSheet: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    let variant: GldfVariant

    var body: some View {
        NavigationStack {
            Variant3DSceneView(variant: variant)
                .environmentObject(appState)
                .navigationTitle("3D Preview: \(variant.name.isEmpty ? variant.id : variant.name)")
                #if os(iOS)
                .navigationBarTitleDisplayMode(.inline)
                #endif
                .toolbar {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Done") { dismiss() }
                    }
                }
        }
        #if os(macOS)
        .frame(minWidth: 800, minHeight: 600)
        #endif
    }
}

// MARK: - 3D Scene View with IES Lights

#if os(macOS)
struct Variant3DSceneView: View {
    @EnvironmentObject var appState: AppState
    let variant: GldfVariant
    @State private var scene: SCNScene?
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var emitterInfos: [EmitterInfo] = []

    struct EmitterInfo: Identifiable {
        let id: String
        let name: String
        let lightSourceType: String
        let lumens: Int32?
        let hasPhotometry: Bool
    }

    var body: some View {
        HSplitView {
            // 3D Scene
            VStack(spacing: 0) {
                if isLoading {
                    ProgressView("Loading 3D scene...")
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let error = errorMessage {
                    VStack(spacing: 16) {
                        Image(systemName: "exclamationmark.triangle")
                            .font(.system(size: 48))
                            .foregroundColor(.orange)
                        Text(error)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if let scene = scene {
                    SceneView(
                        scene: scene,
                        options: [.allowsCameraControl, .autoenablesDefaultLighting]
                    )
                } else {
                    VStack(spacing: 16) {
                        Image(systemName: "cube.transparent")
                            .font(.system(size: 48))
                            .foregroundColor(.secondary)
                        Text("No geometry available")
                            .foregroundColor(.secondary)

                        if !emitterInfos.isEmpty {
                            Text("Emitters are defined but no 3D model is linked")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                }
            }
            .frame(minWidth: 400)

            // Info Panel
            VStack(alignment: .leading, spacing: 0) {
                Text("Variant Info")
                    .font(.headline)
                    .padding()

                Divider()

                List {
                    Section("Details") {
                        LabeledContent("ID", value: variant.id)
                        if !variant.name.isEmpty {
                            LabeledContent("Name", value: variant.name)
                        }
                        if let geomId = variant.geometryId {
                            LabeledContent("Geometry", value: geomId)
                        }
                    }

                    if !emitterInfos.isEmpty {
                        Section("Light Sources (\(emitterInfos.count))") {
                            ForEach(emitterInfos) { info in
                                VStack(alignment: .leading, spacing: 4) {
                                    HStack {
                                        Image(systemName: info.hasPhotometry ? "lightbulb.fill" : "lightbulb")
                                            .foregroundColor(info.hasPhotometry ? .orange : .secondary)
                                        Text(info.name)
                                            .font(.subheadline)
                                    }
                                    HStack(spacing: 12) {
                                        Text(info.lightSourceType.capitalized)
                                            .font(.caption)
                                            .foregroundColor(.secondary)
                                        if let lumens = info.lumens {
                                            Text("\(lumens) lm")
                                                .font(.caption)
                                                .foregroundColor(.secondary)
                                        }
                                        if info.hasPhotometry {
                                            Text("IES/LDT")
                                                .font(.caption)
                                                .padding(.horizontal, 4)
                                                .background(Color.orange.opacity(0.2))
                                                .cornerRadius(3)
                                        }
                                    }
                                }
                                .padding(.vertical, 2)
                            }
                        }
                    }
                }
            }
            .frame(width: 280)
        }
        .onAppear {
            buildScene()
        }
    }

    private func buildScene() {
        isLoading = true

        // Collect emitter info first
        var infos: [EmitterInfo] = []
        for ref in variant.emitterRefs {
            if let data = appState.engine?.getEmitterData(emitterId: ref.emitterId) {
                infos.append(EmitterInfo(
                    id: data.emitterId,
                    name: ref.externalName ?? ref.emitterId,
                    lightSourceType: data.lightSourceType,
                    lumens: data.ratedLuminousFlux,
                    hasPhotometry: data.photometryFileId != nil
                ))
            }
        }
        emitterInfos = infos

        // If no geometry, just show emitter info
        guard let geometryId = variant.geometryId else {
            isLoading = false
            return
        }

        // Try to load L3D/geometry from GLDF
        Task {
            await loadGeometryAndBuildScene(geometryId: geometryId)
        }
    }

    private func loadGeometryAndBuildScene(geometryId: String) async {
        guard let engine = appState.engine else {
            await MainActor.run {
                errorMessage = "No GLDF loaded"
                isLoading = false
            }
            return
        }

        do {
            // Use new FFI method that resolves geometry_id -> ModelGeometry -> File content
            let fileContent = try engine.getGeometryContent(geometryId: geometryId)

            // Parse L3D
            let l3dFile = try GldfKit.parseL3d(data: Data(fileContent.data))

            // Build the scene
            let newScene = await buildSceneKitScene(from: l3dFile, engine: engine)

            await MainActor.run {
                self.scene = newScene
                self.isLoading = false
            }
        } catch {
            await MainActor.run {
                errorMessage = "Failed to load geometry '\(geometryId)': \(error.localizedDescription)"
                isLoading = false
            }
        }
    }

    private func buildSceneKitScene(from l3dFile: L3dFile, engine: GldfEngine) async -> SCNScene {
        let newScene = SCNScene()

        // Background
        newScene.background.contents = NSColor(calibratedRed: 0.15, green: 0.15, blue: 0.2, alpha: 1.0)

        // Ambient light
        let ambientLight = SCNNode()
        ambientLight.light = SCNLight()
        ambientLight.light?.type = .ambient
        ambientLight.light?.color = NSColor.white
        ambientLight.light?.intensity = 300
        newScene.rootNode.addChildNode(ambientLight)

        // Cache for loaded geometries
        var geometryCache: [String: SCNNode] = [:]

        // Load parts
        for part in l3dFile.scene.parts {
            guard let geoDef = l3dFile.scene.geometryDefinitions.first(where: { $0.id == part.geometryId }) else {
                continue
            }

            // Load OBJ if not cached
            if geometryCache[geoDef.id] == nil {
                if let objNode = await loadObjFromL3d(l3dFile: l3dFile, geoDef: geoDef) {
                    geometryCache[geoDef.id] = objNode
                }
            }

            // Clone and apply transform
            if let templateNode = geometryCache[geoDef.id] {
                let partNode = templateNode.clone()
                partNode.name = part.partName

                // Apply world transform
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

            // Add light emitting objects with IES profiles
            for leo in part.lightEmittingObjects {
                let lightNode = await createLightNode(for: leo, engine: engine)
                newScene.rootNode.addChildNode(lightNode)
            }
        }

        return newScene
    }

    private func loadObjFromL3d(l3dFile: L3dFile, geoDef: L3dGeometryDef) async -> SCNNode? {
        guard let assetData = GldfKit.getL3dAsset(l3dFile: l3dFile, filename: geoDef.filename) else {
            return nil
        }

        let tempDir = FileManager.default.temporaryDirectory
        let tempFile = tempDir.appendingPathComponent(geoDef.filename)

        do {
            try assetData.write(to: tempFile)

            // Also try MTL file
            let mtlFilename = geoDef.filename.replacingOccurrences(of: ".obj", with: ".mtl")
            if let mtlData = GldfKit.getL3dAsset(l3dFile: l3dFile, filename: mtlFilename) {
                let mtlFile = tempDir.appendingPathComponent(mtlFilename)
                try mtlData.write(to: mtlFile)
            }

            let loadedScene = try SCNScene(url: tempFile, options: nil)
            try? FileManager.default.removeItem(at: tempFile)

            return loadedScene.rootNode.clone()
        } catch {
            print("Failed to load OBJ: \(error)")
            return nil
        }
    }

    private func createLightNode(for leo: L3dLightEmittingObject, engine: GldfEngine) async -> SCNNode {
        let lightNode = SCNNode()
        lightNode.name = "Light: \(leo.partName)"
        lightNode.position = SCNVector3(Float(leo.position.x), Float(leo.position.y), Float(leo.position.z))

        // Create SCNLight
        let light = SCNLight()
        light.type = .spot
        light.color = NSColor.white
        light.intensity = 1000

        // Try to load IES profile for this emitter
        for ref in variant.emitterRefs {
            if let data = appState.engine?.getEmitterData(emitterId: ref.emitterId),
               let photometryFileId = data.photometryFileId {

                // Get photometry file content
                if let iesURL = await loadPhotometryAsIES(fileId: photometryFileId, engine: engine) {
                    light.iesProfileURL = iesURL
                    light.type = .IES

                    // Scale intensity based on lumens
                    if let lumens = data.ratedLuminousFlux {
                        light.intensity = CGFloat(lumens)
                    }
                }
                break
            }
        }

        lightNode.light = light

        // Add visual indicator (small yellow sphere)
        let indicatorSize: CGFloat = leo.shapeDimensions.first.map { CGFloat($0) } ?? 0.02
        let indicator = SCNNode()
        indicator.geometry = SCNSphere(radius: indicatorSize / 2)
        indicator.geometry?.firstMaterial?.diffuse.contents = NSColor.yellow
        indicator.geometry?.firstMaterial?.emission.contents = NSColor.yellow
        lightNode.addChildNode(indicator)

        return lightNode
    }

    private func loadPhotometryAsIES(fileId: String, engine: GldfEngine) async -> URL? {
        do {
            let content = try engine.getFileContentAsString(fileId: fileId)

            // Determine if IES or LDT
            let isIES = content.trimmingCharacters(in: .whitespacesAndNewlines).hasPrefix("IESNA")

            let tempDir = FileManager.default.temporaryDirectory
            let tempFile: URL

            if isIES {
                // Write IES directly
                tempFile = tempDir.appendingPathComponent("\(fileId).ies")
                try content.write(to: tempFile, atomically: true, encoding: .utf8)
            } else {
                // LDT file - SceneKit doesn't support LDT directly
                // For now, create a basic IES approximation or skip
                // TODO: Implement LDT to IES conversion
                return nil
            }

            return tempFile
        } catch {
            print("Failed to load photometry \(fileId): \(error)")
            return nil
        }
    }
}
#else
// iOS simplified version
struct Variant3DSceneView: View {
    @EnvironmentObject var appState: AppState
    let variant: GldfVariant

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "cube")
                .font(.system(size: 60))
                .foregroundColor(.secondary)

            Text("3D Preview")
                .font(.title2)

            Text("Full 3D preview with IES lighting is available on macOS")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            if let geomId = variant.geometryId {
                Label("Geometry: \(geomId)", systemImage: "cube")
                    .font(.caption)
            }

            if !variant.emitterRefs.isEmpty {
                Label("\(variant.emitterRefs.count) light source(s)", systemImage: "lightbulb.fill")
                    .font(.caption)
            }
        }
        .padding()
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
#endif

// MARK: - Variant Detail View

struct VariantDetailView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    let variant: GldfVariant
    @State private var show3DPreview = false

    var body: some View {
        NavigationStack {
            List {
                // Basic Info Section
                Section("Variant Info") {
                    LabeledContent("ID", value: variant.id)
                    LabeledContent("Name", value: variant.name.isEmpty ? "-" : variant.name)
                    LabeledContent("Description", value: variant.description.isEmpty ? "-" : variant.description)
                    if let geomId = variant.geometryId {
                        HStack {
                            LabeledContent("Geometry ID", value: geomId)
                            Spacer()
                            Button(action: { show3DPreview = true }) {
                                Image(systemName: "cube")
                            }
                            .buttonStyle(.bordered)
                        }
                    }
                }

                // Emitters Section
                if !variant.emitterRefs.isEmpty {
                    Section("Emitters (\(variant.emitterRefs.count))") {
                        ForEach(variant.emitterRefs) { emitterRef in
                            EmitterRefRow(emitterRef: emitterRef)
                        }
                    }
                }

                // Quick Actions
                if variant.geometryId != nil || !variant.emitterRefs.isEmpty {
                    Section("Actions") {
                        Button(action: { show3DPreview = true }) {
                            Label("Open 3D Preview", systemImage: "cube")
                        }
                    }
                }
            }
            .navigationTitle(variant.name.isEmpty ? variant.id : variant.name)
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .sheet(isPresented: $show3DPreview) {
                Variant3DPreviewSheet(variant: variant)
                    .environmentObject(appState)
            }
        }
        #if os(macOS)
        .frame(minWidth: 500, minHeight: 400)
        #endif
    }
}

// MARK: - Emitter Reference Row

struct EmitterRefRow: View {
    @EnvironmentObject var appState: AppState
    let emitterRef: GldfEmitterRef
    @State private var emitterData: GldfEmitterData?
    @State private var showPhotometry = false
    @State private var parsedPhotometry: EulumdatData?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Image(systemName: "lightbulb.fill")
                    .foregroundColor(.orange)
                VStack(alignment: .leading) {
                    Text(emitterRef.emitterId)
                        .font(.headline)
                    if let extName = emitterRef.externalName {
                        Text(extName)
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                Spacer()

                if emitterData?.photometryFileId != nil {
                    Button(action: { showPhotometry = true }) {
                        Label("Diagram", systemImage: "sun.max")
                    }
                    .buttonStyle(.bordered)
                }
            }

            if let data = emitterData {
                HStack(spacing: 16) {
                    Label(data.lightSourceType.capitalized, systemImage: "bolt.fill")
                        .font(.caption)

                    if let flux = data.ratedLuminousFlux {
                        Label("\(flux) lm", systemImage: "rays")
                            .font(.caption)
                    }

                    if let fileId = data.photometryFileId {
                        Label(fileId, systemImage: "doc")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }

            // Mini polar diagram preview
            if let parsed = parsedPhotometry, !parsed.intensities.isEmpty {
                MiniPolarDiagram(data: parsed)
                    .frame(height: 80)
                    .padding(.top, 4)
            }
        }
        .padding(.vertical, 4)
        .onAppear {
            loadEmitterData()
        }
        .sheet(isPresented: $showPhotometry) {
            if let fileId = emitterData?.photometryFileId {
                PhotometryPreviewView(fileId: fileId, emitterData: emitterData)
                    .environmentObject(appState)
            }
        }
    }

    private func loadEmitterData() {
        emitterData = appState.engine?.getEmitterData(emitterId: emitterRef.emitterId)

        // Load and parse photometry for mini diagram
        if let fileId = emitterData?.photometryFileId {
            Task {
                do {
                    let content = try appState.engine?.getFileContentAsString(fileId: fileId)
                    if let content = content {
                        await MainActor.run {
                            parsedPhotometry = GldfKit.parseEulumdat(content: content)
                        }
                    }
                } catch {
                    // Ignore errors for mini preview
                }
            }
        }
    }
}

// MARK: - Mini Polar Diagram

struct MiniPolarDiagram: View {
    let data: EulumdatData

    var body: some View {
        GeometryReader { geometry in
            let size = min(geometry.size.width, geometry.size.height)
            let center = CGPoint(x: geometry.size.width / 2, y: geometry.size.height / 2)
            let radius = size / 2 - 10

            Canvas { context, _ in
                // Background circle
                let circleRect = CGRect(x: center.x - radius, y: center.y - radius, width: radius * 2, height: radius * 2)
                context.stroke(Circle().path(in: circleRect), with: .color(.gray.opacity(0.3)), lineWidth: 0.5)

                // Draw first plane intensity curve
                if !data.intensities.isEmpty && !data.gammaAngles.isEmpty {
                    let gammaCount = Int(data.gammaCount)
                    let firstPlane = Array(data.intensities.prefix(gammaCount))
                    drawMiniCurve(context: context, center: center, radius: radius,
                                 intensities: firstPlane, maxIntensity: data.maxIntensity,
                                 color: .orange, gammaAngles: data.gammaAngles)
                }
            }
        }
    }

    private func drawMiniCurve(context: GraphicsContext, center: CGPoint, radius: CGFloat,
                               intensities: [Double], maxIntensity: Double, color: Color,
                               gammaAngles: [Double]) {
        guard !intensities.isEmpty, maxIntensity > 0 else { return }

        var path = Path()

        for (index, intensity) in intensities.enumerated() {
            let gamma = index < gammaAngles.count ? gammaAngles[index] : Double(index) * 180.0 / Double(max(1, intensities.count - 1))
            let radians = gamma * .pi / 180
            let r = radius * CGFloat(min(1.0, intensity / maxIntensity))

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

        // Mirror
        for (index, intensity) in intensities.reversed().enumerated() {
            let originalIndex = intensities.count - 1 - index
            let gamma = originalIndex < gammaAngles.count ? gammaAngles[originalIndex] : Double(originalIndex) * 180.0 / Double(max(1, intensities.count - 1))
            let radians = (360 - gamma) * .pi / 180
            let r = radius * CGFloat(min(1.0, intensity / maxIntensity))

            let point = CGPoint(
                x: center.x + r * Foundation.sin(radians),
                y: center.y - r * Foundation.cos(radians)
            )
            path.addLine(to: point)
        }

        path.closeSubpath()
        context.fill(path, with: .color(color.opacity(0.2)))
        context.stroke(path, with: .color(color), lineWidth: 1)
    }
}

// MARK: - Photometry Preview View

struct PhotometryPreviewView: View {
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    let fileId: String
    let emitterData: GldfEmitterData?

    @State private var photometryContent: String?
    @State private var parsedData: EulumdatData?
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var showRawData = false

    var body: some View {
        NavigationStack {
            Group {
                if isLoading {
                    ProgressView("Loading photometry...")
                } else if let error = errorMessage {
                    VStack(spacing: 16) {
                        Image(systemName: "exclamationmark.triangle")
                            .font(.system(size: 48))
                            .foregroundColor(.orange)
                        Text(error)
                            .foregroundColor(.secondary)
                    }
                } else if let parsed = parsedData {
                    VStack(spacing: 0) {
                        // Header info
                        HStack {
                            if let data = emitterData {
                                Label(data.lightSourceType.capitalized, systemImage: "bolt.fill")
                                if let flux = data.ratedLuminousFlux {
                                    Label("\(flux) lm", systemImage: "rays")
                                }
                            }
                            Spacer()
                            Text("\(Int(parsed.maxIntensity)) cd max")
                                .foregroundColor(.secondary)
                        }
                        .padding()

                        Divider()

                        // Diagram
                        PhotometryDiagramView(data: parsed)
                            .frame(minHeight: 300)

                        Divider()

                        // Info bar
                        HStack(spacing: 20) {
                            VariantInfoItem(label: "Manufacturer", value: parsed.manufacturer)
                            VariantInfoItem(label: "Luminaire", value: parsed.luminaireName)
                            VariantInfoItem(label: "Lumens", value: String(format: "%.0f lm", parsed.totalLumens))
                            VariantInfoItem(label: "C Planes", value: "\(parsed.cPlaneCount)")
                            VariantInfoItem(label: "Gamma", value: "\(parsed.gammaCount) angles")
                        }
                        .padding()
                        .background(Color.secondary.opacity(0.1))

                        // Raw data toggle
                        if showRawData, let content = photometryContent {
                            Divider()
                            ScrollView {
                                Text(content)
                                    .font(.system(.caption, design: .monospaced))
                                    .padding()
                                    .frame(maxWidth: .infinity, alignment: .leading)
                            }
                            .frame(maxHeight: 200)
                        }
                    }
                } else {
                    Text("No photometry data available")
                        .foregroundColor(.secondary)
                }
            }
            .navigationTitle("Photometry: \(fileId)")
            #if os(iOS)
            .navigationBarTitleDisplayMode(.inline)
            #endif
            .toolbar {
                ToolbarItem(placement: .automatic) {
                    Toggle(isOn: $showRawData) {
                        Image(systemName: "doc.text")
                    }
                    .help("Show raw data")
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        #if os(macOS)
        .frame(minWidth: 700, minHeight: 550)
        #endif
        .onAppear {
            loadPhotometryContent()
        }
    }

    private func loadPhotometryContent() {
        Task {
            do {
                let content = try appState.engine?.getFileContentAsString(fileId: fileId)
                await MainActor.run {
                    self.photometryContent = content
                    if let content = content {
                        self.parsedData = GldfKit.parseEulumdat(content: content)
                    }
                    self.isLoading = false
                }
            } catch {
                await MainActor.run {
                    self.errorMessage = error.localizedDescription
                    self.isLoading = false
                }
            }
        }
    }
}

// MARK: - Photometry Diagram View

struct PhotometryDiagramView: View {
    let data: EulumdatData
    @State private var diagramType: DiagramType = .polar

    enum DiagramType: String, CaseIterable {
        case polar = "Polar"
        case cartesian = "Cartesian"
    }

    var body: some View {
        VStack(spacing: 0) {
            // Diagram type picker
            Picker("Diagram", selection: $diagramType) {
                ForEach(DiagramType.allCases, id: \.self) { type in
                    Text(type.rawValue).tag(type)
                }
            }
            .pickerStyle(.segmented)
            .frame(width: 200)
            .padding()

            // Diagram
            Group {
                switch diagramType {
                case .polar:
                    PolarDiagramCanvas(data: data)
                case .cartesian:
                    CartesianDiagramCanvas(data: data)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
    }
}

// MARK: - Polar Diagram Canvas

struct PolarDiagramCanvas: View {
    let data: EulumdatData

    var body: some View {
        GeometryReader { geometry in
            let size = min(geometry.size.width, geometry.size.height)
            let center = CGPoint(x: geometry.size.width / 2, y: geometry.size.height / 2)
            let radius = size / 2 - 50

            Canvas { context, _ in
                // Background circles
                for fraction in [0.25, 0.5, 0.75, 1.0] {
                    let circleRadius = radius * fraction
                    let rect = CGRect(x: center.x - circleRadius, y: center.y - circleRadius, width: circleRadius * 2, height: circleRadius * 2)
                    context.stroke(Circle().path(in: rect), with: .color(.gray.opacity(0.3)), lineWidth: 1)
                }

                // Angle lines
                for i in 0..<12 {
                    let angle = Double(i) * 30 * .pi / 180
                    var path = Path()
                    path.move(to: center)
                    path.addLine(to: CGPoint(x: center.x + radius * sin(angle), y: center.y - radius * cos(angle)))
                    context.stroke(path, with: .color(.gray.opacity(0.3)), lineWidth: 1)
                }

                // Draw intensity curves
                if !data.intensities.isEmpty && !data.gammaAngles.isEmpty {
                    let gammaCount = Int(data.gammaCount)
                    let firstPlane = Array(data.intensities.prefix(gammaCount))
                    drawIntensityCurve(context: context, center: center, radius: radius,
                                      intensities: firstPlane, maxIntensity: data.maxIntensity,
                                      color: .orange, gammaAngles: data.gammaAngles)

                    // C90 plane if available
                    let numCPlanes = data.cAngles.count > 0 ? data.cAngles.count : Int(data.cPlaneCount)
                    if numCPlanes > 1 {
                        let c90Index = numCPlanes / 4
                        if c90Index > 0 && c90Index < numCPlanes {
                            let start = c90Index * gammaCount
                            let end = min(start + gammaCount, data.intensities.count)
                            if start < end {
                                let c90Plane = Array(data.intensities[start..<end])
                                drawIntensityCurve(context: context, center: center, radius: radius,
                                                  intensities: c90Plane, maxIntensity: data.maxIntensity,
                                                  color: .blue, gammaAngles: data.gammaAngles)
                            }
                        }
                    }
                }

                // Angle labels
                for angle in [0, 30, 60, 90, 120, 150, 180] {
                    let radians = Double(angle) * .pi / 180
                    let labelPoint = CGPoint(x: center.x + (radius + 25) * sin(radians), y: center.y - (radius + 25) * cos(radians))
                    context.draw(Text("\(angle)°").font(.caption2).foregroundColor(.secondary), at: labelPoint)
                }
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
            }
            .padding(8)
            .background(Color.secondary.opacity(0.1))
            .cornerRadius(6)
            .position(x: 70, y: 50)
        }
    }

    private func drawIntensityCurve(context: GraphicsContext, center: CGPoint, radius: CGFloat,
                                    intensities: [Double], maxIntensity: Double, color: Color,
                                    gammaAngles: [Double]) {
        guard !intensities.isEmpty, maxIntensity > 0 else { return }

        var path = Path()

        for (index, intensity) in intensities.enumerated() {
            let gamma = index < gammaAngles.count ? gammaAngles[index] : Double(index) * 180.0 / Double(max(1, intensities.count - 1))
            let radians = gamma * .pi / 180
            let r = radius * CGFloat(min(1.0, intensity / maxIntensity))
            let point = CGPoint(x: center.x + r * Foundation.sin(radians), y: center.y - r * Foundation.cos(radians))

            if index == 0 {
                path.move(to: point)
            } else {
                path.addLine(to: point)
            }
        }

        // Mirror
        for (index, intensity) in intensities.reversed().enumerated() {
            let originalIndex = intensities.count - 1 - index
            let gamma = originalIndex < gammaAngles.count ? gammaAngles[originalIndex] : Double(originalIndex) * 180.0 / Double(max(1, intensities.count - 1))
            let radians = (360 - gamma) * .pi / 180
            let r = radius * CGFloat(min(1.0, intensity / maxIntensity))
            let point = CGPoint(x: center.x + r * Foundation.sin(radians), y: center.y - r * Foundation.cos(radians))
            path.addLine(to: point)
        }

        path.closeSubpath()
        context.fill(path, with: .color(color.opacity(0.15)))
        context.stroke(path, with: .color(color), lineWidth: 2)
    }
}

// MARK: - Cartesian Diagram Canvas

struct CartesianDiagramCanvas: View {
    let data: EulumdatData

    var body: some View {
        GeometryReader { geometry in
            let margin: CGFloat = 60
            let width = geometry.size.width - margin * 2
            let height = geometry.size.height - margin * 2
            let origin = CGPoint(x: margin, y: geometry.size.height - margin)

            Canvas { context, _ in
                // Axes
                var axisPath = Path()
                axisPath.move(to: CGPoint(x: margin, y: margin))
                axisPath.addLine(to: origin)
                axisPath.addLine(to: CGPoint(x: geometry.size.width - margin, y: origin.y))
                context.stroke(axisPath, with: .color(.gray), lineWidth: 1)

                // Grid
                for i in 0...4 {
                    let y = origin.y - height * CGFloat(i) / 4
                    var gridPath = Path()
                    gridPath.move(to: CGPoint(x: margin, y: y))
                    gridPath.addLine(to: CGPoint(x: geometry.size.width - margin, y: y))
                    context.stroke(gridPath, with: .color(.gray.opacity(0.3)), lineWidth: 1)
                    context.draw(Text("\(i * 25)%").font(.caption2).foregroundColor(.secondary), at: CGPoint(x: margin - 25, y: y))
                }

                // Draw curves
                if !data.intensities.isEmpty && !data.gammaAngles.isEmpty {
                    let maxAngle = data.gammaAngles.last ?? 90
                    let gammaCount = Int(data.gammaCount)
                    let firstPlane = Array(data.intensities.prefix(gammaCount))

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
            }

            // Legend
            VStack(alignment: .leading, spacing: 4) {
                HStack(spacing: 4) {
                    Rectangle().fill(Color.orange).frame(width: 12, height: 3)
                    Text("C0-C180").font(.caption2)
                }
            }
            .padding(8)
            .background(Color.secondary.opacity(0.1))
            .cornerRadius(6)
            .position(x: geometry.size.width - 80, y: 50)
        }
    }
}

// MARK: - Variant Info Item (local variant to avoid conflict with FileViewerView)

struct VariantInfoItem: View {
    let label: String
    let value: String

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Text(value.isEmpty ? "-" : value)
                .font(.caption)
                .lineLimit(1)
        }
    }
}

#Preview {
    VariantsView()
        .environmentObject(AppState())
}
