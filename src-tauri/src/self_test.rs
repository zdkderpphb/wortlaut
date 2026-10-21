//! `wortlaut --self-test <report-file>`: checks an installed copy without a
//! window or a model. CI runs it after installing the real installer, so a
//! missing DLL or a broken backend shows up before users see it.

use std::fmt::Write as _;
use std::path::Path;

use crate::transcript::{Segment, Transcript};

/// Runs every check, writes a report to `report_path`, returns the exit code.
pub fn run(report_path: &Path) -> i32 {
    let mut report = format!("Wortlaut {} self-test\n", env!("CARGO_PKG_VERSION"));
    let checks: [(&str, fn() -> Result<String, String>); 3] = [
        ("compute devices", check_devices),
        ("audio decoding", check_audio),
        ("word export", check_docx),
    ];
    let mut failed = false;
    for (name, check) in checks {
        match check() {
            Ok(detail) => writeln!(report, "OK   {name}: {detail}"),
            Err(err) => {
                failed = true;
                writeln!(report, "FAIL {name}: {err}")
            }
        }
        .expect("writing to a String cannot fail");
    }
    let _ = std::fs::write(report_path, &report);
    print!("{report}");
    i32::from(failed)
}

fn check_devices() -> Result<String, String> {
    let devices = crate::engine::compute_devices().map_err(|e| e.to_string())?;
    if devices.is_empty() {
        return Err("no compute device registered".into());
    }
    Ok(devices.join(", "))
}

fn check_audio() -> Result<String, String> {
    let dir = tempdir()?;
    let path = dir.join("tone.wav");
    std::fs::write(&path, sine_wav(44_100, 1.0)).map_err(|e| e.to_string())?;
    let pcm = crate::audio::decode_to_mono_16k(&path).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&dir);
    if !(15_000..=17_000).contains(&pcm.len()) {
        return Err(format!("expected ~16000 samples, got {}", pcm.len()));
    }
    Ok(format!("{} samples at 16 kHz", pcm.len()))
}

fn check_docx() -> Result<String, String> {
    let dir = tempdir()?;
    let path = dir.join("test.docx");
    let transcript = Transcript {
        title: "Selbsttest".into(),
        language: Some("de".into()),
        segments: vec![Segment { start_ms: 0, end_ms: 1000, text: "Grüß Gott!".into(), speaker: None }],
        ..Default::default()
    };
    let labels = crate::export::ExportLabels::default();
    crate::export::write(&transcript, crate::export::Format::Docx, true, &labels, &path).map_err(|e| e.to_string())?;
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    let _ = std::fs::remove_dir_all(&dir);
    Ok(format!("{size} bytes"))
}

fn tempdir() -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir().join(format!("wortlaut-self-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// A mono 16-bit PCM WAV with a 440 Hz tone.
fn sine_wav(rate: u32, secs: f32) -> Vec<u8> {
    let samples: Vec<i16> = (0..(rate as f32 * secs) as u32)
        .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / rate as f32).sin() * 12_000.0) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&rate.to_le_bytes());
    wav.extend_from_slice(&(rate * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    #[test]
    fn self_test_passes_on_this_machine() {
        let dir = tempfile::tempdir().unwrap();
        let report = dir.path().join("report.txt");
        assert_eq!(super::run(&report), 0, "{}", std::fs::read_to_string(&report).unwrap());
    }
}
