# F10 - Exam-deck review loop (Wednesday proof)

This is the end-to-end proof that the mandatory Rust engine change **F5
(points-at-stake review order)** works on a real, multi-topic Step 2 CK deck run
through the actual v3 scheduler. On a synthetic 9-card / 3-topic deck (all cards
tied at gather time) the demo shows the same deck under the stock `DAY` order
(interleaved) versus `REVIEW_CARD_ORDER_POINTS_AT_STAKE` (grouped by
`blueprint_weight x weakness`, descending), then answers **every** due card
through the scheduler until the session drains, verifies `pragma
integrity_check` is `ok`, and undoes the last answer to show it returns to the
due queue. The automated, re-runnable version of the same proof lives in
`anki/pylib/tests/test_speedrun_review_loop.py`.

## Run it

From the repo's `anki/` directory (after `just build`):

```bash
PYTHONPATH=$(pwd)/out/pylib ANKI_TEST_MODE=1 ./out/pyenv/bin/python \
    ../proof/wednesday/review_loop_demo.py
```
