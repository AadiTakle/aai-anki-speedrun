# F13 - Speedrun engine benchmark harness (challenge 7h skeleton)

A [criterion](https://bheisler.github.io/criterion.rs/book/) benchmark harness
that measures the three Speedrun engine hot paths on one synthetic large deck:
**F5** points-at-stake `Collection::build_queues` (gather + reorder by
`blueprint_weight x weakness`), **F4** `Collection::topic_mastery` (per-topic
mastered count + average FSRS recall over the full card->topic crosswalk), and
**F6** `Collection::memory_score` (the blueprint-weighted memory score, run
through its real scoring path rather than the abstain shortcut). The synthetic
deck is built once per run by `anki::speedrun::bench::build_synthetic_collection`
(gated behind the `bench` cargo feature): **10,000** due review cards spread
round-robin across **20** topics, each with an FSRS memory state, seeded
blueprint weights + per-topic weakness + the full crosswalk, 500 revlog rows and
100% blueprint coverage so `memory_score` scores instead of abstaining. The deck
size is a parameter (`build_synthetic_collection(n)`); 10k keeps a full run quick
while the challenge-7h stretch target is 50,000. Note that `build_queues` is
bounded by Anki's daily review cap (`reviews_per_day` maxes out at 9,999), so F5
measures a full **9,999**-card gather+reorder; F4 mastery and F6 score iterate
the entire 10,000-card crosswalk. A one-time assertion in the harness fails loudly
if that gather is ever accidentally empty, so the numbers can't silently measure
a no-op.

## Run it

From the repo's `anki/` directory (after `just build`):

```bash
just bench
```

(equivalent to `CARGO_TARGET_DIR=out/rust cargo bench -p anki --features bench`).

## Results

Measured on the dev machine (Apple Silicon / macOS, release build). Criterion
reports `[lower  estimate  upper]` — the 95% confidence interval around the
estimate; the middle value is the p50-style point estimate and the bounds bracket
the run-to-run spread (a p95-style upper bound):

| Benchmark (10,000-card deck) | lower | **estimate (p50)** | upper |
|---|---|---|---|
| F5 `build_queues` (points-at-stake, 9,999 gathered) | 9.0629 ms | **9.1144 ms** | 9.1734 ms |
| F4 `topic_mastery` (20 topics) | 6.8629 ms | **6.9929 ms** | 7.1778 ms |
| F6 `memory_score` | 6.8345 ms | **6.8542 ms** | 6.8829 ms |

For reference the pre-existing `anki_tag_parse` micro-benchmark, kept alongside
the new ones, reports `[457.08 ns  459.58 ns  463.65 ns]`.

All three Speedrun paths score a 10k-card deck in single-digit milliseconds, so
the engine comfortably stays interactive at deck sizes far beyond a real Step 2
CK QBank; re-run with a larger `n` to track the 50k target.
