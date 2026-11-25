#!/usr/bin/env python3
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

import genanki

BASIC = "basic"
CLOZE = "cloze"
MODELS = (BASIC, CLOZE)

FIELD_KEYS = {
    BASIC: (("question", "front", "q"), ("answer", "back", "a")),
    CLOZE: (("text", "front", "q"), ("extra", "back", "a")),
}

CLOZE_MARKER = re.compile(r"\{\{c\d+::")


def stable_id(label):
    digest = hashlib.sha256(label.encode("utf-8")).digest()
    return int.from_bytes(digest[:8], "big") % (10**9 - 1) + 1


def normalize_tags(tags):
    if tags is None:
        return []
    if isinstance(tags, str):
        tags = re.split(r"[,;]", tags)
    result = []
    for tag in tags:
        tag = str(tag).strip().replace(" ", "_")
        if tag:
            result.append(tag)
    return result


def make_model(card_type):
    if card_type == BASIC:
        return genanki.Model(
            stable_id("model:basic"),
            "apkggen Basic",
            fields=[{"name": "Question"}, {"name": "Answer"}],
            templates=[
                {
                    "name": "Card 1",
                    "qfmt": "{{Question}}",
                    "afmt": '{{FrontSide}}<hr id="answer">{{Answer}}',
                }
            ],
        )
    return genanki.Model(
        stable_id("model:cloze"),
        "apkggen Cloze",
        fields=[{"name": "Text"}, {"name": "Extra"}],
        templates=[
            {
                "name": "Cloze",
                "qfmt": "{{cloze:Text}}",
                "afmt": "{{cloze:Text}}<br>{{Extra}}",
            }
        ],
        model_type=genanki.Model.CLOZE,
    )


def parse_txt(text, model_override=None):
    card_type = model_override or BASIC
    cards = []
    for block in re.split(r"\n\s*\n", text):
        lines = [
            line.strip()
            for line in block.splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        if not lines:
            continue
        if card_type == CLOZE:
            for line in lines:
                if not CLOZE_MARKER.search(line):
                    raise ValueError(f"cloze card is missing a {{{{c1::...}}}} marker: {line}")
                cards.append({"text": line})
            continue
        front, _, back = lines[0].partition("::")
        parts = [part.strip() for part in [back] + lines[1:] if part.strip()]
        answer = "<br>".join(parts)
        if not front.strip() or not answer:
            raise ValueError(f"card needs a question and an answer: {lines[0]}")
        cards.append({"question": front.strip(), "answer": answer})
    return None, cards


def first_present(item, keys):
    for key in keys:
        if key in item and item[key] is not None:
            return str(item[key]).strip()
    return None


def parse_json(text, model_override=None):
    data = json.loads(text)
    if isinstance(data, dict):
        deck = data.get("deck")
        declared = data.get("model")
        if declared is not None and declared not in MODELS:
            raise ValueError(f"unknown model {declared!r}; expected one of {', '.join(MODELS)}")
        raw_cards = data.get("cards")
        if raw_cards is None:
            raise ValueError('JSON object must contain a "cards" list')
        inherited_tags = normalize_tags(data.get("tags"))
    elif isinstance(data, list):
        deck = None
        declared = None
        raw_cards = data
        inherited_tags = []
    else:
        raise ValueError("JSON input must be a list of cards or an object with a cards list")

    card_type = model_override or declared or BASIC
    if not isinstance(raw_cards, list):
        raise ValueError('"cards" must be a list')

    cards = []
    for index, item in enumerate(raw_cards, 1):
        if isinstance(item, str):
            values = {"text" if card_type == CLOZE else "question": item}
        elif isinstance(item, dict):
            values = item
        else:
            raise ValueError(f"card {index} must be an object or a string")

        front_keys, back_keys = FIELD_KEYS[card_type]
        front = first_present(values, front_keys)
        back = first_present(values, back_keys) or ""
        if not front:
            wanted = "/".join(front_keys)
            raise ValueError(f"card {index} is missing a {wanted} field")
        if card_type == CLOZE:
            if not CLOZE_MARKER.search(front):
                raise ValueError(f"card {index} is missing a {{{{c1::...}}}} marker")
            card = {"text": front}
            if back:
                card["extra"] = back
        else:
            if not back:
                wanted = "/".join(back_keys)
                raise ValueError(f"card {index} is missing an {wanted} field")
            card = {"question": front, "answer": back}

        tags = normalize_tags(values.get("tags")) + inherited_tags
        if tags:
            card["tags"] = tags
        cards.append(card)
    return deck, cards


def build_package(paths, deck_override=None, model_override=None, tags_override=None):
    decks = {}
    models = {}
    total = 0
    global_tags = normalize_tags(tags_override)
    for path in paths:
        text = path.read_text(encoding="utf-8")
        if path.suffix.lower() == ".txt":
            file_deck, cards = parse_txt(text, model_override)
            card_type = model_override or BASIC
        elif path.suffix.lower() == ".json":
            file_deck, cards = parse_json(text, model_override)
            declared = None
            if model_override is None:
                data = json.loads(text)
                if isinstance(data, dict):
                    declared = data.get("model")
            card_type = model_override or declared or BASIC
        else:
            raise ValueError(f"unsupported input type {path.suffix!r}; expected .txt or .json")

        deck_name = deck_override or file_deck or path.stem.replace("_", " ")
        if deck_name not in decks:
            decks[deck_name] = genanki.Deck(stable_id(f"deck:{deck_name}"), deck_name)
        if card_type not in models:
            models[card_type] = make_model(card_type)

        for card in cards:
            if card_type == BASIC:
                fields = [card["question"], card["answer"]]
            else:
                fields = [card["text"], card.get("extra", "")]
            tags = global_tags + normalize_tags(card.get("tags"))
            decks[deck_name].add_note(
                genanki.Note(model=models[card_type], fields=fields, tags=tags)
            )
            total += 1
    return genanki.Package(list(decks.values())), total


def main(argv=None):
    parser = argparse.ArgumentParser(
        description="Generate an Anki .apkg package from .txt or .json files."
    )
    parser.add_argument("inputs", nargs="+", type=Path, metavar="INPUT")
    parser.add_argument("-o", "--output", type=Path, help="output .apkg path")
    parser.add_argument("-d", "--deck", help="deck name (overrides names in the input files)")
    parser.add_argument("-m", "--model", choices=MODELS, help="note type for .txt input")
    parser.add_argument("-t", "--tags", default="", help="comma-separated tags added to every card")
    args = parser.parse_args(argv)

    try:
        package, total = build_package(
            args.inputs,
            deck_override=args.deck,
            model_override=args.model,
            tags_override=args.tags,
        )
        output = args.output or args.inputs[0].with_name(
            f"{args.inputs[0].stem}.apkg" if len(args.inputs) == 1 else "decks.apkg"
        )
        output.parent.mkdir(parents=True, exist_ok=True)
        package.write_to_file(str(output))
    except (OSError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(f"Wrote {output} with {total} card(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
