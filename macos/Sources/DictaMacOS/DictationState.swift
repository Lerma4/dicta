import Foundation

enum DictationPhase: String, Codable, CaseIterable {
    case idle
    case preparing
    case recording
    case processing
    case inserting
    case completed
    case cancelled
    case error

    var isActive: Bool {
        switch self {
        case .preparing, .recording, .processing, .inserting: true
        case .idle, .completed, .cancelled, .error: false
        }
    }

    func transition(to next: DictationPhase) throws -> DictationPhase {
        let permitted: Bool
        switch (self, next) {
        case (.idle, .preparing), (.completed, .preparing), (.cancelled, .preparing), (.error, .preparing),
             (.preparing, .recording), (.recording, .processing),
             (.processing, .inserting), (.processing, .completed), (.inserting, .completed),
             (.preparing, .error), (.recording, .error), (.processing, .error), (.inserting, .error),
             (.preparing, .cancelled), (.recording, .cancelled), (.processing, .cancelled), (.inserting, .cancelled),
             (.completed, .idle), (.cancelled, .idle), (.error, .idle):
            permitted = true
        default:
            permitted = false
        }
        guard permitted else { throw DictationTransitionError.invalid(from: self, to: next) }
        return next
    }
}

enum DictationTransitionError: Error, LocalizedError, Equatable {
    case invalid(from: DictationPhase, to: DictationPhase)

    var errorDescription: String? {
        guard case let .invalid(from, to) = self else { return nil }
        return "Invalid dictation transition: \(from.rawValue) → \(to.rawValue)"
    }
}

struct DictationSnapshot: Codable, Equatable {
    var state: DictationPhase = .idle
    var sessionID: String? = nil
    var elapsedMilliseconds: UInt64 = 0
    var level: Float = 0
    var speech = false
    var message: String? = nil
    var recoveryText: String? = nil
    var recoveryPending = false

    private enum CodingKeys: String, CodingKey {
        case state
        case sessionID = "session_id"
        case elapsedMilliseconds = "elapsed_ms"
        case level
        case speech
        case message
        case recoveryText = "recovery_text"
        case recoveryPending = "recovery_pending"
    }

    init() {}

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(state, forKey: .state)
        try container.encode(sessionID, forKey: .sessionID)
        try container.encode(elapsedMilliseconds, forKey: .elapsedMilliseconds)
        try container.encode(level, forKey: .level)
        try container.encode(speech, forKey: .speech)
        try container.encode(message, forKey: .message)
        try container.encode(recoveryText, forKey: .recoveryText)
        try container.encode(recoveryPending, forKey: .recoveryPending)
    }
}
