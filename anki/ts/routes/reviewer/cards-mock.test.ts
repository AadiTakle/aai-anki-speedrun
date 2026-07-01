// Copyright: Ankitects Pty Ltd and contributors
// License: GNU AGPL, version 3 or later; http://www.gnu.org/licenses/agpl.html

import { expect, test } from "vitest";

import {
    pacingAcuity,
    reframePrompt,
    SESSION_CARDS,
    sessionSystems,
    toRuns,
} from "./cards-mock";

test("toRuns splits **bold** markers into alternating runs", () => {
    expect(toRuns("plain")).toEqual([{ text: "plain", strong: false }]);
    expect(toRuns("a **b** c")).toEqual([
        { text: "a ", strong: false },
        { text: "b", strong: true },
        { text: " c", strong: false },
    ]);
});

test("toRuns is lossless once the markers are removed", () => {
    for (const card of SESSION_CARDS) {
        for (const source of [card.question, card.answer]) {
            const rebuilt = toRuns(source)
                .map((run) => run.text)
                .join("");
            expect(rebuilt).toBe(source.replace(/\*\*/g, ""));
        }
    }
});

test("every card marks exactly one emphasised clause", () => {
    for (const card of SESSION_CARDS) {
        expect(toRuns(card.question).filter((r) => r.strong)).toHaveLength(1);
        expect(toRuns(card.answer).filter((r) => r.strong)).toHaveLength(1);
    }
});

test("sessionSystems returns the distinct interleave systems in queue order", () => {
    const systems = sessionSystems(SESSION_CARDS);
    expect(systems[0]).toBe("Renal");
    // No duplicates.
    expect(new Set(systems).size).toBe(systems.length);
    // At least three systems so "interleaved" is meaningful.
    expect(systems.length).toBeGreaterThanOrEqual(3);
});

test("reframePrompt asks how the vignette must change for the wrong answer", () => {
    const [first] = SESSION_CARDS;
    const prompt = reframePrompt(first);
    expect(prompt).toContain(first.reframe.wrongAnswer);
    expect(prompt).toContain("How would the vignette need to change");
});

test("pacingAcuity flags accurate-but-slow only past the target", () => {
    expect(pacingAcuity(10, 90)).toBe("stable");
    expect(pacingAcuity(90, 90)).toBe("stable"); // exactly at target is not yet slow
    expect(pacingAcuity(95, 90)).toBe("watch");
});

test("the seeded deck is front-loaded by points-at-stake (non-increasing)", () => {
    for (let i = 1; i < SESSION_CARDS.length; i++) {
        expect(SESSION_CARDS[i - 1].points).toBeGreaterThanOrEqual(SESSION_CARDS[i].points);
    }
});

test("the seeded deck interleaves — no two adjacent cards share a system", () => {
    for (let i = 1; i < SESSION_CARDS.length; i++) {
        expect(SESSION_CARDS[i].provenance.system).not.toBe(
            SESSION_CARDS[i - 1].provenance.system,
        );
    }
});

test("every card is a complete, uniquely-identified review item", () => {
    const ids = new Set<string>();
    for (const card of SESSION_CARDS) {
        ids.add(card.id);
        expect(card.question.length).toBeGreaterThan(0);
        expect(card.answer.length).toBeGreaterThan(0);
        expect(card.reframe.wrongAnswer.length).toBeGreaterThan(0);
        expect(card.reframe.takeaway.length).toBeGreaterThan(0);
        // All four Anki grades carry an interval label.
        expect(Object.values(card.intervals).every((v) => v.length > 0)).toBe(true);
        expect(card.targetSeconds).toBeGreaterThan(0);
    }
    expect(ids.size).toBe(SESSION_CARDS.length);
});
