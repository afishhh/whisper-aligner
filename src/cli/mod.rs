use std::path::PathBuf;

use clap::Parser;

mod align;

#[derive(Parser)]
pub enum Opts {
    Transcribe {
        file: PathBuf,
        #[clap(short, long)]
        output: PathBuf,
        #[clap(short, long)]
        model: PathBuf,
        #[clap(short, long)]
        language: String,
    },
    Align(align::Opts),
}

pub fn main(opts: Opts) {
    match opts {
        Opts::Transcribe {
            file,
            output,
            model,
            language,
        } => {
            let transcription =
                crate::whisper::transcribe(std::fs::File::open(file).unwrap(), language, model);
            serde_json::to_writer(std::fs::File::create(output).unwrap(), &transcription).unwrap();
        }
        Opts::Align(opts) => align::main(opts),
    }
}
