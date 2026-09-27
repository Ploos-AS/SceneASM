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

    #[test]
    fn sprite_dma_follows_y_window() {
        let timing = C64Timing::pal();
        let mut state = VicState::default();
        state.display_enabled = false;
        state.sprite_enable_mask = 1;
        state.sprite_y[0] = 100;
        assert_eq!(timing.vic_line(99, state).unwrap().active_sprites, 0);
        assert_eq!(timing.vic_line(100, state).unwrap().active_sprites, 1);
        assert_eq!(timing.vic_line(120, state).unwrap().active_sprites, 1);
        assert_eq!(timing.vic_line(121, state).unwrap().active_sprites, 0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VicState {
    pub display_enabled: bool,
    pub y_scroll: u8,
    pub sprite_enable_mask: u8,
    pub sprite_y: [u8; 8],
}

impl Default for VicState {
    fn default() -> Self { Self { display_enabled: true, y_scroll: 3, sprite_enable_mask: 0, sprite_y: [0; 8] } }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VicLineTiming {
    pub badline: bool,
    pub badline_stolen_cycles: u16,
    pub sprite_stolen_cycles: u16,
    pub vic_stolen_cycles: u16,
    pub active_sprites: u8,
    pub sprite_dma_mask: u8,
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
        let badline_stolen = if badline { 40 } else { 0 };
        let mut sprite_dma_mask = 0u8;
        let raster8 = raster_line as u8;
        let mut sprite = 0usize;
        while sprite < 8 {
            if state.sprite_enable_mask & (1 << sprite) != 0 {
                let delta = raster8.wrapping_sub(state.sprite_y[sprite]);
                if delta < 21 {
                    sprite_dma_mask |= 1 << sprite;
                }
            }
            sprite += 1;
        }
        let active_sprites = sprite_dma_mask.count_ones() as u8;
        // Line-level accounting: each active sprite consumes two CPU bus
        // cycles for its graphics fetch group. Cycle-exact slots come later.
        let sprite_stolen = active_sprites as u16 * 2;
        let stolen = badline_stolen + sprite_stolen;
        Some(VicLineTiming {
            badline,
            badline_stolen_cycles: badline_stolen,
            sprite_stolen_cycles: sprite_stolen,
            vic_stolen_cycles: stolen,
            active_sprites,
            sprite_dma_mask,
            cpu_available_cycles: total.saturating_sub(stolen),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterContract {
    pub line: u16,
    pub line_cycles: u16,
    pub available_cycles: u16,
    pub vic_stolen_cycles: u16,
    pub badline_stolen_cycles: u16,
    pub sprite_stolen_cycles: u16,
    pub active_sprites: u8,
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
        let state = VicState { display_enabled: true, y_scroll: 0, sprite_enable_mask: 0, sprite_y: [0; 8] };
        let line = timing.vic_line(0x30, state).unwrap();
        assert!(line.badline);
        assert_eq!(line.vic_stolen_cycles, 40);
        assert_eq!(line.cpu_available_cycles, 23);

        let normal = timing.vic_line(0x31, state).unwrap();
        assert!(!normal.badline);
        assert_eq!(normal.cpu_available_cycles, 63);
    }

    #[test]
    fn sprite_dma_is_accounted_separately() {
        let timing = C64Timing::pal();
        let mut state = VicState { display_enabled: false, y_scroll: 3, sprite_enable_mask: 0b0000_0111, sprite_y: [0; 8] };
        state.sprite_y[0] = 90;
        state.sprite_y[1] = 90;
        state.sprite_y[2] = 90;
        let line = timing.vic_line(100, state).unwrap();
        assert_eq!(line.active_sprites, 3);
        assert_eq!(line.sprite_dma_mask, 0b0000_0111);
        assert_eq!(line.sprite_stolen_cycles, 6);
        assert_eq!(line.badline_stolen_cycles, 0);
        assert_eq!(line.vic_stolen_cycles, 6);
        assert_eq!(line.cpu_available_cycles, 57);
    }
}
