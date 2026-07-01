// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

//! Speedrun (USMLE Step 2 CK) engine additions: topic taxonomy/crosswalk,
//! per-topic mastery, the points-at-stake review order, and the three scores.
//!
//! The protobuf contract lives in `proto/anki/speedrun.proto`
//! (`SpeedrunService`). This module currently provides compiling stubs that the
//! Wednesday-slice features (F1, F4, F6) fill in. See docs/wednesday_plan.md.

mod attempts;
#[cfg(feature = "bench")]
pub mod bench;
mod coverage;
pub mod focus;
pub mod mastery;
mod next_action;
mod performance;
pub mod score;
mod service;
pub mod store;
