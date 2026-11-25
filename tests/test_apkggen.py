import json
import sqlite3
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import apkggen


class ParserTests(unittest.TestCase):
    def test_txt_splits_on_double_colon_and_joins_answer_lines(self):
        text = "# a comment\nCapital of Japan::Tokyo\n\nWhat is 2 + 2?\n4\nbecause math"
        deck, cards = apkggen.parse_txt(text)
        self.assertIsNone(deck)
        self.assertEqual(
            cards,
            [
                {"question": "Capital of Japan", "answer": "Tokyo"},
                {"question": "What is 2 + 2?", "answer": "4<br>because math"},
            ],
        )

    def test_txt_requires_two_sides(self):
        with self.assertRaises(ValueError):
            apkggen.parse_txt("question with no answer")

    def test_cloze_txt_requires_marker(self):
        _, cards = apkggen.parse_txt("The {{c1::sky}} is blue.", "cloze")
        self.assertEqual(cards, [{"text": "The {{c1::sky}} is blue."}])
        with self.assertRaises(ValueError):
            apkggen.parse_txt("No marker here.", "cloze")

    def test_json_accepts_field_aliases_tags_and_deck_name(self):
        text = json.dumps(
            {
                "deck": "Science",
                "model": "basic",
                "tags": ["shared"],
                "cards": [
                    {"front": "H2O?", "back": "Water", "tags": ["chemistry"]},
                    {"q": "Atomic number of gold", "a": "79"},
                ],
            }
        )
        deck, cards = apkggen.parse_json(text)
        self.assertEqual(deck, "Science")
        self.assertEqual(cards[0]["answer"], "Water")
        self.assertEqual(cards[0]["tags"], ["chemistry", "shared"])
        self.assertEqual(cards[1]["question"], "Atomic number of gold")

    def test_json_cloze_and_string_cards(self):
        text = json.dumps(
            {
                "model": "cloze",
                "cards": ["Paris is in {{c1::France}}", {"text": "{{c1::Python}} is a language", "extra": "Guido"}],
            }
        )
        _, cards = apkggen.parse_json(text)
        self.assertEqual(cards[0], {"text": "Paris is in {{c1::France}}"})
        self.assertEqual(cards[1]["extra"], "Guido")

    def test_json_decks_defines_nested_subdecks(self):
        text = json.dumps(
            {
                "tags": ["school"],
                "decks": [
                    {
                        "name": "School::Grade 9",
                        "cards": [{"question": "Q1", "answer": "A1"}],
                    },
                    {
                        "deck": "School::Grade 10::Math",
                        "cards": [{"question": "Q2", "answer": "A2"}],
                    },
                ],
            }
        )
        files = apkggen.parse_json_files(text)
        self.assertEqual([deck for deck, _, _ in files], [
            "School::Grade 9",
            "School::Grade 10::Math",
        ])
        self.assertEqual(files[0][1][0]["tags"], ["school"])

    def test_model_override_beats_json_declared_model(self):
        text = json.dumps(
            {
                "model": "cloze",
                "cards": [
                    {
                        "text": "{{c1::x}}",
                        "question": "Q",
                        "answer": "A",
                    }
                ],
            }
        )
        _, cards = apkggen.parse_json(text, "basic")
        self.assertEqual(cards[0], {"question": "Q", "answer": "A"})


class PackageTests(unittest.TestCase):
    def read_collection(self, apkg_path):
        with zipfile.ZipFile(apkg_path) as archive:
            with tempfile.NamedTemporaryFile(suffix=".anki2") as handle:
                handle.write(archive.read("collection.anki2"))
                handle.flush()
                connection = sqlite3.connect(handle.name)
                rows = {
                    "notes": connection.execute("select * from notes").fetchall(),
                    "cards": connection.execute("select * from cards").fetchall(),
                    "col": connection.execute("select models, decks from col").fetchone(),
                }
                connection.close()
                return rows

    def test_main_writes_combined_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            txt = root / "general_trivia.txt"
            txt.write_text("Capital of Japan::Tokyo\n\nLargest ocean::Pacific", encoding="utf-8")
            js = root / "science.json"
            js.write_text(
                json.dumps(
                    {
                        "deck": "Science",
                        "cards": [{"question": "H2O?", "answer": "Water", "tags": ["chem"]}],
                    }
                ),
                encoding="utf-8",
            )
            output = root / "out" / "combined.apkg"

            self.assertEqual(apkggen.main([str(txt), str(js), "-o", str(output)]), 0)
            rows = self.read_collection(output)

            self.assertEqual(len(rows["notes"]), 3)
            self.assertEqual(len(rows["cards"]), 3)
            decks = json.loads(rows["col"][1])
            names = sorted(
                deck["name"] for deck in decks.values() if deck["name"] != "Default"
            )
            self.assertEqual(names, ["Science", "general trivia"])
            tags = [note[5] for note in rows["notes"]]
            self.assertTrue(any("chem" in tag for tag in tags))

    def test_default_output_name(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "my_cards.txt"
            source.write_text("Q::A", encoding="utf-8")
            self.assertEqual(apkggen.main([str(source)]), 0)
            self.assertTrue((root / "my_cards.apkg").exists())

    def test_missing_input_returns_error(self):
        with tempfile.TemporaryDirectory() as directory:
            bad = Path(directory) / "bad.txt"
            bad.write_text("no answer here", encoding="utf-8")
            self.assertEqual(apkggen.main([str(bad)]), 1)


if __name__ == "__main__":
    unittest.main()
