extern crate wasm_bindgen;
extern crate serde_derive;
extern crate bincode;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};

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


thread_local!(static CHAR_DATA: Vec<CharData> = load_strokes());

fn load_strokes() -> Vec<CharData> {
    let hwbytes = include_bytes!("../data/mmah.bin");
    let reader = std::io::BufReader::new(&hwbytes[..]);
    let res = bincode::deserialize_from(reader).expect("Failed to deserialize.");
    res
}


#[wasm_bindgen]
pub fn barf(_arg: &str, _width: u32, _height: u32) -> i32 {
    let mut res: i32 = 0;
    CHAR_DATA.with(|char_data| { 
        res = char_data.len() as i32;
    });
    res
}
