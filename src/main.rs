use clap::Parser;

mod align;
mod cli;
mod ff;
mod silero;
mod whisper;

fn main() {
    cli::main(cli::Opts::parse())
}
