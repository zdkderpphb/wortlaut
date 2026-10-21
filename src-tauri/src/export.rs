//! Turns a transcript into the files people actually want: txt, md, docx.

use std::path::Path;

use docx_rs::{Docx, LineSpacing, Paragraph, Run, RunFonts, Style, StyleType};

use crate::transcript::Transcript;

pub fn format_timestamp(ms: u64) -> String {
    let secs = ms / 1000;
    format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
}

pub fn to_txt(t: &Transcript, timestamps: bool) -> String {
    paragraphs(t, |start_ms, text| {
        if timestamps {
            format!("[{}] {text}", format_timestamp(start_ms))
        } else {
            text.to_owned()
        }
    })
}

pub fn to_markdown(t: &Transcript, timestamps: bool) -> String {
    let body = paragraphs(t, |start_ms, text| {
        if timestamps {
            format!("**[{}]** {text}", format_timestamp(start_ms))
        } else {
            text.to_owned()
        }
    });
    format!("# {}\n\n{body}", t.title)
}

pub fn write_docx(t: &Transcript, timestamps: bool, path: &Path) -> anyhow::Result<()> {
    let title_style = Style::new("Title", StyleType::Paragraph)
        .name("Title")
        .size(40)
        .bold()
        .line_spacing(LineSpacing::new().after(240));

    let mut doc = Docx::new()
        .default_fonts(RunFonts::new().ascii("Calibri").hi_ansi("Calibri").cs("Calibri"))
        .default_size(24)
        .add_style(title_style)
        .add_paragraph(Paragraph::new().style("Title").add_run(Run::new().add_text(&t.title)));

    for (segment, text) in t.paragraphs() {
        let mut p = Paragraph::new().line_spacing(LineSpacing::new().after(160));
        if timestamps {
            let stamp = format!("[{}] ", format_timestamp(segment.start_ms));
            p = p.add_run(Run::new().add_text(stamp).color("7F7F7F"));
        }
        doc = doc.add_paragraph(p.add_run(Run::new().add_text(text)));
    }

    doc.build().pack(std::fs::File::create(path)?)?;
    Ok(())
}

fn paragraphs(t: &Transcript, render: impl Fn(u64, &str) -> String) -> String {
    let mut out = t
        .paragraphs()
        .map(|(segment, text)| render(segment.start_ms, text))
        .collect::<Vec<_>>()
        .join("\n\n");
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::Segment;
    use std::io::Read;

    fn sample() -> Transcript {
        Transcript {
            title: "Interview Müller & Co".into(),
            language: Some("de".into()),
            segments: vec![
                Segment { start_ms: 0, end_ms: 28_000, text: " Hallo, schön dass Sie da sind. ".into() },
                Segment { start_ms: 28_000, end_ms: 31_000, text: "   ".into() },
                Segment { start_ms: 31_000, end_ms: 3_723_000, text: "Grüße aus Wien & Graz.".into() },
            ],
        }
    }

    #[test]
    fn formats_timestamps_as_hh_mm_ss() {
        assert_eq!(format_timestamp(0), "00:00:00");
        assert_eq!(format_timestamp(59_999), "00:00:59");
        assert_eq!(format_timestamp(3_723_000), "01:02:03");
    }

    #[test]
    fn txt_joins_trimmed_paragraphs_and_skips_empty_ones() {
        assert_eq!(
            to_txt(&sample(), false),
            "Hallo, schön dass Sie da sind.\n\nGrüße aus Wien & Graz.\n"
        );
    }

    #[test]
    fn txt_with_timestamps_prefixes_each_paragraph() {
        assert_eq!(
            to_txt(&sample(), true),
            "[00:00:00] Hallo, schön dass Sie da sind.\n\n[00:00:31] Grüße aus Wien & Graz.\n"
        );
    }

    #[test]
    fn markdown_has_title_heading_and_bold_timestamps() {
        assert_eq!(
            to_markdown(&sample(), true),
            "# Interview Müller & Co\n\n**[00:00:00]** Hallo, schön dass Sie da sind.\n\n**[00:00:31]** Grüße aus Wien & Graz.\n"
        );
        assert!(to_markdown(&sample(), false).contains("\n\nHallo, schön"));
    }

    #[test]
    fn docx_is_a_word_file_containing_title_and_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("out.docx");
        write_docx(&sample(), true, &path).unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
        let mut xml = String::new();
        archive.by_name("word/document.xml").unwrap().read_to_string(&mut xml).unwrap();
        assert!(xml.contains("Interview Müller &amp; Co"), "{xml}");
        assert!(xml.contains("Hallo, schön dass Sie da sind."));
        assert!(xml.contains("Grüße aus Wien &amp; Graz."));
        assert!(xml.contains("[00:00:31]"));
    }
}
