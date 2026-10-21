//! Reads any common audio or video file into 16 kHz mono samples.

use std::path::Path;
use std::sync::OnceLock;

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::registry::CodecRegistry;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia_adapter_libopus::OpusDecoder;

use crate::chunker::SAMPLE_RATE;

#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("unsupported format: {0}")]
    Unsupported(String),
    #[error("file has no audio track")]
    NoAudioTrack,
    #[error("could not read file: {0}")]
    Io(String),
}

pub fn decode_to_mono_16k(path: &Path) -> Result<Vec<f32>, AudioError> {
    let (mono, rate) = decode_mono(path)?;
    resample(&mono, rate)
}

/// Symphonia's codecs plus Opus (WhatsApp voice messages, WebM) via libopus.
fn codecs() -> &'static CodecRegistry {
    static REGISTRY: OnceLock<CodecRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry.register_audio_decoder::<OpusDecoder>();
        registry
    })
}

/// Decodes the first audio track and averages all channels into one.
fn decode_mono(path: &Path) -> Result<(Vec<f32>, u32), AudioError> {
    let file = std::fs::File::open(path).map_err(|e| AudioError::Io(e.to_string()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(&hint, mss, FormatOptions::default(), MetadataOptions::default())
        .map_err(|e| AudioError::Unsupported(e.to_string()))?;
    let track = format.default_track(TrackType::Audio).ok_or(AudioError::NoAudioTrack)?;
    let track_id = track.id;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or(AudioError::NoAudioTrack)?;
    let mut decoder = codecs()
        .make_audio_decoder(params, &AudioDecoderOptions::default())
        .map_err(|e| AudioError::Unsupported(e.to_string()))?;

    let mut mono = Vec::new();
    let mut interleaved: Vec<f32> = Vec::new();
    let mut rate = params.sample_rate.unwrap_or(0);

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) | Err(SymError::ResetRequired) => break,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(AudioError::Unsupported(e.to_string())),
        };
        if packet.track_id != track_id {
            continue;
        }
        let buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            // A damaged packet should not sink a whole recording.
            Err(SymError::DecodeError(_)) | Err(SymError::IoError(_)) => continue,
            Err(e) => return Err(AudioError::Unsupported(e.to_string())),
        };
        let channels = buf.spec().channels().count().max(1);
        rate = buf.spec().rate();
        interleaved.resize(buf.samples_interleaved(), 0.0);
        buf.copy_to_slice_interleaved(&mut interleaved);
        mono.extend(
            interleaved
                .chunks_exact(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32),
        );
    }

    if mono.is_empty() || rate == 0 {
        return Err(AudioError::NoAudioTrack);
    }
    Ok((mono, rate))
}

fn resample(mono: &[f32], rate: u32) -> Result<Vec<f32>, AudioError> {
    if rate as usize == SAMPLE_RATE {
        return Ok(mono.to_vec());
    }
    let resampling_error = |e: &dyn std::fmt::Display| AudioError::Unsupported(format!("resampling: {e}"));
    let mut resampler = Fft::<f32>::new(rate as usize, SAMPLE_RATE, 1024, 1, FixedSync::Input)
        .map_err(|e| resampling_error(&e))?;
    let input = InterleavedSlice::new(mono, 1, mono.len()).map_err(|e| resampling_error(&e))?;
    let output = resampler
        .process_all(&input, mono.len(), None)
        .map_err(|e| resampling_error(&e))?;
    Ok(output.take_data().into_iter().map(|s| s.clamp(-1.0, 1.0)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
    }

    #[test]
    fn decodes_every_supported_format_to_one_second_at_16k() {
        for name in [
            "tone.wav", "tone.mp3", "tone.m4a", "tone.flac", "tone.ogg", "tone.opus", "tone.webm",
            "tone.mp4", "tone.mov",
        ] {
            let pcm = decode_to_mono_16k(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
            let secs = pcm.len() as f32 / SAMPLE_RATE as f32;
            assert!((0.9..=1.1).contains(&secs), "{name}: {secs}s");
            let peak = pcm.iter().fold(0.0f32, |m, s| m.max(s.abs()));
            // ffmpeg's test tone peaks at about 0.09.
            assert!(peak > 0.05 && peak <= 1.0, "{name}: peak {peak}");
        }
    }

    #[test]
    fn rejects_files_that_are_not_media() {
        let err = decode_to_mono_16k(&fixture("not-audio.txt")).unwrap_err();
        assert!(matches!(err, AudioError::Unsupported(_)), "{err:?}");
    }

    #[test]
    fn reports_missing_files_as_io_errors() {
        let err = decode_to_mono_16k(&fixture("does-not-exist.mp3")).unwrap_err();
        assert!(matches!(err, AudioError::Io(_)), "{err:?}");
    }
}
