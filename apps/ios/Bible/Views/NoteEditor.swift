import SwiftUI

/// A note on one verse. Saving an empty note deletes it.
struct NoteEditor: View {
    @Environment(Store.self) private var store
    @Environment(\.dismiss) private var dismiss
    let ref: VerseRef
    let theme: Theme
    @State private var text = ""
    @State private var existed = false
    @State private var failed = false
    @FocusState private var focused: Bool

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text(verse)
                        .font(.reading(16))
                        .foregroundStyle(Color(theme.ink2))
                        .lineSpacing(4)
                        .padding(14)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(Color(theme.fill), in: RoundedRectangle(cornerRadius: 14, style: .continuous))
                    TextField("Write a note", text: $text, axis: .vertical)
                        .font(.body)
                        .lineLimit(6...)
                        .focused($focused)
                }
                .padding(20)
            }
            .navigationTitle(rangeLabel(from: ref, to: ref))
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") { save(text.trimmingCharacters(in: .whitespacesAndNewlines)) }
                        .fontWeight(.semibold)
                }
                if existed {
                    ToolbarItem(placement: .bottomBar) {
                        Button("Delete Note", role: .destructive) { save("") }
                            .foregroundStyle(Color(theme.danger))
                    }
                }
            }
            .alert("Couldn’t save the note", isPresented: $failed) {
                Button("OK") {}
            } message: {
                Text("Your note is still here. Try again in a moment.")
            }
        }
        .onAppear {
            text = store.note(ref)
            existed = !text.isEmpty
            focused = true
        }
    }

    private func save(_ body: String) {
        if store.setNote(ref, body) { dismiss() } else { failed = true }
    }

    /// The verse as it would be copied: text and reference.
    private var verse: String {
        (try? store.bible.copyText(translation: store.settings.translation, verses: [ref])) ?? ""
    }
}
