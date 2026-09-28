use crate::{ExtraCycle, InstructionInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CycleRange {
    pub min: u64,
    pub max: u64,
}

impl CycleRange {
    pub fn add_instruction(&mut self, instruction: &InstructionInfo) {
        let op = instruction.opcode;
        self.min += op.cycles as u64;
        self.max += op.cycles as u64
            + match op.extra_cycle {
                ExtraCycle::None => 0,
                ExtraCycle::PageCross | ExtraCycle::BranchTaken => 1,
                ExtraCycle::BranchTakenAndPageCross => 2,
            };
    }

    pub const fn fits(self, budget: u64) -> bool {
        self.max <= budget
    }

    pub const fn margin(self, budget: u64) -> Option<u64> {
        if self.fits(budget) {
            Some(budget - self.max)
        } else {
            None
        }
    }
}

pub fn analyze(instructions: &[InstructionInfo]) -> CycleRange {
    let mut range = CycleRange::default();
    for instruction in instructions {
        range.add_instruction(instruction);
    }
    range
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{opcode, source::ExpansionTrace, AddressingMode, SourceSpan};

    #[test]
    fn variable_cycles_expand_worst_case() {
        let instructions = vec![
            InstructionInfo {
                address: 0x1000,
                source: SourceSpan {
                    file_id: 0,
                    line: 1,
                    column_start: 1,
                    column_end: 4,
                },
                expansion: ExpansionTrace::default(),
                opcode: opcode("LDA", AddressingMode::AbsoluteX).unwrap(),
            },
            InstructionInfo {
                address: 0x1003,
                source: SourceSpan {
                    file_id: 0,
                    line: 2,
                    column_start: 1,
                    column_end: 4,
                },
                expansion: ExpansionTrace::default(),
                opcode: opcode("BNE", AddressingMode::Relative).unwrap(),
            },
        ];
        let range = analyze(&instructions);
        assert_eq!(range, CycleRange { min: 6, max: 9 });
        assert!(range.fits(9));
        assert_eq!(range.margin(10), Some(1));
    }
}
