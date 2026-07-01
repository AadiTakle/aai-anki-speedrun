// End-to-end proof that a real review session runs on the SHARED engine
// through the C ABI. We call the exported functions exactly as a mobile client
// would (via the C header), using an in-memory collection so the test is
// self-contained. If this passes, the phone path is real: open -> counts ->
// render question -> answer through rslib's scheduler.

use std::ffi::c_char;
use std::ffi::CStr;
use std::ffi::CString;

use speedrun_ffi::speedrun_add_basic_note;
use speedrun_ffi::speedrun_answer_card;
use speedrun_ffi::speedrun_close;
use speedrun_ffi::speedrun_counts;
use speedrun_ffi::speedrun_free_card;
use speedrun_ffi::speedrun_last_error;
use speedrun_ffi::speedrun_next_card;
use speedrun_ffi::speedrun_open;
use speedrun_ffi::speedrun_version;

fn last_err() -> String {
    let p = speedrun_last_error();
    if p.is_null() {
        return "<none>".to_string();
    }
    unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
}

fn counts(col: *mut speedrun_ffi::SpeedrunCollection) -> (i64, i64, i64) {
    let (mut n, mut l, mut r) = (0i64, 0i64, 0i64);
    let rc = unsafe { speedrun_counts(col, &mut n, &mut l, &mut r) };
    assert_eq!(rc, 0, "counts failed: {}", last_err());
    (n, l, r)
}

#[test]
fn review_session_runs_on_shared_engine() {
    // version comes from the shared engine build metadata.
    let version = unsafe { CStr::from_ptr(speedrun_version()) }
        .to_string_lossy()
        .into_owned();
    assert!(!version.is_empty());

    let path = CString::new(":memory:").unwrap();
    let col = unsafe { speedrun_open(path.as_ptr()) };
    assert!(!col.is_null(), "open failed: {}", last_err());

    // Nothing to study yet.
    assert_eq!(counts(col), (0, 0, 0));

    // Seed one Basic card through the engine.
    let front = CString::new("What supplies the SA node in most people?").unwrap();
    let back = CString::new("The right coronary artery").unwrap();
    let note_id = unsafe { speedrun_add_basic_note(col, front.as_ptr(), back.as_ptr()) };
    assert!(note_id > 0, "seed failed: {}", last_err());

    // Exactly one new card is now due.
    let (new, learning, review) = counts(col);
    assert_eq!(
        (new, learning, review),
        (1, 0, 0),
        "expected 1 new card after seeding"
    );

    // Pull the next card from the real queue and check the engine rendered the
    // question we seeded.
    let card = unsafe { speedrun_next_card(col) };
    assert!(!card.is_null(), "next_card returned nothing: {}", last_err());
    let card_id = unsafe { (*card).card_id };
    let question_ptr: *const c_char = unsafe { (*card).question };
    let question = unsafe { CStr::from_ptr(question_ptr) }
        .to_string_lossy()
        .into_owned();
    assert!(card_id > 0);
    assert!(
        question.contains("SA node"),
        "engine-rendered question was: {question:?}"
    );
    unsafe { speedrun_free_card(card) };

    // Answer "Good" through the scheduler transaction.
    let rc = unsafe { speedrun_answer_card(col, card_id, 3, 1500) };
    assert_eq!(rc, 0, "answer failed: {}", last_err());

    // The card is no longer a *new* card (the shared scheduler advanced it).
    let (new_after, _, _) = counts(col);
    assert_eq!(new_after, 0, "new count should drop after answering");

    unsafe { speedrun_close(col) };
}

#[test]
fn open_bad_path_reports_error() {
    // A path under a non-existent directory should fail to open and set an
    // error rather than panic across the boundary.
    let path = CString::new("/definitely/not/a/real/dir/speedrun.anki2").unwrap();
    let col = unsafe { speedrun_open(path.as_ptr()) };
    assert!(col.is_null());
    assert_ne!(last_err(), "<none>");
}
