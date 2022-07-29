use std::path::PathBuf;

use clap::Parser;
use transaction_processor::{self, write_summary_io_csv};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    input: PathBuf,
}

fn main() {
    let Args { input } = Args::parse();
    let reader = match std::fs::File::open(input) {
        Ok(reader) => reader,
        Err(e) => {
            eprintln!("i/o error: {e:?}");
            return;
        }
    };
    let summaries = match transaction_processor::summaries_from_io_csv(reader) {
        Ok(summaries) => summaries,
        Err(e) => {
            eprintln!("error while parsing csv: {e:?}");
            return;
        }
    };
    if let Err(e) = write_summary_io_csv(&summaries, std::io::stdout().lock()) {
        eprintln!("i/o error: {e:?}")
    }
}
