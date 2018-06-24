const NUM_ROWS: usize = 32;

pub struct Display {
    // i64 x 32 rows (64x32 monochrome)
    pub pixels: [u32; NUM_ROWS]
}

impl Display {
    pub fn new() -> Display {
        Display { pixels: [0; NUM_ROWS] }
    }

    pub fn clear(&mut self) {
        for i in 0..NUM_ROWS {
            self.pixels[i] = 0x00;
        }
    }
}
