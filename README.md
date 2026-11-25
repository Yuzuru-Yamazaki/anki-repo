# anki-repo

Shared Anki packages plus the generator used to build them.

## Contents

| Path | Description |
| --- | --- |
| `implementation_plan.md` | Spec for the Anki-compatible flashcards workspace |
| `trivia.apkg` | Sample 20-card general trivia deck |
| `apkggen.py` | CLI that turns `.txt` or `.json` files into `.apkg` packages |
| `tests/test_apkggen.py` | Unit tests for the parsers and package writer |

## Setup

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

Requires Python 3.8+ and `genanki` 0.13.1.

## Usage

```bash
python3 apkggen.py cards.txt
python3 apkggen.py cards.json -o decks/trivia.apkg -d "General Trivia" -t "trivia,shared"
python3 apkggen.py basic_cards.txt cloze_cards.json -o combined.apkg
python3 apkggen.py cloze.txt -m cloze
```

| Flag | Effect |
| --- | --- |
| `-o, --output PATH` | Output `.apkg` path; defaults to the first input's directory |
| `-d, --deck NAME` | Deck name for every input; overrides a name declared in JSON |
| `-m, --model {basic,cloze}` | Note type for `.txt` input; overrides a model declared in JSON |
| `-t, --tags TAGS` | Comma- or semicolon-separated tags added to every card |

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

1. `apkggen.py` reads each input as UTF-8 and dispatches to `parse_txt()` or `parse_json()` based on the file extension.
2. The parser returns normalized cards: `question` + `answer` for basic notes, or `text` + optional `extra` for cloze notes, with tags attached. Malformed cards raise a `ValueError` naming the card.
3. The deck name comes from `--deck`, then the JSON `deck` field, then the input filename with underscores replaced by spaces.
4. Two note types are defined through genanki: an `apkggen Basic` front/back model and an `apkggen Cloze` model with `{{cloze:Text}}`. Model and deck IDs are derived from their names with SHA-256, so regenerating a deck produces stable IDs that Anki can update cleanly.
5. Each card becomes a `genanki.Note`, and each note is added to a `genanki.Deck`.
6. `genanki.Package.write_to_file()` writes a ZIP containing `collection.anki2` (the Anki SQLite collection) and a `media` manifest. Open the result in Anki with `File > Import`.

## Tests

```bash
python3 -m unittest discover -s tests -v
```

The tests cover text and JSON parsing, cloze markers, field aliases, tag merging, CLI output naming, error handling, and the SQLite contents of the generated package.
