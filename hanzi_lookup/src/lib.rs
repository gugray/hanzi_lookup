#![allow(dead_code)]
#![allow(unused_imports)]

extern crate wasm_bindgen;
extern crate serde_derive;
extern crate bincode;

mod match_collector;
mod entities;
mod analyzed_character;
mod matcher;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};

use match_collector::*;
use analyzed_character::*;
use matcher::*;


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
pub fn barf(_input: &JsValue) -> String {
    // let input: Input = input.into_serde().unwrap();
    // let mut char_data_len: usize = 0;
    // CHAR_DATA.with(|char_data| { 
    //     char_data_len = char_data.len();
    // });
    // let res = format!("Got {} actions in input.\nThere are {} characters in recognition data.", input.actions.len(), char_data_len);
    // res
    String::from("Sure thing.")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: u8,
    pub y: u8,
}

#[derive(Debug)]
pub struct Stroke {
    pub points: Vec<Point>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub hanzi: char,
    pub score: f32,
}

thread_local!(static MATCHER: Matcher = Matcher::new());

pub fn match_typed(strokes: &Vec<Stroke>, max: u32) -> Vec<Match> {
    let mut res: Vec<Match> = Vec::with_capacity(max as usize);
    let mut collector = MatchCollector::new(&mut res, max);
    MATCHER.with(|matcher| {
        matcher.recognize(strokes, &collector);
    });
    
    let mc = Match {
        hanzi: '雞',
        score: 0.99,
    };
    collector.file_match(mc);
    res
}
