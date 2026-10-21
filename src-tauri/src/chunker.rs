//! Splits long recordings into 20–30 s pieces, cutting in speech pauses.
//!
//! Small pieces keep memory flat (8 GB machines), give a smooth progress bar
//! and let text appear while the rest is still being processed.

pub const SAMPLE_RATE: usize = 16_000;
pub const MIN_SECS: usize = 20;
pub const MAX_SECS: usize = 30;

/// Sample range `start..end` of one piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chunk {
    pub start: usize,
    pub end: usize,
}

impl Chunk {
    pub fn start_ms(&self) -> u64 {
        (self.start * 1000 / SAMPLE_RATE) as u64
    }

    pub fn end_ms(&self) -> u64 {
        (self.end * 1000 / SAMPLE_RATE) as u64
    }
}

/// Loudness is measured over 100 ms windows, moved in 50 ms steps.
const WINDOW: usize = SAMPLE_RATE / 10;
const HOP: usize = SAMPLE_RATE / 20;
/// A tail shorter than this is glued to the previous piece.
const MIN_TAIL: usize = SAMPLE_RATE;
/// ...as long as that piece stays at most this long.
const MAX_MERGED: usize = (MAX_SECS + 1) * SAMPLE_RATE;

pub fn split(pcm: &[f32]) -> Vec<Chunk> {
    let (min, max) = (MIN_SECS * SAMPLE_RATE, MAX_SECS * SAMPLE_RATE);
    let mut chunks = Vec::new();
    let mut start = 0;

    while pcm.len() - start > max {
        let cut = quietest_point(pcm, start + min, start + max);
        chunks.push(Chunk { start, end: cut });
        start = cut;
    }

    if start < pcm.len() {
        let tail = pcm.len() - start;
        match chunks.last_mut() {
            Some(prev) if tail < MIN_TAIL && pcm.len() - prev.start <= MAX_MERGED => {
                prev.end = pcm.len();
            }
            _ => chunks.push(Chunk { start, end: pcm.len() }),
        }
    }
    chunks
}

/// Centre of the quietest window within `from..to`. Near-ties (within 5 %)
/// go to the later window, which keeps pieces long when there is no pause.
fn quietest_point(pcm: &[f32], from: usize, to: usize) -> usize {
    let windows: Vec<(usize, f32)> = (from..=to - WINDOW)
        .step_by(HOP)
        .map(|pos| (pos, pcm[pos..pos + WINDOW].iter().map(|s| s * s).sum()))
        .collect();
    let quietest = windows.iter().map(|w| w.1).fold(f32::INFINITY, f32::min);
    let (pos, _) = windows
        .iter()
        .rev()
        .find(|w| w.1 <= quietest * 1.05 + 1e-6)
        .expect("search range holds at least one window");
    pos + WINDOW / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(secs: f32) -> Vec<f32> {
        let n = (secs * SAMPLE_RATE as f32) as usize;
        (0..n)
            .map(|i| 0.5 * (i as f32 * 220.0 * std::f32::consts::TAU / SAMPLE_RATE as f32).sin())
            .collect()
    }

    fn silence_at(pcm: &mut [f32], center_secs: f32, width_ms: usize) {
        let center = (center_secs * SAMPLE_RATE as f32) as usize;
        let half = width_ms * SAMPLE_RATE / 1000 / 2;
        pcm[center - half..center + half].fill(0.0);
    }

    fn assert_covers(chunks: &[Chunk], len: usize) {
        assert_eq!(chunks.first().unwrap().start, 0);
        assert_eq!(chunks.last().unwrap().end, len);
        for pair in chunks.windows(2) {
            assert_eq!(pair[0].end, pair[1].start, "gap or overlap: {chunks:?}");
        }
    }

    #[test]
    fn empty_input_has_no_chunks() {
        assert!(split(&[]).is_empty());
    }

    #[test]
    fn short_input_is_one_chunk() {
        let pcm = tone(5.0);
        assert_eq!(split(&pcm), vec![Chunk { start: 0, end: pcm.len() }]);
    }

    #[test]
    fn cuts_land_in_the_pauses() {
        let mut pcm = tone(75.0);
        silence_at(&mut pcm, 25.0, 400);
        silence_at(&mut pcm, 52.0, 400);
        let chunks = split(&pcm);
        assert_covers(&chunks, pcm.len());
        assert_eq!(chunks.len(), 3, "{chunks:?}");
        let cut1 = chunks[0].end as f32 / SAMPLE_RATE as f32;
        let cut2 = chunks[1].end as f32 / SAMPLE_RATE as f32;
        assert!((cut1 - 25.0).abs() < 0.3, "first cut at {cut1}");
        assert!((cut2 - 52.0).abs() < 0.3, "second cut at {cut2}");
    }

    #[test]
    fn continuous_sound_respects_min_and_max_length() {
        let pcm = tone(70.0);
        let chunks = split(&pcm);
        assert_covers(&chunks, pcm.len());
        for (i, c) in chunks.iter().enumerate() {
            let secs = (c.end - c.start) as f32 / SAMPLE_RATE as f32;
            assert!(secs <= MAX_SECS as f32 + 0.001, "chunk {i} is {secs}s");
            if i + 1 < chunks.len() {
                assert!(secs >= MIN_SECS as f32 - 0.001, "chunk {i} is {secs}s");
            }
        }
    }

    #[test]
    fn tiny_tail_is_merged_into_the_previous_chunk() {
        let pcm = tone(30.5);
        let chunks = split(&pcm);
        assert_eq!(chunks, vec![Chunk { start: 0, end: pcm.len() }]);
    }

    #[test]
    fn chunk_times_are_in_milliseconds() {
        let c = Chunk { start: 16_000, end: 48_000 };
        assert_eq!((c.start_ms(), c.end_ms()), (1_000, 3_000));
    }
}
