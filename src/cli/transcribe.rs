use std::path::PathBuf;

use clap::Parser;

use crate::whisper::SileroOptions;

#[derive(Parser)]
pub struct Opts {
    file: PathBuf,
    #[clap(short, long)]
    output: PathBuf,
    #[clap(short, long)]
    model: PathBuf,
    #[clap(short, long)]
    language: String,
    #[clap(flatten)]
    vad: Option<VadOpts>,
}

#[derive(Parser)]
pub struct VadOpts {
    #[clap(long = "vad", default_value_t = true, requires = "path")]
    enabled: bool,
    #[clap(long = "vad-silero-path")]
    path: PathBuf,
    #[clap(long = "vad-threshold", default_value_t = 0.3)]
    speech_threshold: f32,
    #[clap(long = "vad-min-duration", default_value_t = 5.0)]
    min_silence_seconds: f32,
    #[clap(long = "vad-padding-duration", default_value_t = 0.5)]
    padding_seconds: f32,
}

pub fn main(
    Opts {
        file,
        output,
        model,
        language,
        vad,
    }: Opts,
) {
    let transcription = crate::whisper::transcribe(
        std::fs::File::open(file).unwrap(),
        language,
        model,
        vad.map(|x| SileroOptions {
            path: x.path,
            threshold: x.speech_threshold,
            min_silence_seconds: x.min_silence_seconds,
            min_trim_silence_seconds: 2.0,
            speech_padding_seconds: x.padding_seconds,
        }),
    );
    serde_json::to_writer(std::fs::File::create(output).unwrap(), &transcription).unwrap();
}
