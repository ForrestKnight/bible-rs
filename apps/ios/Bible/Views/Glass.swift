import SwiftUI

extension View {
    /// The design's glass: Liquid Glass from iOS 26, the system material before it.
    @ViewBuilder
    func glass<S: Shape>(in shape: S, theme: Theme, interactive: Bool = true) -> some View {
        #if compiler(>=6.2)
            if #available(iOS 26, *) {
                self.glassEffect(interactive ? .regular.interactive() : .regular, in: shape)
            } else {
                materialGlass(in: shape, theme: theme)
            }
        #else
            materialGlass(in: shape, theme: theme)
        #endif
    }

    private func materialGlass<S: Shape>(in shape: S, theme: Theme) -> some View {
        background(.regularMaterial, in: shape)
            .overlay(shape.stroke(Color(theme.border), lineWidth: 0.5))
            .shadow(color: .black.opacity(theme.isDark ? 0.3 : 0.08), radius: 12, y: 4)
    }
}

/// Sheets keep the system's look, tinted with the theme's accent; in sepia they take its paper.
struct SheetChrome: ViewModifier {
    let theme: Theme

    func body(content: Content) -> some View {
        if theme.name == .sepia {
            content
                .tint(Color(theme.accent))
                .scrollContentBackground(.hidden)
                .background(Color(theme.paper))
                .presentationBackground(Color(theme.paper))
        } else {
            content.tint(Color(theme.accent))
        }
    }
}

extension View {
    func sheetChrome(_ theme: Theme) -> some View { modifier(SheetChrome(theme: theme)) }
}

/// Literata for verse text in lists, scaled with Dynamic Type.
extension Font {
    static func reading(_ size: CGFloat, italic: Bool = false, relativeTo style: UIFont.TextStyle = .body) -> Font {
        let scaled = UIFontMetrics(forTextStyle: style).scaledValue(for: size)
        return Font(Fonts.literata(scaled, italic: italic) as CTFont)
    }
}
