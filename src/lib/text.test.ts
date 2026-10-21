import { describe, expect, it } from "vitest";
import type { Segment, Transcript } from "./api";
import { countInTranscript, countMatches, plainText, replaceInSegments, replaceInTranscript } from "./text";

const seg = (startMs: number, text: string, speaker: number | null = null): Segment => ({
  startMs,
  endMs: startMs + 1000,
  text,
  speaker,
});

const transcript = (segments: Segment[], speakers: string[] = []): Transcript => ({
  id: "1-aa",
  title: "Test",
  language: "de",
  sourcePath: null,
  createdAt: 0,
  durationMs: 0,
  speakers,
  summary: null,
  segments,
});

describe("plainText", () => {
  it("joins paragraphs and skips empty ones", () => {
    const t = transcript([seg(0, " Hallo "), seg(1000, "  "), seg(31_000, "Tschüss")]);
    expect(plainText(t, false)).toBe("Hallo\n\nTschüss");
  });

  it("adds timestamps and speaker names like the text export", () => {
    const t = transcript([seg(0, "Hallo", 0), seg(65_000, "Servus", 1)], ["Anna", "Flo"]);
    expect(plainText(t, true)).toBe("[00:00:00] Anna: Hallo\n\n[00:01:05] Flo: Servus");
  });
});

describe("search and replace", () => {
  const segments = [seg(0, "Frau Hofer sagt: hofer ist da."), seg(1000, "Keine Treffer hier"), seg(2000, "HOFER!")];

  it("counts matches ignoring case", () => {
    expect(countMatches(segments, "hofer")).toBe(3);
    expect(countMatches(segments, "")).toBe(0);
  });

  it("replaces every match and reports how many", () => {
    const copy = segments.map((s) => ({ ...s }));
    expect(replaceInSegments(copy, "hofer", "Huber")).toBe(3);
    expect(copy.map((s) => s.text)).toEqual(["Frau Huber sagt: Huber ist da.", "Keine Treffer hier", "Huber!"]);
  });

  it("treats the search text literally, not as a pattern", () => {
    const copy = [seg(0, "Kosten (ca.) 5 € und (ca.) 6 €")];
    expect(countMatches(copy, "(ca.)")).toBe(2);
    expect(replaceInSegments(copy, "(ca.)", "etwa")).toBe(2);
    expect(copy[0].text).toBe("Kosten etwa 5 € und etwa 6 €");
  });
});

describe("search and replace across the whole transcript", () => {
  it("includes the summary", () => {
    const t = transcript([seg(0, "Projekt in Linz")]);
    t.summary = "Es geht um Linz. Linz ist schön.";
    expect(countInTranscript(t, "linz")).toBe(3);
    expect(replaceInTranscript(t, "Linz", "Graz")).toBe(3);
    expect(t.segments[0].text).toBe("Projekt in Graz");
    expect(t.summary).toBe("Es geht um Graz. Graz ist schön.");
  });

  it("copes with a missing summary", () => {
    const t = transcript([seg(0, "Linz")]);
    expect(countInTranscript(t, "Linz")).toBe(1);
    expect(replaceInTranscript(t, "Linz", "Wien")).toBe(1);
    expect(t.summary).toBeNull();
  });
});
