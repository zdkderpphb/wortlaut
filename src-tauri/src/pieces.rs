//! What gets transcribed: one piece per speaker contribution when speakers
//! were detected, otherwise 20–30 s chunks cut in pauses.

use crate::chunker::{self, SAMPLE_RATE};

/// A stretch where one speaker talks, in samples, as diarization reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Turn {
    pub start: usize,
    pub end: usize,
    pub speaker: usize,
}

/// A stretch of audio Voxtral transcribes in one go, in samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub start: usize,
    pub end: usize,
    pub speaker: Option<usize>,
}

impl Piece {
    pub fn start_ms(&self) -> u64 {
        (self.start * 1000 / SAMPLE_RATE) as u64
    }

    pub fn end_ms(&self) -> u64 {
        (self.end * 1000 / SAMPLE_RATE) as u64
    }
}

/// Pauses of the same speaker shorter than this stay in one piece.
const BRIDGE: usize = SAMPLE_RATE;
/// Shorter fragments are mostly coughs and "mhm"s – not worth a paragraph.
const MIN_PIECE: usize = SAMPLE_RATE * 6 / 10;
const MAX_PIECE: usize = chunker::MAX_SECS * SAMPLE_RATE;

pub fn pieces_without_speakers(pcm: &[f32]) -> Vec<Piece> {
    chunker::split(pcm)
        .into_iter()
        .map(|c| Piece { start: c.start, end: c.end, speaker: None })
        .collect()
}

/// Turns may overlap and arrive in any order; the result never overlaps,
/// is sorted, and numbers speakers 0, 1, … by first appearance.
pub fn pieces_from_turns(turns: &[Turn], pcm: &[f32]) -> Vec<Piece> {
    let mut sorted = turns.to_vec();
    sorted.sort_by_key(|t| (t.start, t.end));

    let mut merged: Vec<Turn> = Vec::new();
    for mut turn in sorted {
        turn.end = turn.end.min(pcm.len());
        if let Some(prev) = merged.last() {
            turn.start = turn.start.max(prev.end);
        }
        if turn.end <= turn.start {
            continue;
        }
        match merged.last_mut() {
            Some(prev) if prev.speaker == turn.speaker && turn.start - prev.end < BRIDGE => prev.end = turn.end,
            _ => merged.push(turn),
        }
    }
    merged.retain(|t| t.end - t.start >= MIN_PIECE);

    let mut order: Vec<usize> = Vec::new();
    let mut pieces = Vec::new();
    for turn in merged {
        let speaker = match order.iter().position(|&s| s == turn.speaker) {
            Some(i) => i,
            None => {
                order.push(turn.speaker);
                order.len() - 1
            }
        };
        if turn.end - turn.start <= MAX_PIECE {
            pieces.push(Piece { start: turn.start, end: turn.end, speaker: Some(speaker) });
            continue;
        }
        for chunk in chunker::split(&pcm[turn.start..turn.end]) {
            pieces.push(Piece { start: turn.start + chunk.start, end: turn.start + chunk.end, speaker: Some(speaker) });
        }
    }
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    const S: usize = SAMPLE_RATE;

    fn turn(start_s: f32, end_s: f32, speaker: usize) -> Turn {
        Turn { start: (start_s * S as f32) as usize, end: (end_s * S as f32) as usize, speaker }
    }

    fn secs(p: &Piece) -> (f32, f32, Option<usize>) {
        (p.start as f32 / S as f32, p.end as f32 / S as f32, p.speaker)
    }

    fn tone(secs: f32) -> Vec<f32> {
        (0..(secs * S as f32) as usize).map(|i| 0.3 * (i as f32 * 0.07).sin()).collect()
    }

    #[test]
    fn without_speakers_pieces_are_the_chunks() {
        let pcm = tone(70.0);
        let pieces = pieces_without_speakers(&pcm);
        assert!(pieces.len() >= 3);
        assert!(pieces.iter().all(|p| p.speaker.is_none()));
        assert_eq!((pieces[0].start, pieces.last().unwrap().end), (0, pcm.len()));
    }

    #[test]
    fn turns_become_pieces_in_order() {
        let pcm = tone(20.0);
        let pieces = pieces_from_turns(&[turn(6.0, 12.0, 1), turn(0.5, 5.0, 0)], &pcm);
        let got: Vec<_> = pieces.iter().map(secs).collect();
        assert_eq!(got, vec![(0.5, 5.0, Some(0)), (6.0, 12.0, Some(1))]);
    }

    #[test]
    fn overlaps_are_cut_so_nothing_is_transcribed_twice() {
        let pcm = tone(20.0);
        let pieces = pieces_from_turns(&[turn(0.0, 6.0, 0), turn(5.0, 10.0, 1)], &pcm);
        let got: Vec<_> = pieces.iter().map(secs).collect();
        assert_eq!(got, vec![(0.0, 6.0, Some(0)), (6.0, 10.0, Some(1))]);
    }

    #[test]
    fn short_pauses_of_the_same_speaker_are_bridged() {
        let pcm = tone(30.0);
        let pieces = pieces_from_turns(&[turn(0.0, 4.0, 0), turn(4.6, 9.0, 0), turn(11.0, 15.0, 0)], &pcm);
        let got: Vec<_> = pieces.iter().map(secs).collect();
        assert_eq!(got, vec![(0.0, 9.0, Some(0)), (11.0, 15.0, Some(0))]);
    }

    #[test]
    fn tiny_fragments_are_dropped() {
        let pcm = tone(20.0);
        let pieces = pieces_from_turns(&[turn(0.0, 5.0, 0), turn(6.0, 6.4, 1), turn(8.0, 12.0, 0)], &pcm);
        assert!(pieces.iter().all(|p| p.end - p.start >= S / 2), "{pieces:?}");
        assert_eq!(pieces.len(), 2);
    }

    #[test]
    fn long_contributions_are_split_in_pauses() {
        let pcm = tone(80.0);
        let pieces = pieces_from_turns(&[turn(2.0, 75.0, 0)], &pcm);
        assert!(pieces.len() >= 3, "{pieces:?}");
        assert!(pieces.iter().all(|p| p.end - p.start <= 31 * S));
        assert_eq!((pieces[0].start, pieces.last().unwrap().end), (2 * S, 75 * S));
        for pair in pieces.windows(2) {
            assert_eq!(pair[0].end, pair[1].start);
        }
    }

    #[test]
    fn speakers_are_numbered_by_first_appearance() {
        let pcm = tone(30.0);
        let pieces = pieces_from_turns(&[turn(0.0, 5.0, 7), turn(6.0, 10.0, 2), turn(11.0, 15.0, 7)], &pcm);
        let speakers: Vec<_> = pieces.iter().map(|p| p.speaker).collect();
        assert_eq!(speakers, vec![Some(0), Some(1), Some(0)]);
    }

    #[test]
    fn turns_beyond_the_audio_are_clamped() {
        let pcm = tone(5.0);
        let pieces = pieces_from_turns(&[turn(1.0, 9.0, 0)], &pcm);
        assert_eq!(pieces.last().unwrap().end, pcm.len());
    }
}
