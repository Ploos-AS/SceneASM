#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoStandard {
    Pal,
    Ntsc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct C64Timing {
    pub standard: VideoStandard,
    pub cycles_per_line: u16,
    pub lines_per_frame: u16,
}

impl C64Timing {
    pub const fn pal() -> Self {
        Self { standard: VideoStandard::Pal, cycles_per_line: 63, lines_per_frame: 312 }
    }

    pub const fn ntsc() -> Self {
        Self { standard: VideoStandard::Ntsc, cycles_per_line: 65, lines_per_frame: 263 }
    }

    pub const fn line_budget(self, raster_line: u16) -> Option<u16> {
        if raster_line < self.lines_per_frame { Some(self.cycles_per_line) } else { None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterContract {
    pub line: u16,
    pub available_cycles: u16,
    pub min_cycles: u64,
    pub max_cycles: u64,
}

impl RasterContract {
    pub const fn fits(self) -> bool {
        self.max_cycles <= self.available_cycles as u64
    }

    pub const fn margin(self) -> i64 {
        self.available_cycles as i64 - self.max_cycles as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_c64_raster_geometry() {
        assert_eq!(C64Timing::pal().line_budget(100), Some(63));
        assert_eq!(C64Timing::pal().line_budget(312), None);
        assert_eq!(C64Timing::ntsc().line_budget(100), Some(65));
        assert_eq!(C64Timing::ntsc().lines_per_frame, 263);
    }
}
