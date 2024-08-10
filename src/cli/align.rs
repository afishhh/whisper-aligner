use std::{fs::File, io::Write, mem::ManuallyDrop, ops::Range, path::PathBuf};

use clap::Parser;
#[cfg(feature = "vibrato")]
use vibrato::{tokenizer::worker::Worker, Dictionary};

use crate::whisper::Transcription;

#[derive(Parser)]
pub struct Opts {
    transcription: PathBuf,
    reference: PathBuf,
    #[clap(long)]
    output_vtt: PathBuf,
    #[cfg(feature = "vibrato")]
    #[clap(long)]
    vibrato_dictionary: Option<PathBuf>,
}

trait Tokenizer {
    fn tokenize<'a>(&'a mut self, text: &'a str) -> Box<dyn Iterator<Item = Range<usize>> + 'a>;
}

#[cfg(feature = "vibrato")]
struct VibratoTokenizer {
    tokenizer: *mut vibrato::Tokenizer,
    worker: ManuallyDrop<Worker<'static>>,
}

#[cfg(feature = "vibrato")]
impl VibratoTokenizer {
    fn new(dictionary: Dictionary) -> Self {
        let tokenizer: *mut vibrato::Tokenizer =
            Box::leak(Box::new(vibrato::Tokenizer::new(dictionary))) as *mut _;
        let worker = unsafe { (*tokenizer).new_worker() };
        Self {
            tokenizer,
            worker: ManuallyDrop::new(worker),
        }
    }
}

#[cfg(feature = "vibrato")]
impl Drop for VibratoTokenizer {
    fn drop(&mut self) {
        unsafe {
            ManuallyDrop::drop(&mut self.worker);
            let _ = Box::from_raw(self.tokenizer);
        }
    }
}

#[cfg(feature = "vibrato")]
impl Tokenizer for VibratoTokenizer {
    fn tokenize<'a>(&'a mut self, text: &'a str) -> Box<dyn Iterator<Item = Range<usize>> + 'a> {
        self.worker.reset_sentence(text);
        self.worker.tokenize();
        Box::new(self.worker.token_iter().map(|x| x.range_byte()))
    }
}

struct WhitespaceTokenizer;

impl Tokenizer for WhitespaceTokenizer {
    fn tokenize<'a>(&'a mut self, text: &'a str) -> Box<dyn Iterator<Item = Range<usize>> + 'a> {
        let mut current = text
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(text.len());
        let mut next_whitespace = None;

        Box::new(std::iter::from_fn(move || {
            if let Some(ws) = next_whitespace.take() {
                return Some(ws);
            }

            if current == text.len() {
                return None;
            }

            let next = text[current..]
                .find(char::is_whitespace)
                .map(|x| x + current)
                .unwrap_or(text.len());

            let old_current = current;
            current = text[next..]
                .find(|c: char| !c.is_whitespace())
                .map(|x| x + next)
                .unwrap_or(text.len());
            if next != text.len() {
                next_whitespace = Some(next..current);
            }

            Some(old_current..next)
        }))
    }
}

fn create_tokenizer(opts: &Opts, language: &str) -> Box<dyn Tokenizer> {
    if language == "ja" {
        #[cfg(feature = "vibrato")]
        if let Some(dic) = opts.vibrato_dictionary.as_ref() {
            println!("Loading vibrato dictionary");
            return Box::new(VibratoTokenizer::new(
                vibrato::Dictionary::read(std::fs::File::open(dic).unwrap()).unwrap(),
            ));
        } else {
            eprintln!(
                "[warning] No vibrato dictionary was provided but Japanese is being tokenized."
            );
        }
        #[cfg(not(feature = "vibrato"))]
        {
            eprintln!("[warning] The vibrato feature was disabled during compilation but Japanese is being tokenized.");
        }
        eprintln!("[warning] This will result in terrible alignment quality, consider using the vibrato tokenizer instead.");
    }

    Box::new(WhitespaceTokenizer)
}

struct TimedLine {
    start: i64,
    end: i64,
    text: String,
}

fn timed_lines_to_vtt<'a>(
    language: &str,
    lines: impl IntoIterator<Item = &'a TimedLine>,
    mut output: impl Write,
) {
    let vtt_ts = |ts: i64| {
        let ms = ts * 10;
        let s = ms / 1000;
        let min = s / 60;
        let h = min / 60;
        format!("{:02}:{:02}:{:02}.{:<03}", h, min % 60, s % 60, ms % 1000)
    };

    writeln!(output, "WEBVTT").unwrap();
    writeln!(output, "Kind: captions").unwrap();
    writeln!(output, "Languagee: {language}").unwrap();
    for line in lines.into_iter() {
        writeln!(output).unwrap();
        writeln!(output, "{} --> {}", vtt_ts(line.start), vtt_ts(line.end)).unwrap();
        writeln!(output, "{}", line.text).unwrap();
    }
}

pub fn main(opts: Opts) {
    let transcription: Transcription =
        serde_json::from_reader(File::open(&opts.transcription).unwrap()).unwrap();
    let reference = std::fs::read_to_string(&opts.reference).unwrap();

    let mut tokenizer = create_tokenizer(&opts, &transcription.language);

    #[derive(Clone, Debug)]
    struct WhisperToken {
        text: String,
        start: i64,
        end: i64,
    }

    let mut byte_starts = vec![];
    let mut byte_ends = vec![];
    let mut whisper_sentence = String::new();
    for segment in transcription.segments.iter() {
        for token in segment {
            let bytes = token.text.len();
            let byte_duration = (token.end - token.start) / bytes as i64;
            let mut current = token.start;
            for _ in 0..bytes {
                byte_starts.push(current);
                current += byte_duration;
                byte_ends.push(current);
            }
            whisper_sentence += &token.text
        }

        byte_starts.push(*byte_ends.last().unwrap());
        byte_ends.push(*byte_ends.last().unwrap());
        whisper_sentence += "\n";
    }
    assert_eq!(whisper_sentence.len(), byte_ends.len());
    assert_eq!(whisper_sentence.len(), byte_starts.len());

    println!("Tokenizing whisper sentence");
    let whisper_tokens = tokenizer
        .tokenize(&whisper_sentence)
        .map(|range| WhisperToken {
            text: whisper_sentence[range.clone()].to_string(),
            start: byte_starts[range.start],
            end: byte_ends[range.end - 1],
        })
        .collect::<Vec<_>>();

    println!("Tokenizing reference sentence");
    let reference_tokens = tokenizer
        .tokenize(&reference)
        .map(|x| &reference[x])
        .collect::<Vec<_>>();

    println!("Aligning tokens");
    let alignment = crate::align::text_align(
        whisper_tokens.iter().map(|x| x.text.clone()),
        reference_tokens.iter().copied().map(str::to_string),
    )
    .into_iter()
    .map(|(a, b)| {
        (
            a.map(|i| &whisper_tokens[i]),
            b.map(|i| reference_tokens[i]),
        )
    })
    .collect::<Vec<_>>();

    let mut reference_lines = vec![vec![]];
    for (a, b) in alignment {
        let is_line_boundary = b.as_ref().is_some_and(|x| x.contains("\n"));
        reference_lines.last_mut().unwrap().push((a, b));
        if is_line_boundary {
            reference_lines.push(vec![]);
        }
    }
    if reference_lines.last().unwrap().is_empty() {
        reference_lines.pop();
    }

    let mut timed_lines: Vec<TimedLine> = vec![];

    for i in 0..reference_lines.len() {
        let current = &reference_lines[i];

        if current.iter().all(|x| x.1.is_none()) {
            continue;
        }

        let mut start = None;
        let mut it = current.iter().peekable();
        while let Some((Some(x), None)) = it.peek() {
            start = Some(x.end);
            it.next();
        }
        if let Some((Some(x), Some(_))) = it.next() {
            start = Some(x.start);
        }

        let mut start = if let Some(wt) = start {
            Some(wt)
        } else if i > 0 {
            reference_lines[i - 1]
                .iter()
                .rev()
                .find_map(|x| x.0.as_ref().map(|x| x.end))
        } else if i == 0 {
            Some(0)
        } else {
            None
        };

        if let (Some(start), Some(last)) = (&mut start, timed_lines.last()) {
            if *start < last.end {
                *start = last.end
            }
        }

        let mut end = None;
        let mut it = current.iter().rev().peekable();
        while let Some((Some(x), None)) = it.peek().map(|x| {
            (
                x.0.as_ref(),
                x.1.as_ref()
                    .filter(|x| x.chars().all(|c| c.is_alphanumeric())),
            )
        }) {
            end = Some(x.start);
            it.next();
        }
        while let Some((None, Some(_))) = it.peek().map(|x| {
            (
                x.0.as_ref(),
                x.1.as_ref()
                    .filter(|x| x.chars().all(|c| !c.is_alphanumeric())),
            )
        }) {
            it.next();
        }
        if let Some((Some(x), Some(_))) = it.next() {
            end = Some(x.end);
        }

        // println!("{current:?} {:?}", &reference_lines[i + 1]);
        let end = if let Some(wt) = end {
            Some(wt)
        } else if let Some(end) = reference_lines
            .get(i + 1)
            .and_then(|v| v.iter().find_map(|x| x.0.as_ref().map(|t| t.start)))
        {
            Some(end)
        } else if i == reference_lines.len() - 1 {
            byte_ends.last().copied()
        } else {
            current
                .iter()
                .rev()
                .find_map(|x| x.0.as_ref().map(|x| x.end))
        };

        let line_text = current
            .iter()
            .filter_map(|x| x.1)
            .collect::<String>()
            .trim()
            .to_string();
        if let (Some(start), Some(end)) = (start, end) {
            timed_lines.push(TimedLine {
                start,
                end,
                text: line_text.to_string(),
            });
        } else {
            println!("Skipped line {line_text} (improperly timed)");
        }

        if i != 0 {
            println!()
        }

        println!("{start:?} --- {end:?}");
        for (a, b) in current {
            println!("{a:?} {b:?}")
        }
        println!("{}", line_text)
    }

    timed_lines_to_vtt(
        &transcription.language,
        timed_lines.iter(),
        std::fs::File::create(opts.output_vtt).unwrap(),
    );
}
