import SwiftUI

/// What you can do with the verses you've tapped: highlight, note, bookmark, copy, share.
struct SelectionBar: View {
    @Environment(Store.self) private var store
    let theme: Theme

    var body: some View {
        let marks = store.selectionMarks
        let highlighted = Set(marks.compactMap(\.highlight))
        let bookmarked = !store.selectedRefs.isEmpty && marks.filter(\.bookmark).count == store.selectedRefs.count
        VStack(spacing: 12) {
            HStack {
                Text(store.selectionLabel)
                    .font(.subheadline.weight(.semibold))
                    .foregroundStyle(Color(theme.ink))
                    .lineLimit(1)
                Spacer()
                Button {
                    store.selection = nil
                } label: {
                    Image(systemName: "xmark")
                        .font(.system(size: 13, weight: .bold))
                        .foregroundStyle(Color(theme.ink2))
                        .frame(width: 30, height: 30)
                        .background(Color(theme.fill), in: Circle())
                }
                .accessibilityLabel("Clear selection")
            }

            HStack(spacing: 0) {
                ForEach(HighlightColor.allCases) { color in
                    Button {
                        store.highlightSelection(color)
                    } label: {
                        Circle()
                            .fill(Color(theme.swatch(color)))
                            .frame(width: 30, height: 30)
                            .overlay {
                                if highlighted.contains(color) {
                                    Image(systemName: "checkmark")
                                        .font(.system(size: 13, weight: .bold))
                                        .foregroundStyle(.white)
                                }
                            }
                            .frame(maxWidth: .infinity, minHeight: 44)
                            .contentShape(Rectangle())
                    }
                    .accessibilityLabel("Highlight \(color.label)")
                }
                Button {
                    store.highlightSelection(nil)
                } label: {
                    Image(systemName: "circle.slash")
                        .font(.system(size: 26, weight: .light))
                        .foregroundStyle(Color(theme.ink3))
                        .frame(maxWidth: .infinity, minHeight: 44)
                        .contentShape(Rectangle())
                }
                .disabled(highlighted.isEmpty)
                .opacity(highlighted.isEmpty ? 0.4 : 1)
                .accessibilityLabel("Remove highlight")
            }

            HStack(spacing: 8) {
                action("Note", "note.text.badge.plus") { store.noteOnSelection() }
                action(bookmarked ? "Saved" : "Bookmark", bookmarked ? "bookmark.fill" : "bookmark") {
                    store.toggleBookmarkOnSelection()
                }
                action("Copy", "doc.on.doc") { store.copySelection() }
                ShareLink(item: store.selectionText, subject: Text(store.selectionLabel)) {
                    label("Share", "square.and.arrow.up")
                }
            }
        }
        .padding(16)
        .frame(maxWidth: 520)
        .glass(in: RoundedRectangle(cornerRadius: 28, style: .continuous), theme: theme, interactive: false)
    }

    private func action(_ title: String, _ symbol: String, _ run: @escaping () -> Void) -> some View {
        Button(action: run) { label(title, symbol) }
    }

    private func label(_ title: String, _ symbol: String) -> some View {
        VStack(spacing: 4) {
            Image(systemName: symbol)
                .font(.system(size: 18, weight: .medium))
                .frame(height: 22)
            Text(title)
                .font(.caption.weight(.medium))
        }
        .foregroundStyle(Color(theme.ink))
        .frame(maxWidth: .infinity, minHeight: 52)
        .background(Color(theme.fill), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
        .contentShape(Rectangle())
    }
}
