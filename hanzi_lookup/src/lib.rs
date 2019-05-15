#![allow(dead_code)]
#![allow(unused_imports)]

extern crate wasm_bindgen;
extern crate serde_derive;
extern crate bincode;

mod analyzed_character;
mod cubic_curve_2d;
mod entities;
mod match_collector;
mod matcher;

use serde_derive::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;

use match_collector::*;
use analyzed_character::*;
use match_collector::*;
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
pub fn lookup(input: &JsValue, limit: usize) -> String {
    // Input is vector of vector of vector of numbers - how strokes and their points are represented in JS
    let input: Vec<Vec<Vec<f32>>> = input.into_serde().unwrap();
    // Convert to typed form: vector of strokes
    let mut strokes: Vec<Stroke> = Vec::with_capacity(input.len());
    for i in 0..input.len() {
        let mut stroke = Stroke {
            points: Vec::with_capacity(input[i].len()),
        };
        for j in 0..input[i].len() {
            stroke.points.push(Point {
                x: input[i][j][0].round() as u8,
                y: input[i][j][1].round() as u8,
            });
        }
        strokes.push(stroke);
    }
    let lookup_res = match_typed(&strokes, limit);
    serde_json::to_string(&lookup_res).unwrap()
    // let mut res = String::new();
    // res
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct Match {
    pub hanzi: char,
    pub score: f32,
}

thread_local!(static MATCHER: RefCell<Matcher> = RefCell::new(Matcher::new()));

pub fn match_typed(strokes: &Vec<Stroke>, limit: usize) -> Vec<Match> {
    let mut res: Vec<Match> = Vec::with_capacity(limit);
    let mut collector = MatchCollector::new(&mut res, limit);
    MATCHER.with(|matcher| {
        matcher.borrow_mut().lookup(strokes, &mut collector);
    });
    res
}
