import Foundation

enum TranscriptSafety {
    static func needsReview(_ text: String) -> Bool {
        let normalized = text.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return ["[musica]", "(musica)", "[music]", "(music)"].contains(normalized)
    }
}
