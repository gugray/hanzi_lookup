extern crate serde_derive;
extern crate base64;
extern crate bincode;

use serde_derive::{Serialize, Deserialize};
use serde_json::{Result, Value};
use std::fs::File;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct SubStrokeTriple {
	dir: u8,
	length: u8,
	center: u8,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct CharData {
	hanzi: char,
	stroke_count: u16,
	substrokes: Vec<SubStrokeTriple>,
}

fn parse_json_strokes(fname: &str) -> Result<Vec<CharData>> {
    let file = File::open(fname)
        .expect("file should open read only");
    let json: serde_json::Value = serde_json::from_reader(file)
        .expect("file should be proper JSON");

    let mut res: Vec<CharData> = Vec::new();    

    // "substrokes" member of json is one u8 blob in base64
    let mut bytes: Vec<u8> = Vec::new();
    if let Value::String(substrokes) = &json["substrokes"] {
        bytes = base64::decode(&substrokes).unwrap();
    }

    // "chars" member of json lists concise info about characters
    if let Value::Array(chars) = &json["chars"] {
        for x in chars {
            // Each character is an array of four items like this: ["丿",1,2,0]
            // Character / Stroke Count / Substroke Count / First-substroke-index in byte array
            let mut char_data = CharData {
                hanzi: ' ',
                stroke_count: 0,
                substrokes: Vec::new(),
            };
            // Get our character
            if let Value::String(chr) = &x[0] {
                let first_char = chr.chars().next().unwrap();
                char_data.hanzi = first_char;
            }
            // Stroke count
            if let Value::Number(val) = &x[1] {
                char_data.stroke_count = val.as_u64().unwrap() as u16;
            }
            // Substroke count
            let mut substroke_count: u64 = 0;
            if let Value::Number(val) = &x[2] {
                substroke_count = val.as_u64().unwrap();
            }
            // Start in byte array
            let mut start_ix: u64 = 0;
            if let Value::Number(val) = &x[3] {
                start_ix = val.as_u64().unwrap();
            }
            // Copy out the relevant triplets (as many as there are substrokes)
            for cnt in 0..substroke_count {
                let sst = SubStrokeTriple {
                    dir: bytes[(start_ix + cnt * 3) as usize],
                    length: bytes[(start_ix + cnt * 3 + 1) as usize],
                    center: bytes[(start_ix + cnt * 3 + 2) as usize],
                };
                char_data.substrokes.push(sst);
            }
            // Append to result
            res.push(char_data);
        }
    }
    Ok(res)
}

fn main() {
    let char_data = parse_json_strokes("./data/mmah.json").expect("Failed to parse json.");
    let mut f = File::create("./data/mmah.bin").expect("Failed to create binary file.");
    bincode::serialize_into(&mut f, &char_data).expect("Failed to serialize into binary file.");
}
