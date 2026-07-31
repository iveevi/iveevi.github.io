pub struct Pcg {
    state: u32,
}

impl Pcg {
    pub fn new(seed: u32) -> Pcg {
        return Pcg { state: seed };
    }

    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(747796405).wrapping_add(2891336453);
        let word = ((self.state >> ((self.state >> 28) + 4)) ^ self.state).wrapping_mul(277803737);
        return (word >> 22) ^ word;
    }

    pub fn uniform(&mut self) -> f32 {
        return self.next() as f32 / 4294967296.0;
    }

    pub fn range(&mut self, low: f32, high: f32) -> f32 {
        return low + (high - low) * self.uniform();
    }
}
