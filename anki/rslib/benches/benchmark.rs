// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

use anki::card_rendering::anki_directive_benchmark;
use anki::speedrun::bench::build_synthetic_collection;
use anki::speedrun::bench::run_build_queues_points_at_stake;
use anki::speedrun::bench::run_memory_score;
use anki::speedrun::bench::run_topic_mastery;
use anki::speedrun::bench::MAX_DAILY_REVIEW_LIMIT;
use anki::speedrun::bench::SYNTHETIC_CARD_COUNT;
use anki::speedrun::bench::SYNTHETIC_TOPIC_COUNT;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("anki_tag_parse", |b| b.iter(|| anki_directive_benchmark()));

    speedrun_benchmark(c);
}

/// F13 (challenge 7h) — benchmark the three Speedrun engine entry points on one
/// synthetic large deck: F5 points-at-stake `build_queues`, F4 `topic_mastery`,
/// and F6 `memory_score`.
pub fn speedrun_benchmark(c: &mut Criterion) {
    // Built once (setup is not timed); the three read-mostly ops are then
    // measured against the same collection.
    let mut col = build_synthetic_collection(SYNTHETIC_CARD_COUNT);

    // Sanity-guard the workload: the points-at-stake queue must gather up to
    // Anki's daily review cap, so the benchmark measures a real full-window
    // gather rather than an accidentally-empty one.
    let expected_gathered = SYNTHETIC_CARD_COUNT.min(MAX_DAILY_REVIEW_LIMIT);
    assert_eq!(
        run_build_queues_points_at_stake(&mut col),
        expected_gathered,
        "expected {expected_gathered} synthetic review cards to be gathered"
    );

    let mut group = c.benchmark_group("speedrun");
    group.sample_size(30);

    group.bench_function(
        format!("build_queues_points_at_stake/{SYNTHETIC_CARD_COUNT}_cards"),
        |b| b.iter(|| run_build_queues_points_at_stake(&mut col)),
    );
    group.bench_function(
        format!("topic_mastery/{SYNTHETIC_CARD_COUNT}_cards_{SYNTHETIC_TOPIC_COUNT}_topics"),
        |b| b.iter(|| run_topic_mastery(&mut col)),
    );
    group.bench_function(
        format!("memory_score/{SYNTHETIC_CARD_COUNT}_cards"),
        |b| b.iter(|| run_memory_score(&mut col)),
    );

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
