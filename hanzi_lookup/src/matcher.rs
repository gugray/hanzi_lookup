use super::entities::*;
use super::cubic_curve_2d::*;
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
    sub_strokes: Vec<SubStrokeTriple>,
}

thread_local!(static CHAR_DATA: Vec<CharData> = load_strokes());

fn load_strokes() -> Vec<CharData> {
    let hwbytes = include_bytes!("../data/mmah.bin");
    let reader = std::io::BufReader::new(&hwbytes[..]);
    let res = bincode::deserialize_from(reader).expect("Failed to deserialize.");
    res
}

trait MatcherParams {
    const MAX_CHARACTER_STROKE_COUNT: usize = 48;
    const MAX_CHARACTER_SUB_STROKE_COUNT: usize = 64;
    const DEFAULT_LOOSENESS: f32 = 0.15;
    const AVG_SUBSTROKE_LENGTH: f32 = 0.33; // an average length (out of 1)
    const SKIP_PENALTY_MULTIPLIER: f32 = 1.75; // penalty mulitplier for skipping a stroke
    const CORRECT_NUM_STROKES_BONUS: f32 = 0.1; // max multiplier bonus if characters has the correct number of strokes
    const CORRECT_NUM_STROKES_CAP: usize = 10; // characters with more strokes than this will not be multiplied
}

pub struct Matcher {
    // N*N dimensional matrix where N = MAX_CHARACTER_SUB_STROKE_COUNT + 1
    score_matrix: Vec<Vec<f32>>,
    // Values pre-computed as solutions of a 2D quadratic curve
    direction_score_table: Vec<f32>,
    // Values pre-computed as solutions of a 2D quadratic curve
    length_score_table: Vec<f32>,
    // Values pre-computed as solutions of a 2D quadratic curve
    pos_score_table: Vec<f32>,
}

impl MatcherParams for Matcher {}

impl Matcher {
    pub fn new() -> Matcher {
        let mut res = Matcher {
            score_matrix: Vec::with_capacity(Matcher::MAX_CHARACTER_SUB_STROKE_COUNT + 1),
            direction_score_table: Vec::with_capacity(256),
            length_score_table: Vec::with_capacity(129),
            pos_score_table: Vec::with_capacity(450),
        };
        init_score_tables(&mut res.direction_score_table, &mut res.length_score_table, &mut res.pos_score_table);
        res
    }

    pub fn lookup(&mut self, strokes: &Vec<Stroke>, collector: &mut MatchCollector) {
        init_score_matrix(&mut self.score_matrix);
        let input_char = AnalyzedCharacter::from_strokes(strokes);

        // Edge case: empty input should return no matches; but permissive lookup does find a few...
        if input_char.analyzed_strokes.len() == 0 {
            return;
        }

        CHAR_DATA.with(|char_data| {

            // Flat format: matching needs this. Only transform once.
            let input_sub_strokes = input_char.get_analyzed_strokes();

            // Some pre-computed looseness magic
            let stroke_count = input_char.analyzed_strokes.len();
            let sub_stroke_count = input_char.sub_stroke_count;
            // Get the range of strokes to compare against based on the loosness.
            // Characters with fewer strokes than stroke_count - stroke_range
            // or more than stroke_count + stroke_range won't even be considered.
            let stroke_range = get_strokes_range(stroke_count, Matcher::DEFAULT_LOOSENESS);
            let minimum_strokes = usize::max(stroke_count - stroke_range, 1);
            let maximum_strokes = usize::min(stroke_count + stroke_range, Matcher::MAX_CHARACTER_STROKE_COUNT);
            // Get the range of substrokes to compare against based on looseness.
            // When trying to match sub stroke patterns, won't compare sub strokes
            // that are farther about in sequence than this range.  This is to make
            // computing matches less expensive for low loosenesses.
            let sub_strokes_range = get_sub_strokes_range(sub_stroke_count, Matcher::DEFAULT_LOOSENESS);
            let min_sub_strokes = usize::max(sub_stroke_count - sub_strokes_range, 1);
            let max_sub_strokes = usize::min(sub_stroke_count + sub_strokes_range, Matcher::MAX_CHARACTER_SUB_STROKE_COUNT);
            // Iterate over all characters in repo
            for cix in 0..char_data.len() {
                let repo_char = &char_data[cix];
                let cmp_stroke_count = repo_char.stroke_count;
                let cmp_sub_strokes = &repo_char.sub_strokes;
                if (cmp_stroke_count as usize) < minimum_strokes || cmp_stroke_count as usize > maximum_strokes {
                    continue;
                }
                if cmp_sub_strokes.len() < min_sub_strokes || cmp_sub_strokes.len() > max_sub_strokes {
                    continue;
                }
                // Match against character in repo
                let char_match = self.match_one(stroke_count, &input_sub_strokes, sub_strokes_range, &repo_char);
                // File; collector takes care of comparisons and keeping N-best
                collector.file_match(char_match);
            }
       });
    }

    fn match_one(   &mut self,
                    input_stroke_count: usize,
                    input_sub_strokes: &Vec<SubStroke>,
                    sub_strokes_range: usize,
                    repo_char: &CharData) -> Match {
        // Calculate score. This is the *actual* meat.
        let mut score = self.compute_match_score(input_sub_strokes, sub_strokes_range, repo_char);
        // If the input character and the character in the repository have the same number of strokes, assign a small bonus.
        // Might be able to remove this, doesn't really add much, only semi-useful for characters with only a couple strokes.
        if input_stroke_count == repo_char.stroke_count as usize && input_stroke_count < Matcher::CORRECT_NUM_STROKES_CAP {
            // The bonus declines linearly as the number of strokes increases, writing 2 instead of 3 strokes is worse than 9 for 10.
            let bonus = Matcher::CORRECT_NUM_STROKES_BONUS * 
                (i32::max(Matcher::CORRECT_NUM_STROKES_CAP as i32 - input_stroke_count as i32, 0) as f32) / 
                (Matcher::CORRECT_NUM_STROKES_CAP as f32);
            score += bonus * score;
        }
        Match {
            hanzi: repo_char.hanzi,
            score: score,
        }
    }

    fn compute_match_score( &mut self,
                            input_sub_strokes: &Vec<SubStroke>,
                            sub_strokes_range: usize,
                            repo_char: &CharData) -> f32 {
        // 
        for x in 0..input_sub_strokes.len() {
            // For each of the input substrokes...
            let input_direction = input_sub_strokes[x].direction.round() as u8;
            let input_length = input_sub_strokes[x].length.round() as u8;
            let input_center = Point {
                x: input_sub_strokes[x].center_x as u8,
                y: input_sub_strokes[x].center_y as u8,
            };
            for y in 0..repo_char.sub_strokes.len() {
                // For each of the compare substrokes...
                // initialize the score as being not usable, it will only be set to a good
                // value if the two substrokes are within the range.
                let mut new_score = std::f32::MIN;
                let range = ((x as i32) - (y as i32).abs()) as usize;
                if range <= sub_strokes_range {
                    // The range is based on looseness.  If the two substrokes fall out of the range
                    // then the comparison score for those two substrokes remains Double.MIN_VALUE and will not be used.
                    let cmp_dir = repo_char.sub_strokes[y].dir;
                    let cmp_length = repo_char.sub_strokes[y].length;
                    let cmp_center = Point {
                        x: (repo_char.sub_strokes[y].center & 0xf0).wrapping_shr(4),
                        y: repo_char.sub_strokes[y].center & 0x0f,
                    };
                    // We incur penalties for skipping substrokes.
                    // Get the scores that would be incurred either for skipping the substroke from the descriptor, or from the repository.
                    let skip1_score = self.score_matrix[x][y + 1] -
                        (input_length as f32 / 256.0 * Matcher::SKIP_PENALTY_MULTIPLIER);
                    let skip2_score = self.score_matrix[x + 1][y] - 
                        (cmp_length as f32 / 256.0 * Matcher::SKIP_PENALTY_MULTIPLIER);
                    // The skip score is the maximum of the scores that would result from skipping one of the substrokes.
                    let skip_score = f32::max(skip1_score, skip2_score);
                    // The match_score is the score of actually comparing the two substrokes.
                    let match_score = self.compute_sub_stroke_score(input_direction,
                        input_length, 
                        cmp_dir, 
                        cmp_length, 
                        input_center, 
                        cmp_center);
                    // Previous score is the score we'd add to if we compared the two substrokes.
                    let prev_score = self.score_matrix[x][y];
                    // Result score is the maximum of skipping a substroke, or comparing the two.
                    new_score = f32::max(prev_score + match_score, skip_score);
                }
                // Set the score for comparing the two substrokes.
                self.score_matrix[x + 1][y + 1] = new_score;
            }
        }
        // At the end the score is the score at the opposite corner of the matrix...
        // don't need to use count - 1 since seed values occupy indices 0
        self.score_matrix[input_sub_strokes.len()][repo_char.sub_strokes.len()]
    }

    fn compute_sub_stroke_score(&self,
                                input_direction: u8,
                                input_length: u8,
                                repo_direction: u8, 
                                repo_length: u8, 
                                input_center: Point,
                                repo_center: Point) -> f32 {
        // Score drops off after directions get sufficiently apart, start to rise again as the substrokes approach opposite directions.
        // This in particular reflects that occasionally strokes will be written backwards, this isn't totally bad, they get
        // some score for having the stroke oriented correctly.
        let direction_score = self.get_direction_score(input_direction, repo_direction, input_length);
        //var direction_score = Math.max(Math.cos(2.0 * theta), 0.3 * Math.cos((1.5 * theta) + (Math.PI / 3.0)));

        // Length score gives an indication of how similar the lengths of the substrokes are.
        // Get the ratio of the smaller of the lengths over the longer of the lengths.
        let length_score = self.get_length_score(input_length, repo_length);
        // Ratios that are within a certain range are fine, but after that they drop off, scores not more than 1.
        //var length_score = Math.log(length_score + (1.0 / Math.E)) + 1;
        //length_score = Math.min(length_score, 1.0);

        // For the final "classic" score we just multiply the two scores together.
        let mut score = length_score * direction_score;

        // Reduce score if strokes are farther apart
        let dx = input_center.x as i32 - repo_center.x as i32;
        let dy = input_center.y as i32 - repo_center.y as i32;
        let closeness = self.pos_score_table[(dx * dx + dy * dy) as usize];

        // var dist = Math.sqrt(dx * dx + dy * dy);
        // // Distance is [0 .. 21.21] because X and Y are all [0..15]
        // // Square distance is [0..450]
        // // TO-DO: a cubic function for this too
        // var closeness = 1 - dist / 22;
        // Closeness is always [0..1]. We reduce positive score, and make negative more negative.
        if score > 0.0 { score *= closeness; }
        else { score /= closeness; }
        
        // Done
        score
    }

    fn get_direction_score(&self, direction1: u8, direction2: u8, input_length: u8) -> f32 {
        // Both directions are [0..255], integer
        let theta = (direction1 as i32 - direction2 as i32).abs() as usize;
        // Lookup table for actual score function
        let mut direction_score = self.direction_score_table[theta];
        // Add bonus if the input length is small.
        // Directions doesn't really matter for small dian-like strokes.
        if input_length < 64 {
            let short_length_bonus_max = f32::min(1.0, 1.0 - direction_score);
            let short_length_bonus = short_length_bonus_max * (1.0 - (input_length as f32 / 64.0));
            direction_score += short_length_bonus;
        }
        direction_score
    }

    fn get_length_score(&self, length1: u8, length2: u8) -> f32 {
        // Get the ratio between the two lengths less than one.
        let ratio: usize;
        // Shift for "times 128"
        if length1 > length2 { ratio = ((length2 as f32 * 128.0) / length1 as f32).round() as usize; }
        else { ratio = ((length1 as f32 * 128.0) / length2 as f32).round() as usize; }
        // Lookup table for actual score function
        self.length_score_table[ratio]
    }
}

fn init_score_matrix(sm: &mut Vec<Vec<f32>>) {
    // Allocate if this is the first time we're initializing
    if sm.len() == 0 {
        for i in 0..Matcher::MAX_CHARACTER_SUB_STROKE_COUNT + 1 {
            sm.push(Vec::with_capacity(Matcher::MAX_CHARACTER_SUB_STROKE_COUNT + 1));
            for _ in 0..Matcher::MAX_CHARACTER_SUB_STROKE_COUNT + 1 {
                sm[i].push(0f32);
            }
        }
    }
    // For starters, everythig is zero
    for i in 0..sm.len() {
        for j in 0..sm[i].len() {
            sm[i][j] = 0f32;
        }
    }
    // Seed the first row and column with base values.
    // Starting from a cell that isn't at 0,0 to skip strokes incurs a penalty.
    for i in 0..sm.len() {
        let penalty = -Matcher::AVG_SUBSTROKE_LENGTH * Matcher::SKIP_PENALTY_MULTIPLIER * (i as f32);
        sm[i][0] = penalty;
        sm[0][i] = penalty;
    }
}


fn init_score_tables(direction_score_table: &mut Vec<f32>, length_score_table: &mut Vec<f32>, pos_score_table: &mut Vec<f32>) {
    // Builds a precomputed array of values to use when getting the score between two substroke directions.
    // Two directions should differ by 0 - Pi, and the score should be the (difference / Pi) * score table's length
    // The curve drops as the difference grows, but rises again some at the end because
    // a stroke that is 180 degrees from the expected direction maybe OK passable.
    let curve = CubicCurve2D::new(0f32, 1.0, 0.5, 1.0, 0.25, -2.0, 1.0, 1.0);
    init_sc_from_curve(direction_score_table, &curve, 256);

    // Builds a precomputed array of values to use when getting the score between two substroke lengths.
    // A ratio less than one is computed for the two lengths, and the score should be the ratio * score table's length.
    // Curve grows rapidly as the ratio grows and levels off quickly.
    // This is because we don't really expect lengths to lety a lot.
    // We are really just trying to distinguish between tiny strokes and long strokes.
    let curve = CubicCurve2D::new(0f32, 0f32, 0.25, 1.0, 0.75, 1.0, 1.0, 1.0);
    init_sc_from_curve(length_score_table, &curve, 129);

    pos_score_table.clear();
    for i in 0..450 {
        pos_score_table.push(1.0 - (i as f32).sqrt() / 22.0);
    }
}

fn init_sc_from_curve(score_table: &mut Vec<f32>, curve: &CubicCurve2D, samples: usize) {
    score_table.clear();
    let x1 = curve.x1;
    let x2 = curve.x2;
    let range = x2 - x1;
    let x_inc = range / (samples as f32);  // even incrementer to increment x value by when sampling across the curve
    let mut x = x1;
    // Sample evenly across the curve and set the samples into the table.
    for _ in 0..samples {
        let t = curve.get_first_solution_for_x(f32::min(x, x2));
        score_table.push(curve.get_y_on_curve(t));
        x += x_inc;
    }
}

fn get_strokes_range(stroke_count: usize, looseness: f32) -> usize {
    if looseness == 0f32 { return 0; }
    if looseness == 1f32 { return Matcher::MAX_CHARACTER_STROKE_COUNT; }
    // We use a CubicCurve that grows slowly at first and then rapidly near the end to the maximum.
    // This is so a looseness at or near 1.0 will return a range that will consider all characters.
    let ctrl1_x = 0.35;
    let ctrl1_y = (stroke_count as f32) * 0.4;
    let ctrl2_x = 0.6;
    let ctrl2_y = stroke_count as f32;
    let curve = CubicCurve2D::new(0.0, 0.0, ctrl1_x, ctrl1_y, ctrl2_x, ctrl2_y, 1.0, Matcher::MAX_CHARACTER_STROKE_COUNT as f32);
    let t = curve.get_first_solution_for_x(looseness);
    // We get the t value on the parametrized curve where the x value matches the looseness.
    // Then we compute the y value for that t. This gives the range.
    let res = curve.get_y_on_curve(t).round();
    return res as usize;
}

fn get_sub_strokes_range(sub_stroke_count: usize, looseness: f32) -> usize {
    // Return the maximum if looseness = 1.0.
    // Otherwise we'd have to ensure that the floating point value led to exactly the right int count.
    if looseness == 1.0 { return Matcher::MAX_CHARACTER_SUB_STROKE_COUNT; }
    // We use a CubicCurve that grows slowly at first and then rapidly near the end to the maximum.
    let y0 = (sub_stroke_count as f32) * 0.25;
    let ctrl1_x = 0.4;
    let ctrl1_y = 1.5 * y0;
    let ctrl2_x = 0.75;
    let ctrl2_y = 1.5 * ctrl1_y;
    let curve = CubicCurve2D::new(0.0, y0, ctrl1_x, ctrl1_y, ctrl2_x, ctrl2_y, 1.0, Matcher::MAX_CHARACTER_SUB_STROKE_COUNT as f32);
    let t = curve.get_first_solution_for_x(looseness);
    // We get the t value on the parametrized curve where the x value matches the looseness.
    // Then we compute the y value for that t. This gives the range.
    let res = curve.get_y_on_curve(t).round();
    return res as usize;
}


#[cfg(test)]
mod tests {
    use std::fmt::Write;
    use super::*;

    #[test]
    fn test_score_tables() {
        let mut direction_score_table: Vec<f32> = Vec::new();
        let mut length_score_table: Vec<f32> = Vec::new();
        let mut pos_score_table: Vec<f32> = Vec::new();
        init_score_tables(&mut direction_score_table, &mut length_score_table, &mut pos_score_table);

        assert_eq!(direction_score_table.len(), 256);
        assert!(direction_score_table[0] > 0.99);
        assert!(direction_score_table[96] > 0.0);
        assert!(direction_score_table[97] < 0.0);
        assert!(direction_score_table[183] < 0.0);
        assert!(direction_score_table[184] > 0.0);
        assert!(direction_score_table[255] > 0.98);

        assert_eq!(length_score_table.len(), 129);
        assert!(length_score_table[0] >= 0.0);
        assert!(length_score_table[0] < 0.01);
        assert!(length_score_table[23] < 0.5);
        assert!(length_score_table[24] > 0.5);
        assert!(length_score_table[128] > 0.99);

        assert!(pos_score_table.len() == 450);
        assert!(pos_score_table[0] == 1.0);
        assert!(pos_score_table[121] == 0.5);
        assert!(pos_score_table[449] < 0.04);
    }

    // These manual samples are custom-saved from a tweaked version of the HanziLookupJS demo
    // This is a hand-drawn 一
    static STROKES_1: &str = "[[[70,124],[71,124],[79,124],[104,124],[119,124],[132,125],[151,126],[168,126],[169,126],[189,125],[191,124],[191,124]]]";
    // This is a hand-drawn 十
    static STROKES_2: &str = "[[[76,127],[77,127],[84,127],[97,128],[119,128],[125,129],[138,130],[147,130],[153,131],[154,131],[158,131],[162,131],[167,131],[168,131],[169,131],[169,131]],[[129,60],[129,62],[128,74],[128,102],[128,118],[129,143],[130,162],[130,170],[130,178],[131,184],[131,188],[131,193],[131,196],[131,198],[131,203],[131,203]]]";
    // This is a hand-drawn 元
    static STROKES_3: &str = "[[[86,65],[98,66],[146,69],[152,69],[161,69],[166,69],[170,68],[170,68]],[[47,97],[48,97],[54,97],[89,103],[117,104],[146,101],[169,100],[176,98],[180,98],[184,98],[189,98],[193,98],[195,98],[195,98]],[[103,109],[103,110],[99,132],[91,156],[70,180],[56,190],[53,192]],[[143,105],[143,106],[142,114],[140,134],[138,149],[138,160],[138,167],[140,174],[144,182],[150,186],[155,190],[161,193],[166,194],[172,196],[188,197],[193,197],[197,197],[206,197],[206,196],[207,196],[208,196],[208,194],[204,182],[203,174],[202,174],[202,175],[202,176]]]";
    // This is a hand-drawn 氣
    static STROKES_4: &str = "[[[76,32],[76,33],[75,37],[73,43],[70,51],[67,58],[64,66],[61,72],[57,77],[52,82],[50,85],[50,85]],[[68,58],[69,58],[76,58],[90,59],[100,60],[110,62],[118,62],[132,62],[136,62],[141,62],[145,62],[146,62],[148,62],[148,62]],[[68,95],[69,95],[77,96],[96,96],[105,96],[110,96],[126,97],[144,98],[146,98],[154,98],[156,98],[156,98]],[[59,126],[60,126],[67,126],[90,130],[107,131],[120,132],[134,132],[149,132],[151,132],[156,132],[158,133],[158,134],[156,142],[154,147],[153,155],[152,160],[151,166],[150,172],[150,179],[150,183],[150,186],[150,190],[151,194],[152,199],[156,204],[158,206],[162,209],[167,213],[171,215],[175,216],[184,220],[192,222],[196,223],[200,224],[204,225],[208,225],[210,225],[214,225],[218,223],[218,222],[216,214],[214,208],[214,207],[214,207]],[[79,147],[82,148],[87,155],[91,161],[91,161]],[[124,148],[123,148],[116,155],[110,162],[108,164],[108,164]],[[73,175],[75,175],[88,178],[98,180],[104,180],[111,182],[117,182],[122,182],[125,182]],[[100,148],[100,151],[102,172],[102,195],[103,204],[103,211],[104,216],[104,220],[104,224]],[[94,189],[93,189],[81,204],[72,210],[71,210]],[[109,192],[112,194],[120,199],[132,208],[133,210],[133,210]]]";

    fn parse_sample(str_strokes: &str) -> Vec<Stroke> {
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
        strokes
    }

    #[test]
    fn test_matches() {
        let mut barf = String::new();
        let mut matcher = Matcher::new();
        let mut res: Vec<Match> = Vec::with_capacity(8);
        {
            let sample = parse_sample(STROKES_1);
            res.clear();
            let mut collector = MatchCollector::new(&mut res, 8);
            matcher.lookup(&sample, &mut collector);
            write!(&mut barf, "#1: {}", res[0].hanzi).unwrap();
            // assert!(res[0].hanzi == '一');
        }
        {
            let sample = parse_sample(STROKES_2);
            res.clear();
            let mut collector = MatchCollector::new(&mut res, 8);
            matcher.lookup(&sample, &mut collector);
            write!(&mut barf, "#1: {}  #2: {}  #3: {}  #4: {}", res[0].hanzi, res[1].hanzi, res[2].hanzi, res[3].hanzi).unwrap();
            // assert!(res[0].hanzi == '十');
        }
        {
            let sample = parse_sample(STROKES_3);
            res.clear();
            let mut collector = MatchCollector::new(&mut res, 8);
            matcher.lookup(&sample, &mut collector);
            write!(&mut barf, "#1: {}  #2: {}  #3: {}  #4: {}", res[0].hanzi, res[1].hanzi, res[2].hanzi, res[3].hanzi).unwrap();
            // assert!(res[0].hanzi == '元');
        }
        {
            let sample = parse_sample(STROKES_4);
            res.clear();
            let mut collector = MatchCollector::new(&mut res, 8);
            matcher.lookup(&sample, &mut collector);
            write!(&mut barf, "#1: {}  #2: {}  #3: {}  #4: {}", res[0].hanzi, res[1].hanzi, res[2].hanzi, res[3].hanzi).unwrap();
            // assert!(res[0].hanzi == '氣');
        }
    }
}
