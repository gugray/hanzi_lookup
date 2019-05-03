extern crate serde_derive;
extern crate hanzi_lookup;

use serde_derive::{Serialize, Deserialize};
use std::time::{Instant};
use std::fs::File;
use std::io::{BufRead, BufReader};
use hanzi_lookup::{Stroke, Point};

const ITERS: usize = 10_000;

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

fn read_inputs(fname: &str) -> Vec<Input> {
    let mut res: Vec<Input> = Vec::new();
    let file = File::open(fname).expect("Failed to open file.");
    for line in BufReader::new(file).lines() {
        let line = line.expect("Line huh?");
        if line.is_empty() { continue; }
        let json = &line[20..];
        let input: Input = serde_json::from_str(json).unwrap();
        res.push(input);
    }
    return res;
}

fn get_strokes(actions: &Vec<Action>) -> Vec<Stroke> {
    let mut strokes: Vec<Stroke> = Vec::with_capacity(actions.len());
    for action in actions {
        let mut points: Vec<Point> = Vec::with_capacity(action.points.len());
        for pt in &action.points {
            let point = Point {
                x: pt[0],
                y: pt[1],
            };
            points.push(point);
        }
        let stroke = Stroke {
            points: points,
        };
        strokes.push(stroke);
    }
    return strokes;
}

fn main() {
    println!("Loading evaluation data.");
    let inputs = read_inputs("debug/inputs.txt");
    println!("Loaded {} inputs; starting {} cycles of evaluation.", inputs.len(), ITERS);
    let start = Instant::now();
    let mut guessed = 0;
    for _ in 0..ITERS {
        for input in &inputs {
            let strokes = get_strokes(&input.actions);
            let matches = hanzi_lookup::match_typed(&strokes, 16);
            if matches.len() > 0 && matches[0].hanzi == input.char.chars().next().unwrap() {
                guessed += 1;
            }
        }
    }
    let duration = start.elapsed();
    println!("Finished in {:?}. Correct guesses: {}.", duration, guessed);
}
