pub struct LCG {
    state: u32,
}

impl LCG {
    pub fn new(seed: u32) -> Self {
        LCG { state: seed }
    }

    pub fn next(&mut self) -> u32 {
        const A: u32 = 1664525;
        const C: u32 = 1013904223;

        self.state = self.state.wrapping_mul(A).wrapping_add(C);
        self.state
    }
}