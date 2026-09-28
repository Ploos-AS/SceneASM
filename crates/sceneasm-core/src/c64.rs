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
        Self {
            standard: VideoStandard::Pal,
            cycles_per_line: 63,
            lines_per_frame: 312,
        }
    }

    pub const fn ntsc() -> Self {
        Self {
            standard: VideoStandard::Ntsc,
            cycles_per_line: 65,
            lines_per_frame: 263,
        }
    }

    pub const fn line_budget(self, raster_line: u16) -> Option<u16> {
        if raster_line < self.lines_per_frame {
            Some(self.cycles_per_line)
        } else {
            None
        }
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
    fn default() -> Self {
        Self {
            display_enabled: true,
            y_scroll: 3,
            sprite_enable_mask: 0,
            sprite_y: [0; 8],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusOwner {
    Cpu,
    Badline,
    Sprite(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusMap {
    pub slots: Vec<BusOwner>,
}

impl BusMap {
    pub fn cpu_cycles(&self) -> u16 {
        self.slots
            .iter()
            .filter(|owner| matches!(owner, BusOwner::Cpu))
            .count() as u16
    }

    pub fn stolen_cycles(&self) -> u16 {
        self.slots.len() as u16 - self.cpu_cycles()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledInstruction {
    pub address: u16,
    pub source: crate::SourceSpan,
    pub expansion: crate::source::ExpansionTrace,
    pub nominal_cycles: u16,
    pub start_cycle: u16,
    pub end_cycle: u16,
    pub stalled_cycles: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuSchedule {
    pub instructions: Vec<ScheduledInstruction>,
    pub end_cycle: u16,
    pub stalled_cycles: u16,
}

impl BusMap {
    pub fn schedule(&self, instructions: &[crate::InstructionInfo]) -> CpuSchedule {
        let mut cycle = 0usize;
        let mut total_stalls = 0u16;
        let mut scheduled = Vec::new();

        for instruction in instructions {
            let start = cycle as u16;
            let nominal = instruction.opcode.cycles as usize;
            let mut executed = 0usize;
            let mut stalls = 0u16;

            while executed < nominal && cycle < self.slots.len() {
                match self.slots[cycle] {
                    BusOwner::Cpu => executed += 1,
                    _ => {
                        stalls += 1;
                        total_stalls += 1;
                    }
                }
                cycle += 1;
            }

            scheduled.push(ScheduledInstruction {
                address: instruction.address,
                source: instruction.source.clone(),
                expansion: instruction.expansion.clone(),
                nominal_cycles: nominal as u16,
                start_cycle: start,
                end_cycle: cycle as u16,
                stalled_cycles: stalls,
            });
        }

        CpuSchedule {
            instructions: scheduled,
            end_cycle: cycle as u16,
            stalled_cycles: total_stalls,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VicLineTiming {
    pub badline: bool,
    pub badline_stolen_cycles: u16,
    pub sprite_stolen_cycles: u16,
    pub vic_stolen_cycles: u16,
    pub active_sprites: u8,
    pub sprite_dma_mask: u8,
    pub cpu_available_cycles: u16,
    pub bus: BusMap,
}

impl C64Timing {
    pub fn vic_line(self, raster_line: u16, state: VicState) -> Option<VicLineTiming> {
        let total = match self.line_budget(raster_line) {
            Some(v) => v,
            None => return None,
        };
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
        let mut slots = vec![BusOwner::Cpu; total as usize];
        if badline {
            // Character matrix fetch window. This is an explicit occupancy
            // model so exact VIC revisions can refine the slot positions.
            for slot in slots.iter_mut().take(54).skip(14) {
                *slot = BusOwner::Badline;
            }
        }
        // Initial conservative sprite slot placement at the end of the line.
        // Each active sprite occupies two CPU-visible bus cycles.
        let mut cursor = total as usize;
        for sprite in (0u8..8).rev() {
            if sprite_dma_mask & (1 << sprite) != 0 {
                for _ in 0..2 {
                    if cursor > 0 {
                        cursor -= 1;
                        if matches!(slots[cursor], BusOwner::Cpu) {
                            slots[cursor] = BusOwner::Sprite(sprite);
                        }
                    }
                }
            }
        }
        let bus = BusMap { slots };
        let stolen = bus.stolen_cycles();
        Some(VicLineTiming {
            badline,
            badline_stolen_cycles: badline_stolen,
            sprite_stolen_cycles: sprite_stolen,
            vic_stolen_cycles: stolen,
            active_sprites,
            sprite_dma_mask,
            cpu_available_cycles: bus.cpu_cycles(),
            bus,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RasterContract {
    pub line: u16,
    pub scheduled_end_cycle: u16,
    pub scheduled_stall_cycles: u16,
    pub schedule: Vec<ScheduledInstruction>,
    pub line_cycles: u16,
    pub available_cycles: u16,
    pub vic_stolen_cycles: u16,
    pub badline_stolen_cycles: u16,
    pub sprite_stolen_cycles: u16,
    pub active_sprites: u8,
    pub sprite_dma_mask: u8,
    pub badline: bool,
    pub min_cycles: u64,
    pub max_cycles: u64,
}

impl RasterContract {
    pub const fn fits(&self) -> bool {
        self.max_cycles <= self.available_cycles as u64
    }

    pub const fn margin(&self) -> i64 {
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
        let state = VicState {
            display_enabled: true,
            y_scroll: 0,
            sprite_enable_mask: 0,
            sprite_y: [0; 8],
        };
        let line = timing.vic_line(0x30, state).unwrap();
        assert!(line.badline);
        assert_eq!(line.vic_stolen_cycles, 40);
        assert_eq!(line.cpu_available_cycles, 23);

        let normal = timing.vic_line(0x31, state).unwrap();
        assert!(!normal.badline);
        assert_eq!(normal.cpu_available_cycles, 63);
    }

    #[test]
    fn bus_map_stalls_cpu_instruction_timeline() {
        let mut bus = BusMap {
            slots: vec![BusOwner::Cpu; 12],
        };
        bus.slots[2] = BusOwner::Badline;
        bus.slots[3] = BusOwner::Badline;
        let instructions = vec![
            crate::InstructionInfo {
                address: 0x1000,
                source: crate::SourceSpan {
                    file_id: 0,
                    line: 10,
                    column_start: 1,
                    column_end: 7,
                },
                expansion: crate::source::ExpansionTrace::default(),
                opcode: crate::opcode("LDA", crate::AddressingMode::Immediate).unwrap(),
            },
            crate::InstructionInfo {
                address: 0x1002,
                source: crate::SourceSpan {
                    file_id: 0,
                    line: 11,
                    column_start: 1,
                    column_end: 4,
                },
                expansion: crate::source::ExpansionTrace::default(),
                opcode: crate::opcode("RTS", crate::AddressingMode::Implied).unwrap(),
            },
        ];
        let schedule = bus.schedule(&instructions);
        assert_eq!(schedule.instructions[0].end_cycle, 2);
        assert_eq!(schedule.instructions[0].source.line, 10);
        assert!(schedule.instructions[0].expansion.frames.is_empty());
        assert_eq!(schedule.instructions[1].stalled_cycles, 2);
        assert_eq!(schedule.instructions[1].end_cycle, 10);
        assert_eq!(schedule.stalled_cycles, 2);
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

    #[test]
    fn sprite_dma_is_accounted_separately() {
        let timing = C64Timing::pal();
        let mut state = VicState {
            display_enabled: false,
            y_scroll: 3,
            sprite_enable_mask: 0b0000_0111,
            sprite_y: [0; 8],
        };
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
        assert_eq!(line.bus.stolen_cycles(), 6);
        assert!(line
            .bus
            .slots
            .iter()
            .any(|owner| matches!(owner, BusOwner::Sprite(0))));
    }
}
