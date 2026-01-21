const LEFT: i32 = 0;
const TOP: i32 = 0;
const WIDTH: i32 = 1642 - 11 * 2 + 1 * 2;
const HEIGHT: i32 = 952 - 7 + 1;

pub struct CapturePos {
    pub rect: (i32, i32, i32, i32),
}

impl CapturePos {
    pub const KEY_READY: Self = Self { rect:(LEFT + 169, TOP + 858, 21, 12)};
    pub const ENERGY_FOUR: Self = Self { rect:(LEFT + 180, TOP + 415, 40, 44)};
    pub const ENERGY_ZERO: Self = Self { rect:(LEFT + 180, TOP + 650, 40, 44)};
    pub const PSE_TARGET: Self = Self { rect:(LEFT + 645, TOP + 535, 295, 50)};
    pub const PSE_QTE: Self = Self { rect:(LEFT + 790, TOP + 427, 90, 110)};
    pub const COIN_COUNT: Self = Self { rect:(LEFT + 920, TOP + 890, 15, 15)};
}
