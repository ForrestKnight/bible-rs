import SwiftUI

/// Search as you type, in the current translation.
struct SearchView: View {
    @Environment(Store.self) private var store
    @Environment(\.dismiss) private var dismiss
    let theme: Theme

    @State private var query = ""
    @State private var scope = Scope.all
    @State private var focused = true
    @State private var hits: [SearchHit] = []
    @State private var total: UInt32 = 0
    @State private var elapsed: Double = 0
    @State private var suggestion: SearchSuggestion?
    @State private var searched = ""
    @State private var task: Task<Void, Never>?
    @State private var paging = false
    @State private var requests = 0
    /// The search whose results are listed. Later pages repeat it exactly and are dropped if
    /// another search's results have replaced it by the time they arrive.
    @State private var listed: Request?

    private struct Request {
        let id: Int
        let text: String
        let translation: String
        let scope: SearchScope
        let wholeWords: Bool
    }

    enum Scope: Hashable { case all, old, new, book }

    private static let pageSize: UInt32 = 50

    var body: some View {
        NavigationStack {
            List {
                if !hits.isEmpty {
                    Section {
                        ForEach(hits, id: \.verse) { hit in
                            Button {
                                store.go(to: Int(hit.chapterIndex), verse: Int(hit.verse.verse), flash: true)
                                dismiss()
                            } label: {
                                HitRow(hit: hit, theme: theme)
                            }
                            .onAppear {
                                if hit.verse == hits.last?.verse { more() }
                            }
                        }
                    } header: {
                        Text(summary)
                            .textCase(nil)
                    }
                }
            }
            .listStyle(.plain)
            .overlay { emptyState }
            .searchable(
                text: $query, isPresented: $focused, placement: .navigationBarDrawer(displayMode: .always),
                prompt: "Search the \(store.translation?.abbrev ?? "Bible")"
            )
            .searchScopes($scope, activation: .onSearchPresentation) {
                Text("All").tag(Scope.all)
                Text("Old").tag(Scope.old)
                Text("New").tag(Scope.new)
                Text(store.book(of: store.position.chapter).name).tag(Scope.book)
            }
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .onChange(of: query) { search() }
            // The search field's close button closes search altogether.
            .onChange(of: focused) {
                if !focused { dismiss() }
            }
            .onChange(of: scope) { search(debounce: false) }
            .navigationTitle("Search")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .confirmationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }

    @ViewBuilder
    private var emptyState: some View {
        let trimmed = query.trimmingCharacters(in: .whitespaces)
        if trimmed.isEmpty {
            ContentUnavailableView {
                Label("Search the \(store.translation?.abbrev ?? "Bible")", systemImage: "magnifyingglass")
            } description: {
                Text("Every word must appear. Use “quotes” for a phrase and -word to leave a word out.")
            }
        } else if hits.isEmpty && searched == trimmed {
            ContentUnavailableView {
                Label("No verses found", systemImage: "text.magnifyingglass")
            } description: {
                if let suggestion {
                    Text("Did you mean “\(suggestion.query)”?")
                } else {
                    Text("Check the spelling or try fewer words.")
                }
            } actions: {
                if let suggestion {
                    Button("Search “\(suggestion.query)” (\(suggestion.total))") { query = suggestion.query }
                        .buttonStyle(.borderedProminent)
                }
            }
        }
    }

    private var summary: String {
        let verses = total == 1 ? "1 verse" : "\(total.formatted()) verses"
        return "\(verses) · \(elapsed < 1 ? "<1" : elapsed.formatted(.number.precision(.fractionLength(0)))) ms"
    }

    private var coreScope: SearchScope {
        switch scope {
        case .all: .all
        case .old: .oldTestament
        case .new: .newTestament
        case .book: .book(book: store.book(of: store.position.chapter).index)
        }
    }

    private func search(debounce: Bool = true) {
        task?.cancel()
        let text = query.trimmingCharacters(in: .whitespaces)
        guard !text.isEmpty else {
            hits = []
            total = 0
            suggestion = nil
            searched = ""
            listed = nil
            paging = false
            return
        }
        requests += 1
        let request = Request(
            id: requests, text: text, translation: store.settings.translation, scope: coreScope,
            wholeWords: store.settings.wholeWords)
        task = Task {
            if debounce { try? await Task.sleep(for: .milliseconds(80)) }
            guard !Task.isCancelled else { return }
            let page = await fetch(request, from: 0)
            guard !Task.isCancelled else { return }
            hits = page?.hits ?? []
            total = page?.total ?? 0
            elapsed = page?.elapsedMs ?? 0
            suggestion = page?.suggestion
            searched = text
            listed = request
            paging = false
        }
    }

    private func more() {
        guard let request = listed, hits.count < total, !paging else { return }
        paging = true
        let offset = UInt32(hits.count)
        Task {
            let page = await fetch(request, from: offset)
            guard listed?.id == request.id else { return }
            paging = false
            guard let page, hits.count == offset else { return }
            hits += page.hits
        }
    }

    private func fetch(_ request: Request, from offset: UInt32) async -> SearchPage? {
        let bible = store.bible
        let limit = Self.pageSize
        return await Task.detached(priority: .userInitiated) {
            try? bible.search(
                translation: request.translation, query: request.text, scope: request.scope,
                wholeWords: request.wholeWords, offset: offset, limit: limit)
        }.value
    }
}

private struct HitRow: View {
    let hit: SearchHit
    let theme: Theme

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(hit.reference)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(Color(theme.accent))
            Text(marked)
                .font(.reading(16))
                .foregroundStyle(Color(theme.ink))
                .lineSpacing(3)
        }
        .padding(.vertical, 4)
        .accessibilityElement(children: .combine)
    }

    /// The verse with its matches marked, as the core found them (UTF-16 ranges).
    private var marked: AttributedString {
        var text = AttributedString(hit.text)
        let ns = hit.text as NSString
        for m in hit.matches {
            let range = NSRange(location: Int(m.start), length: Int(m.length))
            guard NSMaxRange(range) <= ns.length, let r = Range(range, in: text) else { continue }
            text[r].backgroundColor = Color(theme.highlight(.yellow))
        }
        return text
    }
}
