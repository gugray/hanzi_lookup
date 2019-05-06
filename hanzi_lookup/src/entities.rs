// One analyzed stroke
pub struct AnalyzedStroke<'a> {
    // The stroke's points
    pub points: &'a Vec<super::Point>,
    // Indexes of pivot points delimiting substrokes
    pub pivot_indexes: Vec<usize>,
    // The substrokes delineated by the identified pivot points
    pub sub_strokes: Vec<SubStroke>,
}

// A single analyzed substroke
#[derive(Debug, Clone, Copy)]
pub struct SubStroke {
    // The substroke's direction; normalized into 0..256 from 0..2*PI
    pub direction: f32,
    // The substroke's length, normalized into 0..256, from 0..1
    pub length: f32,
    // The substroke centerpoint's X coordinate, in 0..256
    pub center_x: f32,
    // The substroke centerpoint's Y coordinate, in 0..256
    pub center_y: f32,
}