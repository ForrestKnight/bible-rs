import SwiftUI

/// Bookmarks, highlights and notes, newest first.
struct StudyView: View {
    @Environment(Store.self) private var store
    @Environment(\.dismiss) private var dismiss
    let theme: Theme
    @State private var kind = StudyKind.bookmarks
    @State private var items: [StudyItem] = []

    var body: some View {
        NavigationStack {
            List {
                ForEach(items, id: \.rowID) { item in
                    Button {
                        store.go(to: Int(item.chapterIndex), verse: Int(item.verse.verse), flash: true)
                        dismiss()
                    } label: {
                        StudyRow(item: item, kind: kind, theme: theme)
                    }
                    .swipeActions {
                        Button(role: .destructive) {
                            remove(item)
                        } label: {
                            Label("Remove", systemImage: "trash")
                        }
                    }
                    .contextMenu {
                        if kind == .notes {
                            Button {
                                store.sheet = .note(item.verse)
                            } label: {
                                Label("Edit Note", systemImage: "square.and.pencil")
                            }
                        }
                        Button(role: .destructive) {
                            remove(item)
                        } label: {
                            Label("Remove", systemImage: "trash")
                        }
                    }
                }
            }
            .listStyle(.plain)
            .overlay {
                if items.isEmpty { empty }
            }
            .safeAreaInset(edge: .top) {
                Picker("Show", selection: $kind) {
                    Text("Bookmarks").tag(StudyKind.bookmarks)
                    Text("Highlights").tag(StudyKind.highlights)
                    Text("Notes").tag(StudyKind.notes)
                }
                .pickerStyle(.segmented)
                .padding(.horizontal, 16)
                .padding(.bottom, 8)
            }
            .navigationTitle("Study")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .onAppear(perform: load)
            .onChange(of: kind) { load() }
            .onChange(of: store.marksVersion) { load() }
        }
    }

    private var empty: some View {
        let (title, symbol, hint): (String, String, String) =
            switch kind {
            case .bookmarks: ("No bookmarks", "bookmark", "Tap a verse, then Bookmark.")
            case .highlights: ("No highlights", "highlighter", "Tap a verse, then pick a colour.")
            case .notes: ("No notes", "note.text", "Tap a verse, then Note.")
            }
        return ContentUnavailableView(title, systemImage: symbol, description: Text(hint))
    }

    private func load() {
        items = store.items(kind)
    }

    private func remove(_ item: StudyItem) {
        switch kind {
        case .bookmarks: store.setBookmark(item.verse, false)
        case .highlights: store.removeHighlight(item.verse, item.start)
        case .notes: store.setNote(item.verse, "")
        }
    }
}

private extension StudyItem {
    /// A verse can hold several highlights (parts of it, made on the Mac or the web): each is a
    /// row of its own, told apart by where it starts.
    var rowID: String { "\(verse.book).\(verse.chapter).\(verse.verse).\(start)" }
}

private struct StudyRow: View {
    let item: StudyItem
    let kind: StudyKind
    let theme: Theme

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            if let color = item.highlight, kind == .highlights {
                RoundedRectangle(cornerRadius: 2)
                    .fill(Color(theme.swatch(color)))
                    .frame(width: 4)
            }
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(item.reference)
                        .font(.subheadline.weight(.semibold))
                        .foregroundStyle(Color(theme.accent))
                    Spacer()
                    Text(Date(timeIntervalSince1970: TimeInterval(item.updated)), format: .relative(presentation: .named))
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
                if kind == .notes && !item.body.isEmpty {
                    Text(item.body)
                        .font(.body)
                        .foregroundStyle(Color(theme.ink))
                        .lineLimit(4)
                }
                Text(item.text)
                    .font(.reading(15))
                    .foregroundStyle(kind == .notes ? Color(theme.ink2) : Color(theme.ink))
                    .lineLimit(kind == .notes ? 2 : 3)
            }
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }
}
