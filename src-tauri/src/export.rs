//! Turns a transcript into the files people actually want: Word, text,
//! Markdown, subtitles and the formats of qualitative analysis software.

use std::path::Path;

use docx_rs::{Docx, LineSpacing, Paragraph, Run, RunFonts, Style, StyleType};
use serde::{Deserialize, Serialize};

use crate::transcript::{Segment, Transcript};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Docx,
    Txt,
    Md,
    Srt,
    Maxqda,
    Atlasti,
    Nvivo,
    Json,
}

impl Format {
    pub const ALL: [Format; 8] = [
        Format::Docx,
        Format::Txt,
        Format::Md,
        Format::Srt,
        Format::Maxqda,
        Format::Atlasti,
        Format::Nvivo,
        Format::Json,
    ];

    pub fn extension(self) -> &'static str {
        match self {
            Format::Docx | Format::Atlasti => "docx",
            Format::Txt | Format::Maxqda | Format::Nvivo => "txt",
            Format::Md => "md",
            Format::Srt => "srt",
            Format::Json => "json",
        }
    }

    /// Added to the file name so several exports can share a folder.
    pub fn file_suffix(self) -> &'static str {
        match self {
            Format::Maxqda => " (MAXQDA)",
            Format::Atlasti => " (ATLAS.ti)",
            Format::Nvivo => " (NVivo)",
            _ => "",
        }
    }
}

/// Section headings, in the language of the interface.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExportLabels {
    pub summary: String,
    pub transcript: String,
}

impl Default for ExportLabels {
    fn default() -> Self {
        ExportLabels { summary: "Zusammenfassung".into(), transcript: "Transkript".into() }
    }
}

pub fn format_timestamp(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

pub fn write(t: &Transcript, format: Format, timestamps: bool, labels: &ExportLabels, path: &Path) -> anyhow::Result<()> {
    let bytes = match format {
        Format::Docx => to_docx(t, timestamps, labels)?,
        Format::Atlasti => to_atlasti_docx(t)?,
        Format::Txt => to_txt(t, timestamps, labels).into_bytes(),
        Format::Md => to_markdown(t, timestamps, labels).into_bytes(),
        Format::Srt => to_srt(t).into_bytes(),
        Format::Maxqda => to_maxqda(t).into_bytes(),
        Format::Nvivo => to_nvivo(t).into_bytes(),
        Format::Json => to_json(t).into_bytes(),
    };
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn to_txt(t: &Transcript, timestamps: bool, labels: &ExportLabels) -> String {
    let body = paragraphs(t, |segment, text| {
        let stamp = if timestamps { format!("[{}] ", format_timestamp(segment.start_ms)) } else { String::new() };
        format!("{stamp}{}{text}", speaker_prefix(t, segment))
    });
    match t.summary() {
        Some(summary) => format!("{}\n\n{summary}\n\n{}\n\n{body}", labels.summary, labels.transcript),
        None => body,
    }
}

pub fn to_markdown(t: &Transcript, timestamps: bool, labels: &ExportLabels) -> String {
    let body = paragraphs(t, |segment, text| {
        let stamp = timestamps.then(|| format!("[{}]", format_timestamp(segment.start_ms)));
        let speaker = t.speaker_name(segment).map(|name| format!("{name}:"));
        let label = [stamp, speaker].into_iter().flatten().collect::<Vec<_>>().join(" ");
        if label.is_empty() { text.to_owned() } else { format!("**{label}** {text}") }
    });
    let mut out = format!("# {}\n\n", t.title);
    if let Some(summary) = t.summary() {
        out += &format!("## {}\n\n{summary}\n\n## {}\n\n", labels.summary, labels.transcript);
    }
    out + &body
}

/// Subtitles: long paragraphs become several cues of at most
/// `SRT_CUE_CHARS`, timed in proportion to their length.
pub fn to_srt(t: &Transcript) -> String {
    let mut out = String::new();
    let mut index = 1;
    for (segment, text) in t.paragraphs() {
        let cues = srt_cues(text);
        let total: usize = cues.iter().map(|c| c.chars().count()).sum::<usize>().max(1);
        let duration = segment.end_ms.saturating_sub(segment.start_ms);
        let mut before = 0;
        for (i, cue) in cues.iter().enumerate() {
            let len = cue.chars().count();
            let start = segment.start_ms + duration * before as u64 / total as u64;
            let end = segment.start_ms + duration * (before + len) as u64 / total as u64;
            let prefix = if i == 0 { speaker_prefix(t, segment) } else { String::new() };
            out += &format!("{index}\n{} --> {}\n{prefix}{cue}\n\n", srt_time(start), srt_time(end));
            before += len;
            index += 1;
        }
    }
    out
}

/// MAXQDA: speaker first ("Name: "), timestamp at the end of the paragraph.
pub fn to_maxqda(t: &Transcript) -> String {
    paragraphs(t, |segment, text| {
        format!("{}{text} [{}]", speaker_prefix(t, segment), format_timestamp(segment.end_ms))
    })
}

/// NVivo: tab-separated rows – timespan, speaker (if any), content.
pub fn to_nvivo(t: &Transcript) -> String {
    let has_speakers = t.segments.iter().any(|s| s.speaker.is_some());
    let mut out = String::new();
    for (segment, text) in t.paragraphs() {
        let content = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let span = format!("{}-{}", nvivo_time(segment.start_ms), nvivo_time(segment.end_ms));
        if has_speakers {
            out += &format!("{span}\t{}\t{content}\n", t.speaker_name(segment).unwrap_or_default());
        } else {
            out += &format!("{span}\t{content}\n");
        }
    }
    out
}

pub fn to_json(t: &Transcript) -> String {
    serde_json::to_string_pretty(t).expect("a transcript always serializes")
}

pub fn to_docx(t: &Transcript, timestamps: bool, labels: &ExportLabels) -> anyhow::Result<Vec<u8>> {
    let mut doc = base_docx()
        .add_paragraph(Paragraph::new().style("Title").add_run(Run::new().add_text(&t.title)));

    if let Some(summary) = t.summary() {
        doc = doc.add_paragraph(Paragraph::new().style("Heading1").add_run(Run::new().add_text(&labels.summary)));
        for line in summary.lines().map(str::trim).filter(|l| !l.is_empty()) {
            doc = doc.add_paragraph(body_paragraph().add_run(Run::new().add_text(line)));
        }
        doc = doc.add_paragraph(Paragraph::new().style("Heading1").add_run(Run::new().add_text(&labels.transcript)));
    }

    for (segment, text) in t.paragraphs() {
        let mut p = body_paragraph();
        if timestamps {
            let stamp = format!("[{}] ", format_timestamp(segment.start_ms));
            p = p.add_run(Run::new().add_text(stamp).color("7F7F7F"));
        }
        if let Some(name) = t.speaker_name(segment) {
            p = p.add_run(Run::new().add_text(format!("{name}: ")).bold());
        }
        doc = doc.add_paragraph(p.add_run(Run::new().add_text(text)));
    }
    pack(doc)
}

/// ATLAS.ti: plain paragraphs starting with "[hh:mm:ss]", no title or summary,
/// so the timestamps can be synchronised with the recording.
pub fn to_atlasti_docx(t: &Transcript) -> anyhow::Result<Vec<u8>> {
    let mut doc = base_docx();
    for (segment, text) in t.paragraphs() {
        let line = format!("[{}] {}{text}", format_timestamp(segment.start_ms), speaker_prefix(t, segment));
        doc = doc.add_paragraph(body_paragraph().add_run(Run::new().add_text(line)));
    }
    pack(doc)
}

const SRT_CUE_CHARS: usize = 90;

fn speaker_prefix(t: &Transcript, segment: &Segment) -> String {
    t.speaker_name(segment).map(|name| format!("{name}: ")).unwrap_or_default()
}

fn paragraphs(t: &Transcript, render: impl Fn(&Segment, &str) -> String) -> String {
    let mut out = t.paragraphs().map(|(segment, text)| render(segment, text)).collect::<Vec<_>>().join("\n\n");
    out.push('\n');
    out
}

/// Sentences, packed into cues of at most `SRT_CUE_CHARS` (overlong
/// sentences are broken between words).
fn srt_cues(text: &str) -> Vec<String> {
    let mut pieces: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let fits = current.chars().count() + 1 + word.chars().count() <= SRT_CUE_CHARS;
        if !current.is_empty() && !fits {
            pieces.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
        if word.ends_with(['.', '!', '?', '…']) {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }

    // Join short sentences while they still fit on one cue.
    let mut cues: Vec<String> = Vec::new();
    for piece in pieces {
        match cues.last_mut() {
            Some(last) if last.chars().count() + 1 + piece.chars().count() <= SRT_CUE_CHARS => {
                last.push(' ');
                last.push_str(&piece);
            }
            _ => cues.push(piece),
        }
    }
    cues
}

fn srt_time(ms: u64) -> String {
    format!("{},{:03}", format_timestamp(ms), ms % 1000)
}

fn nvivo_time(ms: u64) -> String {
    format!("{}.{}", format_timestamp(ms), ms % 1000 / 100)
}

fn base_docx() -> Docx {
    let title = Style::new("Title", StyleType::Paragraph)
        .name("Title")
        .size(40)
        .bold()
        .line_spacing(LineSpacing::new().after(240));
    let heading = Style::new("Heading1", StyleType::Paragraph)
        .name("heading 1")
        .size(28)
        .bold()
        .line_spacing(LineSpacing::new().before(240).after(120));
    Docx::new()
        .default_fonts(RunFonts::new().ascii("Calibri").hi_ansi("Calibri").cs("Calibri"))
        .default_size(24)
        .add_style(title)
        .add_style(heading)
}

fn body_paragraph() -> Paragraph {
    Paragraph::new().line_spacing(LineSpacing::new().after(160))
}

fn pack(doc: Docx) -> anyhow::Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    doc.build().pack(&mut cursor)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    fn seg(start_ms: u64, end_ms: u64, text: &str, speaker: Option<usize>) -> Segment {
        Segment { start_ms, end_ms, text: text.into(), speaker }
    }

    /// No speakers, no summary – what 0.1 produced.
    fn plain() -> Transcript {
        Transcript {
            title: "Interview Müller & Co".into(),
            language: Some("de".into()),
            segments: vec![
                seg(0, 28_000, " Hallo, schön dass Sie da sind. ", None),
                seg(28_000, 31_000, "   ", None),
                seg(31_000, 3_723_000, "Grüße aus Wien & Graz.", None),
            ],
            ..Default::default()
        }
    }

    fn with_speakers() -> Transcript {
        Transcript {
            title: "Runde".into(),
            speakers: vec!["Anna".into(), "Flo".into()],
            summary: Some("Kurz gesagt: alles gut.\n- Punkt eins".into()),
            segments: vec![
                seg(0, 6_000, "Guten Morgen.", Some(0)),
                seg(6_500, 13_000, "Danke für die Einladung.", Some(1)),
                seg(13_000, 16_500, "Was war schwierig?", Some(0)),
            ],
            ..Default::default()
        }
    }

    fn docx_xml(bytes: &[u8]) -> String {
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut xml = String::new();
        archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        xml
    }

    #[test]
    fn formats_timestamps_as_hh_mm_ss() {
        assert_eq!(format_timestamp(0), "00:00:00");
        assert_eq!(format_timestamp(59_999), "00:00:59");
        assert_eq!(format_timestamp(3_723_000), "01:02:03");
    }

    #[test]
    fn extensions_and_suffixes_keep_files_apart_in_one_folder() {
        let names: Vec<String> =
            Format::ALL.iter().map(|f| format!("x{}.{}", f.file_suffix(), f.extension())).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "{names:?}");
        assert_eq!(Format::Maxqda.extension(), "txt");
        assert_eq!(Format::Atlasti.extension(), "docx");
    }

    #[test]
    fn txt_joins_trimmed_paragraphs_and_skips_empty_ones() {
        let labels = ExportLabels::default();
        assert_eq!(to_txt(&plain(), false, &labels), "Hallo, schön dass Sie da sind.\n\nGrüße aus Wien & Graz.\n");
        assert_eq!(
            to_txt(&plain(), true, &labels),
            "[00:00:00] Hallo, schön dass Sie da sind.\n\n[00:00:31] Grüße aus Wien & Graz.\n"
        );
    }

    #[test]
    fn txt_names_speakers_and_puts_the_summary_first() {
        let txt = to_txt(&with_speakers(), true, &ExportLabels::default());
        assert!(txt.starts_with("Zusammenfassung\n\nKurz gesagt: alles gut.\n- Punkt eins\n\nTranskript\n\n"), "{txt}");
        assert!(txt.contains("[00:00:06] Flo: Danke für die Einladung.\n"), "{txt}");
    }

    #[test]
    fn markdown_has_title_summary_and_bold_labels() {
        let labels = ExportLabels::default();
        assert_eq!(
            to_markdown(&plain(), true, &labels),
            "# Interview Müller & Co\n\n**[00:00:00]** Hallo, schön dass Sie da sind.\n\n**[00:00:31]** Grüße aus Wien & Graz.\n"
        );
        let md = to_markdown(&with_speakers(), false, &labels);
        assert!(md.contains("## Zusammenfassung\n\nKurz gesagt: alles gut.\n- Punkt eins\n\n## Transkript\n\n"), "{md}");
        assert!(md.contains("**Anna:** Guten Morgen.\n\n**Flo:** Danke"), "{md}");
        assert!(to_markdown(&with_speakers(), true, &labels).contains("**[00:00:13] Anna:** Was war schwierig?"));
    }

    #[test]
    fn docx_contains_title_summary_speakers_and_escaped_text() {
        let xml = docx_xml(&to_docx(&plain(), true, &ExportLabels::default()).unwrap());
        assert!(xml.contains("Interview Müller &amp; Co"), "{xml}");
        assert!(xml.contains("Grüße aus Wien &amp; Graz."));
        assert!(xml.contains("[00:00:31]"));

        let xml = docx_xml(&to_docx(&with_speakers(), false, &ExportLabels::default()).unwrap());
        assert!(xml.contains("Zusammenfassung") && xml.contains("Kurz gesagt: alles gut."));
        assert!(xml.contains("Flo: ") && xml.contains("Danke für die Einladung."));
    }

    #[test]
    fn srt_numbers_cues_and_uses_comma_milliseconds() {
        let srt = to_srt(&with_speakers());
        assert!(
            srt.starts_with("1\n00:00:00,000 --> 00:00:06,000\nAnna: Guten Morgen.\n\n2\n00:00:06,500 --> 00:00:13,000\nFlo: Danke für die Einladung.\n\n"),
            "{srt}"
        );
    }

    #[test]
    fn srt_splits_long_paragraphs_into_readable_cues() {
        let long = "Das ist der erste Satz mit einigen Wörtern darin. Hier kommt ein zweiter Satz, der ebenfalls nicht ganz kurz ist. Und noch ein dritter Satz zum Abschluss.";
        let t = Transcript { segments: vec![seg(10_000, 40_000, long, None)], ..Default::default() };
        let srt = to_srt(&t);
        let cues: Vec<&str> = srt.trim_end().split("\n\n").collect();
        assert!(cues.len() >= 2, "{srt}");
        for cue in &cues {
            let text = cue.lines().skip(2).collect::<Vec<_>>().join(" ");
            assert!(text.chars().count() <= 90, "too long: {text}");
        }
        assert!(cues[0].lines().nth(1).unwrap().starts_with("00:00:10,000 --> "));
        assert!(cues.last().unwrap().lines().nth(1).unwrap().ends_with(" --> 00:00:40,000"));
    }

    #[test]
    fn maxqda_puts_speaker_first_and_end_time_last() {
        let txt = to_maxqda(&with_speakers());
        assert_eq!(
            txt,
            "Anna: Guten Morgen. [00:00:06]\n\nFlo: Danke für die Einladung. [00:00:13]\n\nAnna: Was war schwierig? [00:00:16]\n"
        );
    }

    #[test]
    fn nvivo_is_tab_separated_with_equal_columns() {
        let txt = to_nvivo(&with_speakers());
        assert_eq!(txt.lines().next().unwrap(), "00:00:00.0-00:00:06.0\tAnna\tGuten Morgen.");
        let tabs: Vec<usize> = txt.lines().map(|l| l.matches('\t').count()).collect();
        assert!(tabs.iter().all(|&n| n == 2), "{txt}");

        let t = Transcript { segments: vec![seg(0, 1000, "eins\tzwei\ndrei", None)], ..Default::default() };
        assert_eq!(to_nvivo(&t), "00:00:00.0-00:00:01.0\teins zwei drei\n");
    }

    #[test]
    fn atlasti_docx_starts_each_paragraph_with_its_time() {
        let xml = docx_xml(&to_atlasti_docx(&with_speakers()).unwrap());
        assert!(xml.contains("[00:00:06] Flo: Danke für die Einladung."), "{xml}");
        assert!(!xml.contains("Zusammenfassung"));
    }

    #[test]
    fn json_round_trips() {
        let t = with_speakers();
        let back: Transcript = serde_json::from_str(&to_json(&t)).unwrap();
        assert_eq!(back, t);
    }

    #[test]
    fn transcripts_saved_by_version_0_1_still_load() {
        let old = r#"{"title":"Alt","language":"de","segments":[{"startMs":0,"endMs":1000,"text":"Hallo"}]}"#;
        let t: Transcript = serde_json::from_str(old).unwrap();
        assert_eq!(t.segments[0].speaker, None);
        assert!(t.speakers.is_empty() && t.summary.is_none());
    }

    #[test]
    fn write_creates_every_format() {
        let dir = tempfile::tempdir().unwrap();
        for format in Format::ALL {
            let path = dir.path().join(format!("t{}.{}", format.file_suffix(), format.extension()));
            write(&with_speakers(), format, true, &ExportLabels::default(), &path).unwrap();
            assert!(std::fs::metadata(&path).unwrap().len() > 0, "{format:?}");
        }
    }
}
