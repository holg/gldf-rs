// PlatformColors.swift
// Cross-platform color helpers

import SwiftUI

extension Color {
    /// A background color that adapts to the platform
    static var platformBackground: Color {
        #if os(macOS)
        return Color(nsColor: .windowBackgroundColor)
        #else
        return Color(uiColor: .systemBackground)
        #endif
    }

    /// A secondary/control background color that adapts to the platform
    static var platformSecondaryBackground: Color {
        #if os(macOS)
        return Color(nsColor: .controlBackgroundColor)
        #else
        return Color(uiColor: .secondarySystemBackground)
        #endif
    }

    /// A tertiary/text background color that adapts to the platform
    static var platformTertiaryBackground: Color {
        #if os(macOS)
        return Color(nsColor: .textBackgroundColor)
        #else
        return Color(uiColor: .tertiarySystemBackground)
        #endif
    }

    /// A selected content background color
    static var platformSelectedBackground: Color {
        #if os(macOS)
        return Color(nsColor: .selectedContentBackgroundColor)
        #else
        return Color.accentColor.opacity(0.2)
        #endif
    }
}
