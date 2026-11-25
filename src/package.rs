use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json::{Map, Value};
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::{Compression, DeckInput, NoteType, model, parser};

const SCHEMA: &str = include_str!("schema.sql");
const CONF: &str = include_str!("conf.json");
const DCONF: &str = include_str!("dconf.json");

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

pub fn write_package(
    decks: &[DeckInput],
    output: &Path,
    compression: Compression,
) -> Result<usize> {
    let temporary = NamedTempFile::new().context("could not create temporary database")?;
    let mut connection = Connection::open(temporary.path())?;
    connection.execute_batch(
        "PRAGMA journal_mode=OFF; PRAGMA synchronous=OFF; PRAGMA temp_store=MEMORY; PRAGMA locking_mode=EXCLUSIVE;",
    )?;
    connection.execute_batch(SCHEMA)?;

    let timestamp_ms = now_millis();
    let timestamp = timestamp_ms / 1000;
    connection.execute(
        "INSERT INTO col VALUES (NULL,1411124400,1425279151694,1425279151690,11,0,0,0,?1,?2,?3,?4,?5)",
        params![CONF, "{}", "{}", DCONF, "{}"],
    )?;

    let mut notes_written = 0;
    {
        let transaction = connection.transaction()?;
        let mut insert_note =
            transaction.prepare("INSERT INTO notes VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)")?;
        let mut insert_card = transaction.prepare(
            "INSERT INTO cards VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        )?;
        let mut models = Map::new();
        let mut deck_map = Map::new();
        deck_map.insert("1".to_owned(), model::default_deck_json());
        let mut next_id = timestamp_ms;

        for deck in decks {
            let deck_id = model::stable_id(&format!("deck:{}", deck.name));
            deck_map.insert(deck_id.to_string(), model::deck_json(&deck.name, deck_id));

            let mut uses_basic = false;
            let mut uses_cloze = false;
            for card in &deck.cards {
                match card.note_type {
                    NoteType::Basic => uses_basic = true,
                    NoteType::Cloze => uses_cloze = true,
                }
            }
            for note_type in [NoteType::Basic, NoteType::Cloze] {
                if (note_type == NoteType::Basic && uses_basic)
                    || (note_type == NoteType::Cloze && uses_cloze)
                {
                    let model_id = note_type.model_id();
                    models.insert(
                        model_id.to_string(),
                        model::model_json(note_type, model_id, deck_id, timestamp),
                    );
                }
            }

            for card in &deck.cards {
                let model_id = card.note_type.model_id();
                let note_id = next_id;
                next_id += 1;
                let guid = model::guid(&card.fields);
                let tags = format!(" {} ", card.tags.join(" "));
                let fields = format!("{}\u{1f}{}", card.fields[0], card.fields[1]);
                insert_note.execute(params![
                    note_id,
                    guid,
                    model_id,
                    timestamp,
                    -1,
                    tags,
                    fields,
                    card.fields[0],
                    0,
                    0,
                    "",
                ])?;
                notes_written += 1;

                let ords = match card.note_type {
                    NoteType::Basic => vec![0],
                    NoteType::Cloze => {
                        let ords = parser::cloze_ords(&card.fields[0]);
                        if ords.is_empty() { vec![0] } else { ords }
                    }
                };
                for ord in ords {
                    let card_id = next_id;
                    next_id += 1;
                    insert_card.execute(params![
                        card_id, note_id, deck_id, ord as i64, timestamp, -1, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, "",
                    ])?;
                }
            }
        }

        drop(insert_note);
        drop(insert_card);
        transaction.execute(
            "UPDATE col SET models=?1, decks=?2",
            params![
                Value::Object(models).to_string(),
                Value::Object(deck_map).to_string()
            ],
        )?;
        transaction.commit()?;
    }
    drop(connection);

    let (method, level) = compression.settings();
    let file =
        File::create(output).with_context(|| format!("could not create {}", output.display()))?;
    let writer = BufWriter::with_capacity(1 << 20, file);
    let mut archive = ZipWriter::new(writer);
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(method)
        .compression_level(level)
        .large_file(true);
    archive.start_file("collection.anki2", options)?;
    let mut database = BufReader::with_capacity(1 << 20, File::open(temporary.path())?);
    std::io::copy(&mut database, &mut archive)?;
    let media_options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);
    archive.start_file("media", media_options)?;
    archive.write_all(b"{}")?;
    archive.finish()?;

    Ok(notes_written)
}
