extern crate wasm_bindgen;
extern crate serde_derive;
extern crate bincode;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};

mod match_collector;
use match_collector::*;

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

#[derive(Serialize, Deserialize)]
struct Action {
    action: String,
    points: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize)]
struct Input {
    char: String,
    ix: i64,
    duration: i64,
    actions: Vec<Action>,
}


#[wasm_bindgen]
pub fn barf(input: &JsValue) -> String {
    let input: Input = input.into_serde().unwrap();
    let mut char_data_len: usize = 0;
    CHAR_DATA.with(|char_data| { 
        char_data_len = char_data.len();
    });
    let res = format!("Got {} actions in input.\nThere are {} characters in recognition data.", input.actions.len(), char_data_len);
    res
}

pub struct Point {
  pub x: u8,
  pub y: u8,
}

pub struct Stroke {
  pub points: Vec<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
  pub hanzi: char,
  pub score: f32,
}

pub fn match_typed(strokes: &Vec<Stroke>, max: u32) -> Vec<Match> {
  let mut res: Vec<Match> = Vec::with_capacity(max as usize);
  let mut collector = MatchCollector::new(&mut res, max);
  let mc = Match {
    hanzi: '雞',
    score: 0.99,
  };
  collector.file_match(mc);
  res
}
