use clap::Parser;

mod align;
mod ff;
mod whisper;
mod cli;

fn main() {
    cli::main(cli::Opts::parse())
}
