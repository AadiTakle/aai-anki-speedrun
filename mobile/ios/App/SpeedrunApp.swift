// iOS app entry point. This file belongs to the Xcode *app* target (it is NOT
// part of the SwiftPM library package above), because SwiftPM cannot emit an
// iOS `.app` bundle on its own. See README.md "Run the app in Xcode".
//
// The app opens a real on-device collection (in the app's Documents dir),
// seeds a few Step-2-flavoured demo cards on first launch, and drives a review
// session on the SHARED Rust engine via SpeedrunKit.

#if canImport(SwiftUI) && canImport(SpeedrunKit)
import SwiftUI
import SpeedrunKit

@main
@available(iOS 15.0, *)
struct SpeedrunApp: App {
    @StateObject private var session: ReviewSession

    init() {
        _session = StateObject(wrappedValue: ReviewSession(engine: SpeedrunApp.makeEngine()))
    }

    var body: some Scene {
        WindowGroup {
            ReviewView(session: session)
        }
    }

    private static func makeEngine() -> SpeedrunEngine {
        let docs = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let path = docs.appendingPathComponent("speedrun.anki2").path
        let isFresh = !FileManager.default.fileExists(atPath: path)
        do {
            let engine = try SpeedrunEngine(path: path)
            if isFresh {
                try engine.addBasicNote(
                    front: "Which artery supplies the SA node in most people?",
                    back: "The right coronary artery")
                try engine.addBasicNote(
                    front: "First-line management of stable narrow-complex SVT?",
                    back: "Vagal maneuvers, then IV adenosine")
                try engine.addBasicNote(
                    front: "Most common cause of spontaneous subarachnoid hemorrhage?",
                    back: "Ruptured saccular (berry) aneurysm")
            }
            return engine
        } catch {
            fatalError("failed to open Speedrun engine: \(error)")
        }
    }
}
#endif
