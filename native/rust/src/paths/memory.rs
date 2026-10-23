//! What a pathfinder has seen of the ground (PathMemory, spec 30): the cells its plans have
//! looked at within its sight, kept between plans, so a wall it has felt along stays known
//! when it turns its back on it. A plan counts a known cell as seen (at its real cost, read
//! live from the ground). The GDScript keeps it between plans and hands it over.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Memory {
    pub known: Vec<u8>,
}

impl Memory {
    /// Whether it has seen the cell at `index` of a grid of `count` cells.
    pub fn knows(&self, index: usize, count: usize) -> bool {
        self.known.len() == count && self.known[index] == 1
    }

    /// Remembers seeing the cell at `index` of a grid of `count` cells.
    pub fn learn(&mut self, index: usize, count: usize) {
        if self.known.len() != count {
            self.known = vec![0; count];
        }
        self.known[index] = 1;
    }
}
