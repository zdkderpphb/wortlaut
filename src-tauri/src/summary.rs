//! A short summary of a transcript, written by Voxtral. Long transcripts are
//! summarized block by block, then the block summaries once more.

use crate::engine::{Engine, EngineError};
use crate::transcript::Transcript;

/// Comfortably inside the instruct context (~2 000 tokens of text).
pub const BLOCK_WORDS: usize = 1200;

/// Paragraph texts packed into blocks of at most `BLOCK_WORDS` words,
/// split at paragraph boundaries (a giant paragraph is split by words).
pub fn blocks(paragraphs: &[String]) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut words = 0;
    for paragraph in paragraphs {
        let all: Vec<&str> = paragraph.split_whitespace().collect();
        for part in all.chunks(BLOCK_WORDS) {
            if words + part.len() > BLOCK_WORDS && !current.is_empty() {
                blocks.push(current.join("\n\n"));
                current.clear();
                words = 0;
            }
            words += part.len();
            current.push(part.join(" "));
        }
    }
    if !current.is_empty() {
        blocks.push(current.join("\n\n"));
    }
    blocks
}

pub fn summarize(
    engine: &mut Engine,
    transcript: &Transcript,
    locale: &str,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<String, EngineError> {
    let paragraphs: Vec<String> = transcript
        .paragraphs()
        .map(|(segment, text)| match transcript.speaker_name(segment) {
            Some(name) => format!("{name}: {text}"),
            None => text.to_owned(),
        })
        .collect();
    let prompts = Prompts::for_locale(locale);
    let blocks = blocks(&paragraphs);
    let steps = if blocks.len() > 1 { blocks.len() + 1 } else { 1 };

    let summary = if blocks.len() <= 1 {
        on_progress(0, steps);
        engine.instruct(&prompts.whole(blocks.first().map_or("", String::as_str)))?
    } else {
        let mut partial = Vec::new();
        for (i, block) in blocks.iter().enumerate() {
            on_progress(i, steps);
            partial.push(engine.instruct(&prompts.block(block))?);
        }
        on_progress(blocks.len(), steps);
        engine.instruct(&prompts.combine(&partial.join("\n\n")))?
    };
    on_progress(steps, steps);
    Ok(clean(&summary))
}

/// The model likes Markdown bold; exports show plain text.
fn clean(text: &str) -> String {
    text.replace("**", "").trim().to_owned()
}

struct Prompts {
    german: bool,
}

impl Prompts {
    fn for_locale(locale: &str) -> Prompts {
        Prompts { german: locale.starts_with("de") }
    }

    fn whole(&self, text: &str) -> String {
        if self.german {
            format!(
                "Hier ist das Transkript eines Gesprächs:\n\n{text}\n\nFasse das Gespräch in drei bis vier Sätzen auf Deutsch zusammen. Nenne danach die wichtigsten Punkte als kurze Stichpunkte, jeweils beginnend mit „- “."
            )
        } else {
            format!(
                "Here is the transcript of a conversation:\n\n{text}\n\nSummarize the conversation in three to four sentences in English. Then list the key points as short bullet points, each starting with \"- \"."
            )
        }
    }

    fn block(&self, text: &str) -> String {
        if self.german {
            format!("Hier ist ein Abschnitt aus dem Transkript eines Gesprächs:\n\n{text}\n\nFasse diesen Abschnitt in drei bis fünf Sätzen auf Deutsch zusammen.")
        } else {
            format!("Here is a section of a conversation transcript:\n\n{text}\n\nSummarize this section in three to five sentences in English.")
        }
    }

    fn combine(&self, partials: &str) -> String {
        if self.german {
            format!(
                "Hier sind Zusammenfassungen der aufeinanderfolgenden Abschnitte eines Gesprächs:\n\n{partials}\n\nFasse das gesamte Gespräch in drei bis vier Sätzen auf Deutsch zusammen. Nenne danach die wichtigsten Punkte als kurze Stichpunkte, jeweils beginnend mit „- “."
            )
        } else {
            format!(
                "Here are summaries of consecutive sections of a conversation:\n\n{partials}\n\nSummarize the whole conversation in three to four sentences in English. Then list the key points as short bullet points, each starting with \"- \"."
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(n: usize) -> String {
        vec!["wort"; n].join(" ")
    }

    fn count(block: &str) -> usize {
        block.split_whitespace().count()
    }

    #[test]
    fn short_transcripts_are_one_block() {
        let b = blocks(&[words(10), words(20)]);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0], format!("{}\n\n{}", words(10), words(20)));
    }

    #[test]
    fn blocks_stay_under_the_limit_and_break_between_paragraphs() {
        let paragraphs: Vec<String> = (0..10).map(|_| words(300)).collect();
        let b = blocks(&paragraphs);
        assert!(b.len() >= 3);
        assert!(b.iter().all(|block| count(block) <= BLOCK_WORDS));
        assert_eq!(b.iter().map(|block| count(block)).sum::<usize>(), 3000);
    }

    #[test]
    fn a_giant_paragraph_is_split_by_words() {
        let b = blocks(&[words(3000)]);
        assert_eq!(b.iter().map(|block| count(block)).collect::<Vec<_>>(), vec![1200, 1200, 600]);
    }

    #[test]
    fn markdown_bold_is_removed() {
        assert_eq!(clean("**Wichtige Punkte:**\n- eins "), "Wichtige Punkte:\n- eins");
    }

    /// Runs only with a real model: `WORTLAUT_MODEL=/path/to/model.gguf`.
    #[test]
    fn summarizes_a_conversation_with_the_real_model() {
        let Ok(model) = std::env::var("WORTLAUT_MODEL") else {
            eprintln!("skipped: set WORTLAUT_MODEL to run");
            return;
        };
        let mut engine = Engine::load(std::path::Path::new(&model)).unwrap();
        let segment = |text: &str, speaker| crate::transcript::Segment { start_ms: 0, end_ms: 1, text: text.into(), speaker: Some(speaker) };
        let transcript = Transcript {
            speakers: vec!["Anna".into(), "Flo".into()],
            segments: vec![
                segment("Wir planen das Sommerfest am 12. Juli im Stadtpark von Graz.", 0),
                segment("Gut, ich kümmere mich um die Getränke und die Musik.", 1),
                segment("Danke, dann übernehme ich die Einladungen.", 0),
            ],
            ..Default::default()
        };
        let mut steps = Vec::new();
        let summary = summarize(&mut engine, &transcript, "de", |done, total| steps.push((done, total))).unwrap();
        eprintln!("{summary}");
        assert!(summary.to_lowercase().contains("sommerfest"), "{summary}");
        assert_eq!(steps.last(), Some(&(1, 1)));
    }
}
