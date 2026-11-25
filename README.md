# anki-repo

Shared Anki packages plus a fast generator that builds them from plain text or JSON.

## Contents

| Path | Description |
| --- | --- |
| `implementation_plan.md` | Spec for the Anki-compatible flashcards workspace |
| `trivia.apkg` | Sample 20-card general trivia deck |
| `src/` | Rust implementation of `apkggen` (library + CLI) |
| `tests/cli.rs` | Rust integration tests |
| `apkggen.py` | Original Python implementation, kept as a reference |
| `tests/test_apkggen.py` | Python reference tests |

## Build and run (Rust)

Requires a stable Rust toolchain. SQLite is compiled into the binary, so there is no runtime dependency.

```bash
cargo build --release
./target/release/apkggen cards.txt
./target/release/apkggen cards.json -o decks/trivia.apkg -d "General Trivia" -t "trivia,shared"
./target/release/apkggen basic_cards.txt cloze_cards.json -o combined.apkg
./target/release/apkggen cloze.txt -m cloze
```

| Flag | Effect |
| --- | --- |
| `-o, --output PATH` | Output `.apkg` path; defaults to the first input's directory |
| `-d, --deck NAME` | Deck name for every input; overrides a name declared in JSON |
| `-m, --model {basic,cloze}` | Note type for `.txt` input; overrides a model declared in JSON |
| `-t, --tags TAGS` | Comma- or semicolon-separated tags added to every card |
| `--compression {balanced,fast,best}` | Deflate level 6, 1, or 9 (default: balanced) |

With one input the default output is `<input-stem>.apkg`; with several inputs it is `decks.apkg`. Multiple inputs are combined into one package with one deck per file, unless `--deck` merges them all.

## `.txt` format

Each non-empty paragraph (blank-line separated) is one card. Lines starting with `#` are ignored.

```text
# Capital cities
Capital of Japan::Tokyo

What does HTTP stand for?
HyperText Transfer Protocol

Largest ocean::Pacific Ocean
```

The first line may use `Question::Answer`, or put the question on the first line and the answer on the lines below it. Multi-line answers are joined with `<br>` so they render correctly in Anki. A card missing either side is an error.

For cloze notes, every non-empty line is a card and must contain at least one `{{c1::...}}` marker:

```text
The {{c1::capital}} of Japan is Tokyo.
{{c1::Paris}} is the capital of {{c2::France}}.
```

## `.json` format

Either a bare list of card objects, or an object with a `cards` list plus optional `deck`, `model`, and `tags`:

```json
{
  "deck": "General Trivia",
  "model": "basic",
  "tags": ["trivia"],
  "cards": [
    { "question": "Capital of Japan?", "answer": "Tokyo", "tags": ["geography"] },
    { "front": "Largest ocean?", "back": "Pacific Ocean" },
    { "q": "Symbol for gold?", "a": "Au" }
  ]
}
```

Accepted field names are `question` / `front` / `q` and `answer` / `back` / `a`. A plain string in the `cards` list is treated as the question for basic notes.

For `"model": "cloze"`, use `text` / `front` / `q` for the cloze text and optional `extra` / `back` / `a` for the answer-side note. Plain strings are treated as cloze text.

```json
{
  "deck": "Cloze Practice",
  "model": "cloze",
  "cards": [
    { "text": "The {{c1::sky}} is blue.", "extra": "During the day" },
    "Anki uses {{c1::FSRS}} scheduling."
  ]
}
```

Per-card `tags` are merged with top-level `tags` and `--tags`. Spaces in tags become underscores.

## How it works

1. `generate()` reads every input file and parses them in parallel with Rayon. Dispatch is by extension: `parse_txt_bytes()` or `parse_json_bytes()`.
2. The text parser is a single pass over `str::lines()` with a reusable block buffer; comments are skipped and `::` is split with `split_once()`. Cloze ordinals are found with a byte-level scanner, not a regex. JSON is decoded straight from bytes with `serde_json`, keeping field values as borrowed `&str` until a card needs to be owned.
3. Each card is normalized to two fields plus tags. Deck names resolve from `--deck`, then the JSON `deck` field, then the input filename with underscores replaced by spaces. Cards with the same deck name are merged.
4. `write_package()` creates a temporary SQLite database with the Anki 2.1 schema, then writes the `col`, `notes`, and `cards` rows inside one transaction using reused prepared statements. `journal_mode=OFF`, `synchronous=OFF`, and `temp_store=MEMORY` remove transaction overhead for this throwaway database.
5. Note GUIDs are Anki's base91 encoding of the first eight bytes of SHA-256 over `field0__field1`. Model and deck IDs are stable SHA-256-derived integers, so regenerating a deck produces IDs Anki can update cleanly. The base91 conversion uses a fixed stack buffer.
6. Cloze notes produce one card per distinct cloze ordinal (`c1` → `ord 0`, `c2` → `ord 1`), matching genanki and Anki.
7. The database is streamed into a ZIP through 1 MiB buffered readers/writers alongside a `media` manifest of `{}`. No intermediate ZIP is created. The release profile uses `opt-level = 3`, 16 codegen units, `panic = "abort"`, and strips symbols.

The output is Anki-compatible: note GUIDs, `\x1f` field encoding, tags, model JSON, deck JSON, and card rows match the Python/genanki reference for the same input (only timestamps differ). Open the result in Anki with `File > Import`.

## Performance

Measured on 50,000 basic cards on the same machine:

| Implementation | Time | Package size |
| --- | --- | --- |
| Python (`apkggen.py`) | 13.3 s | 12 MB |
| Rust release, balanced deflate | 2.07 s | 2.9 MB |

The main wins are parallel parsing, no regex, borrowed JSON strings, one SQLite transaction with prepared statements, and streaming ZIP output.

## Tests

```bash
cargo test
python3 -m unittest discover -s tests -v
```

The Rust tests cover text and JSON parsing, cloze ordinals, tag merging, error handling, and the SQLite contents of generated packages. The Python suite remains as a cross-check for the reference implementation.
