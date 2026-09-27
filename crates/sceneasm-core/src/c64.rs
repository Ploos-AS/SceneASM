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
pub struct VicState {
    pub display_enabled: bool,
    pub y_scroll: u8,
}

impl Default for VicState {
    fn default() -> Self { Self { display_enabled: true, y_scroll: 3 } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VicLineTiming {
    pub badline: bool,
    pub vic_stolen_cycles: u16,
    pub cpu_available_cycles: u16,
}

impl C64Timing {
    pub const fn vic_line(self, raster_line: u16, state: VicState) -> Option<VicLineTiming> {
        let total = match self.line_budget(raster_line) { Some(v) => v, None => return None };
        // Badline condition for the normal display window. The VIC-II performs
        // 40 character-matrix fetches, taking 40 CPU bus cycles.
        let badline = state.display_enabled
            && raster_line >= 0x30
            && raster_line <= 0xf7
            && (raster_line & 7) == (state.y_scroll as u16 & 7);
        let stolen = if badline { 40 } else { 0 };
        Some(VicLineTiming {
            badline,
            vic_stolen_cycles: stolen,
            cpu_available_cycles: total - stolen,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterContract {
    pub line: u16,
    pub line_cycles: u16,
    pub available_cycles: u16,
    pub vic_stolen_cycles: u16,
    pub badline: bool,
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

    #[test]
    fn pal_badline_exposes_vic_bus_stealing() {
        let timing = C64Timing::pal();
        let state = VicState { display_enabled: true, y_scroll: 0 };
        let line = timing.vic_line(0x30, state).unwrap();
        assert!(line.badline);
        assert_eq!(line.vic_stolen_cycles, 40);
        assert_eq!(line.cpu_available_cycles, 23);

        let normal = timing.vic_line(0x31, state).unwrap();
        assert!(!normal.badline);
        assert_eq!(normal.cpu_available_cycles, 63);
    }
}
