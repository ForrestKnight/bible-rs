import SwiftUI
import UIKit

/// The theme the reader asked for; `system` follows the phone between light and dark.
enum Appearance: String, Codable, CaseIterable, Identifiable {
    case system, light, sepia, dark

    var id: Self { self }

    var label: String {
        switch self {
        case .system: "Automatic"
        case .light: "Light"
        case .sepia: "Sepia"
        case .dark: "Dark"
        }
    }

    func resolved(_ scheme: ColorScheme) -> ThemeName {
        switch self {
        case .system: scheme == .dark ? .dark : .light
        case .light: .light
        case .sepia: .sepia
        case .dark: .dark
        }
    }
}

extension UIColor {
    /// `0xRRGGBBAA`, as the core hands colours over.
    convenience init(rgba: UInt32) {
        self.init(
            red: CGFloat(rgba >> 24 & 0xFF) / 255,
            green: CGFloat(rgba >> 16 & 0xFF) / 255,
            blue: CGFloat(rgba >> 8 & 0xFF) / 255,
            alpha: CGFloat(rgba & 0xFF) / 255)
    }
}

extension HighlightColor: CaseIterable, Identifiable {
    public static let allCases: [HighlightColor] = [.yellow, .green, .blue, .pink, .purple, .orange]

    public var id: Int { order }

    var order: Int { Self.allCases.firstIndex(of: self) ?? 0 }

    var label: String {
        switch self {
        case .yellow: "Yellow"
        case .green: "Green"
        case .blue: "Blue"
        case .pink: "Pink"
        case .purple: "Purple"
        case .orange: "Orange"
        }
    }
}

/// A theme's colours: the design tokens the Mac app is drawn with, read through the core.
struct Theme: Equatable {
    let name: ThemeName
    let paper, surface, glass, fill, fillStrong, separator, border: UIColor
    let ink, ink2, ink3, verse, rubric: UIColor
    let accent, onAccent, accentSoft, selection, danger: UIColor
    private let highlights: [UIColor]
    private let swatches: [UIColor]

    static let light = Theme(.light)
    static let sepia = Theme(.sepia)
    static let dark = Theme(.dark)

    static func named(_ name: ThemeName) -> Theme {
        switch name {
        case .light: light
        case .sepia: sepia
        case .dark: dark
        }
    }

    private init(_ name: ThemeName) {
        let p = palette(theme: name)
        self.name = name
        paper = UIColor(rgba: p.paper)
        surface = UIColor(rgba: p.surface)
        glass = UIColor(rgba: p.glass)
        fill = UIColor(rgba: p.fill)
        fillStrong = UIColor(rgba: p.fillStrong)
        separator = UIColor(rgba: p.separator)
        border = UIColor(rgba: p.border)
        ink = UIColor(rgba: p.ink)
        ink2 = UIColor(rgba: p.ink2)
        ink3 = UIColor(rgba: p.ink3)
        verse = UIColor(rgba: p.verse)
        rubric = UIColor(rgba: p.rubric)
        accent = UIColor(rgba: p.accent)
        onAccent = UIColor(rgba: p.onAccent)
        accentSoft = UIColor(rgba: p.accentSoft)
        selection = UIColor(rgba: p.selection)
        danger = UIColor(rgba: p.danger)
        highlights = p.highlights.map(UIColor.init(rgba:))
        swatches = p.swatches.map(UIColor.init(rgba:))
    }

    static func == (a: Theme, b: Theme) -> Bool { a.name == b.name }

    var isDark: Bool { name == .dark }
    var colorScheme: ColorScheme { isDark ? .dark : .light }

    /// The fill under highlighted text.
    func highlight(_ color: HighlightColor) -> UIColor { highlights[color.order] }
    /// The colour as a swatch in pickers and lists.
    func swatch(_ color: HighlightColor) -> UIColor { swatches[color.order] }
}

extension Color {
    init(_ color: UIColor) { self.init(uiColor: color) }
}

/// Literata for reading and Inter for verse numbers and labels, the Mac app's fonts. Weight and
/// optical size are pinned as the Mac pins them, rather than left to Core Text's automatic
/// optical sizing, so a page reads the same on both.
enum Fonts {
    private struct Key: Hashable {
        let name: String
        let size: CGFloat
        let weight: CGFloat
        let opsz: CGFloat
        let smallCaps: Bool
    }

    /// Chapters are typeset off the main thread too.
    private static let lock = NSLock()
    nonisolated(unsafe) private static var cache: [Key: UIFont] = [:]

    /// Body text sits at Literata's default optical size, as on the Mac.
    static func literata(
        _ size: CGFloat, weight: CGFloat = 400, opsz: CGFloat = 12, italic: Bool = false, smallCaps: Bool = false
    ) -> UIFont {
        font(Key(
            name: italic ? "Literata-Italic" : "Literata-Regular", size: size, weight: weight, opsz: opsz,
            smallCaps: smallCaps))
    }

    static func inter(_ size: CGFloat, weight: CGFloat = 600) -> UIFont {
        font(Key(name: "Inter-Regular", size: size, weight: weight, opsz: 14, smallCaps: false))
    }

    private static func font(_ key: Key) -> UIFont {
        lock.lock()
        defer { lock.unlock() }
        if let font = cache[key] { return font }
        let axes: [NSNumber: CGFloat] = [
            NSNumber(value: fourCC("wght")): key.weight,
            NSNumber(value: fourCC("opsz")): key.opsz,
        ]
        var attributes: [UIFontDescriptor.AttributeName: Any] = [
            .name: key.name,
            UIFontDescriptor.AttributeName(rawValue: kCTFontVariationAttribute as String): axes,
        ]
        if key.smallCaps {
            // `c2sc`: capitals to small capitals; the text arrives upper-cased.
            attributes[.featureSettings] = [[
                UIFontDescriptor.FeatureKey.type: kUpperCaseType,
                UIFontDescriptor.FeatureKey.selector: kUpperCaseSmallCapsSelector,
            ]]
        }
        let font = UIFont(descriptor: UIFontDescriptor(fontAttributes: attributes), size: key.size)
        cache[key] = font
        return font
    }

    private static func fourCC(_ tag: String) -> UInt32 {
        tag.utf8.reduce(0) { $0 << 8 | UInt32($1) }
    }
}
