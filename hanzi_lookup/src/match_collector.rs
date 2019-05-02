pub struct MatchCollector<'a> {
    max: u32,
    matches: &'a mut Vec<super::Match>,
}

impl<'a> MatchCollector<'a> {
    pub fn new(matches: &mut Vec<super::Match>, max: u32) -> MatchCollector {
        assert!(max > 0, "Expected a positive number for the maximum number of matches.");
        assert!(matches.len() == 0, "The pre-existing matches vector must be empty.");
        MatchCollector {
            max: max,
            matches: matches,
        }
    }

    fn remove_existing_lower(&mut self, mc: &super::Match) -> bool {
        let mut ix: i32 = -1;
        for i in 0..self.matches.len() {
            if self.matches[i].hanzi == mc.hanzi {
                ix = i as i32;
                break;
            }
        }
        // Not there yet: we're good, match doesn't need to be skipped
        if ix == -1 {
            return false;
        }
        // New score is not better: skip new match
        if mc.score <= self.matches[ix as usize].score {
            return true;
        }
        // Remove existing match; don't skip new. Means shifting array left.
        self.matches.remove(ix as usize);
        return false;
    }

    pub fn file_match(&mut self, mc: super::Match) {
        // Already at limit: don't bother if new match's score is smaller than current minimum
        if self.matches.len() == self.max as usize && mc.score <= self.matches.last().unwrap().score {
            return;
        }
        // Remove if we already have this character with a lower score
        // If we get "true", we should skip new match (already there with higher score)
        if self.remove_existing_lower(&mc) {
            return;
        }
        // Where does new match go? (Keep array sorted largest score to smallest.)
        // Largest score is always at start of vector.
        let ix = self.matches.iter().position(|x| x.score < mc.score);
        match ix {
            Some(ix) => self.matches.insert(ix, mc),
            None => self.matches.push(mc)
        }
        // Beyond limit? Drop last item.
        if self.matches.len() > self.max as usize {
            self.matches.pop();
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    #[should_panic]
    fn test_new_fail1() {
        let mut matches: Vec<Match> = Vec::new();
        let mut _collector = MatchCollector::new(&mut matches, 0);
    }

    #[test]
    #[should_panic]
    fn test_new_fail2() {
        let mut matches: Vec<Match> = Vec::new();
        matches.push(Match {
            hanzi: '我',
            score: 1.0,
        });
        let mut _collector = MatchCollector::new(&mut matches, 1);
    }

    #[test]
    fn test_filing() {
        let mut matches: Vec<Match> = Vec::new();
        let mut collector = MatchCollector::new(&mut matches, 3);
        let mc1 = Match {
            hanzi: '我',
            score: 0.8,
        };
        let mc2 = Match {
            hanzi: '你',
            score: 0.9,
        };
        let mc3 = Match {
            hanzi: '我',
            score: 0.7,
        };
        let mc4 = Match {
            hanzi: '他',
            score: 0.7,
        };
        let mc5 = Match {
            hanzi: '鸡',
            score: 1.0,
        };
        collector.file_match(mc1);
        collector.file_match(mc2);
        collector.file_match(mc3);
        collector.file_match(mc4); 
        collector.file_match(mc5);
        assert_eq!(matches, [mc5, mc2, mc1]);
    }
}

