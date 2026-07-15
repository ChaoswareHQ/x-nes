pub struct Gamepad {
    pub a: bool,
    pub b: bool,
    pub select: bool,
    pub start: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub strobe: bool,
    index: u8,
    buttons: [u8; 2],
}

impl Gamepad {
    pub fn new() -> Self {
        Self {
            a: false,
            b: false,
            select: false,
            start: false,
            up: false,
            down: false,
            left: false,
            right: false,
            strobe: false,
            index: 0,
            buttons: [0; 2],
        }
    }

    fn latch(&mut self) {
        // NES controller serial output order (first read = bit 0, etc.):
        // A (0x01), B (0x02), Select (0x04), Start (0x08),
        // Up (0x10), Down (0x20), Left (0x40), Right (0x80)
        self.buttons[0] = 0;
        self.buttons[1] = 0;
        if self.a {
            self.buttons[0] |= 0x01;
        }
        if self.b {
            self.buttons[0] |= 0x02;
        }
        if self.select {
            self.buttons[0] |= 0x04;
        }
        if self.start {
            self.buttons[0] |= 0x08;
        }
        if self.up {
            self.buttons[0] |= 0x10;
        }
        if self.down {
            self.buttons[0] |= 0x20;
        }
        if self.left {
            self.buttons[0] |= 0x40;
        }
        if self.right {
            self.buttons[0] |= 0x80;
        }
    }

    pub fn write(&mut self, val: u8) {
        self.strobe = val & 0x01 != 0;
        if self.strobe {
            self.index = 0;
            self.latch();
        }
    }

    pub fn read(&mut self) -> u8 {
        if self.strobe {
            self.latch();
            return self.buttons[0] & 0x01;
        }
        let bit = if self.index < 8 {
            (self.buttons[0] >> self.index) & 0x01
        } else {
            0x01
        };
        self.index = self.index.wrapping_add(1);
        bit
    }
}
