// ElectricalEditView.swift
// View for editing electrical attributes

import SwiftUI
import GldfKit

struct ElectricalEditView: View {
    @EnvironmentObject var appState: AppState
    @State private var electrical: GldfElectrical = GldfElectrical(
        safetyClass: nil,
        ipCode: nil,
        powerFactor: nil,
        constantLightOutput: nil,
        lightDistribution: nil,
        switchingCapacity: nil
    )

    // Local editing state
    @State private var safetyClass: String = ""
    @State private var ipCode: String = ""
    @State private var powerFactorString: String = ""
    @State private var constantLightOutput: Bool = false
    @State private var lightDistribution: String = ""
    @State private var switchingCapacity: String = ""

    let safetyClasses = ["", "I", "II", "III"]
    let lightDistributions = ["", "Direct", "Indirect", "DirectIndirect", "Symmetric", "Asymmetric", "Narrow", "Medium", "Wide"]

    var body: some View {
        Form {
            Section("Electrical Safety") {
                Picker("Safety Class", selection: $safetyClass) {
                    Text("-- Select --").tag("")
                    Text("Class I").tag("I")
                    Text("Class II").tag("II")
                    Text("Class III").tag("III")
                }
                .onChange(of: safetyClass) { newValue in
                    appState.engine?.setElectricalSafetyClass(value: newValue.isEmpty ? nil : newValue)
                    appState.markModified()
                }

                TextField("IP Code", text: $ipCode)
                    .textFieldStyle(.roundedBorder)
                    .onSubmit {
                        appState.engine?.setIpCode(value: ipCode.isEmpty ? nil : ipCode)
                        appState.markModified()
                    }
                    .onChange(of: ipCode) { newValue in
                        if !newValue.isEmpty || electrical.ipCode != nil {
                            appState.engine?.setIpCode(value: newValue.isEmpty ? nil : newValue)
                            appState.markModified()
                        }
                    }

                Text("Protection against solids and liquids (IEC 60529)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Section("Performance") {
                HStack {
                    TextField("Power Factor", text: $powerFactorString)
                        .textFieldStyle(.roundedBorder)
                        #if os(iOS)
                        .keyboardType(.decimalPad)
                        #endif

                    Text("(0.0 - 1.0)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .onChange(of: powerFactorString) { newValue in
                    if let value = Double(newValue), value >= 0 && value <= 1 {
                        appState.engine?.setPowerFactor(value: value)
                        appState.markModified()
                    } else if newValue.isEmpty {
                        appState.engine?.setPowerFactor(value: nil)
                        appState.markModified()
                    }
                }

                Toggle("Constant Light Output (CLO)", isOn: $constantLightOutput)
                    .onChange(of: constantLightOutput) { newValue in
                        appState.engine?.setConstantLightOutput(value: newValue)
                        appState.markModified()
                    }

                Text("Luminaire maintains constant lumen output over lifetime")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Section("Light Distribution") {
                Picker("Distribution Type", selection: $lightDistribution) {
                    Text("-- Select --").tag("")
                    Text("Direct").tag("Direct")
                    Text("Indirect").tag("Indirect")
                    Text("Direct/Indirect").tag("DirectIndirect")
                    Text("Symmetric").tag("Symmetric")
                    Text("Asymmetric").tag("Asymmetric")
                    Text("Narrow").tag("Narrow")
                    Text("Medium").tag("Medium")
                    Text("Wide").tag("Wide")
                }
                .onChange(of: lightDistribution) { newValue in
                    appState.engine?.setLightDistribution(value: newValue.isEmpty ? nil : newValue)
                    appState.markModified()
                }
            }

            Section("Durability") {
                TextField("Switching Capacity", text: $switchingCapacity)
                    .textFieldStyle(.roundedBorder)
                    .onChange(of: switchingCapacity) { newValue in
                        appState.engine?.setSwitchingCapacity(value: newValue.isEmpty ? nil : newValue)
                        appState.markModified()
                    }

                Text("e.g., 50000 cycles")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .formStyle(.grouped)
        .navigationTitle("Electrical Attributes")
        .onAppear {
            loadElectrical()
        }
    }

    private func loadElectrical() {
        guard let engine = appState.engine else { return }
        electrical = engine.getElectrical()

        safetyClass = electrical.safetyClass ?? ""
        ipCode = electrical.ipCode ?? ""
        if let pf = electrical.powerFactor {
            powerFactorString = String(format: "%.2f", pf)
        } else {
            powerFactorString = ""
        }
        constantLightOutput = electrical.constantLightOutput ?? false
        lightDistribution = electrical.lightDistribution ?? ""
        switchingCapacity = electrical.switchingCapacity ?? ""
    }
}

#Preview {
    ElectricalEditView()
        .environmentObject(AppState())
}
