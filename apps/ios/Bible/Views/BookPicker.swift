import SwiftUI

/// Books and chapters, and a field that takes a reference: "jn 3:16", "ps 23", "1 cor 13".
struct BookPicker: View {
    @Environment(Store.self) private var store
    @Environment(\.dismiss) private var dismiss
    let theme: Theme
    @State private var query = ""
    @State private var open: UInt8?

    var body: some View {
        NavigationStack {
            ScrollViewReader { proxy in
                List {
                    if query.trimmingCharacters(in: .whitespaces).isEmpty {
                        testament("Old Testament", store.bookList.filter { !$0.newTestament })
                        testament("New Testament", store.bookList.filter(\.newTestament))
                    } else {
                        let suggestions = suggestReferences(input: query, limit: 8)
                        if suggestions.isEmpty {
                            Text("No book matches “\(query)”")
                                .foregroundStyle(.secondary)
                        }
                        ForEach(suggestions, id: \.label) { reference in
                            Button {
                                go(reference)
                            } label: {
                                Label(reference.label, systemImage: reference.verse == nil ? "book" : "text.quote")
                            }
                        }
                    }
                }
                .listStyle(.insetGrouped)
                .onAppear {
                    let book = store.book(of: store.position.chapter).index
                    open = book
                    proxy.scrollTo(book, anchor: .center)
                }
            }
            .searchable(
                text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Go to… (jn 3:16)"
            )
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .onSubmit(of: .search) {
                if let reference = parseReference(input: query) ?? suggestReferences(input: query, limit: 1).first {
                    go(reference)
                }
            }
            .navigationTitle("Books")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }

    private func testament(_ title: String, _ books: [BookInfo]) -> some View {
        Section(title) {
            ForEach(books, id: \.index) { book in
                row(book)
                    .id(book.index)
            }
        }
    }

    @ViewBuilder
    private func row(_ book: BookInfo) -> some View {
        let current = store.book(of: store.position.chapter).index == book.index
        Button {
            if book.chapters == 1 {
                go(chapter: Int(book.firstChapter))
            } else {
                withAnimation(.smooth(duration: 0.25)) { open = open == book.index ? nil : book.index }
            }
        } label: {
            HStack {
                Text(book.name)
                    .font(.body.weight(current ? .semibold : .regular))
                    .foregroundStyle(current ? Color(theme.accent) : .primary)
                Spacer()
                Text(book.chapters == 1 ? "1 chapter" : "\(book.chapters)")
                    .font(.subheadline)
                    .foregroundStyle(.secondary)
                if book.chapters > 1 {
                    Image(systemName: "chevron.down")
                        .font(.footnote.weight(.semibold))
                        .foregroundStyle(.tertiary)
                        .rotationEffect(.degrees(open == book.index ? 0 : -90))
                }
            }
            .contentShape(Rectangle())
        }
        .accessibilityHint(book.chapters == 1 ? "Opens the book" : "Shows its chapters")
        if open == book.index && book.chapters > 1 {
            chapters(book)
        }
    }

    private func chapters(_ book: BookInfo) -> some View {
        let currentChapter = store.position.chapter
        return LazyVGrid(columns: [GridItem(.adaptive(minimum: 48), spacing: 8)], spacing: 8) {
            ForEach(1...Int(book.chapters), id: \.self) { n in
                let index = Int(book.firstChapter) + n - 1
                let current = index == currentChapter
                Button {
                    go(chapter: index)
                } label: {
                    Text("\(n)")
                        .font(.body.monospacedDigit().weight(current ? .semibold : .regular))
                        .frame(maxWidth: .infinity, minHeight: 44)
                        .foregroundStyle(current ? Color(theme.onAccent) : Color(theme.ink))
                        .background(
                            current ? Color(theme.accent) : Color(theme.fill),
                            in: RoundedRectangle(cornerRadius: 10, style: .continuous))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("\(book.name) \(n)")
            }
        }
        .padding(.vertical, 6)
    }

    private func go(chapter: Int) {
        store.go(to: chapter)
        dismiss()
    }

    private func go(_ reference: Reference) {
        store.go(to: reference)
        dismiss()
    }
}
