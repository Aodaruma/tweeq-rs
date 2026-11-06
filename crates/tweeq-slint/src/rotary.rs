#[derive(Debug)]
pub(crate) struct Rotary {
    position: [f64; 2],
    previous: Option<f64>,
    degrees: f64,
}

impl Rotary {
    pub(crate) fn new(position: [f32; 2], size: [f32; 2], degrees: f64) -> Self {
        let [x, y] = position;
        let [width, height] = size;
        let position = [f64::from(x - width / 2.0), f64::from(y - height / 2.0)];
        Self {
            position,
            previous: bearing(position),
            degrees,
        }
    }

    pub(crate) fn update(&mut self, motion: [f32; 2], fine: bool) -> f64 {
        self.position[0] += f64::from(motion[0]);
        self.position[1] += f64::from(motion[1]);
        let current = bearing(self.position);
        if let (Some(previous), Some(next)) = (self.previous, current) {
            let delta = (next - previous + 180.0).rem_euclid(360.0) - 180.0;
            self.degrees += delta * if fine { 0.1 } else { 1.0 };
        }
        self.previous = current;
        self.degrees
    }
}

fn bearing([x, y]: [f64; 2]) -> Option<f64> {
    (x.hypot(y) >= 2.0).then(|| y.atan2(x).to_degrees())
}

#[cfg(test)]
mod tests {
    use super::Rotary;

    #[test]
    fn crossing_the_branch_cut_accumulates_complete_turns() {
        let mut rotary = Rotary::new([20.0, 10.0], [20.0, 20.0], 15.0);
        let mut degrees = 15.0;
        for _ in 0..3 {
            for motion in [[-10.0, 10.0], [-10.0, -10.0], [10.0, -10.0], [10.0, 10.0]] {
                degrees = rotary.update(motion, false);
            }
        }
        assert!((degrees - 1095.0).abs() < f64::EPSILON);
    }
}
