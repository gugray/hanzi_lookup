pub struct CubicCurve2D {
    pub x1: f32, 
    pub y1: f32, 
    pub ctrlx1: f32, 
    pub ctrly1: f32, 
    pub ctrlx2: f32,
    pub ctrly2: f32, 
    pub x2: f32,
    pub y2: f32,
}

impl CubicCurve2D {
    pub fn new(x1: f32, y1: f32, ctrlx1: f32, ctrly1: f32, ctrlx2: f32, ctrly2: f32, x2: f32, y2: f32) -> CubicCurve2D {
        CubicCurve2D {
            x1: x1, 
            y1: y1, 
            ctrlx1: ctrlx1, 
            ctrly1: ctrly1, 
            ctrlx2: ctrlx2,
            ctrly2: ctrly2, 
            x2: x2,
            y2: y2,
        }
    }

    fn get_cubic_ax(&self) -> f32 {
        return self.x2 - self.x1 - self.get_cubic_bx() - self.get_cubic_cx();
    }
    fn get_cubic_ay(&self) -> f32 {
        return self.y2 - self.y1 - self.get_cubic_by() - self.get_cubic_cy();
    }
    fn get_cubic_bx(&self) -> f32 {
        return 3.0 * (self.ctrlx2 - self.ctrlx1) - self.get_cubic_cx();
    }
    fn get_cubic_by(&self) -> f32 {
        return 3.0 * (self.ctrly2 - self.ctrly1) - self.get_cubic_cy();
    }
    fn get_cubic_cx(&self) -> f32 {
        return 3.0 * (self.ctrlx1 - self.x1);
    }
    fn get_cubic_cy(&self) -> f32 {
        return 3.0 * (self.ctrly1 - self.y1);
    }

    pub fn solve_for_x(&self, x: f32) -> (f32, f32, f32, usize) {
        let mut res = (std::f32::NAN, std::f32::NAN, std::f32::NAN, 0);
        let a = self.get_cubic_ax();
        let b = self.get_cubic_bx();
        let c = self.get_cubic_cx();
        let d = self.x1 - x;
        let f = ((3.0 * c / a) - (b*b / (a*a))) / 3.0;
        let g = ((2.0 * b*b*b / (a*a*a)) - (9.0 * b * c / (a*a)) + (27.0 * d / a)) / 27.0;
        let h = (g * g / 4.0) + (f * f * f / 27.0);
        // There is only one real root
        if h > 0f32 {
            let u = 0f32 - g;
            let r = (u / 2.0) + h.powf(0.5);
            let s6 = r.powf(1.0 / 3.0);
            let s8 = s6;
            let t8 = (u / 2.0) - h.powf(0.5);
            let v7 = (0f32 - t8).powf(1.0 / 3.0);
            let v8 = v7;
            let x3 = (s8 - v8) - (b / (3.0 * a));
            res.0 = x3;
            res.3 = 1;
        }
        // All 3 roots are real and equal
        else if f == 0.0 && g == 0.0 && h == 0.0 {
            res.0 = -(d / a).powf(1.0 / 3.0);
            res.3 = 1;
        }
        // All three roots are real (h <= 0)
        else {
            let i = ((g * g / 4.0) - h).sqrt();
            let j = i.powf(1.0 / 3.0);
            let k = (-g / (2.0 * i)).acos();
            let l = j * -1.0;
            let m = (k / 3.0).cos();
            let n = (3f32).sqrt() * (k / 3.0).sin();
            let p = (b / (3.0 * a)) * -1.0;
            res.0 = 2.0 * j * (k / 3.0).cos() - (b / (3.0 * a));
            res.1 = l * (m + n) + p;
            res.2 = l * (m - n) + p;
            res.3 = 3;
        }
        res
    }

    pub fn get_first_solution_for_x(&self, x: f32) -> f32 {
        let solutions = self.solve_for_x(x);
        for i in 0..solutions.3 {
            let d;
            if i == 0 { d = solutions.0; }
            else if i == 1 { d = solutions.1; }
            else if i == 2 { d = solutions.2; }
            else { unreachable!(); }
            if d >= -0.0000001 && d <= 1.0000001 {
                if d >= 0.0 && d <= 1.0 { return d; }
                if d < 0.0 { return 0.0; }
                return 1.0;
            }
        }
        return std::f32::NAN;
    }

    pub fn get_y_on_curve(&self, t: f32) -> f32 {
        let ay = self.get_cubic_ay();
        let by = self.get_cubic_by();
        let cy = self.get_cubic_cy();
        let t_squared = t * t;
        let t_cubed = t * t_squared;
        let y = (ay * t_cubed) + (by * t_squared) + (cy * t) + self.y1;
        return y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cubic_curve() {
        let curve = CubicCurve2D::new(0f32, 1.0, 0.5, 1.0, 0.25, -2.0, 1.0, 1.0);
        let sol = curve.get_first_solution_for_x(0.0);
        assert_eq!(sol, 0.0);
        let sol = curve.get_first_solution_for_x(1.0);
        assert_eq!(sol, 1.0);
    }
}
