import SwiftUI
import UIKit

/// The UIKit reader in the SwiftUI app.
struct ReaderContainer: UIViewRepresentable {
    let store: Store
    let theme: Theme
    let style: Typography
    let translation: String
    let readingLine: CGFloat
    let selection: VerseSelection?
    let marksVersion: Int
    let jump: JumpRequest?

    func makeCoordinator() -> Coordinator { Coordinator(store: store) }

    func makeUIView(context: Context) -> ReaderView {
        let view = ReaderView()
        view.readerDelegate = context.coordinator
        return view
    }

    func updateUIView(_ view: ReaderView, context: Context) {
        let c = context.coordinator
        let bible = store.bible
        let translation = translation
        let options = style.options
        view.source = ChapterSource(
            content: { bible.chapter(translation: translation, index: UInt16($0), options: options) },
            marks: { bible.marks(translation: translation, chapter: UInt16($0)) })
        view.readingLine = readingLine
        view.configure(style: style, theme: theme, key: translation)
        view.selection = selection
        if c.marksVersion != marksVersion {
            c.marksVersion = marksVersion
            view.reloadMarks()
        }
        if let jump, jump.id != c.jump {
            c.jump = jump.id
            view.jump(to: jump.chapter, verse: jump.verse, animated: jump.animated, flash: jump.flash)
        }
    }

    @MainActor
    final class Coordinator: ReaderViewDelegate {
        let store: Store
        var marksVersion = 0
        var jump: UUID?

        init(store: Store) { self.store = store }

        func reader(_ reader: ReaderView, didReach chapter: Int, verse: Int) {
            store.reached(chapter, verse: verse)
        }

        func reader(_ reader: ReaderView, didTapVerse verse: Int, inChapter chapter: Int) {
            store.tap(verse: verse, chapter: chapter)
        }

        func reader(_ reader: ReaderView, didTapNoteOf verse: Int, inChapter chapter: Int) {
            store.selection = nil
            store.sheet = .note(store.ref(chapter: chapter, verse: verse))
        }

        func readerDidTapPage(_ reader: ReaderView) {
            store.tapPage()
        }

        func reader(_ reader: ReaderView, isReadingOn: Bool) {
            guard store.selection == nil, store.barsHidden != isReadingOn else { return }
            store.barsHidden = isReadingOn
        }
    }
}
