use anyhow::{Result, bail};
use serde_json::{Map, Value};

use crate::{Card, NoteType, ParsedFile};

fn flush_block(block: &mut Vec<&str>, note_type: NoteType, cards: &mut Vec<Card>) -> Result<()> {
    if block.is_empty() {
        return Ok(());
    }
    match note_type {
        NoteType::Cloze => {
            for line in block.iter() {
                if cloze_ords(line).is_empty() {
                    bail!("cloze card is missing a {{{{c1::...}}}} marker: {line}");
                }
                cards.push(Card {
                    fields: [(*line).to_owned(), String::new()],
                    tags: Vec::new(),
                    note_type,
                });
            }
        }
        NoteType::Basic => {
            let first = block[0];
            let (question, raw_answer) = first.split_once("::").unwrap_or((first, ""));
            let mut answer = String::with_capacity(raw_answer.len() + block.len() * 4);
            answer.push_str(raw_answer.trim());
            for extra in block.iter().skip(1) {
                if !answer.is_empty() {
                    answer.push_str("<br>");
                }
                answer.push_str(extra);
            }
            if question.trim().is_empty() || answer.is_empty() {
                bail!("card needs a question and an answer: {first}");
            }
            cards.push(Card {
                fields: [question.trim().to_owned(), answer],
                tags: Vec::new(),
                note_type,
            });
        }
    }
    block.clear();
    Ok(())
}

pub fn parse_txt_bytes(bytes: &[u8], override_type: Option<NoteType>) -> Result<ParsedFile> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| anyhow::anyhow!("input is not valid UTF-8: {error}"))?;
    let note_type = override_type.unwrap_or(NoteType::Basic);
    let mut cards = Vec::new();
    let mut block: Vec<&str> = Vec::with_capacity(8);
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            flush_block(&mut block, note_type, &mut cards)?;
        } else if !trimmed.starts_with('#') {
            block.push(trimmed);
        }
    }
    flush_block(&mut block, note_type, &mut cards)?;
    Ok(ParsedFile { deck: None, cards })
}

pub fn cloze_ords(text: &str) -> Vec<u16> {
    let bytes = text.as_bytes();
    let mut ords: Vec<u16> = Vec::new();
    let mut index = 0;
    while index + 4 < bytes.len() {
        if bytes[index] == b'{' && bytes[index + 1] == b'{' && bytes[index + 2] == b'c' {
            let start = index + 3;
            let mut cursor = start;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor > start
                && cursor + 2 < bytes.len()
                && bytes[cursor] == b':'
                && bytes[cursor + 1] == b':'
            {
                if let Ok(number) = text[start..cursor].parse::<u16>()
                    && number > 0
                {
                    let ord = number - 1;
                    if !ords.contains(&ord) {
                        ords.push(ord);
                    }
                }
                index = cursor + 2;
                continue;
            }
        }
        index += 1;
    }
    ords.sort_unstable();
    ords
}

pub fn parse_tags_text(text: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for part in text.split([',', ';']) {
        let tag = part.trim().replace(' ', "_");
        if !tag.is_empty() {
            tags.push(tag);
        }
    }
    tags
}

fn parse_tags(value: Option<&Value>) -> Vec<String> {
    let mut tags = Vec::new();
    match value {
        Some(Value::String(text)) => tags.extend(parse_tags_text(text)),
        Some(Value::Array(values)) => {
            for item in values {
                if let Some(text) = item.as_str() {
                    tags.extend(parse_tags_text(text));
                }
            }
        }
        _ => {}
    }
    tags
}

fn get_field<'a>(map: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    for key in keys {
        if let Some(text) = map.get(*key).and_then(Value::as_str) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

pub fn parse_json_bytes(bytes: &[u8], override_type: Option<NoteType>) -> Result<ParsedFile> {
    let value: Value = serde_json::from_slice(bytes)?;
    let (deck, declared, raw_cards, inherited_tags) = match &value {
        Value::Object(map) => {
            let deck = map.get("deck").and_then(Value::as_str).map(str::to_owned);
            let declared = match map.get("model") {
                Some(Value::String(name)) => Some(NoteType::parse(name)?),
                Some(_) => bail!("model must be a string"),
                None => None,
            };
            let raw_cards = match map.get("cards") {
                Some(Value::Array(cards)) => cards,
                Some(_) => bail!("cards must be a list"),
                None => bail!("JSON object must contain a cards list"),
            };
            (deck, declared, raw_cards, parse_tags(map.get("tags")))
        }
        Value::Array(cards) => (None, None, cards, Vec::new()),
        _ => bail!("JSON input must be a list of cards or an object with a cards list"),
    };

    let note_type = override_type.or(declared).unwrap_or(NoteType::Basic);
    let mut cards = Vec::with_capacity(raw_cards.len());
    for (position, item) in raw_cards.iter().enumerate() {
        let string_value = match item {
            Value::Object(_) => None,
            Value::String(text) => Some(text.trim()),
            _ => bail!("card {} must be an object or a string", position + 1),
        };
        let map = item.as_object();

        let (front_keys, back_keys) = match note_type {
            NoteType::Basic => (["question", "front", "q"], ["answer", "back", "a"]),
            NoteType::Cloze => (["text", "front", "q"], ["extra", "back", "a"]),
        };
        let front = map
            .and_then(|map| get_field(map, &front_keys))
            .or(string_value)
            .filter(|text| !text.is_empty());
        let front = match front {
            Some(text) => text,
            None => bail!("card {} is missing a {} field", position + 1, front_keys[0]),
        };
        let back = map
            .and_then(|map| get_field(map, &back_keys))
            .unwrap_or_default();

        if note_type == NoteType::Cloze && cloze_ords(front).is_empty() {
            bail!("card {} is missing a {{{{c1::...}}}} marker", position + 1);
        }
        if note_type == NoteType::Basic && back.is_empty() {
            bail!("card {} is missing an {} field", position + 1, back_keys[0]);
        }

        let mut tags = match map.and_then(|map| map.get("tags")) {
            Some(value) => parse_tags(Some(value)),
            None => Vec::new(),
        };
        tags.extend(inherited_tags.iter().cloned());

        cards.push(Card {
            fields: [front.to_owned(), back.to_owned()],
            tags,
            note_type,
        });
    }

    Ok(ParsedFile { deck, cards })
}
