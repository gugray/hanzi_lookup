use super::entities::*;
use super::match_collector::*;
use super::analyzed_character::*;
use super::*;

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

pub struct Matcher {

}

impl Matcher {
    pub fn new() -> Matcher {
        Matcher {
            
        }
    }

    pub fn recognize(&self, strokes: &Vec<Stroke>, _collector: &MatchCollector) {
        let _analyzed_character = AnalyzedCharacter::from_strokes(strokes);
    }

}

