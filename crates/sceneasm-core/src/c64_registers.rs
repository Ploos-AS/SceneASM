#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareRegister {
    pub name: &'static str,
    pub address: u16,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterBitField {
    pub register: u16,
    pub bits: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

pub const VIC_REGISTERS: &[HardwareRegister] = &[
    HardwareRegister { name: "VIC_SPR0_X", address: 0xd000, description: "Sprite 0 X position" },
    HardwareRegister { name: "VIC_SPR0_Y", address: 0xd001, description: "Sprite 0 Y position" },
    HardwareRegister { name: "VIC_SPR_X_MSB", address: 0xd010, description: "Sprite X position high bits" },
    HardwareRegister { name: "VIC_CTRL1", address: 0xd011, description: "VIC-II control register 1: raster high bit, display, bitmap, Y scroll" },
    HardwareRegister { name: "VIC_RASTER", address: 0xd012, description: "Current/IRQ raster line low 8 bits" },
    HardwareRegister { name: "VIC_SPR_ENABLE", address: 0xd015, description: "Sprite enable mask" },
    HardwareRegister { name: "VIC_CTRL2", address: 0xd016, description: "VIC-II control register 2: multicolor, columns, X scroll" },
    HardwareRegister { name: "VIC_SPR_Y_EXPAND", address: 0xd017, description: "Sprite vertical expansion mask" },
    HardwareRegister { name: "VIC_MEMPTR", address: 0xd018, description: "Screen and character/bitmap memory pointers" },
    HardwareRegister { name: "VIC_IRQ_FLAGS", address: 0xd019, description: "VIC-II interrupt status/acknowledge" },
    HardwareRegister { name: "VIC_IRQ_ENABLE", address: 0xd01a, description: "VIC-II interrupt enable mask" },
    HardwareRegister { name: "VIC_SPR_PRIORITY", address: 0xd01b, description: "Sprite/background priority mask" },
    HardwareRegister { name: "VIC_SPR_MULTICOLOR", address: 0xd01c, description: "Sprite multicolor enable mask" },
    HardwareRegister { name: "VIC_SPR_X_EXPAND", address: 0xd01d, description: "Sprite horizontal expansion mask" },
    HardwareRegister { name: "VIC_BORDER", address: 0xd020, description: "Border color" },
    HardwareRegister { name: "VIC_BACKGROUND0", address: 0xd021, description: "Background color 0" },
    HardwareRegister { name: "VIC_BACKGROUND1", address: 0xd022, description: "Background color 1" },
    HardwareRegister { name: "VIC_BACKGROUND2", address: 0xd023, description: "Background color 2" },
    HardwareRegister { name: "VIC_BACKGROUND3", address: 0xd024, description: "Background color 3" },
];

pub const SID_REGISTERS: &[HardwareRegister] = &[
    HardwareRegister { name: "SID_V1_FREQ_LO", address: 0xd400, description: "Voice 1 frequency low byte" },
    HardwareRegister { name: "SID_V1_FREQ_HI", address: 0xd401, description: "Voice 1 frequency high byte" },
    HardwareRegister { name: "SID_V1_PW_LO", address: 0xd402, description: "Voice 1 pulse width low byte" },
    HardwareRegister { name: "SID_V1_PW_HI", address: 0xd403, description: "Voice 1 pulse width high nibble" },
    HardwareRegister { name: "SID_V1_CTRL", address: 0xd404, description: "Voice 1 control and waveform register" },
    HardwareRegister { name: "SID_V1_AD", address: 0xd405, description: "Voice 1 attack and decay" },
    HardwareRegister { name: "SID_V1_SR", address: 0xd406, description: "Voice 1 sustain and release" },
    HardwareRegister { name: "SID_V2_FREQ_LO", address: 0xd407, description: "Voice 2 frequency low byte" },
    HardwareRegister { name: "SID_V2_FREQ_HI", address: 0xd408, description: "Voice 2 frequency high byte" },
    HardwareRegister { name: "SID_V2_CTRL", address: 0xd40b, description: "Voice 2 control and waveform register" },
    HardwareRegister { name: "SID_V3_FREQ_LO", address: 0xd40e, description: "Voice 3 frequency low byte" },
    HardwareRegister { name: "SID_V3_FREQ_HI", address: 0xd40f, description: "Voice 3 frequency high byte" },
    HardwareRegister { name: "SID_V3_CTRL", address: 0xd412, description: "Voice 3 control and waveform register" },
    HardwareRegister { name: "SID_FC_LO", address: 0xd415, description: "Filter cutoff low bits" },
    HardwareRegister { name: "SID_FC_HI", address: 0xd416, description: "Filter cutoff high byte" },
    HardwareRegister { name: "SID_RES_FILT", address: 0xd417, description: "Filter resonance and voice routing" },
    HardwareRegister { name: "SID_MODE_VOL", address: 0xd418, description: "Filter mode and master volume" },
];

pub const CIA1_REGISTERS: &[HardwareRegister] = &[
    HardwareRegister { name: "CIA1_PRA", address: 0xdc00, description: "CIA1 port A data" },
    HardwareRegister { name: "CIA1_PRB", address: 0xdc01, description: "CIA1 port B data" },
    HardwareRegister { name: "CIA1_DDRA", address: 0xdc02, description: "CIA1 port A data direction" },
    HardwareRegister { name: "CIA1_DDRB", address: 0xdc03, description: "CIA1 port B data direction" },
    HardwareRegister { name: "CIA1_TA_LO", address: 0xdc04, description: "CIA1 timer A low byte" },
    HardwareRegister { name: "CIA1_TA_HI", address: 0xdc05, description: "CIA1 timer A high byte" },
    HardwareRegister { name: "CIA1_TB_LO", address: 0xdc06, description: "CIA1 timer B low byte" },
    HardwareRegister { name: "CIA1_TB_HI", address: 0xdc07, description: "CIA1 timer B high byte" },
    HardwareRegister { name: "CIA1_ICR", address: 0xdc0d, description: "CIA1 interrupt control/status" },
    HardwareRegister { name: "CIA1_CRA", address: 0xdc0e, description: "CIA1 timer A control" },
    HardwareRegister { name: "CIA1_CRB", address: 0xdc0f, description: "CIA1 timer B control" },
];

pub const CIA2_REGISTERS: &[HardwareRegister] = &[
    HardwareRegister { name: "CIA2_PRA", address: 0xdd00, description: "CIA2 port A data; VIC-II bank selection uses bits 0-1" },
    HardwareRegister { name: "CIA2_PRB", address: 0xdd01, description: "CIA2 port B data" },
    HardwareRegister { name: "CIA2_DDRA", address: 0xdd02, description: "CIA2 port A data direction" },
    HardwareRegister { name: "CIA2_DDRB", address: 0xdd03, description: "CIA2 port B data direction" },
    HardwareRegister { name: "CIA2_TA_LO", address: 0xdd04, description: "CIA2 timer A low byte" },
    HardwareRegister { name: "CIA2_TA_HI", address: 0xdd05, description: "CIA2 timer A high byte" },
    HardwareRegister { name: "CIA2_TB_LO", address: 0xdd06, description: "CIA2 timer B low byte" },
    HardwareRegister { name: "CIA2_TB_HI", address: 0xdd07, description: "CIA2 timer B high byte" },
    HardwareRegister { name: "CIA2_ICR", address: 0xdd0d, description: "CIA2 interrupt control/status" },
    HardwareRegister { name: "CIA2_CRA", address: 0xdd0e, description: "CIA2 timer A control" },
    HardwareRegister { name: "CIA2_CRB", address: 0xdd0f, description: "CIA2 timer B control" },
];

pub const BIT_FIELDS: &[RegisterBitField] = &[
    RegisterBitField { register: 0xd011, bits: "7", name: "RST8", description: "Raster compare bit 8" },
    RegisterBitField { register: 0xd011, bits: "6", name: "ECM", description: "Extended color text mode" },
    RegisterBitField { register: 0xd011, bits: "5", name: "BMM", description: "Bitmap mode" },
    RegisterBitField { register: 0xd011, bits: "4", name: "DEN", description: "Display enable" },
    RegisterBitField { register: 0xd011, bits: "3", name: "RSEL", description: "24/25 row select" },
    RegisterBitField { register: 0xd011, bits: "2-0", name: "YSCROLL", description: "Fine vertical scroll" },
    RegisterBitField { register: 0xd016, bits: "4", name: "MCM", description: "Multicolor mode" },
    RegisterBitField { register: 0xd016, bits: "3", name: "CSEL", description: "38/40 column select" },
    RegisterBitField { register: 0xd016, bits: "2-0", name: "XSCROLL", description: "Fine horizontal scroll" },
    RegisterBitField { register: 0xd018, bits: "7-4", name: "VM", description: "Screen memory pointer within VIC bank" },
    RegisterBitField { register: 0xd018, bits: "3-1", name: "CB", description: "Character/bitmap memory pointer within VIC bank" },
    RegisterBitField { register: 0xd019, bits: "7", name: "IRQ", description: "VIC-II IRQ status" },
    RegisterBitField { register: 0xd019, bits: "3", name: "LP", description: "Light pen IRQ flag" },
    RegisterBitField { register: 0xd019, bits: "2", name: "MMC", description: "Sprite-sprite collision IRQ flag" },
    RegisterBitField { register: 0xd019, bits: "1", name: "MBC", description: "Sprite-background collision IRQ flag" },
    RegisterBitField { register: 0xd019, bits: "0", name: "RST", description: "Raster IRQ flag" },
    RegisterBitField { register: 0xd01a, bits: "3-0", name: "IRQ_ENABLE", description: "Enable VIC-II LP/MMC/MBC/raster IRQ sources" },
    RegisterBitField { register: 0xd404, bits: "7", name: "NOISE", description: "Noise waveform" },
    RegisterBitField { register: 0xd404, bits: "6", name: "PULSE", description: "Pulse waveform" },
    RegisterBitField { register: 0xd404, bits: "5", name: "SAW", description: "Sawtooth waveform" },
    RegisterBitField { register: 0xd404, bits: "4", name: "TRI", description: "Triangle waveform" },
    RegisterBitField { register: 0xd404, bits: "3", name: "TEST", description: "Test bit" },
    RegisterBitField { register: 0xd404, bits: "2", name: "RING", description: "Ring modulation" },
    RegisterBitField { register: 0xd404, bits: "1", name: "SYNC", description: "Oscillator synchronization" },
    RegisterBitField { register: 0xd404, bits: "0", name: "GATE", description: "Envelope gate" },
    RegisterBitField { register: 0xd418, bits: "7", name: "3OFF", description: "Disconnect voice 3 from direct output" },
    RegisterBitField { register: 0xd418, bits: "6-4", name: "MODE", description: "High-pass, band-pass and low-pass filter enables" },
    RegisterBitField { register: 0xd418, bits: "3-0", name: "VOL", description: "Master volume" },
    RegisterBitField { register: 0xdc0d, bits: "7", name: "SETCLR", description: "Set/clear selected interrupt mask bits when written" },
    RegisterBitField { register: 0xdc0d, bits: "4-0", name: "IRQ", description: "CIA1 FLAG/serial/timer B/timer A interrupt sources" },
    RegisterBitField { register: 0xdd00, bits: "1-0", name: "VIC_BANK", description: "Inverted VIC-II 16 KiB bank selection" },
];

pub fn bit_fields(address: u16) -> impl Iterator<Item = RegisterBitField> {
    BIT_FIELDS.iter().copied().filter(move |field| field.register == address)
}

pub fn registers() -> impl Iterator<Item = HardwareRegister> {
    VIC_REGISTERS.iter()
        .chain(SID_REGISTERS)
        .chain(CIA1_REGISTERS)
        .chain(CIA2_REGISTERS)
        .copied()
}

pub fn register_by_name(name: &str) -> Option<HardwareRegister> {
    registers().find(|register| register.name.eq_ignore_ascii_case(name))
}

pub fn register_by_address(address: u16) -> Option<HardwareRegister> {
    registers().find(|register| register.address == address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raster_register_is_available_by_name_and_address() {
        assert_eq!(register_by_name("vic_raster").unwrap().address, 0xd012);
        assert_eq!(register_by_address(0xd012).unwrap().name, "VIC_RASTER");
    }
}
