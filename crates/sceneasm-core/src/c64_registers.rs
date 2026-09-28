#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HardwareRegister {
    pub name: &'static str,
    pub address: u16,
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

pub fn register_by_name(name: &str) -> Option<HardwareRegister> {
    VIC_REGISTERS.iter().copied().find(|register| register.name.eq_ignore_ascii_case(name))
}

pub fn register_by_address(address: u16) -> Option<HardwareRegister> {
    VIC_REGISTERS.iter().copied().find(|register| register.address == address)
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
