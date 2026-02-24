pub struct CapturePos {
    pub rect: (i32, i32, i32, i32),
}

impl CapturePos {
    pub const fn key_ready(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 158, offset_y + 813, 21, 12),
        }
    }
    pub const fn energy_four(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 169, offset_y + 370, 40, 44),
        }
    }
    pub const fn energy_zero(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 169, offset_y + 605, 40, 44),
        }
    }
    pub const fn target(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 634, offset_y + 490, 295, 50),
        }
    }
    pub const fn qte(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 779, offset_y + 382, 90, 110),
        }
    }
    pub const fn coin_count(offset_x: i32, offset_y: i32) -> Self {
        Self {
            rect: (offset_x + 909, offset_y + 845, 15, 15),
        }
    }
}
