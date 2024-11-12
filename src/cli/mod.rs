use clap::Parser;

mod align;
mod transcribe;

#[derive(Parser)]
pub enum Opts {
    Transcribe(transcribe::Opts),
    Align(align::Opts),
}

pub fn main(opts: Opts) {
    match opts {
        Opts::Transcribe(opts) => transcribe::main(opts),
        Opts::Align(opts) => align::main(opts),
    }
}
