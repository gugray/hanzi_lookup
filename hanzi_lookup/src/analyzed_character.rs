extern crate serde_derive;

use super::entities::*;
use super::*;

const MIN_SEGMENT_LENGTH: f32 = 12.5;
const MAX_LOCAL_LENGTH_RATIO: f32 = 1.1;
const MAX_RUNNING_LENGTH_RATIO: f32 = 1.09;

struct Rect {
    pub top: f32,
    pub bottom: f32,
    pub left: f32,
    pub right: f32,
}

pub struct AnalyzedCharacter<'a> {
    pub analyzed_strokes: Vec<AnalyzedStroke<'a>>,
    pub sub_stroke_count: usize,
}

impl<'a> AnalyzedCharacter<'a> {
    pub fn from_strokes(strokes: &Vec<Stroke>) -> AnalyzedCharacter {
        let bounding_rect = get_bounding_rect(strokes);
        let analyzed_strokes: Vec<AnalyzedStroke> = build_analyzed_strokes(strokes, &bounding_rect);
        let mut sub_stroke_count: usize = 0;
        for i in 0..analyzed_strokes.len() {
            sub_stroke_count += analyzed_strokes[i].sub_strokes.len();
        }
        AnalyzedCharacter {
            analyzed_strokes: analyzed_strokes,
            sub_stroke_count: sub_stroke_count,
        }
    }

    pub fn get_analyzed_strokes(&self) -> Vec<SubStroke> {
        let mut res: Vec<SubStroke> = Vec::with_capacity(self.sub_stroke_count);
        for i in 0..self.analyzed_strokes.len() {
            for j in 0..self.analyzed_strokes[i].sub_strokes.len() {
                res.push(self.analyzed_strokes[i].sub_strokes[j]);
            }
        }
        res
    }
}

// Gets distance between two points
fn dist(a: Point, b: Point) -> f32 {
    let dx = (a.x as f32) - (b.x as f32);
    let dy = (a.y as f32) - (b.y as f32);
    (dx * dx + dy * dy).sqrt()
}

// Gets normalized distance between two points
// Normalized based on bounding rectangle
fn norm_dist(a: Point, b: Point, bounding_rect: &Rect) -> f32 {
    let width = bounding_rect.right - bounding_rect.left;
    let height = bounding_rect.bottom - bounding_rect.top;
    // normalizer is a diagonal along a square with sides of size the larger dimension of the bounding box
    let dim_squared;
    if width > height { dim_squared = width * width; }
    else { dim_squared = height * height; }
    let normalizer = (dim_squared + dim_squared).sqrt();
    let dist_norm = dist(a, b) / normalizer;
    // Cap at 1 (...why is this needed??)
    f32::min(dist_norm, 1f32)
}

// Gets direction, in radians, from point a to b
// 0 is to the right, PI / 2 is up, etc.
fn dir(a: Point, b: Point) -> f32 {
    let dx = (a.x as f32) - (b.x as f32);
    let dy = (a.y as f32) - (b.y as f32);
    let dir = dy.atan2(dx);
    std::f32::consts::PI - dir
}

fn get_norm_center(a: Point, b: Point, bounding_rect: &Rect) -> (f32, f32) {
    let mut x = ((a.x as f32) + (b.x as f32)) / 2f32;
    let mut y = ((a.y as f32) + (b.y as f32)) / 2f32;
    let side;
    // Bounding rect is landscape
    if bounding_rect.right - bounding_rect.left > bounding_rect.bottom - bounding_rect.top {
        side = bounding_rect.right - bounding_rect.left;
        let height = bounding_rect.bottom - bounding_rect.top;
        x = x - bounding_rect.left;
        y = y - bounding_rect.top + (side - height) / 2f32;
    }
    // Portrait
    else {
        side = bounding_rect.bottom - bounding_rect.top;
        let width = bounding_rect.right - bounding_rect.left;
        x = x - bounding_rect.left + (side - width) / 2f32;
        y = y - bounding_rect.top;
    }
    (x / side, y / side)
}

// Calculates array with indexes of pivot points in raw stroke
fn get_pivot_indexes(stroke: &Stroke) -> Vec<usize> {

    let points = &stroke.points;

    // One item for each point: true if it's a pivot
    let mut markers: Vec<bool> = Vec::with_capacity(points.len());
    for _ in 0..points.len() { markers.push(false); }

    // Cycle variables
    let mut prev_pt_ix = 0;
    let mut first_pt_ix = 0;
    let mut pivot_pt_ix = 1;

    // The first point of a Stroke is always a pivot point.
    markers[0] = true;

    // localLength keeps track of the immediate distance between the latest three points.
    // We can use localLength to find an abrupt change in substrokes, such as at a corner.
    // We do this by checking localLength against the distance between the first and last
    // of the three points. If localLength is more than a certain amount longer than the
    // length between the first and last point, then there must have been a corner of some kind.
    let mut local_length = dist(points[first_pt_ix], points[pivot_pt_ix]);

    // runningLength keeps track of the length between the start of the current SubStroke
    // and the point we are currently examining.  If the runningLength becomes a certain
    // amount longer than the straight distance between the first point and the current
    // point, then there is a new SubStroke.  This accounts for a more gradual change
    // from one SubStroke segment to another, such as at a longish curve.
    let mut running_length = local_length;

    // Cycle through rest of stroke points.
    let mut i = 2;
    while i < points.len() {
        let next_point = points[i];

        // pivotPoint is the point we're currently examining to see if it's a pivot.
        // We get the distance between this point and the next point and add it
        // to the length sums we're using.
        let pivot_length = dist(points[pivot_pt_ix], next_point);
        local_length += pivot_length;
        running_length += pivot_length;

        // Check the lengths against the ratios.  If the lengths are a certain among
        // longer than a straight line between the first and last point, then we
        // mark the point as a pivot.
        let dist_from_previous = dist(points[prev_pt_ix], next_point);
        let dist_from_first = dist(points[first_pt_ix], next_point);
        if  local_length > MAX_LOCAL_LENGTH_RATIO * dist_from_previous || 
            running_length > MAX_RUNNING_LENGTH_RATIO * dist_from_first {
            // If the previous point was a pivot and was very close to this point,
            // which we are about to mark as a pivot, then unmark the previous point as a pivot.
            if markers[prev_pt_ix] && dist(points[prev_pt_ix], points[pivot_pt_ix]) < MIN_SEGMENT_LENGTH {
                markers[prev_pt_ix] = false;
            }
            markers[pivot_pt_ix] = true;
            running_length = pivot_length;
            first_pt_ix = pivot_pt_ix;
        }
        local_length = pivot_length;
        prev_pt_ix = pivot_pt_ix;
        pivot_pt_ix = i;

        i += 1;
    }

    // last point (currently referenced by pivotPoint) has to be a pivot
    markers[pivot_pt_ix] = true;
    // Point before the final point may need to be handled specially.
    // Often mouse action will produce an unintended small segment at the end.
    // We'll want to unmark the previous point if it's also a pivot and very close to the lat point.
    // However if the previous point is the first point of the stroke, then don't unmark it, because
    // then we'd only have one pivot.
    if markers[prev_pt_ix] && dist(points[prev_pt_ix], points[pivot_pt_ix]) < MIN_SEGMENT_LENGTH && prev_pt_ix != 0 {
        markers[prev_pt_ix] = false;
    }

    // Return result in the form of an index array: includes indexes where marker is true
    let mut marker_count = 0;
    for x in &markers {
        if *x {
            marker_count += 1;
        }
    }
    let mut res: Vec<usize> = Vec::with_capacity(marker_count);
    for ix in 0..markers.len() {
        if markers[ix] {
            res.push(ix);
        }
    }
    res
}

// Builds array of substrokes from stroke's points, pivots, and character's bounding rectangle
fn build_sub_strokes(stroke: &Stroke, pivot_indexes: &Vec<usize>, bounding_rect: &Rect) -> Vec<SubStroke> {
    let mut res: Vec<SubStroke> = Vec::new();
    let mut prev_ix: usize = 0;
    for i in 0..pivot_indexes.len() {
        let ix = pivot_indexes[i];
        if ix == prev_ix { continue; }
        let mut direction = dir(stroke.points[prev_ix], stroke.points[ix]);
        direction = (direction * 256f32 / std::f32::consts::PI / 2f32).round();
        if direction >= 256f32 { direction = 0f32; }
        let mut norm_length = norm_dist(stroke.points[prev_ix], stroke.points[ix], bounding_rect);
        norm_length = (norm_length * 255f32).round();
        let center = get_norm_center(stroke.points[prev_ix], stroke.points[ix], bounding_rect);
        res.push(SubStroke {
            direction: direction,
            length: norm_length,
            center_x: (center.0 * 15f32).round(),
            center_y: (center.1 * 15f32).round(),
        });
        prev_ix = ix;
    }
    res
}

// Analyze raw input, store result in _analyzedStrokes member.
fn build_analyzed_strokes<'a>(strokes: &'a Vec<Stroke>, bounding_rect: &Rect) -> Vec<AnalyzedStroke<'a>> {
    let mut res: Vec<AnalyzedStroke> = Vec::new();
    // Process each stroke
    for stroke in strokes {
        // Identify pivot points
        let pivot_indexes = get_pivot_indexes(stroke);
        // Abstract away substrokes
        let sub_strokes = build_sub_strokes(stroke, &pivot_indexes, bounding_rect);
        // Store all this
        res.push(AnalyzedStroke{
            points: &stroke.points,
            pivot_indexes: pivot_indexes,
            sub_strokes: sub_strokes,
        });
    }
    res
}

fn get_bounding_rect(strokes: &Vec<Stroke>) -> Rect {
    let mut res = Rect {
        top: std::f32::MAX,
        bottom: std::f32::MIN,
        left: std::f32::MAX,
        right: std::f32::MIN,
    };
    for stroke in strokes {
        for pt in &stroke.points {
            if (pt.x as f32) < res.left { res.left = pt.x as f32; }
            if (pt.x as f32) > res.right { res.right = pt.x as f32; }
            if (pt.y as f32) < res.top { res.top = pt.y as f32; }
            if (pt.y as f32) > res.bottom { res.bottom = pt.y as f32; }
        }
    }
    if res.top > 255f32 { res.top = 0f32; }
    if res.bottom < 0f32 { res.bottom = 255f32; }
    if res.left > 255f32 { res.left = 0f32; }
    if res.right < 0f32 { res.right = 255f32; }
    res
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    use serde_derive::{Serialize, Deserialize};

    use super::*;
    use super::super::Point;

    #[derive(Serialize, Deserialize)]
    struct SampleAnSubStroke {
        direction: u8,
        length: u8,
        centerX: u8,
        centerY: u8,
    }

    #[derive(Serialize, Deserialize)]
    struct SampleAnStroke {
        points: Vec<Vec<u8>>,
        pivotIndexes: Vec<usize>,
        subStrokes: Vec<SampleAnSubStroke>,
    }

    #[derive(Serialize, Deserialize)]
    struct SampleAnChar {
        top: u8,
        bottom: u8,
        left: u8,
        right: u8,
        analyzedStrokes: Vec<SampleAnStroke>,
        subStrokeCount: usize,
    }

    // These manual samples are custom-saved from a tweaked version of the HanziLookupJS demo

    // This is a hand-drawn 一
    static STROKES_1: &str = "[[[70,124],[71,124],[79,124],[104,124],[119,124],[132,125],[151,126],[168,126],[169,126],[189,125],[191,124],[191,124]]]";
    static AN_CHAR_1: &str = "{\"top\":124,\"bottom\":126,\"left\":70,\"right\":191,\"analyzedStrokes\":[{\"points\":[[70,124],[71,124],[79,124],[104,124],[119,124],[132,125],[151,126],[168,126],[169,126],[189,125],[191,124],[191,124]],\"pivotIndexes\":[0,11],\"subStrokes\":[{\"direction\":0,\"length\":180,\"centerX\":8,\"centerY\":7}]}],\"subStrokeCount\":1}";

    // This is a hand-drawn 十
    static STROKES_2: &str = "[[[76,127],[77,127],[84,127],[97,128],[119,128],[125,129],[138,130],[147,130],[153,131],[154,131],[158,131],[162,131],[167,131],[168,131],[169,131],[169,131]],[[129,60],[129,62],[128,74],[128,102],[128,118],[129,143],[130,162],[130,170],[130,178],[131,184],[131,188],[131,193],[131,196],[131,198],[131,203],[131,203]]]";
    static AN_CHAR_2: &str = "{\"top\":60,\"bottom\":203,\"left\":76,\"right\":169,\"analyzedStrokes\":[{\"points\":[[76,127],[77,127],[84,127],[97,128],[119,128],[125,129],[138,130],[147,130],[153,131],[154,131],[158,131],[162,131],[167,131],[168,131],[169,131],[169,131]],\"pivotIndexes\":[0,15],\"subStrokes\":[{\"direction\":254,\"length\":117,\"centerX\":8,\"centerY\":7}]},{\"points\":[[129,60],[129,62],[128,74],[128,102],[128,118],[129,143],[130,162],[130,170],[130,178],[131,184],[131,188],[131,193],[131,196],[131,198],[131,203],[131,203]],\"pivotIndexes\":[0,15],\"subStrokes\":[{\"direction\":193,\"length\":180,\"centerX\":8,\"centerY\":8}]}],\"subStrokeCount\":2}";

    // This is a hand-drawn 元
    static STROKES_3: &str = "[[[86,65],[98,66],[146,69],[152,69],[161,69],[166,69],[170,68],[170,68]],[[47,97],[48,97],[54,97],[89,103],[117,104],[146,101],[169,100],[176,98],[180,98],[184,98],[189,98],[193,98],[195,98],[195,98]],[[103,109],[103,110],[99,132],[91,156],[70,180],[56,190],[53,192]],[[143,105],[143,106],[142,114],[140,134],[138,149],[138,160],[138,167],[140,174],[144,182],[150,186],[155,190],[161,193],[166,194],[172,196],[188,197],[193,197],[197,197],[206,197],[206,196],[207,196],[208,196],[208,194],[204,182],[203,174],[202,174],[202,175],[202,176]]]";
    static AN_CHAR_3: &str = "{\"top\":65,\"bottom\":197,\"left\":47,\"right\":208,\"analyzedStrokes\":[{\"points\":[[86,65],[98,66],[146,69],[152,69],[161,69],[166,69],[170,68],[170,68]],\"pivotIndexes\":[0,7],\"subStrokes\":[{\"direction\":255,\"length\":94,\"centerX\":8,\"centerY\":1}]},{\"points\":[[47,97],[48,97],[54,97],[89,103],[117,104],[146,101],[169,100],[176,98],[180,98],[184,98],[189,98],[193,98],[195,98],[195,98]],\"pivotIndexes\":[0,13],\"subStrokes\":[{\"direction\":0,\"length\":166,\"centerX\":7,\"centerY\":4}]},{\"points\":[[103,109],[103,110],[99,132],[91,156],[70,180],[56,190],[53,192]],\"pivotIndexes\":[0,6],\"subStrokes\":[{\"direction\":170,\"length\":109,\"centerX\":3,\"centerY\":9}]},{\"points\":[[143,105],[143,106],[142,114],[140,134],[138,149],[138,160],[138,167],[140,174],[144,182],[150,186],[155,190],[161,193],[166,194],[172,196],[188,197],[193,197],[197,197],[206,197],[206,196],[207,196],[208,196],[208,194],[204,182],[203,174],[202,174],[202,175],[202,176]],\"pivotIndexes\":[0,10,18,20,24,26],\"subStrokes\":[{\"direction\":198,\"length\":96,\"centerX\":10,\"centerY\":9},{\"direction\":251,\"length\":58,\"centerX\":12,\"centerY\":13},{\"direction\":0,\"length\":2,\"centerX\":15,\"centerY\":14},{\"direction\":75,\"length\":26,\"centerX\":15,\"centerY\":13},{\"direction\":192,\"length\":2,\"centerX\":14,\"centerY\":12}]}],\"subStrokeCount\":8}";

    // This is a hand-drawn 氣
    static STROKES_4: &str = "[[[76,32],[76,33],[75,37],[73,43],[70,51],[67,58],[64,66],[61,72],[57,77],[52,82],[50,85],[50,85]],[[68,58],[69,58],[76,58],[90,59],[100,60],[110,62],[118,62],[132,62],[136,62],[141,62],[145,62],[146,62],[148,62],[148,62]],[[68,95],[69,95],[77,96],[96,96],[105,96],[110,96],[126,97],[144,98],[146,98],[154,98],[156,98],[156,98]],[[59,126],[60,126],[67,126],[90,130],[107,131],[120,132],[134,132],[149,132],[151,132],[156,132],[158,133],[158,134],[156,142],[154,147],[153,155],[152,160],[151,166],[150,172],[150,179],[150,183],[150,186],[150,190],[151,194],[152,199],[156,204],[158,206],[162,209],[167,213],[171,215],[175,216],[184,220],[192,222],[196,223],[200,224],[204,225],[208,225],[210,225],[214,225],[218,223],[218,222],[216,214],[214,208],[214,207],[214,207]],[[79,147],[82,148],[87,155],[91,161],[91,161]],[[124,148],[123,148],[116,155],[110,162],[108,164],[108,164]],[[73,175],[75,175],[88,178],[98,180],[104,180],[111,182],[117,182],[122,182],[125,182]],[[100,148],[100,151],[102,172],[102,195],[103,204],[103,211],[104,216],[104,220],[104,224]],[[94,189],[93,189],[81,204],[72,210],[71,210]],[[109,192],[112,194],[120,199],[132,208],[133,210],[133,210]]]";
    static AN_CHAR_4: &str = "{\"top\":32,\"bottom\":225,\"left\":50,\"right\":218,\"analyzedStrokes\":[{\"points\":[[76,32],[76,33],[75,37],[73,43],[70,51],[67,58],[64,66],[61,72],[57,77],[52,82],[50,85],[50,85]],\"pivotIndexes\":[0,11],\"subStrokes\":[{\"direction\":173,\"length\":55,\"centerX\":2,\"centerY\":2}]},{\"points\":[[68,58],[69,58],[76,58],[90,59],[100,60],[110,62],[118,62],[132,62],[136,62],[141,62],[145,62],[146,62],[148,62],[148,62]],\"pivotIndexes\":[0,13],\"subStrokes\":[{\"direction\":254,\"length\":75,\"centerX\":5,\"centerY\":2}]},{\"points\":[[68,95],[69,95],[77,96],[96,96],[105,96],[110,96],[126,97],[144,98],[146,98],[154,98],[156,98],[156,98]],\"pivotIndexes\":[0,11],\"subStrokes\":[{\"direction\":255,\"length\":82,\"centerX\":6,\"centerY\":5}]},{\"points\":[[59,126],[60,126],[67,126],[90,130],[107,131],[120,132],[134,132],[149,132],[151,132],[156,132],[158,133],[158,134],[156,142],[154,147],[153,155],[152,160],[151,166],[150,172],[150,179],[150,183],[150,186],[150,190],[151,194],[152,199],[156,204],[158,206],[162,209],[167,213],[171,215],[175,216],[184,220],[192,222],[196,223],[200,224],[204,225],[208,225],[210,225],[214,225],[218,223],[218,222],[216,214],[214,208],[214,207],[214,207]],\"pivotIndexes\":[0,10,26,39,43],\"subStrokes\":[{\"direction\":253,\"length\":93,\"centerX\":6,\"centerY\":8},{\"direction\":194,\"length\":71,\"centerX\":10,\"centerY\":11},{\"direction\":247,\"length\":54,\"centerX\":12,\"centerY\":14},{\"direction\":75,\"length\":15,\"centerX\":14,\"centerY\":14}]},{\"points\":[[79,147],[82,148],[87,155],[91,161],[91,161]],\"pivotIndexes\":[0,4],\"subStrokes\":[{\"direction\":221,\"length\":17,\"centerX\":4,\"centerY\":9}]},{\"points\":[[124,148],[123,148],[116,155],[110,162],[108,164],[108,164]],\"pivotIndexes\":[0,5],\"subStrokes\":[{\"direction\":160,\"length\":21,\"centerX\":6,\"centerY\":10}]},{\"points\":[[73,175],[75,175],[88,178],[98,180],[104,180],[111,182],[117,182],[122,182],[125,182]],\"pivotIndexes\":[0,8],\"subStrokes\":[{\"direction\":251,\"length\":49,\"centerX\":5,\"centerY\":11}]},{\"points\":[[100,148],[100,151],[102,172],[102,195],[103,204],[103,211],[104,216],[104,220],[104,224]],\"pivotIndexes\":[0,8],\"subStrokes\":[{\"direction\":194,\"length\":71,\"centerX\":5,\"centerY\":12}]},{\"points\":[[94,189],[93,189],[81,204],[72,210],[71,210]],\"pivotIndexes\":[0,4],\"subStrokes\":[{\"direction\":158,\"length\":29,\"centerX\":3,\"centerY\":13}]},{\"points\":[[109,192],[112,194],[120,199],[132,208],[133,210],[133,210]],\"pivotIndexes\":[0,5],\"subStrokes\":[{\"direction\":230,\"length\":28,\"centerX\":6,\"centerY\":13}]}],\"subStrokeCount\":13}";

    fn parse_sample(str_strokes: &str, str_an_char: &str) -> (Vec<Stroke>, SampleAnChar) {
        let vec_strokes: Vec<Vec<Vec<u8>>> = serde_json::from_str(str_strokes).unwrap();
        let mut strokes: Vec<Stroke> = Vec::new();
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
        let an_char: SampleAnChar = serde_json::from_str(str_an_char).unwrap();
        (strokes, an_char)
    }

    fn assert_same(sample_anc: &SampleAnChar, anc: &AnalyzedCharacter) {
        assert!(sample_anc.analyzedStrokes.len() == anc.analyzed_strokes.len(), "Expected same number of analyzed strokes.");
        for as_ix in 0..sample_anc.analyzedStrokes.len() {
            // Reference same analyzed stroke
            let sample_ans = &sample_anc.analyzedStrokes[as_ix];
            let ans = &anc.analyzed_strokes[as_ix];
            // Analyze stroke must have same points
            assert!(sample_ans.points.len() == ans.points.len(), "Analyzed stroke expected to have same number of points.");
            for pt_ix in 0..sample_ans.points.len() {
                assert!(sample_ans.points[pt_ix][0] == ans.points[pt_ix].x, "Analyzed stroke expected to have the exact same points.");
                assert!(sample_ans.points[pt_ix][1] == ans.points[pt_ix].y, "Analyzed stroke expected to have the exact same points.");
            }
            // Analyze stroke must have same pivot indexes
            assert!(sample_ans.pivotIndexes.len() == ans.pivot_indexes.len(), "Analyzed stroke expected to have same number of pivot indexes.");
            for pivot_ix in 0..sample_ans.pivotIndexes.len() {
                assert!(sample_ans.pivotIndexes[pivot_ix] == ans.pivot_indexes[pivot_ix], "Analyzed stroke expected to have the exact same pivot indexes.");
            }
            // Analyze stroke must have same substrokes
            assert!(sample_ans.subStrokes.len() == ans.sub_strokes.len(), "Analyzed stroke expected to have same number of substrokes.");
            for ss_ix in 0..sample_ans.subStrokes.len() {
                assert!(sample_ans.subStrokes[ss_ix].direction as f32 == ans.sub_strokes[ss_ix].direction,
                        "Analyzed stroke must have the exact same substrokes.");
                assert!(sample_ans.subStrokes[ss_ix].length as f32 == ans.sub_strokes[ss_ix].length,
                        "Analyzed stroke must have the exact same substrokes.");
                assert!(sample_ans.subStrokes[ss_ix].centerX as f32 == ans.sub_strokes[ss_ix].center_x,
                        "Analyzed stroke must have the exact same substrokes.");
                assert!(sample_ans.subStrokes[ss_ix].centerY as f32 == ans.sub_strokes[ss_ix].center_y,
                        "Analyzed stroke must have the exact same substrokes.");
            }
        }
    }

    #[test]
    fn test_samples() {
        {
            let sample = parse_sample(STROKES_1, AN_CHAR_1);
            let ac = AnalyzedCharacter::from_strokes(&sample.0);
            assert_same(&sample.1, &ac);
        }
        {
            let sample = parse_sample(STROKES_2, AN_CHAR_2);
            let ac = AnalyzedCharacter::from_strokes(&sample.0);
            assert_same(&sample.1, &ac);
        }
        {
            let sample = parse_sample(STROKES_3, AN_CHAR_3);
            let ac = AnalyzedCharacter::from_strokes(&sample.0);
            assert_same(&sample.1, &ac);
        }
        {
            let sample = parse_sample(STROKES_4, AN_CHAR_4);
            let ac = AnalyzedCharacter::from_strokes(&sample.0);
            assert_same(&sample.1, &ac);
        }
    }
}
