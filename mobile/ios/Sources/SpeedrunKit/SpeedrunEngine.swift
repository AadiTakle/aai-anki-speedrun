// Idiomatic Swift wrapper over the C ABI exported by the Rust `speedrun-ffi`
// crate. Every method here forwards to the SHARED Anki engine (rslib): the
// scheduler, FSRS and SQLite storage all run in Rust. This file contains no
// scheduling logic of its own.

import Foundation
import SpeedrunFFI

/// A single card to review, as produced by the engine's queue + renderer.
public struct SpeedrunCard: Equatable {
    public let cardID: Int64
    public let noteID: Int64
    /// Engine-rendered question HTML.
    public let questionHTML: String

    /// A best-effort plain-text version of the question for simple UIs.
    public var questionText: String { SpeedrunCard.stripHTML(questionHTML) }

    static func stripHTML(_ html: String) -> String {
        let stripped = html.replacingOccurrences(
            of: "<[^>]+>", with: " ", options: .regularExpression
        )
        return stripped
            .replacingOccurrences(of: "&nbsp;", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

/// Due-card counts for the current deck.
public struct SpeedrunCounts: Equatable {
    public let new: Int64
    public let learning: Int64
    public let review: Int64
    public var total: Int64 { new + learning + review }
}

/// The four answer buttons, matching the engine's `Rating`.
public enum SpeedrunEase: Int32, CaseIterable {
    case again = 1
    case hard = 2
    case good = 3
    case easy = 4

    public var label: String {
        switch self {
        case .again: return "Again"
        case .hard: return "Hard"
        case .good: return "Good"
        case .easy: return "Easy"
        }
    }
}

public enum SpeedrunError: Error, CustomStringConvertible {
    case open(String)
    case engine(String)

    public var description: String {
        switch self {
        case .open(let m), .engine(let m): return m
        }
    }
}

/// An open collection backed by the shared Rust engine. Not thread-safe: use
/// from a single thread / actor.
public final class SpeedrunEngine {
    private let handle: OpaquePointer

    /// The shared engine's build hash / version string.
    public static var engineVersion: String {
        guard let p = speedrun_version() else { return "unknown" }
        return String(cString: p)
    }

    /// Open (or create) a collection at `path`. Pass ":memory:" for a scratch
    /// in-memory collection.
    public init(path: String) throws {
        guard let handle = speedrun_open(path) else {
            throw SpeedrunError.open(SpeedrunEngine.lastError() ?? "failed to open collection")
        }
        self.handle = handle
    }

    deinit {
        speedrun_close(handle)
    }

    private static func lastError() -> String? {
        guard let p = speedrun_last_error() else { return nil }
        let s = String(cString: p)
        return s.isEmpty ? nil : s
    }

    /// Current due counts for the active deck.
    public func counts() throws -> SpeedrunCounts {
        var newCount: Int64 = 0
        var learning: Int64 = 0
        var review: Int64 = 0
        guard speedrun_counts(handle, &newCount, &learning, &review) == 0 else {
            throw SpeedrunError.engine(SpeedrunEngine.lastError() ?? "counts failed")
        }
        return SpeedrunCounts(new: newCount, learning: learning, review: review)
    }

    /// The next card to study, or `nil` if nothing is due.
    public func nextCard() throws -> SpeedrunCard? {
        guard let cardPtr = speedrun_next_card(handle) else {
            if let err = SpeedrunEngine.lastError() {
                throw SpeedrunError.engine(err)
            }
            return nil  // nothing due
        }
        defer { speedrun_free_card(cardPtr) }
        let card = cardPtr.pointee
        let question = card.question.map { String(cString: $0) } ?? ""
        return SpeedrunCard(cardID: card.card_id, noteID: card.note_id, questionHTML: question)
    }

    /// Answer a card through the real scheduler (undo-safe).
    public func answer(cardID: Int64, ease: SpeedrunEase, millisTaken: Int64 = 0) throws {
        guard speedrun_answer_card(handle, cardID, ease.rawValue, millisTaken) == 0 else {
            throw SpeedrunError.engine(SpeedrunEngine.lastError() ?? "answer failed")
        }
    }

    /// Add a Basic (front/back) note to the default deck. Handy for demos.
    @discardableResult
    public func addBasicNote(front: String, back: String) throws -> Int64 {
        let noteID = speedrun_add_basic_note(handle, front, back)
        guard noteID >= 0 else {
            throw SpeedrunError.engine(SpeedrunEngine.lastError() ?? "add note failed")
        }
        return noteID
    }
}
