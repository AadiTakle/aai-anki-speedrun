// Host CLI that runs a real review session on the SHARED Rust engine, entirely
// through the Swift wrapper (Swift -> C ABI -> rslib). This is the "phone code
// path" exercised on the host: no scheduling logic lives in Swift.
//
// Run with:  swift run speedrun-demo   (after building the host static lib,
// see README "Prove it from Swift"). Links ../speedrun-ffi/target/debug.

import SpeedrunKit

print("shared engine build: \(SpeedrunEngine.engineVersion)")

let engine = try SpeedrunEngine(path: ":memory:")
try engine.addBasicNote(front: "Which artery supplies the SA node in most people?",
                        back: "Right coronary artery")
try engine.addBasicNote(front: "First-line management of stable narrow-complex SVT?",
                        back: "Vagal maneuvers, then IV adenosine")

var counts = try engine.counts()
print("seeded -> new=\(counts.new) learning=\(counts.learning) review=\(counts.review)")

var answered = 0
while let card = try engine.nextCard() {
    print("Q: \(card.questionText)")
    try engine.answer(cardID: card.cardID, ease: .good)
    answered += 1
    counts = try engine.counts()
    print("  answered Good -> new=\(counts.new) learning=\(counts.learning) review=\(counts.review)")
    if answered >= 10 { break }  // safety valve
}

print("review session complete; \(answered) card(s) answered on the shared engine.")
