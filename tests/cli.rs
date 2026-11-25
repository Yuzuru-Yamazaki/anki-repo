use apkggen::parser::{cloze_ords, parse_json_bytes, parse_txt_bytes};
use apkggen::{Compression, GenerateOptions, NoteType, generate};
use rusqlite::Connection;
use std::io::Write;
use std::path::PathBuf;
use tempfile::NamedTempFile;
use zip::ZipArchive;

fn read_db(path: &std::path::Path) -> (Vec<String>, Vec<i64>, Vec<String>, Vec<String>) {
    let mut archive = ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(
        &mut archive.by_name("collection.anki2").unwrap(),
        &mut buffer,
    )
    .unwrap();
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&buffer).unwrap();
    file.flush().unwrap();
    let connection = Connection::open(file.path()).unwrap();
    let notes: Vec<String> = connection
        .prepare("SELECT flds FROM notes ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let cards: Vec<i64> = connection
        .prepare("SELECT ord FROM cards ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let decks_json: String = connection
        .prepare("SELECT decks FROM col")
        .unwrap()
        .query_row([], |row| row.get(0))
        .unwrap();
    let tags: Vec<String> = connection
        .prepare("SELECT tags FROM notes ORDER BY id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let decks: Vec<String> = serde_json::from_str::<serde_json::Value>(&decks_json)
        .unwrap()
        .as_object()
        .unwrap()
        .values()
        .map(|deck| deck["name"].as_str().unwrap().to_owned())
        .collect();
    (notes, cards, decks, tags)
}

#[test]
fn txt_splits_on_double_colon_and_joins_answer_lines() {
    let text = b"# comment\nCapital of Japan::Tokyo\n\nWhat is 2 + 2?\n4\nbecause math";
    let parsed = parse_txt_bytes(text, None).unwrap();
    let parsed = &parsed[0];
    assert_eq!(parsed.cards[0].fields, ["Capital of Japan", "Tokyo"]);
    assert_eq!(
        parsed.cards[1].fields,
        ["What is 2 + 2?", "4<br>because math"]
    );
}

#[test]
fn txt_requires_both_sides() {
    assert!(parse_txt_bytes(b"question only", None).is_err());
}

#[test]
fn cloze_ords_are_zero_based_and_unique() {
    assert_eq!(
        cloze_ords("{{c2::b}} and {{c1::a}} and {{c1::again}}"),
        vec![0, 1]
    );
    assert!(cloze_ords("no markers").is_empty());
}

#[test]
fn json_accepts_aliases_tags_and_cloze() {
    let text = br#"{
        "deck": "Science",
        "tags": ["shared"],
        "cards": [
            {"front": "H2O?", "back": "Water", "tags": ["chem"]},
            {"q": "Gold?", "a": "Au"}
        ]
    }"#;
    let parsed = parse_json_bytes(text, None).unwrap();
    let parsed = &parsed[0];
    assert_eq!(parsed.deck.as_deref(), Some("Science"));
    assert_eq!(parsed.cards[0].fields, ["H2O?", "Water"]);
    assert_eq!(parsed.cards[0].tags, ["chem", "shared"]);
    assert_eq!(parsed.cards[1].fields, ["Gold?", "Au"]);

    let cloze = br#"{"model":"cloze","cards":["Paris is in {{c1::France}}"]}"#;
    let parsed = parse_json_bytes(cloze, None).unwrap();
    assert_eq!(parsed[0].cards[0].note_type, NoteType::Cloze);
}

#[test]
fn json_decks_defines_nested_subdecks() {
    let text = br#"{
        "tags": ["school"],
        "decks": [
            {"name": "School::Grade 9", "cards": [{"question": "Q1", "answer": "A1"}]},
            {"deck": "School::Grade 10::Math", "cards": [{"question": "Q2", "answer": "A2"}]}
        ]
    }"#;
    let parsed = parse_json_bytes(text, None).unwrap();
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].deck.as_deref(), Some("School::Grade 9"));
    assert_eq!(parsed[1].deck.as_deref(), Some("School::Grade 10::Math"));
    assert_eq!(parsed[0].cards[0].tags, ["school"]);
}

#[test]
fn generate_writes_valid_multi_deck_package() {
    let directory = tempfile::tempdir().unwrap();
    let txt: PathBuf = directory.path().join("general_trivia.txt");
    std::fs::write(&txt, "Capital of Japan::Tokyo\n\nLargest ocean::Pacific").unwrap();
    let json: PathBuf = directory.path().join("science.json");
    std::fs::write(
        &json,
        r#"{"deck":"Science","cards":[{"question":"H2O?","answer":"Water","tags":["chem"]}]}"#,
    )
    .unwrap();
    let output = directory.path().join("combined.apkg");
    let options = GenerateOptions {
        deck: None,
        model: None,
        tags: "demo",
        compression: Compression::Balanced,
    };

    let count = generate(&[txt, json], &options, &output).unwrap();
    assert_eq!(count, 3);
    let (notes, cards, decks, tags) = read_db(&output);
    assert_eq!(notes.len(), 3);
    assert_eq!(cards.len(), 3);
    assert!(decks.iter().any(|name| name == "Science"));
    assert!(decks.iter().any(|name| name == "general trivia"));
    assert!(tags.iter().any(|tag| tag.contains("chem demo")));
}

#[test]
fn generate_cloze_creates_one_card_per_ordinal() {
    let directory = tempfile::tempdir().unwrap();
    let txt: PathBuf = directory.path().join("cloze.txt");
    std::fs::write(&txt, "The {{c1::sky}} is {{c2::blue}}.").unwrap();
    let output = directory.path().join("cloze.apkg");
    let options = GenerateOptions {
        deck: None,
        model: Some(NoteType::Cloze),
        tags: "",
        compression: Compression::Balanced,
    };

    assert_eq!(generate(&[txt], &options, &output).unwrap(), 1);
    let (_, cards, _, _) = read_db(&output);
    assert_eq!(cards, vec![0, 1]);
}

#[test]
fn invalid_input_returns_error() {
    let directory = tempfile::tempdir().unwrap();
    let txt: PathBuf = directory.path().join("bad.txt");
    std::fs::write(&txt, "no answer").unwrap();
    let output = directory.path().join("bad.apkg");
    let options = GenerateOptions {
        deck: None,
        model: None,
        tags: "",
        compression: Compression::Balanced,
    };
    assert!(generate(&[txt], &options, &output).is_err());
}
