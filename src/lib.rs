pub mod model;
pub mod package;
pub mod parser;

use anyhow::{Context, Result, bail};
use clap::ValueEnum;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum NoteType {
    Basic,
    Cloze,
}

impl NoteType {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "basic" => Ok(Self::Basic),
            "cloze" => Ok(Self::Cloze),
            other => bail!("unknown model {other:?}; expected basic or cloze"),
        }
    }

    pub fn model_id(self) -> i64 {
        match self {
            Self::Basic => model::stable_id("model:basic"),
            Self::Cloze => model::stable_id("model:cloze"),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum Compression {
    #[default]
    Balanced,
    Fast,
    Best,
}

impl Compression {
    pub fn settings(self) -> (zip::CompressionMethod, Option<i64>) {
        match self {
            Self::Fast => (zip::CompressionMethod::Deflated, Some(1)),
            Self::Balanced => (zip::CompressionMethod::Deflated, Some(6)),
            Self::Best => (zip::CompressionMethod::Deflated, Some(9)),
        }
    }
}

#[derive(Debug)]
pub struct Card {
    pub fields: [String; 2],
    pub tags: Vec<String>,
    pub note_type: NoteType,
}

#[derive(Debug)]
pub struct ParsedFile {
    pub deck: Option<String>,
    pub cards: Vec<Card>,
}

#[derive(Debug)]
pub struct DeckInput {
    pub name: String,
    pub cards: Vec<Card>,
}

pub struct GenerateOptions<'a> {
    pub deck: Option<&'a str>,
    pub model: Option<NoteType>,
    pub tags: &'a str,
    pub compression: Compression,
}

pub fn generate(paths: &[PathBuf], options: &GenerateOptions<'_>, output: &Path) -> Result<usize> {
    let global_tags = parser::parse_tags_text(options.tags);

    let parsed: Vec<Result<Vec<ParsedFile>>> = paths
        .par_iter()
        .map(|path| {
            let bytes = std::fs::read(path)
                .with_context(|| format!("could not read {}", path.display()))?;
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("txt") => parser::parse_txt_bytes(&bytes, options.model),
                Some("json") => parser::parse_json_bytes(&bytes, options.model),
                Some(other) => bail!("unsupported input type .{other}; expected .txt or .json"),
                None => bail!("input {} has no extension", path.display()),
            }
        })
        .collect();

    let mut decks: Vec<DeckInput> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (path, result) in paths.iter().zip(parsed) {
        for mut file in result? {
            let name = options
                .deck
                .map(str::to_owned)
                .or(file.deck)
                .unwrap_or_else(|| {
                    path.file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("Deck")
                        .replace('_', " ")
                });
            for card in &mut file.cards {
                if !global_tags.is_empty() {
                    card.tags.reserve(global_tags.len());
                    card.tags.extend(global_tags.iter().cloned());
                }
            }
            match index.get(&name) {
                Some(&position) => decks[position].cards.extend(file.cards),
                None => {
                    index.insert(name.clone(), decks.len());
                    decks.push(DeckInput {
                        name,
                        cards: file.cards,
                    });
                }
            }
        }
    }

    package::write_package(&decks, output, options.compression)
}
