/*
 * speedrun_ffi.h  --  Minimal C ABI over Anki's shared Rust engine (rslib).
 *
 * Speedrun feature F12 (mobile). This header is the contract between the Rust
 * `speedrun-ffi` static/dynamic library and native mobile clients (Swift on
 * iOS, JNI on Android). Every function here is implemented in
 * `src/lib.rs` by calling straight into the shared `anki` crate -- the
 * scheduler / FSRS / storage code is NOT reimplemented on the client.
 *
 * Threading: a SpeedrunCollection handle is NOT thread-safe. Use it from a
 * single thread (e.g. one serial queue / actor).
 *
 * Error handling: functions that can fail return NULL / -1 and set a
 * thread-local error string retrievable via speedrun_last_error(). A NULL
 * from speedrun_next_card() with an EMPTY last error means "no card due".
 *
 * Ownership: strings returned by the library (SpeedrunCard.question,
 * speedrun_last_error) are owned by the library. Copy them before the next
 * call. Free a SpeedrunCard with speedrun_free_card().
 */
#ifndef SPEEDRUN_FFI_H
#define SPEEDRUN_FFI_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle to an open collection (wraps rslib's Collection). */
typedef struct SpeedrunCollection SpeedrunCollection;

/* One card to review. `question` is the engine-rendered question HTML. */
typedef struct SpeedrunCard {
    int64_t card_id;
    int64_t note_id;
    char *question; /* owned; freed by speedrun_free_card */
} SpeedrunCard;

/* Engine build hash / version string (static; do not free). */
const char *speedrun_version(void);

/* Last error on the current thread, or NULL if none. Do not free. */
const char *speedrun_last_error(void);

/*
 * Open (or create) a collection at `path`. Pass ":memory:" for a throwaway
 * in-memory collection. Returns NULL on error (see speedrun_last_error).
 */
SpeedrunCollection *speedrun_open(const char *path);

/* Close a collection opened by speedrun_open. Safe to call with NULL. */
void speedrun_close(SpeedrunCollection *col);

/*
 * Write the current due counts into the out-params (any may be NULL).
 * Returns 0 on success, -1 on error.
 */
int speedrun_counts(SpeedrunCollection *col,
                    int64_t *new_out,
                    int64_t *learning_out,
                    int64_t *review_out);

/*
 * Return the next card to study (top of the engine's queue), or NULL.
 * NULL + empty last error => nothing due. Free the result with
 * speedrun_free_card().
 */
SpeedrunCard *speedrun_next_card(SpeedrunCollection *col);

/* Free a SpeedrunCard returned by speedrun_next_card. Safe with NULL. */
void speedrun_free_card(SpeedrunCard *card);

/*
 * Answer `card_id` with `ease` (1=Again, 2=Hard, 3=Good, 4=Easy),
 * recording `millis_taken` as the time spent. This runs the real scheduler
 * transaction (undo-safe). Returns 0 on success, -1 on error.
 */
int speedrun_answer_card(SpeedrunCollection *col,
                         int64_t card_id,
                         int ease,
                         int64_t millis_taken);

/*
 * Convenience for demos/tests: add a "Basic" (front/back) note to the default
 * deck so there is something to review. Returns the new note id, or -1.
 */
int64_t speedrun_add_basic_note(SpeedrunCollection *col,
                                const char *front,
                                const char *back);

#ifdef __cplusplus
}
#endif

#endif /* SPEEDRUN_FFI_H */
