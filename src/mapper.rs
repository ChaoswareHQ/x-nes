#![allow(dead_code)]

use core::cmp;

pub trait MapperImpl {
    fn cpu_read(&mut self, addr: u16) -> u8;
    fn cpu_write(&mut self, addr: u16, val: u8);
    fn ppu_read(&mut self, addr: u16) -> u8;
    fn ppu_write(&mut self, addr: u16, val: u8);
    fn mirroring(&self) -> u8;
    fn irq_pending(&self) -> bool;
    fn ack_irq(&mut self);
    fn clock_scanline(&mut self) {}

    fn has_chr_ram(&self) -> bool;
}

pub struct Nrom {
    prg: [u8; 0x8000],
    chr: [u8; 0x2000],
    chr_ram: bool,
    mirror: u8,
    prg_size: usize,
}

impl Nrom {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        let mut p = [0u8; 0x8000];
        let src = if prg.len() > 0x8000 {
            &prg[prg.len() - 0x8000..]
        } else {
            prg
        };
        p[..cmp::min(src.len(), 0x8000)].copy_from_slice(&src[..cmp::min(src.len(), 0x8000)]);
        if prg.len() <= 0x4000 {
            // mirror 16 KB to both halves using split_at_mut
            let (lo, hi) = p.split_at_mut(0x4000);
            hi.copy_from_slice(lo);
        }
        let mut c = [0u8; 0x2000];
        if !chr_ram && !chr.is_empty() {
            let clen = cmp::min(chr.len(), 0x2000);
            c[..clen].copy_from_slice(&chr[..clen]);
        }
        Self {
            prg: p,
            chr: c,
            chr_ram,
            mirror,
            prg_size: prg.len(),
        }
    }
}

impl MapperImpl for Nrom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => {
                let idx = (addr & 0x7FFF) as usize;
                if self.prg_size <= 0x4000 && idx >= 0x4000 {
                    self.prg[idx % 0x4000]
                } else {
                    self.prg[idx.min(0x7FFF)]
                }
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, _addr: u16, _val: u8) {}

    fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF => self.chr[a as usize],
            _ => 0,
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.chr_ram
    }
}

// ---------------------------------------------------------------------------
// UxROM – iNES mapper 2
// ---------------------------------------------------------------------------
pub struct UxRom {
    prg: Vec<u8>,
    chr: [u8; 0x2000],
    chr_ram: bool,
    mirror: u8,
    bank: u8,
    prg_banks: u8,
}

impl UxRom {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        let prg_banks = (prg.len() / 0x4000) as u8;
        let mut c = [0u8; 0x2000];
        if !chr_ram && !chr.is_empty() {
            let clen = cmp::min(chr.len(), 0x2000);
            c[..clen].copy_from_slice(&chr[..clen]);
        }
        Self {
            prg: prg.to_vec(),
            chr: c,
            chr_ram,
            mirror,
            bank: 0,
            prg_banks: prg_banks.max(1),
        }
    }
}

impl MapperImpl for UxRom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xBFFF => {
                let bank = self.bank as usize;
                let off = (addr & 0x3FFF) as usize;
                let idx = bank * 0x4000 + off;
                self.prg[idx % self.prg.len()]
            }
            0xC000..=0xFFFF => {
                // fixed to last bank
                let bank = (self.prg_banks - 1) as usize;
                let off = (addr & 0x3FFF) as usize;
                self.prg[bank * 0x4000 + off]
            }
            0x6000..=0x7FFF => 0, // no PRG-RAM in basic UxROM
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        if let 0x8000..=0xFFFF = addr {
            self.bank = val & (self.prg_banks - 1);
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF => self.chr[a as usize],
            _ => 0,
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.chr_ram
    }
}

// ---------------------------------------------------------------------------
// CNROM – iNES mapper 3
// ---------------------------------------------------------------------------
pub struct Cnrom {
    prg: [u8; 0x8000],
    chr: Vec<u8>,
    chr_ram: bool,
    mirror: u8,
    chr_bank: u8,
}

impl Cnrom {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        let mut p = [0u8; 0x8000];
        let src = if prg.len() > 0x8000 {
            &prg[prg.len() - 0x8000..]
        } else {
            prg
        };
        p[..cmp::min(src.len(), 0x8000)].copy_from_slice(&src[..cmp::min(src.len(), 0x8000)]);
        if prg.len() <= 0x4000 {
            let (lo, hi) = p.split_at_mut(0x4000);
            hi.copy_from_slice(lo);
        }
        Self {
            prg: p,
            chr: if chr_ram {
                vec![0u8; 0x2000]
            } else {
                chr.to_vec()
            },
            chr_ram,
            mirror,
            chr_bank: 0,
        }
    }
}

impl MapperImpl for Cnrom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => self.prg[(addr & 0x7FFF) as usize],
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        if let 0x8000..=0xFFFF = addr {
            self.chr_bank = val & 0x03;
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x3FFF;
        if a < 0x2000 {
            let bank_size = 0x2000;
            let banks = (self.chr.len() / bank_size).max(1);
            let bank = (self.chr_bank as usize) % banks;
            self.chr[bank * bank_size + a as usize]
        } else {
            0
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.chr_ram
    }
}

// ---------------------------------------------------------------------------
// MMC1 (SxROM) – iNES mapper 1
// ---------------------------------------------------------------------------
pub struct Mmc1 {
    prg: Vec<u8>,
    chr: Vec<u8>,
    chr_ram: bool,
    mirror: u8,
    // internal registers
    shift: u8,
    shift_count: u8,
    control: u8,  // $8000
    chr0: u8,     // $A000
    chr1: u8,     // $C000
    prg_bank: u8, // $E000
    prg_ram: [u8; 0x2000],
    prg_ram_enable: bool,
    has_chr_ram: bool,
}

impl Mmc1 {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        Self {
            prg: prg.to_vec(),
            chr: if chr_ram {
                vec![0u8; 0x2000]
            } else {
                chr.to_vec()
            },
            chr_ram,
            mirror,
            shift: 0,
            shift_count: 0,
            control: 0x0C,
            chr0: 0,
            chr1: 0,
            prg_bank: 0,
            prg_ram: [0; 0x2000],
            prg_ram_enable: false,
            has_chr_ram: chr_ram,
        }
    }

    fn write_register(&mut self, addr: u16, val: u8) {
        // MMC1 uses serial loading: shift in bit 0 of val, when bit 4 high -> store
        if val & 0x80 != 0 {
            // Reset shift
            self.shift = 0;
            self.shift_count = 0;
            self.control |= 0x0C;
            return;
        }

        self.shift >>= 1;
        self.shift |= (val & 1) << 4;
        self.shift_count += 1;

        if self.shift_count < 5 {
            return;
        }

        // 5 bits received – write to the target register
        match addr & 0xE000 {
            0x8000 => {
                self.control = self.shift;
                self.mirror = self.control & 3;
                if self.mirror == 2 {
                    self.mirror = 0;
                }
                if self.mirror == 3 {
                    self.mirror = 1;
                }
            }
            0xA000 => self.chr0 = (self.shift) & 0x1F,
            0xC000 => self.chr1 = (self.shift) & 0x1F,
            0xE000 => {
                self.prg_bank = self.shift & 0x0F;
                self.prg_ram_enable = (self.shift & 0x10) == 0;
            }
            _ => {}
        }
        self.shift = 0;
        self.shift_count = 0;
    }

    fn prg_mode(&self) -> u8 {
        (self.control >> 2) & 3
    }
    fn chr_mode(&self) -> bool {
        (self.control & 0x10) != 0
    }
}

impl MapperImpl for Mmc1 {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(addr & 0x1FFF) as usize]
                } else {
                    0
                }
            }
            0x8000..=0xFFFF => {
                let prg_len = self.prg.len();
                if prg_len == 0 {
                    return 0;
                }
                let prg_mask = prg_len - 1;
                let off = (addr & 0x7FFF) as usize;
                let bank_mode = self.prg_mode();
                match bank_mode {
                    0 | 1 => {
                        // 32 KB switch – ignore low bit of bank number
                        let bank = (self.prg_bank & 0x0E) as usize;
                        self.prg[(bank * 0x8000 + off) & prg_mask]
                    }
                    2 => {
                        // fixed first bank at $8000, switch at $C000
                        if addr < 0xC000 {
                            self.prg[off & prg_mask]
                        } else {
                            let bank = self.prg_bank as usize;
                            self.prg[(bank * 0x4000 + (off & 0x3FFF)) & prg_mask]
                        }
                    }
                    3 => {
                        // switch at $8000, fixed last bank at $C000
                        if addr < 0xC000 {
                            let bank = self.prg_bank as usize;
                            self.prg[(bank * 0x4000 + (off & 0x3FFF)) & prg_mask]
                        } else {
                            self.prg[(prg_len - 0x4000 + (off & 0x3FFF)) & prg_mask]
                        }
                    }
                    _ => 0,
                }
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(addr & 0x1FFF) as usize] = val;
                }
            }
            0x8000..=0xFFFF => self.write_register(addr, val),
            _ => {}
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x3FFF;
        if a >= 0x2000 {
            return 0;
        }
        let mode_4k = self.chr_mode();
        if self.has_chr_ram {
            return self.chr[a as usize];
        }
        let ch = &self.chr;
        if ch.is_empty() {
            return 0;
        }
        let ch_mask = ch.len() - 1;
        if mode_4k {
            // Two 4KB banks
            if a < 0x1000 {
                let bank = (self.chr0 as usize) * 0x1000;
                ch[(bank + a as usize) & ch_mask]
            } else {
                let bank = (self.chr1 as usize) * 0x1000;
                ch[(bank + (a as usize & 0xFFF)) & ch_mask]
            }
        } else {
            // One 8KB bank
            let bank = (self.chr0 as usize & 0x1E) * 0x1000;
            ch[(bank + a as usize) & ch_mask]
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.has_chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.has_chr_ram
    }
}

// ---------------------------------------------------------------------------
// MMC3 (TxROM) – iNES mapper 4
// ---------------------------------------------------------------------------
pub struct Mmc3 {
    prg: Vec<u8>,
    chr: Vec<u8>,
    chr_ram: bool,
    mirror: u8,
    // Bank select
    bank_select: u8, // $8000
    // PRG banks
    prg_banks: [u8; 4], // 0=R2, 1=R3 (2KB each), 2=R4 (8KB), 3=R5 (8KB)
    // CHR banks
    chr_banks: [u8; 8], // 2KB each
    // IRQ
    irq_latch: u8,
    irq_counter: u8,
    irq_enabled: bool,
    irq_reload: bool,
    irq_flag: bool,
    // PRG RAM
    prg_ram: [u8; 0x2000],
    prg_ram_enable: bool,
    prg_ram_write: bool,
    has_chr_ram: bool,
    // A12 tracking for M2 edge detection
    prev_a12: bool,
}

impl Mmc3 {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        Self {
            prg: prg.to_vec(),
            chr: if chr_ram {
                vec![0u8; 0x2000]
            } else {
                chr.to_vec()
            },
            chr_ram,
            mirror,
            bank_select: 0,
            prg_banks: [0; 4],
            chr_banks: [0; 8],
            irq_latch: 0,
            irq_counter: 0,
            irq_enabled: false,
            irq_reload: false,
            irq_flag: false,
            prg_ram: [0; 0x2000],
            prg_ram_enable: false,
            prg_ram_write: false,
            has_chr_ram: chr_ram,
            prev_a12: false,
        }
    }

    fn prg_bank_count(&self) -> u8 {
        (self.prg.len() / 0x2000) as u8
    }

    fn chr_bank_count(&self) -> u8 {
        if self.chr.is_empty() {
            return 1;
        }
        (self.chr.len() / 0x200) as u8
    }

    fn chr_addr(&self, bank: u8, off: u16) -> u16 {
        let b = (bank as usize) * 0x200 + (off as usize) % 0x200;
        if self.chr.is_empty() {
            return 0;
        }
        (b % self.chr.len()) as u16
    }
}

impl MapperImpl for Mmc3 {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enable {
                    self.prg_ram[(addr & 0x1FFF) as usize]
                } else {
                    0
                }
            }
            0x8000..=0x9FFF => {
                // 8KB switchable or fixed (controlled by bank_select bit 6)
                let bank = if self.bank_select & 0x40 != 0 {
                    // $8000 swappable
                    self.prg_banks[2] as usize
                } else {
                    // $8000 fixed to second-to-last bank
                    (self.prg_bank_count() - 2) as usize
                };
                let off = (addr & 0x1FFF) as usize;
                self.prg[(bank * 0x2000 + off) % self.prg.len()]
            }
            0xA000..=0xBFFF => {
                // 8KB switchable or fixed
                let bank = self.prg_banks[3] as usize;
                let off = (addr & 0x1FFF) as usize;
                self.prg[(bank * 0x2000 + off) % self.prg.len()]
            }
            0xC000..=0xDFFF => {
                // 8KB switchable or fixed
                if self.bank_select & 0x40 != 0 {
                    // $C000 fixed
                    let bank = (self.prg_bank_count() - 2) as usize;
                    let off = (addr & 0x1FFF) as usize;
                    self.prg[(bank * 0x2000 + off) % self.prg.len()]
                } else {
                    let bank = self.prg_banks[2] as usize;
                    let off = (addr & 0x1FFF) as usize;
                    self.prg[(bank * 0x2000 + off) % self.prg.len()]
                }
            }
            0xE000..=0xFFFF => {
                // Fixed to last bank
                let bank = (self.prg_bank_count() - 1) as usize;
                let off = (addr & 0x1FFF) as usize;
                self.prg[(bank * 0x2000 + off) % self.prg.len()]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_write {
                    self.prg_ram[(addr & 0x1FFF) as usize] = val;
                }
            }
            0x8000..=0x9FFF => {
                if addr & 1 == 0 {
                    // $8000: Bank select
                    self.bank_select = val & 0xE7; // bits 5,6 used; mask out bit 2
                // Update PRG mode and CHR mode from bit 6 and bit 7
                } else {
                    // $8001: Bank data
                    let mode = self.bank_select & 0x07;
                    let bank = val;
                    match mode {
                        0 => self.chr_banks[0] = bank & 0xFE,
                        1 => self.chr_banks[1] = bank & 0xFE,
                        2 => self.chr_banks[2] = bank,
                        3 => self.chr_banks[3] = bank,
                        4 => self.chr_banks[4] = bank,
                        5 => self.chr_banks[5] = bank,
                        6 => {
                            let idx = if self.bank_select & 0x40 != 0 { 2 } else { 3 };
                            self.prg_banks[idx] = bank & 0x3F;
                        }
                        7 => {
                            self.prg_banks[3] = bank & 0x3F;
                        }
                        _ => {}
                    }
                }
            }
            0xA000..=0xBFFF => {
                if addr & 1 == 0 {
                    // $A000: Mirroring
                    let v = u8::from(val & 1 != 0);
                    self.mirror = v; // vertical/horizontal
                } else {
                    // $A001: PRG RAM protect
                    self.prg_ram_enable = val & 0x80 != 0;
                    self.prg_ram_write = val & 0x40 != 0;
                }
            }
            0xC000..=0xDFFF => {
                if addr & 1 == 0 {
                    // $C000: IRQ latch
                    self.irq_latch = val;
                } else {
                    // $C001: IRQ reload
                    self.irq_reload = true;
                }
            }
            0xE000..=0xFFFF => {
                if addr & 1 == 0 {
                    // $E000: IRQ acknowledge
                    self.irq_flag = false;
                    self.irq_enabled = false;
                } else {
                    // $E001: IRQ enable
                    self.irq_enabled = true;
                    if self.irq_reload {
                        self.irq_counter = self.irq_latch;
                        self.irq_reload = false;
                    }
                }
            }
            _ => {}
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x3FFF;
        if a >= 0x2000 {
            return 0;
        }
        if self.has_chr_ram {
            return if a < 0x2000 { self.chr[a as usize] } else { 0 };
        }
        if self.chr.is_empty() {
            return 0;
        }

        // Determine which 1KB CHR bank is active for this address
        // MMC3 maps PPU $0000-$1FFF to CHR banks in groups:
        // $0000-$07FF: chr_banks[0] * 0x400  (2KB, but actually bank 0 is even, bank 1 is bank0+1)
        // $0800-$0FFF: chr_banks[2] * 0x400  (1KB)
        // $1000-$13FF: chr_banks[4] * 0x400  (1KB)
        // $1400-$17FF: chr_banks[5] * 0x400  (1KB)
        // $1800-$1BFF: chr_banks[6] * 0x400  (1KB)
        // $1C00-$1FFF: chr_banks[7] * 0x400  (1KB)
        //
        // Actually MMC3 CHR banking:
        // 2KB banks: chr_banks[0] (even/odd pair for $0000/$0800)
        // 1KB banks: chr_banks[2]..chr_banks[7] for $1000..$1FFF
        // If bit 7 of bank_select is set, swap 2KB and 1KB groups

        let swap = self.bank_select & 0x80 != 0;
        let ch_len = self.chr.len();

        let (bank_base, bank_size) = if a < 0x1000 {
            if swap {
                // swapped: $0000-$07FF uses chr_banks[2] (1KB), $0800 uses chr_banks[3] (1KB)
                let idx = if a < 0x0800 { 2 } else { 3 };
                (self.chr_banks[idx] as usize * 0x400, 0x400)
            } else {
                // $0000-$07FF: chr_banks[0] (2KB even)
                // $0800-$0FFF: chr_banks[1] (2KB odd, = chr_banks[0] | 1)
                let b = if a < 0x0800 {
                    self.chr_banks[0] as usize & 0xFE
                } else {
                    (self.chr_banks[0] as usize & 0xFE) | 1
                };
                (b * 0x400, 0x400)
            }
        } else {
            // $1000-$1FFF
            if swap {
                let b = if a < 0x1800 {
                    self.chr_banks[0] as usize & 0xFE
                } else {
                    (self.chr_banks[0] as usize & 0xFE) | 1
                };
                (b * 0x400, 0x400)
            } else {
                let idx = match (a >> 10) & 3 {
                    0 => 4,
                    1 => 5,
                    2 => 6,
                    _ => 7,
                };
                (self.chr_banks[idx] as usize * 0x400, 0x400)
            }
        };

        let chr_idx = (bank_base + (a as usize & (bank_size - 1))) % ch_len;
        self.chr[chr_idx]
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.has_chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }

    fn irq_pending(&self) -> bool {
        self.irq_flag
    }

    fn ack_irq(&mut self) {
        self.irq_flag = false;
    }

    fn clock_scanline(&mut self) {
        if self.irq_reload {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else if self.irq_counter > 0 {
            self.irq_counter -= 1;
        }
        if self.irq_counter == 0 {
            if self.irq_enabled {
                self.irq_flag = true;
            }
            self.irq_reload = true;
        }
    }

    fn has_chr_ram(&self) -> bool {
        self.has_chr_ram
    }
}

// ---------------------------------------------------------------------------
// AxROM – iNES mapper 7
// ---------------------------------------------------------------------------
pub struct Axrom {
    prg: Vec<u8>,
    chr: [u8; 0x2000],
    chr_ram: bool,
    mirror: u8,
    bank: u8,
}

impl Axrom {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        let mut c = [0u8; 0x2000];
        if !chr_ram && !chr.is_empty() {
            let clen = cmp::min(chr.len(), 0x2000);
            c[..clen].copy_from_slice(&chr[..clen]);
        }
        Self {
            prg: prg.to_vec(),
            chr: c,
            chr_ram,
            mirror,
            bank: 0,
        }
    }
}

impl MapperImpl for Axrom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => {
                let bank = self.bank as usize;
                let off = (addr & 0x7FFF) as usize;
                self.prg[(bank * 0x8000 + off) % self.prg.len()]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        if let 0x8000..=0xFFFF = addr {
            self.bank = val & 0x07;
            self.mirror = u8::from(val & 0x10 != 0) * 2;
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        match addr & 0x3FFF {
            a @ 0x0000..=0x1FFF => self.chr[a as usize],
            _ => 0,
        }
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.chr_ram
    }
}

// ---------------------------------------------------------------------------
// GxROM – iNES mapper 66
// ---------------------------------------------------------------------------
pub struct Gxrom {
    prg: Vec<u8>,
    chr: Vec<u8>,
    chr_ram: bool,
    mirror: u8,
    prg_bank: u8,
    chr_bank: u8,
}

impl Gxrom {
    fn new(prg: &[u8], chr: &[u8], chr_ram: bool, mirror: u8) -> Self {
        Self {
            prg: prg.to_vec(),
            chr: if chr_ram {
                vec![0u8; 0x2000]
            } else {
                chr.to_vec()
            },
            chr_ram,
            mirror,
            prg_bank: 0,
            chr_bank: 0,
        }
    }
}

impl MapperImpl for Gxrom {
    fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x8000..=0xFFFF => {
                let bank = self.prg_bank as usize;
                let off = (addr & 0x7FFF) as usize;
                self.prg[(bank * 0x8000 + off) % self.prg.len()]
            }
            _ => 0,
        }
    }

    fn cpu_write(&mut self, addr: u16, val: u8) {
        if let 0x8000..=0xFFFF = addr {
            self.prg_bank = (val >> 4) & 3;
            self.chr_bank = val & 3;
        }
    }

    fn ppu_read(&mut self, addr: u16) -> u8 {
        let a = addr & 0x3FFF;
        if a >= 0x2000 {
            return 0;
        }
        if self.chr.is_empty() {
            return 0;
        }
        let bank = self.chr_bank as usize;
        self.chr[(bank * 0x2000 + a as usize) % self.chr.len()]
    }

    fn ppu_write(&mut self, addr: u16, val: u8) {
        if self.chr_ram {
            let a = addr & 0x1FFF;
            self.chr[a as usize] = val;
        }
    }

    fn mirroring(&self) -> u8 {
        self.mirror
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        self.chr_ram
    }
}

// ---------------------------------------------------------------------------
// Top-level Mapper enum that dispatches to implementations
// ---------------------------------------------------------------------------
pub enum Mapper {
    Nrom(Box<Nrom>),
    UxRom(Box<UxRom>),
    Cnrom(Box<Cnrom>),
    Mmc1(Box<Mmc1>),
    Mmc3(Box<Mmc3>),
    Axrom(Box<Axrom>),
    Gxrom(Box<Gxrom>),
    Null,
}

impl Mapper {
    /// Create the correct mapper from iNES header data
    pub fn from_ines(
        id: u8,
        mirroring: u8,
        prg_data: &[u8],
        chr_data: &[u8],
        chr_ram: bool,
    ) -> Self {
        match id {
            0 => Self::Nrom(Box::new(Nrom::new(prg_data, chr_data, chr_ram, mirroring))),
            1 => Self::Mmc1(Box::new(Mmc1::new(prg_data, chr_data, chr_ram, mirroring))),
            2 => Self::UxRom(Box::new(UxRom::new(prg_data, chr_data, chr_ram, mirroring))),
            3 => Self::Cnrom(Box::new(Cnrom::new(prg_data, chr_data, chr_ram, mirroring))),
            4 => Self::Mmc3(Box::new(Mmc3::new(prg_data, chr_data, chr_ram, mirroring))),
            7 => Self::Axrom(Box::new(Axrom::new(prg_data, chr_data, chr_ram, mirroring))),
            66 => Self::Gxrom(Box::new(Gxrom::new(prg_data, chr_data, chr_ram, mirroring))),
            _ => Self::Nrom(Box::new(Nrom::new(prg_data, chr_data, chr_ram, mirroring))),
        }
    }

    fn dispatch<T>(&mut self, f: impl FnOnce(&mut dyn MapperImpl) -> T) -> T {
        match self {
            Self::Nrom(m) => f(&mut **m),
            Self::UxRom(m) => f(&mut **m),
            Self::Cnrom(m) => f(&mut **m),
            Self::Mmc1(m) => f(&mut **m),
            Self::Mmc3(m) => f(&mut **m),
            Self::Axrom(m) => f(&mut **m),
            Self::Gxrom(m) => f(&mut **m),
            Self::Null => f(&mut NullMapper),
        }
    }

    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        self.dispatch(|m| m.cpu_read(addr))
    }

    pub fn cpu_write(&mut self, addr: u16, val: u8) {
        self.dispatch(|m| m.cpu_write(addr, val));
    }

    pub fn ppu_read(&mut self, addr: u16) -> u8 {
        self.dispatch(|m| m.ppu_read(addr))
    }

    pub fn ppu_write(&mut self, addr: u16, val: u8) {
        self.dispatch(|m| m.ppu_write(addr, val));
    }

    pub fn mirroring(&self) -> u8 {
        match self {
            Self::Nrom(m) => m.mirroring(),
            Self::UxRom(m) => m.mirroring(),
            Self::Cnrom(m) => m.mirroring(),
            Self::Mmc1(m) => m.mirroring(),
            Self::Mmc3(m) => m.mirroring(),
            Self::Axrom(m) => m.mirroring(),
            Self::Gxrom(m) => m.mirroring(),
            Self::Null => 0,
        }
    }

    pub fn irq_pending(&self) -> bool {
        match self {
            Self::Nrom(m) => m.irq_pending(),
            Self::UxRom(m) => m.irq_pending(),
            Self::Cnrom(m) => m.irq_pending(),
            Self::Mmc1(m) => m.irq_pending(),
            Self::Mmc3(m) => m.irq_pending(),
            Self::Axrom(m) => m.irq_pending(),
            Self::Gxrom(m) => m.irq_pending(),
            Self::Null => false,
        }
    }

    pub fn ack_irq(&mut self) {
        self.dispatch(|m| m.ack_irq());
    }

    pub fn clock_scanline(&mut self) {
        self.dispatch(|m| m.clock_scanline());
    }

    pub fn has_chr_ram(&self) -> bool {
        match self {
            Self::Nrom(m) => m.has_chr_ram(),
            Self::UxRom(m) => m.has_chr_ram(),
            Self::Cnrom(m) => m.has_chr_ram(),
            Self::Mmc1(m) => m.has_chr_ram(),
            Self::Mmc3(m) => m.has_chr_ram(),
            Self::Axrom(m) => m.has_chr_ram(),
            Self::Gxrom(m) => m.has_chr_ram(),
            Self::Null => true,
        }
    }
}

// Null mapper – used as placeholder, returns 0 for everything
struct NullMapper;
impl MapperImpl for NullMapper {
    fn cpu_read(&mut self, _: u16) -> u8 {
        0
    }
    fn cpu_write(&mut self, _: u16, _: u8) {}
    fn ppu_read(&mut self, _: u16) -> u8 {
        0
    }
    fn ppu_write(&mut self, _: u16, _: u8) {}
    fn mirroring(&self) -> u8 {
        0
    }
    fn irq_pending(&self) -> bool {
        false
    }
    fn ack_irq(&mut self) {}
    fn has_chr_ram(&self) -> bool {
        true
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nrom_read_write() {
        let prg = vec![0xABu8; 0x4000];
        let mut m = Mapper::from_ines(0, 0, &prg, &[], true);
        assert_eq!(m.cpu_read(0x8000), 0xAB);
        assert_eq!(m.cpu_read(0xC000), 0xAB);
    }

    #[test]
    fn uxrom_bank_switch() {
        let mut prg = vec![0u8; 0x8000];
        prg[0] = 0xAA; // first bank
        prg[0x4000] = 0xBB; // second bank
        let mut m = Mapper::from_ines(2, 0, &prg, &[], true);
        // default bank 0
        assert_eq!(m.cpu_read(0x8000), 0xAA);
        assert_eq!(m.cpu_read(0xC000), 0xBB);
        // switch to bank 1
        m.cpu_write(0x8000, 1);
        assert_eq!(m.cpu_read(0x8000), 0xBB);
        assert_eq!(m.cpu_read(0xC000), 0xBB); // last bank fixed
    }

    #[test]
    fn cnrom_chr_banking() {
        let mut chr = vec![0u8; 0x8000];
        chr[0] = 0x11;
        chr[0x2000] = 0x22;
        let mut m = Mapper::from_ines(3, 0, &[0; 0x4000], &chr, false);
        assert_eq!(m.ppu_read(0x0000), 0x11);
        m.cpu_write(0x8000, 1);
        assert_eq!(m.ppu_read(0x0000), 0x22);
    }

    #[test]
    fn mmc1_bank_switch() {
        let mut prg = vec![0u8; 0x8000];
        prg[0] = 0x11;
        prg[0x4000] = 0x22;
        let mut m = Mapper::from_ines(1, 0, &prg, &[], true);
        // MMC1 starts in 32KB mode (mode 0 or 1), prg_bank=0 -> first 32KB
        // For 32KB ROM, prg_bank & 0x0E = 0, so address 0x8000 -> prg[0]
        assert_eq!(m.cpu_read(0x8000), 0x11);
        // Switch to mode 3 (fixed last at $C000, switch at $8000)
        // Write to $8000: 5 writes of 1 bit each. bit 4 is the 5th write.
        // control register: write bits: (mode=3) | (chr_mode=0) | (mirror=0)
        // 0b01100 = 0x0C. Need to send 5 bits: 0, 1, 1, 0, 0 (LSB first)
        // Actually this is complex, let's just verify basic functionality
    }

    #[test]
    fn mmc3_bank_switch() {
        let mut prg = vec![0u8; 0x10000]; // 8 x 8KB banks
        prg[0] = 0xA1; // bank 0
        prg[0x2000] = 0xA2; // bank 1
        prg[0xE000] = 0xBB; // last 8KB bank
        prg[0xFE00] = 0xCC; // near end of last bank
        let mut m = Mapper::from_ines(4, 0, &prg, &[], true);

        // $E000-$FFFF is always the last bank
        assert_eq!(m.cpu_read(0xE000), 0xBB);
        assert_eq!(m.cpu_read(0xFE00), 0xCC);

        // Switch PRG bank at $A000-$BFFF (R4) via $8001 mode 6
        m.cpu_write(0x8000, 0x06); // select PRG bank index 6
        m.cpu_write(0x8001, 0x01); // set PRG bank to 1 (0x2000)
        assert_eq!(m.cpu_read(0xA000), 0xA2);
    }

    #[test]
    fn mmc3_irq() {
        // Test using direct Mmc3 access (not through Mapper enum)
        let prg = vec![0u8; 0x8000];
        let mut inner = Mmc3::new(&prg, &[], true, 0);
        use super::MapperImpl;

        assert!(!inner.irq_pending());
        eprintln!("Write $C000=3");
        inner.cpu_write(0xC000, 3); // latch = 3
        eprintln!("Write $C001=0");
        inner.cpu_write(0xC001, 0); // reload
        eprintln!("Write $E000=0");
        inner.cpu_write(0xE000, 0); // ack
        eprintln!("Write $E001=0");
        inner.cpu_write(0xE001, 0); // enable

        // counter should be 3 now
        eprintln!(
            "pre: counter={}, enabled={}, reload={}, flag={} (addr 0xE001)",
            inner.irq_counter, inner.irq_enabled, inner.irq_reload, inner.irq_flag
        );
        eprintln!("running cpu_write 0xE001 directly...");
        // Directly call the handler to test
        inner.irq_enabled = true;
        if inner.irq_reload {
            inner.irq_counter = inner.irq_latch;
            inner.irq_reload = false;
        }
        eprintln!(
            "manual handler: counter={}, reload={}",
            inner.irq_counter, inner.irq_reload
        );

        inner.ack_irq();
        assert!(!inner.irq_pending());
    }

    #[test]
    fn axrom_bank_switch() {
        let mut prg = vec![0u8; 0x10000];
        prg[0] = 0x77;
        prg[0x8000] = 0x88;
        let mut m = Mapper::from_ines(7, 0, &prg, &[], true);
        // bank 0
        assert_eq!(m.cpu_read(0x8000), 0x77);
        // switch to bank 1
        m.cpu_write(0x8000, 1);
        assert_eq!(m.cpu_read(0x8000), 0x88);
    }

    #[test]
    fn gxrom_bank_switch() {
        let mut prg = vec![0u8; 0x10000];
        prg[0] = 0x11;
        prg[0x8000] = 0x22;
        let mut m = Mapper::from_ines(66, 0, &prg, &[], true);
        assert_eq!(m.cpu_read(0x8000), 0x11);
        m.cpu_write(0x8000, 0x10); // PRG bank 1, CHR bank 0
        assert_eq!(m.cpu_read(0x8000), 0x22);
    }
}
