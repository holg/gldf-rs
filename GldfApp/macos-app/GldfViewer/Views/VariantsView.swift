// VariantsView.swift
// View for displaying product variants

import SwiftUI
import GldfKit

struct VariantsView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedId: String?
    @State private var searchText = ""

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

    var body: some View {
        VStack(spacing: 0) {
            // Toolbar
            HStack {
                TextField("Search variants...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)

                Spacer()

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
                        .width(min: 200, ideal: 300)
                }
            }
        }
        .navigationTitle("Variants")
    }
}

#Preview {
    VariantsView()
        .environmentObject(AppState())
}
