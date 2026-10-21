// Text helpers for "Kopieren" and "Suchen & Ersetzen".

import type { Segment, Transcript } from "./api";
import { formatStamp } from "./format";

/** Plain text for the clipboard – same layout as the .txt export. */
export function plainText(t: Transcript, timestamps: boolean): string {
  return t.segments
    .map((s) => ({ s, text: s.text.trim() }))
    .filter(({ text }) => text.length > 0)
    .map(({ s, text }) => {
      const stamp = timestamps ? `${formatStamp(s.startMs)} ` : "";
      const name = s.speaker != null ? t.speakers[s.speaker] : undefined;
      return `${stamp}${name ? `${name}: ` : ""}${text}`;
    })
    .join("\n\n");
}

function pattern(query: string): RegExp | null {
  if (!query) return null;
  return new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi");
}

export function countMatches(segments: Segment[], query: string): number {
  const re = pattern(query);
  if (!re) return 0;
  return segments.reduce((n, s) => n + (s.text.match(re)?.length ?? 0), 0);
}

/** Replaces in place; returns how many matches were replaced. */
export function replaceInSegments(segments: Segment[], query: string, replacement: string): number {
  const re = pattern(query);
  if (!re) return 0;
  let count = 0;
  for (const s of segments) {
    s.text = s.text.replace(re, () => {
      count++;
      return replacement;
    });
  }
  return count;
}

/** Paragraphs and summary – what "Alle ersetzen" changes. */
export function countInTranscript(t: Transcript, query: string): number {
  const re = pattern(query);
  const inSummary = re && t.summary ? (t.summary.match(re)?.length ?? 0) : 0;
  return countMatches(t.segments, query) + inSummary;
}

export function replaceInTranscript(t: Transcript, query: string, replacement: string): number {
  let count = replaceInSegments(t.segments, query, replacement);
  const re = pattern(query);
  if (re && t.summary) {
    t.summary = t.summary.replace(re, () => {
      count++;
      return replacement;
    });
  }
  return count;
}
