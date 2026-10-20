import UIKit

extension NSAttributedString.Key {
    /// The verse (an `Int`) a character belongs to, numbers included.
    static let verse = NSAttributedString.Key("BibleVerse")
    /// The font whose face a paragraph's lines are centred on (see `Baselines`).
    static let lineFont = NSAttributedString.Key("BibleLineFont")
}

/// How the reader sets type. Sizes are the Mac reader's, given in design points at 20 pt and
/// scaled to the reading size.
struct Typography: Equatable {
    var size: CGFloat
    var lineSpacing: CGFloat
    var redLetters: Bool
    var options: ContentOptions
    var theme: ThemeName
    /// The reader's width and the safe area beside it (landscape).
    var width: CGFloat
    var safeLeft: CGFloat = 0
    var safeRight: CGFloat = 0

    var scale: CGFloat { size / 20 }
    var lineHeight: CGFloat { (size * lineSpacing).rounded() }

    func s(_ pt: CGFloat) -> CGFloat { pt * scale }

    /// The text column: the design's 640 pt measure at 20 pt, centred, inside phone margins.
    var column: (x: CGFloat, width: CGFloat) {
        let margin: CGFloat = width >= 600 ? 48 : 22
        let left = max(margin, safeLeft + 12)
        let right = max(margin, safeRight + 12)
        let available = max(width - left - right, 120)
        let measure = min(available, s(640).rounded())
        return ((left + (available - measure) / 2).rounded(), measure)
    }

    /// Verse numbers in poetry and verse-per-line text sit in a lane this wide at the column's
    /// left (the Mac hangs them in its wider margin).
    var gutter: CGFloat { s(28).rounded() }
}

/// One chapter set in type: TextKit 1 layout of the core's blocks, plus the geometry the reader
/// needs to draw highlights, hit-test taps and track the reading position.
final class ChapterLayout {
    let index: Int
    let content: ChapterContent
    let typography: Typography
    let height: CGFloat
    /// Each verse's top, in order.
    let verseTops: [(verse: Int, y: CGFloat)]
    /// Highlights, notes and bookmarks by verse.
    var marks: [Int: VerseMarks] = [:]

    private let storage: NSTextStorage
    private let manager: NSLayoutManager
    private let container: NSTextContainer
    private let origin: CGPoint
    private let words: [Int: [NSRange]]
    private let spoken: [Int: String]
    private let headings: [(range: NSRange, text: String)]
    private let bodyFont: UIFont

    var book: Int { Int(content.book) }
    var chapter: Int { Int(content.chapter) }
    var isPsalm: Bool { content.book == 18 }
    var verseCount: Int { verseTops.last?.verse ?? 0 }

    init(content: ChapterContent, typography t: Typography) {
        index = Int(content.index)
        self.content = content
        typography = t
        bodyFont = Fonts.literata(t.size)
        var set = Typesetter(t)
        set.chapter(content)
        let storage = NSTextStorage(attributedString: set.out)
        words = set.words
        spoken = set.spoken
        headings = set.headings

        let column = t.column
        let top = content.book == 18 ? 0 : t.s(16)
        let origin = CGPoint(x: column.x, y: top)
        let container = NSTextContainer(size: CGSize(width: column.width, height: .greatestFiniteMagnitude))
        container.lineFragmentPadding = 0
        let manager = NSLayoutManager()
        manager.delegate = Baselines.shared
        manager.usesFontLeading = false
        manager.addTextContainer(container)
        storage.addLayoutManager(manager)
        manager.ensureLayout(for: container)
        let used = manager.usedRect(for: container)
        height = (top + used.maxY + t.s(80) + 32).rounded()
        self.storage = storage
        self.manager = manager
        self.container = container
        self.origin = origin

        // Verse tops, from the line holding each verse's first character.
        var tops: [(Int, CGFloat)] = []
        var seen = Set<Int>()
        storage.enumerateAttribute(.verse, in: NSRange(location: 0, length: storage.length)) { value, range, _ in
            guard let v = value as? Int, !seen.contains(v) else { return }
            seen.insert(v)
            let glyph = manager.glyphIndexForCharacter(at: range.location)
            let line = manager.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
            let band = Self.band(line: line, glyph: glyph, manager: manager)
            tops.append((v, origin.y + band.minY))
        }
        verseTops = tops.sorted { $0.1 < $1.1 }
    }

    // MARK: Geometry (chapter coordinates: x across the reader, y down from the chapter's top)

    /// The verse under a point, if the point is on text.
    func verse(at point: CGPoint) -> Int? {
        let p = CGPoint(x: point.x - origin.x, y: point.y - origin.y)
        guard storage.length > 0, p.y >= 0, p.y <= manager.usedRect(for: container).maxY else { return nil }
        let glyph = manager.glyphIndex(for: p, in: container)
        let line = manager.lineFragmentRect(forGlyphAt: glyph, effectiveRange: nil)
        let band = Self.band(line: line, glyph: glyph, manager: manager)
        guard band.insetBy(dx: 0, dy: -4).contains(CGPoint(x: line.midX, y: p.y)) else { return nil }
        let used = manager.lineFragmentUsedRect(forGlyphAt: glyph, effectiveRange: nil)
        guard p.x >= used.minX - 22, p.x <= used.maxX + 22 else { return nil }
        let char = manager.characterIndexForGlyph(at: glyph)
        return storage.attribute(.verse, at: min(char, storage.length - 1), effectiveRange: nil) as? Int
    }

    /// The verse at the reading line `y`: the last verse starting at or above it.
    func verse(atY y: CGFloat) -> Int {
        var found = verseTops.first?.verse ?? 1
        for top in verseTops {
            if top.y > y + 1 { break }
            found = top.verse
        }
        return found
    }

    func top(of verse: Int) -> CGFloat? {
        verseTops.first { $0.verse == verse }?.y ?? verseTops.last { $0.verse < verse }?.y
    }

    /// One full-line band per line a verse's words run across.
    func bands(of verse: Int) -> [CGRect] {
        guard let ranges = words[verse] else { return [] }
        var rects: [CGRect] = []
        for range in ranges {
            let glyphs = manager.glyphRange(forCharacterRange: range, actualCharacterRange: nil)
            manager.enumerateLineFragments(forGlyphRange: glyphs) { line, _, container, lineGlyphs, _ in
                let part = NSIntersectionRange(lineGlyphs, glyphs)
                guard part.length > 0 else { return }
                let trimmed = self.trimmingSpaces(part)
                guard trimmed.length > 0 else { return }
                let x = self.manager.boundingRect(forGlyphRange: trimmed, in: container)
                let band = Self.band(line: line, glyph: lineGlyphs.location, manager: self.manager)
                let pad = self.typography.s(2)
                rects.append(CGRect(x: x.minX - pad, y: band.minY, width: x.width + pad * 2, height: band.height)
                    .offsetBy(dx: self.origin.x, dy: self.origin.y))
            }
        }
        return merged(rects)
    }

    /// The text of a verse for VoiceOver, and the chapter's headings.
    func spokenText(of verse: Int) -> String? { spoken[verse] }

    var headingElements: [(text: String, frame: CGRect)] {
        headings.map { heading in
            let glyphs = manager.glyphRange(forCharacterRange: heading.range, actualCharacterRange: nil)
            let rect = manager.boundingRect(forGlyphRange: glyphs, in: container)
            return (heading.text, rect.offsetBy(dx: origin.x, dy: origin.y))
        }
    }

    /// Where a verse's note and bookmark markers sit: in the right margin by its first line.
    func markerFrame(of verse: Int) -> CGRect? {
        guard let first = bands(of: verse).first else { return nil }
        let column = typography.column
        let size = min(typography.s(14).rounded(), 16)
        return CGRect(
            x: column.x + column.width + 4, y: first.midY - size / 2, width: size, height: size)
    }

    // MARK: Drawing

    struct Decorations {
        var theme: Theme
        var selected: Set<Int> = []
    }

    /// Draws the part of the chapter inside `rect` (chapter coordinates) into the current
    /// context, translated so the chapter's top-left is at the origin.
    func draw(_ rect: CGRect, decorations d: Decorations) {
        guard let ctx = UIGraphicsGetCurrentContext() else { return }
        let radius = typography.s(4)
        for (verse, mark) in marks {
            guard let color = mark.highlight else { continue }
            fill(bands(of: verse), in: rect, color: d.theme.highlight(color), radius: radius, ctx: ctx)
        }
        for verse in d.selected {
            let bands = bands(of: verse)
            fill(bands, in: rect, color: d.theme.selection, radius: radius, ctx: ctx)
            underline(bands, in: rect, color: d.theme.accent, ctx: ctx)
        }
        let local = rect.offsetBy(dx: -origin.x, dy: -origin.y)
        let glyphs = manager.glyphRange(forBoundingRect: local, in: container)
        if glyphs.length > 0 {
            manager.drawGlyphs(forGlyphRange: glyphs, at: origin)
        }
        for (verse, mark) in marks where mark.note || mark.bookmark {
            guard let frame = markerFrame(of: verse), frame.insetBy(dx: 0, dy: -frame.height).intersects(rect)
            else { continue }
            let symbol = mark.note ? "note.text" : "bookmark.fill"
            let config = UIImage.SymbolConfiguration(pointSize: frame.height * 0.8, weight: .medium)
            let color = mark.note ? d.theme.verse : d.theme.accent
            guard let image = UIImage(systemName: symbol, withConfiguration: config)?
                .withTintColor(color, renderingMode: .alwaysOriginal)
            else { continue }
            let size = image.size
            image.draw(at: CGPoint(x: frame.midX - size.width / 2, y: frame.midY - size.height / 2))
            if mark.note && mark.bookmark, let ribbon = UIImage(systemName: "bookmark.fill", withConfiguration: config)?
                .withTintColor(d.theme.accent, renderingMode: .alwaysOriginal)
            {
                ribbon.draw(at: CGPoint(x: frame.midX - ribbon.size.width / 2, y: frame.maxY + 2))
            }
        }
    }

    /// A dotted rule under each line of a selected verse, so a selection never reads as a highlight.
    private func underline(_ bands: [CGRect], in clip: CGRect, color: UIColor, ctx: CGContext) {
        let pad = typography.s(2)
        let lh = typography.lineHeight
        let face = bodyFont.ascender - bodyFont.descender
        let below = (lh - face) / 2 + bodyFont.ascender + typography.s(4)
        let dot = max(1.5, typography.s(1.6)).rounded(.up)
        ctx.saveGState()
        ctx.setStrokeColor(color.cgColor)
        ctx.setLineWidth(dot)
        ctx.setLineCap(.round)
        ctx.setLineDash(phase: 0, lengths: [0, dot * 2.5])
        for band in bands where band.intersects(clip) {
            let y = (band.minY + below).rounded() + 0.5
            ctx.move(to: CGPoint(x: band.minX + pad, y: y))
            ctx.addLine(to: CGPoint(x: band.maxX - pad, y: y))
        }
        ctx.strokePath()
        ctx.restoreGState()
    }

    private func fill(_ rects: [CGRect], in clip: CGRect, color: UIColor, radius: CGFloat, ctx: CGContext) {
        let visible = rects.filter { $0.intersects(clip) }
        guard !visible.isEmpty else { return }
        ctx.setFillColor(color.cgColor)
        for r in visible {
            ctx.addPath(UIBezierPath(roundedRect: r, cornerRadius: radius).cgPath)
        }
        ctx.fillPath()
    }

    // MARK: Helpers

    /// The band of the line holding `glyph`: its line box without the paragraph spacing above.
    private static func band(line: CGRect, glyph: Int, manager: NSLayoutManager) -> CGRect {
        var lh = line.height
        if let storage = manager.textStorage, storage.length > 0 {
            let char = min(manager.characterIndexForGlyph(at: glyph), storage.length - 1)
            if let style = storage.attribute(.paragraphStyle, at: char, effectiveRange: nil) as? NSParagraphStyle,
                style.maximumLineHeight > 0
            {
                lh = min(style.maximumLineHeight, line.height)
            }
        }
        return CGRect(x: line.minX, y: line.maxY - lh, width: line.width, height: lh)
    }

    private func trimmingSpaces(_ glyphs: NSRange) -> NSRange {
        let chars = manager.characterRange(forGlyphRange: glyphs, actualGlyphRange: nil)
        let text = storage.string as NSString
        var start = chars.location
        var end = NSMaxRange(chars)
        let space = CharacterSet.whitespacesAndNewlines
        while start < end, let u = UnicodeScalar(text.character(at: start)), space.contains(u) { start += 1 }
        while end > start, let u = UnicodeScalar(text.character(at: end - 1)), space.contains(u) { end -= 1 }
        return manager.glyphRange(
            forCharacterRange: NSRange(location: start, length: end - start), actualCharacterRange: nil)
    }

    /// Joins rects on the same line that touch, so a highlight reads as one band.
    private func merged(_ rects: [CGRect]) -> [CGRect] {
        var out: [CGRect] = []
        for r in rects {
            if let last = out.last, abs(last.minY - r.minY) < 0.5, r.minX - last.maxX < typography.s(8) {
                out[out.count - 1] = last.union(r)
            } else {
                out.append(r)
            }
        }
        return out
    }
}

/// Centres each line's face in its line box, as CSS does (the design is CSS), instead of TextKit's
/// sitting it at the bottom. Paragraph spacing stays above the box.
final class Baselines: NSObject, NSLayoutManagerDelegate {
    static let shared = Baselines()

    func layoutManager(
        _ manager: NSLayoutManager, shouldSetLineFragmentRect rect: UnsafeMutablePointer<CGRect>,
        lineFragmentUsedRect used: UnsafeMutablePointer<CGRect>, baselineOffset: UnsafeMutablePointer<CGFloat>,
        in container: NSTextContainer, forGlyphRange range: NSRange
    ) -> Bool {
        guard let storage = manager.textStorage, storage.length > 0 else { return false }
        let char = min(manager.characterIndexForGlyph(at: range.location), storage.length - 1)
        guard let font = storage.attribute(.lineFont, at: char, effectiveRange: nil) as? UIFont,
            let style = storage.attribute(.paragraphStyle, at: char, effectiveRange: nil) as? NSParagraphStyle,
            style.maximumLineHeight > 0
        else { return false }
        let lh = style.maximumLineHeight
        let above = max(rect.pointee.height - lh, 0)
        let face = font.ascender - font.descender
        baselineOffset.pointee = (above + (lh - face) / 2 + font.ascender).rounded()
        return true
    }
}

/// Turns the core's blocks into an attributed string.
private struct Typesetter {
    let t: Typography
    let theme: Theme
    let out = NSMutableAttributedString()
    var words: [Int: [NSRange]] = [:]
    var spoken: [Int: String] = [:]
    var headings: [(range: NSRange, text: String)] = []
    private var style = NSParagraphStyle()
    private var lineFont: UIFont

    init(_ t: Typography) {
        self.t = t
        theme = Theme.named(t.theme)
        lineFont = Fonts.literata(t.size)
    }

    mutating func chapter(_ c: ChapterContent) {
        let s = t.s
        var firstSpace: CGFloat = 0
        if c.book == 18 {
            label("PSALM \(c.chapter)", before: 0, align: .natural)
        } else if c.chapter == 1 {
            display(c.bookName, before: 0)
            if !c.bookTitle.isEmpty {
                line(
                    c.bookTitle, font: Fonts.literata(s(17).rounded(), italic: true), lh: s(24), before: s(4),
                    color: theme.ink2, align: .center)
            }
            label("CHAPTER 1", before: s(16), align: .center)
            firstSpace = s(24)
        } else {
            label(c.bookName.uppercased(), before: 0, align: .center)
            display("\(c.chapter)", before: s(4))
            firstSpace = s(24)
        }

        guard c.status == .ready else {
            let message =
                switch c.status {
                case .missing: "This chapter isn’t in this translation."
                case .failed(let message): message
                default: "This chapter isn’t available."
                }
            line(
                message, font: Fonts.literata(s(17).rounded(), italic: true), lh: s(24), before: firstSpace + s(16),
                color: theme.ink3, align: .center)
            return
        }

        for (i, block) in c.blocks.enumerated() {
            self.block(block, before: CGFloat(block.spaceBefore) * t.scale + (i == 0 ? firstSpace : 0))
        }
    }

    // MARK: Blocks

    private mutating func block(_ b: Block, before: CGFloat) {
        let s = t.s
        let lh = t.lineHeight
        let body = Fonts.literata(t.size)
        let poetryIndent = s(32)
        switch b.kind {
        case .heading, .psalmHeading, .psalmTitle, .speaker:
            heading(b, before: before)
            return
        default:
            break
        }

        let p = NSMutableParagraphStyle()
        p.minimumLineHeight = lh
        p.maximumLineHeight = lh
        p.paragraphSpacingBefore = before
        p.lineBreakMode = .byWordWrapping
        var textStart: CGFloat = 0
        switch b.kind {
        case .prose(let indent):
            if b.gutter != nil || t.options.versePerLine {
                textStart = t.gutter
                p.headIndent = textStart
                p.firstLineHeadIndent = textStart
            } else {
                p.firstLineHeadIndent = indent ? s(32) : 0
            }
        case .poetry(let level):
            textStart = t.gutter + poetryIndent * CGFloat(max(Int(level), 1) - 1)
            p.firstLineHeadIndent = textStart
            p.headIndent = textStart + poetryIndent
        case .right:
            p.alignment = .right
        case .center:
            p.alignment = .center
        default:
            break
        }
        if b.gutter != nil {
            p.firstLineHeadIndent = 0
            p.tabStops = [
                NSTextTab(textAlignment: .right, location: t.gutter - s(8)),
                NSTextTab(textAlignment: .left, location: textStart),
            ]
        }
        paragraph(p, font: body)

        if let number = b.gutter {
            let verse = Int(b.gutterVerse)
            add("\t\(number)\t", numberAttributes(size: s(12).rounded(), raise: 0), verse: verse)
        }
        for run in b.runs {
            let verse = Int(run.verse)
            if run.number {
                add(run.text, numberAttributes(size: s(11).rounded(), raise: s(6)), verse: verse)
            } else {
                add(run.text, textAttributes(run, size: t.size), verse: verse, words: true)
            }
        }
    }

    private mutating func heading(_ b: Block, before: CGFloat) {
        let s = t.s
        let (font, lh, color, italic): (UIFont, CGFloat, UIColor, Bool) =
            switch b.kind {
            case .psalmHeading: (Fonts.literata(s(28).rounded(), opsz: 28), s(36), theme.ink, false)
            case .psalmTitle, .speaker:
                (Fonts.literata(s(17).rounded(), italic: true), s(24), theme.ink2, true)
            default: (Fonts.literata(s(17).rounded(), weight: 600, opsz: 16), s(24), theme.ink, false)
            }
        let p = NSMutableParagraphStyle()
        p.minimumLineHeight = lh.rounded()
        p.maximumLineHeight = lh.rounded()
        p.paragraphSpacingBefore = before
        paragraph(p, font: font)
        let start = out.length
        var text = ""
        for run in b.runs {
            let runFont =
                run.smallCaps
                ? Fonts.literata(font.pointSize, weight: run.bold ? 600 : 400, italic: italic || run.italic, smallCaps: true)
                : font
            add(run.text, [.font: runFont, .foregroundColor: color], verse: Int(run.verse), words: run.verse > 0)
            text += run.smallCaps ? run.text.capitalized : run.text
        }
        headings.append((NSRange(location: start, length: out.length - start), text))
    }

    // MARK: Head lines

    private mutating func label(_ text: String, before: CGFloat, align: NSTextAlignment) {
        let size = t.s(11).rounded()
        line(
            text, font: Fonts.inter(size), lh: t.s(16), before: before, color: theme.accent, align: align,
            kern: size * 0.12)
    }

    private mutating func display(_ text: String, before: CGFloat) {
        let size = t.s(48).rounded()
        line(
            text, font: Fonts.literata(size, weight: 300, opsz: 48), lh: t.s(56), before: before, color: theme.ink,
            align: .center, kern: size * -0.015)
    }

    private mutating func line(
        _ text: String, font: UIFont, lh: CGFloat, before: CGFloat, color: UIColor, align: NSTextAlignment,
        kern: CGFloat = 0
    ) {
        let p = NSMutableParagraphStyle()
        p.minimumLineHeight = lh.rounded()
        p.maximumLineHeight = lh.rounded()
        p.paragraphSpacingBefore = before
        p.alignment = align
        paragraph(p, font: font)
        let start = out.length
        add(text, [.font: font, .foregroundColor: color, .kern: kern])
        headings.append((NSRange(location: start, length: out.length - start), text))
    }

    // MARK: Runs

    private func textAttributes(_ run: Run, size: CGFloat) -> [NSAttributedString.Key: Any] {
        let font = Fonts.literata(
            size, weight: run.bold ? 600 : 400, italic: run.italic, smallCaps: run.smallCaps)
        let color = run.red && t.redLetters ? theme.rubric : theme.ink
        var a: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: color]
        if run.smallCaps { a[.kern] = size * 0.04 }
        return a
    }

    private func numberAttributes(size: CGFloat, raise: CGFloat) -> [NSAttributedString.Key: Any] {
        [.font: Fonts.inter(size), .foregroundColor: theme.verse, .baselineOffset: raise]
    }

    private mutating func paragraph(_ style: NSParagraphStyle, font: UIFont) {
        if out.length > 0 {
            out.append(NSAttributedString(
                string: "\n", attributes: [.paragraphStyle: self.style, .font: lineFont, .lineFont: lineFont]))
        }
        self.style = style
        lineFont = font
    }

    private mutating func add(
        _ text: String, _ attributes: [NSAttributedString.Key: Any], verse: Int = 0, words isWords: Bool = false
    ) {
        guard !text.isEmpty else { return }
        var a = attributes
        a[.paragraphStyle] = style
        a[.lineFont] = lineFont
        if verse > 0 { a[.verse] = verse }
        let start = out.length
        out.append(NSAttributedString(string: text, attributes: a))
        guard isWords, verse > 0 else { return }
        let range = NSRange(location: start, length: out.length - start)
        if let last = words[verse]?.last, NSMaxRange(last) == start {
            words[verse]![words[verse]!.count - 1] = NSUnionRange(last, range)
        } else {
            words[verse, default: []].append(range)
        }
        spoken[verse, default: ""] += text
    }
}
