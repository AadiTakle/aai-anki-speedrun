// A minimal single-screen review UI driven entirely by the shared Rust engine
// through `SpeedrunEngine`. Shows the engine's due counts + the next card's
// question, and grades it with Again/Hard/Good/Easy. Kept to cross-platform
// SwiftUI so it also compiles under `swift build` on macOS.

#if canImport(SwiftUI)
import SwiftUI

@available(iOS 15.0, macOS 12.0, *)
@MainActor
public final class ReviewSession: ObservableObject {
    @Published public private(set) var card: SpeedrunCard?
    @Published public private(set) var counts = SpeedrunCounts(new: 0, learning: 0, review: 0)
    @Published public private(set) var status = ""
    /// A persistent status line used by the auto-review demo (see startAutoDemo).
    @Published public private(set) var banner = ""
    /// The button momentarily "pressed" by the auto-review demo, for a visible cue.
    @Published public private(set) var flash: SpeedrunEase?

    private let engine: SpeedrunEngine
    private var autoDemoTask: Task<Void, Never>?

    public init(engine: SpeedrunEngine) {
        self.engine = engine
        refresh()
    }

    public func refresh() {
        do {
            counts = try engine.counts()
            card = try engine.nextCard()
            status = card == nil ? "All caught up." : ""
        } catch {
            status = "Error: \(error)"
        }
    }

    public func answer(_ ease: SpeedrunEase) {
        guard let card else { return }
        do {
            try engine.answer(cardID: card.cardID, ease: ease)
            refresh()
        } catch {
            status = "Error: \(error)"
        }
    }

    /// Auto-drive a full review session on the SHARED engine, for a hands-off
    /// Simulator recording (`simctl` can't inject taps headlessly). Every step
    /// is a real engine call — pull the queued card, answer "Good" through the
    /// actual scheduler transaction, then re-read counts — just paced with
    /// pauses so each state is visible on screen. No scheduling logic here.
    public func startAutoDemo(stepMillis: UInt64 = 1300) {
        guard autoDemoTask == nil else { return }
        banner = "auto-review running on the shared engine…"
        autoDemoTask = Task { @MainActor in
            try? await Task.sleep(nanoseconds: stepMillis * 1_000_000)
            var answered = 0
            while self.card != nil {
                self.flash = .good
                try? await Task.sleep(nanoseconds: 300 * 1_000_000)
                self.answer(.good)
                self.flash = nil
                answered += 1
                self.banner = "auto-review · answered Good ×\(answered)"
                try? await Task.sleep(nanoseconds: stepMillis * 1_000_000)
            }
            self.banner = "auto-review complete · \(answered) card(s) answered on the shared engine"
            self.autoDemoTask = nil
        }
    }
}

@available(iOS 15.0, macOS 12.0, *)
public struct ReviewView: View {
    @ObservedObject private var session: ReviewSession

    public init(session: ReviewSession) {
        self.session = session
    }

    public var body: some View {
        VStack(spacing: 24) {
            countsBar

            if !session.banner.isEmpty {
                Text(session.banner)
                    .font(.caption)
                    .foregroundColor(.accentColor)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal)
            }

            Spacer()

            if let card = session.card {
                Text(card.questionText.isEmpty ? "(empty question)" : card.questionText)
                    .font(.title2)
                    .multilineTextAlignment(.center)
                    .padding()
                Spacer()
                answerButtons
            } else {
                Text(session.status.isEmpty ? "No cards due." : session.status)
                    .font(.title3)
                    .foregroundColor(.secondary)
            }

            Spacer()

            Text("shared engine build \(SpeedrunEngine.engineVersion)")
                .font(.footnote)
                .foregroundColor(.secondary)
        }
        .padding()
    }

    private var countsBar: some View {
        HStack(spacing: 24) {
            countPill("New", session.counts.new, .blue)
            countPill("Learn", session.counts.learning, .red)
            countPill("Review", session.counts.review, .green)
        }
    }

    private func countPill(_ label: String, _ value: Int64, _ color: Color) -> some View {
        VStack(spacing: 2) {
            Text("\(value)").font(.headline).foregroundColor(color)
            Text(label).font(.caption).foregroundColor(.secondary)
        }
    }

    private var answerButtons: some View {
        HStack(spacing: 10) {
            ForEach(SpeedrunEase.allCases, id: \.rawValue) { ease in
                Button(ease.label) { session.answer(ease) }
                    .buttonStyle(.borderedProminent)
                    .scaleEffect(session.flash == ease ? 1.18 : 1.0)
                    .animation(.easeInOut(duration: 0.2), value: session.flash)
            }
        }
    }
}
#endif
