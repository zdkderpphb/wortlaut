// Plain-text rendering for "Kopieren" – mirrors export::to_txt in Rust.

import type { Segment, Transcript } from "./api";
import { formatStamp } from "./format";

export function plainText(segments: Segment[], timestamps: boolean): string {
  return segments
    .map((s) => ({ s, text: s.text.trim() }))
    .filter(({ text }) => text.length > 0)
    .map(({ s, text }) => (timestamps ? `${formatStamp(s.startMs)} ${text}` : text))
    .join("\n\n");
}

export function toTranscript(title: string, language: string, segments: Segment[]): Transcript {
  return { title, language: language === "auto" ? null : language, segments };
}
