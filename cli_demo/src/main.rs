extern crate serde_derive;
extern crate hanzi_lookup;

use std::time::{Instant};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::fmt::Write;
use hanzi_lookup::{Stroke, Point};

const ITERS: usize = 10;


fn parse_sample(str_strokes: &str) -> Vec<Stroke> {
    let vec_strokes: Vec<Vec<Vec<u8>>> = serde_json::from_str(str_strokes).unwrap();
    let mut strokes: Vec<Stroke>  = Vec::new();
    for vec_stroke in &vec_strokes {
        let mut points: Vec<Point> = Vec::new();
        for vec_point in vec_stroke {
            points.push(Point {
                x: vec_point[0],
                y: vec_point[1],
            });
        }
        strokes.push(Stroke {
            points: points,
        });
    }
    strokes
}

fn read_inputs(fname: &str) -> Vec<Vec<Stroke>> {
    let mut res: Vec<Vec<Stroke>> = Vec::new();
    let file = File::open(fname).expect("Failed to open file.");
    for line in BufReader::new(file).lines() {
        let line = line.expect("Line huh?");
        if line.is_empty() { continue; }
        let strokes = parse_sample(&line);
        res.push(strokes);
    }
    return res;
}

fn clone_stroke(stroke: &Stroke) -> Stroke {
    let mut res = Stroke {
        points: Vec::with_capacity(stroke.points.len()),
    };
    for i in 0..stroke.points.len() {
        res.points.push(Point {
            x: stroke.points[i].x,
            y: stroke.points[i].y,
        });
    }
    res
}

fn incremental_replay(chars: &Vec<Vec<Stroke>>) -> Vec<Vec<Stroke>> {
    let mut res: Vec<Vec<Stroke>> = Vec::new();
    for i in 0..chars.len() {
        let this_char = &chars[i];
        for j in 1..this_char.len() {
            res.push(Vec::new());
            let strokes: &mut Vec<Stroke> = res.last_mut().unwrap();
            for k in 0 ..j {
                strokes.push(clone_stroke(&this_char[k]));
            }
        }
    }
    res
}

fn main() {
    println!("Loading evaluation data.");
    let inputs = read_inputs("debug/inputs.txt");
    println!("Loaded {} inputs.", inputs.len());
    let inputs = incremental_replay(&inputs);
    println!("Generated {} inputs with stroke-by-stroke replay of characters.", inputs.len());
    println!("Running {} lookup iterations.", ITERS);
    for _ in 0..ITERS {
        for input in &inputs {
            let start = Instant::now();
            let matches = hanzi_lookup::match_typed(&input, 8);
            let duration = start.elapsed();
            let mut chars = String::new();
            for i in 0..matches.len() {
                write!(&mut chars, "{}", matches[i].hanzi).unwrap();
            }
            println!("{:?}    {} strokes   Chars: {}", duration, input.len(), chars);
        }
    }
}
