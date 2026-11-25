use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use apkggen::{Compression, GenerateOptions, NoteType, generate};

#[derive(Parser)]
#[command(
    name = "apkggen",
    version,
    about = "Generate Anki .apkg packages from .txt or .json files"
)]
struct Args {
    #[arg(value_name = "INPUT", required = true)]
    inputs: Vec<PathBuf>,

    #[arg(short = 'o', long, value_name = "PATH")]
    output: Option<PathBuf>,

    #[arg(short = 'd', long, value_name = "NAME")]
    deck: Option<String>,

    #[arg(short = 'm', long, value_enum, value_name = "TYPE")]
    model: Option<NoteType>,

    #[arg(short = 't', long, default_value = "", value_name = "TAGS")]
    tags: String,

    #[arg(long, value_enum, default_value_t = Compression::Balanced)]
    compression: Compression,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let options = GenerateOptions {
        deck: args.deck.as_deref(),
        model: args.model,
        tags: &args.tags,
        compression: args.compression,
    };
    let output = args.output.unwrap_or_else(|| {
        if args.inputs.len() == 1 {
            let mut path = args.inputs[0].clone();
            path.set_extension("apkg");
            path
        } else {
            let mut path = args.inputs[0].clone();
            path.set_file_name("decks.apkg");
            path
        }
    });

    let notes = generate(&args.inputs, &options, &output)?;
    println!("Wrote {} with {notes} card(s)", output.display());
    Ok(())
}
