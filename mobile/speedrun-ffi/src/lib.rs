// Copyright: Speedrun contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html
//
// Minimal C ABI over Anki's shared Rust engine (`anki` crate / rslib), enough
// to drive a *real* review session from a mobile client. This is the F12
// (mobile) foundation: the phone links this as a static/dynamic library and
// calls the functions below, which run the genuine FSRS scheduler + SQLite
// storage in rslib. Nothing here reimplements scheduling.
//
// See `include/speedrun_ffi.h` for the C contract, and `tests/review_loop.rs`
// for an end-to-end proof that a review session runs through this ABI.

use std::cell::RefCell;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::CStr;
use std::ffi::CString;
use std::panic::catch_unwind;
use std::panic::AssertUnwindSafe;
use std::ptr;
use std::sync::OnceLock;

use anki::card::CardId;
use anki::collection::Collection;
use anki::collection::CollectionBuilder;
use anki::decks::DeckId;
use anki::scheduler::answering::CardAnswer;
use anki::scheduler::answering::Rating;
use anki::timestamp::TimestampMillis;

// ---------------------------------------------------------------------------
// Error handling (thread-local last error)
// ---------------------------------------------------------------------------

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: String) {
    let c = CString::new(msg).unwrap_or_else(|_| CString::new("error (nul byte)").unwrap());
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(c));
}

fn clear_last_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
}

/// Run `f`, converting any panic into a set error + `fallback` so panics never
/// unwind across the FFI boundary (which would be undefined behaviour).
fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            set_last_error("panic inside speedrun-ffi".to_string());
            fallback
        }
    }
}

// ---------------------------------------------------------------------------
// Handle + small helpers
// ---------------------------------------------------------------------------

/// Opaque handle exposed to C. Wraps the shared-engine `Collection`.
pub struct SpeedrunCollection {
    col: Collection,
}

/// # Safety
/// `handle` must be a pointer returned by `speedrun_open` and not yet closed.
unsafe fn col_mut<'a>(handle: *mut SpeedrunCollection) -> Option<&'a mut Collection> {
    handle.as_mut().map(|h| &mut h.col)
}

/// # Safety
/// `ptr` must be NULL or a valid, NUL-terminated C string.
unsafe fn cstr_to_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    CStr::from_ptr(ptr).to_str().ok().map(str::to_owned)
}

// ---------------------------------------------------------------------------
// Exported C ABI
// ---------------------------------------------------------------------------

/// Engine build hash / version string. Static lifetime; do not free.
#[no_mangle]
pub extern "C" fn speedrun_version() -> *const c_char {
    static VERSION: OnceLock<CString> = OnceLock::new();
    VERSION
        .get_or_init(|| {
            CString::new(anki::version::buildhash()).unwrap_or_else(|_| CString::new("dev").unwrap())
        })
        .as_ptr()
}

/// Last error string on the current thread, or NULL. Do not free.
#[no_mangle]
pub extern "C" fn speedrun_last_error() -> *const c_char {
    LAST_ERROR.with(|e| match e.borrow().as_ref() {
        Some(c) => c.as_ptr(),
        None => ptr::null(),
    })
}

/// Open (or create) a collection at `path` (":memory:" for in-memory).
/// Returns NULL on error.
///
/// # Safety
/// `path` must be NULL or a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn speedrun_open(path: *const c_char) -> *mut SpeedrunCollection {
    guard(ptr::null_mut(), || {
        clear_last_error();
        let path = match cstr_to_string(path) {
            Some(p) => p,
            None => {
                set_last_error("speedrun_open: null or non-UTF8 path".to_string());
                return ptr::null_mut();
            }
        };
        match CollectionBuilder::new(path).build() {
            Ok(col) => Box::into_raw(Box::new(SpeedrunCollection { col })),
            Err(e) => {
                set_last_error(format!("open failed: {e}"));
                ptr::null_mut()
            }
        }
    })
}

/// Close a collection. Safe to call with NULL.
///
/// # Safety
/// `handle` must be NULL or a pointer previously returned by `speedrun_open`
/// (and not already closed).
#[no_mangle]
pub unsafe extern "C" fn speedrun_close(handle: *mut SpeedrunCollection) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Write the current due counts into the out-params (any may be NULL).
/// Returns 0 on success, -1 on error.
///
/// # Safety
/// `handle` must be valid; the out-pointers must be NULL or writable.
#[no_mangle]
pub unsafe extern "C" fn speedrun_counts(
    handle: *mut SpeedrunCollection,
    new_out: *mut i64,
    learning_out: *mut i64,
    review_out: *mut i64,
) -> c_int {
    guard(-1, || {
        clear_last_error();
        let col = match col_mut(handle) {
            Some(c) => c,
            None => {
                set_last_error("speedrun_counts: null handle".to_string());
                return -1;
            }
        };
        // fetch_limit=1: we only need the counts, not the whole queue.
        match col.get_queued_cards(1, false) {
            Ok(q) => {
                if !new_out.is_null() {
                    *new_out = q.new_count as i64;
                }
                if !learning_out.is_null() {
                    *learning_out = q.learning_count as i64;
                }
                if !review_out.is_null() {
                    *review_out = q.review_count as i64;
                }
                0
            }
            Err(e) => {
                set_last_error(format!("counts failed: {e}"));
                -1
            }
        }
    })
}

/// Return the next card to study, or NULL (NULL + empty last error => nothing
/// due). Free the result with `speedrun_free_card`.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn speedrun_next_card(handle: *mut SpeedrunCollection) -> *mut SpeedrunCard {
    guard(ptr::null_mut(), || {
        clear_last_error();
        let col = match col_mut(handle) {
            Some(c) => c,
            None => {
                set_last_error("speedrun_next_card: null handle".to_string());
                return ptr::null_mut();
            }
        };
        // Ask the shared engine for the top of the (real) study queue.
        let queued = match col.get_next_card() {
            Ok(Some(q)) => q,
            Ok(None) => return ptr::null_mut(), // nothing due; leave error cleared
            Err(e) => {
                set_last_error(format!("next_card failed: {e}"));
                return ptr::null_mut();
            }
        };
        let cid = queued.card.id();
        let note_id = queued.card.note_id().0;
        // Render the question with the engine's template renderer.
        let question = match col.render_existing_card(cid, false, false) {
            Ok(r) => r.question().into_owned(),
            Err(e) => {
                set_last_error(format!("render failed: {e}"));
                return ptr::null_mut();
            }
        };
        let question = CString::new(question).unwrap_or_else(|_| CString::new("").unwrap());
        Box::into_raw(Box::new(SpeedrunCard {
            card_id: cid.0,
            note_id,
            question: question.into_raw(),
        }))
    })
}

/// One card to review. Mirrors the C struct in `speedrun_ffi.h`.
#[repr(C)]
pub struct SpeedrunCard {
    pub card_id: i64,
    pub note_id: i64,
    /// Engine-rendered question HTML; owned, freed by `speedrun_free_card`.
    pub question: *mut c_char,
}

/// Free a card returned by `speedrun_next_card`. Safe with NULL.
///
/// # Safety
/// `card` must be NULL or a pointer returned by `speedrun_next_card` (not freed
/// yet).
#[no_mangle]
pub unsafe extern "C" fn speedrun_free_card(card: *mut SpeedrunCard) {
    if card.is_null() {
        return;
    }
    let card = Box::from_raw(card);
    if !card.question.is_null() {
        drop(CString::from_raw(card.question));
    }
}

/// Answer `card_id` with `ease` (1=Again..4=Easy), recording `millis_taken`.
/// Runs the real (undo-safe) scheduler transaction. Returns 0 / -1.
///
/// # Safety
/// `handle` must be valid.
#[no_mangle]
pub unsafe extern "C" fn speedrun_answer_card(
    handle: *mut SpeedrunCollection,
    card_id: i64,
    ease: c_int,
    millis_taken: i64,
) -> c_int {
    guard(-1, || {
        clear_last_error();
        let col = match col_mut(handle) {
            Some(c) => c,
            None => {
                set_last_error("speedrun_answer_card: null handle".to_string());
                return -1;
            }
        };
        let rating = match ease {
            1 => Rating::Again,
            2 => Rating::Hard,
            3 => Rating::Good,
            4 => Rating::Easy,
            _ => {
                set_last_error("ease must be 1 (Again), 2 (Hard), 3 (Good) or 4 (Easy)".to_string());
                return -1;
            }
        };
        let cid = CardId(card_id);
        // Recompute the scheduling states from the engine so `current_state`
        // matches the card exactly, then pick the button the user pressed.
        let states = match col.get_scheduling_states(cid) {
            Ok(s) => s,
            Err(e) => {
                set_last_error(format!("scheduling states failed: {e}"));
                return -1;
            }
        };
        let new_state = match rating {
            Rating::Again => states.again,
            Rating::Hard => states.hard,
            Rating::Good => states.good,
            Rating::Easy => states.easy,
        };
        let mut answer = CardAnswer {
            card_id: cid,
            current_state: states.current,
            new_state,
            rating,
            answered_at: TimestampMillis::now(),
            milliseconds_taken: millis_taken.max(0) as u32,
            custom_data: None,
            from_queue: true,
        };
        match col.answer_card(&mut answer) {
            Ok(_) => 0,
            Err(e) => {
                set_last_error(format!("answer failed: {e}"));
                -1
            }
        }
    })
}

/// Add a Basic (front/back) note to the default deck. Returns the new note id
/// or -1. Convenience for demos/tests so there is something to review.
///
/// # Safety
/// `handle` must be valid; `front`/`back` must be NULL or valid C strings.
#[no_mangle]
pub unsafe extern "C" fn speedrun_add_basic_note(
    handle: *mut SpeedrunCollection,
    front: *const c_char,
    back: *const c_char,
) -> i64 {
    guard(-1, || {
        clear_last_error();
        let col = match col_mut(handle) {
            Some(c) => c,
            None => {
                set_last_error("speedrun_add_basic_note: null handle".to_string());
                return -1;
            }
        };
        let front = cstr_to_string(front).unwrap_or_default();
        let back = cstr_to_string(back).unwrap_or_default();
        match add_basic_note(col, &front, &back) {
            Ok(nid) => nid,
            Err(msg) => {
                set_last_error(msg);
                -1
            }
        }
    })
}

fn add_basic_note(col: &mut Collection, front: &str, back: &str) -> Result<i64, String> {
    let notetype = col
        .get_notetype_by_name("Basic")
        .map_err(|e| format!("lookup Basic notetype failed: {e}"))?
        .ok_or_else(|| "collection has no 'Basic' notetype".to_string())?;
    let mut note = notetype.new_note();
    note.set_field(0, front)
        .map_err(|e| format!("set front failed: {e}"))?;
    note.set_field(1, back)
        .map_err(|e| format!("set back failed: {e}"))?;
    // DeckId(1) is the always-present Default deck.
    col.add_note(&mut note, DeckId(1))
        .map_err(|e| format!("add_note failed: {e}"))?;
    Ok(note.id.0)
}
