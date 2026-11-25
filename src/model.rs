use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::NoteType;

const BASE91: &[u8; 91] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!#$%&()*+,-./:;<=>?@[]^_`{|}~";

const LATEX_PRE: &str = "\\documentclass[12pt]{article}\n\\special{papersize=3in,5in}\n\\usepackage[utf8]{inputenc}\n\\usepackage{amssymb,amsmath}\n\\pagestyle{empty}\n\\setlength{\\parindent}{0in}\n\\begin{document}\n";

pub fn stable_id(label: &str) -> i64 {
    let digest = Sha256::digest(label.as_bytes());
    let mut number = 0u64;
    for byte in digest.iter().take(8) {
        number = (number << 8) | u64::from(*byte);
    }
    (number % 999_999_999) as i64 + 1
}

pub fn guid(fields: &[String; 2]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(fields[0].as_bytes());
    hasher.update(b"__");
    hasher.update(fields[1].as_bytes());
    let digest = hasher.finalize();

    let mut number = 0u64;
    for byte in digest.iter().take(8) {
        number = (number << 8) | u64::from(*byte);
    }

    let mut reversed = [0u8; 16];
    let mut length = 0;
    while number > 0 {
        reversed[length] = BASE91[(number % 91) as usize];
        length += 1;
        number /= 91;
    }
    let mut result = String::with_capacity(length);
    for byte in reversed[..length].iter().rev() {
        result.push(char::from(*byte));
    }
    result
}

fn field(name: &str, ord: usize) -> Value {
    json!({
        "name": name,
        "ord": ord,
        "font": "Liberation Sans",
        "media": [],
        "rtl": false,
        "size": 20,
        "sticky": false,
    })
}

pub fn model_json(note_type: NoteType, model_id: i64, deck_id: i64, timestamp: i64) -> Value {
    match note_type {
        NoteType::Basic => json!({
            "css": "",
            "did": deck_id,
            "flds": [field("Question", 0), field("Answer", 1)],
            "id": model_id.to_string(),
            "latexPost": "\\end{document}",
            "latexPre": LATEX_PRE,
            "latexsvg": false,
            "mod": timestamp,
            "name": "apkggen Basic",
            "req": [[0, "all", [0]]],
            "sortf": 0,
            "tags": [],
            "tmpls": [{
                "name": "Card 1",
                "qfmt": "{{Question}}",
                "afmt": "{{FrontSide}}<hr id=\"answer\">{{Answer}}",
                "ord": 0,
                "bafmt": "",
                "bqfmt": "",
                "bfont": "",
                "bsize": 0,
                "did": null,
            }],
            "type": 0,
            "usn": -1,
            "vers": [],
        }),
        NoteType::Cloze => json!({
            "css": "",
            "did": deck_id,
            "flds": [field("Text", 0), field("Extra", 1)],
            "id": model_id.to_string(),
            "latexPost": "\\end{document}",
            "latexPre": LATEX_PRE,
            "latexsvg": false,
            "mod": timestamp,
            "name": "apkggen Cloze",
            "req": [[0, "all", [0, 1]]],
            "sortf": 0,
            "tags": [],
            "tmpls": [{
                "name": "Cloze",
                "qfmt": "{{cloze:Text}}",
                "afmt": "{{cloze:Text}}<br>{{Extra}}",
                "ord": 0,
                "bafmt": "",
                "bqfmt": "",
                "bfont": "",
                "bsize": 0,
                "did": null,
            }],
            "type": 1,
            "usn": -1,
            "vers": [],
        }),
    }
}

pub fn default_deck_json() -> Value {
    json!({
        "collapsed": false,
        "conf": 1,
        "desc": "",
        "dyn": 0,
        "extendNew": 10,
        "extendRev": 50,
        "id": 1,
        "lrnToday": [0, 0],
        "mod": 1425279151,
        "name": "Default",
        "newToday": [0, 0],
        "revToday": [0, 0],
        "timeToday": [0, 0],
        "usn": 0,
    })
}

pub fn deck_json(name: &str, deck_id: i64) -> Value {
    json!({
        "collapsed": false,
        "conf": 1,
        "desc": "",
        "dyn": 0,
        "extendNew": 0,
        "extendRev": 50,
        "id": deck_id,
        "lrnToday": [163, 2],
        "mod": 1425278051,
        "name": name,
        "newToday": [163, 2],
        "revToday": [163, 0],
        "timeToday": [163, 23598],
        "usn": -1,
    })
}
