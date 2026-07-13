# 3. Memory & I/O

The NES CPU can address 64KB of memory, but not all of it is RAM. Different address ranges connect to different hardware components. The bus routes each read/write to the correct destination.

## 3.1 NES Memory Map

```
$0000 - $07FF   2KB     CPU RAM
$0800 - $1FFF           RAM Mirrors
$2000 - $3FFF           PPU Registers
$4000 - $4017           APU & I/O Registers
$4018 - $401F           Test Mode (unused)
$4020 - $FFFF           Cartridge PRG-ROM
```

## 3.2 The Bus Implementation

The bus is the central connector in x-nes. Every CPU memory access passes through it:

```rust
pub struct Bus<'a> {
    pub ram: [u8; 2048],  // 2KB internal RAM
    pub prg: &'a [u8],    // borrowed ROM data
    pub ppu: Ppu,         // Picture Processing Unit
    pub apu: Apu,         // Audio Processing Unit
}
```

The `prg` field borrows ROM data from the caller (the person running the emulator). This means the emulator never needs to allocate memory for ROM — it uses whatever the caller provides.

### 3.2.1 Reading from the Bus

```rust
pub fn read(&mut self, addr: u16) -> u8 {
    let top = (addr >> 12) as u8;
    match top {
        0 | 1 => self.ram[(addr & 0x07FF) as usize],
        2 | 3 => self.read_ppu(addr),
        4 if addr < 0x4020 => self.apu.read(addr),
        _ => self.read_prg(addr),
    }
}
```

The address is shifted right by 12 to extract the top nibble (0-15), which identifies the memory region. This compiles to a single `shr` instruction followed by an indexed jump.

### 3.2.2 Writing to the Bus

```rust
pub fn write(&mut self, addr: u16, val: u8) {
    let top = (addr >> 12) as u8;
    match top {
        0 | 1 => self.ram[(addr & 0x07FF) as usize] = val,
        2 | 3 => self.write_ppu(addr, val),
        4 if addr < 0x4020 => {
            if addr == 0x4014 {
                self.oam_dma(val);
            } else {
                self.apu.write(addr, val);
            }
        }
        _ => {}  // ROM writes are ignored (no mapper yet)
    }
}
```

## 3.3 CPU RAM

The NES has 2KB of RAM ($0000-$07FF). Addresses outside this range, up to $1FFF, are **mirrors** — they reference the same physical memory. The CPU sees its RAM at every 2KB boundary:

```
$0000-$07FF:  Actual RAM
$0800-$0FFF:  Mirror of $0000-$07FF
$1000-$17FF:  Mirror of $0000-$07FF
$1800-$1FFF:  Mirror of $0000-$07FF
```

x-nes handles this with a single mask operation: `addr & 0x07FF` maps all addresses in the range to 0-2047.

## 3.4 PPU Register Access

The PPU has 8 registers ($2000-$2007), mirrored across the entire $2000-$3FFF range. The mask `addr & 7` extracts the register number:

```rust
fn read_ppu(&mut self, addr: u16) -> u8 {
    match addr & 7 {
        2 => self.ppu.read_status(),
        4 => self.ppu.read_oam_data(),
        7 => self.ppu.read_data(),
        _ => 0,
    }
}

fn write_ppu(&mut self, addr: u16, val: u8) {
    match addr & 7 {
        0 => self.ppu.write_ctrl(val),
        1 => self.ppu.write_mask(val),
        3 => self.ppu.write_oam_addr(val),
        4 => self.ppu.write_oam_data(val),
        5 => self.ppu.write_scroll(val),
        6 => self.ppu.write_addr(val),
        7 => self.ppu.write_data(val),
        _ => {}
    }
}
```

Some PPU registers are read-only ($2002), others are write-only ($2000, $2001, $2003, $2005, $2006). Reading a write-only register returns 0.

### 3.4.1 PPU Register Descriptions

**PPUCTRL ($2000)** — Controls high-level PPU operation:

```
Bit 7:   NMI enable (V) — generate NMI at start of vblank
Bit 6:   PPU master/slave (P)
Bit 5:   Sprite size (H) — 0=8x8, 1=8x16
Bit 4:   Background pattern table (B) — 0=$0000, 1=$1000
Bit 3:   Sprite pattern table (S) — 0=$0000, 1=$1000
Bit 2:   VRAM increment (I) — 0=+1, 1=+32
Bits 1-0: Base nametable (NN)
```

**PPUMASK ($2001)** — Controls rendering:

```
Bit 7-5:  Color emphasis (blue, green, red)
Bit 4:    Show sprites
Bit 3:    Show background
Bit 2:    Show sprites in left 8 columns
Bit 1:    Show background in left 8 columns
Bit 0:    Greyscale
```

**PPUSTATUS ($2002)** — Read-only status:

```
Bit 7:    Vblank flag (V) — set when PPU enters vblank
Bit 6:    Sprite 0 hit (S)
Bit 5:    Sprite overflow (O)
Bits 4-0: Unused
```

Reading $2002 clears bit 7 and the write toggle (used by $2005/$2006).

## 3.5 APU & I/O Access

The APU occupies $4000-$4017, with the important exception of $4014 (OAM DMA):

```rust
4 if addr < 0x4020 => {
    if addr == 0x4014 {
        self.oam_dma(val);
    } else {
        self.apu.write(addr, val);
    }
}
```

### 3.5.1 OAM DMA ($4014)

Writing to $4014 triggers a DMA transfer: 256 bytes are copied from CPU memory (page specified by the written value) to PPU sprite RAM (OAM):

```
Writing $02 to $4014 copies $0200-$02FF into OAM
```

```rust
fn oam_dma(&mut self, page: u8) {
    let base = (page as u16) << 8;
    for i in 0..256 {
        let addr = base | i;
        let val = self.read(addr);  // reads from CPU memory or ROM
        self.ppu.oam[self.ppu.oam_addr as usize] = val;
        self.ppu.oam_addr = self.ppu.oam_addr.wrapping_add(1);
    }
}
```

## 3.6 Cartridge PRG-ROM

The cartridge ROM is mapped to $4020-$FFFF. For NROM (mapper 0), the simplest cartridge type:

```rust
fn read_prg(&self, addr: u16) -> u8 {
    if addr < 0x8000 || self.prg.is_empty() {
        return 0;
    }
    self.prg[((addr - 0x8000) as usize) % self.prg.len()]
}
```

16KB ROMs are mirrored — $C000-$FFFF wraps around to $8000-$BFFF. 32KB ROMs fill the entire $8000-$FFFF range without mirroring.

## 3.7 NMI Handling

The PPU can interrupt the CPU at the start of vblank (when it finishes drawing the visible scanlines). This is called a **Non-Maskable Interrupt (NMI)** — the CPU must handle it.

In x-nes, after each instruction executes, we check if the PPU raised an NMI:

```rust
if bus.poll_nmi() {
    nmi(cpu, bus);
}
```

The NMI handler pushes the program counter and status register onto the stack, then loads a new PC from the NMI vector ($FFFA):

```rust
pub fn nmi(cpu: &mut Cpu6502, bus: &mut Bus<'_>) {
    crate::ops::push(cpu, bus, (cpu.pc() >> 8) as u8);
    crate::ops::push(cpu, bus, cpu.pc() as u8);
    let sr = cpu.sr() | 0x20;  // set unused bit
    crate::ops::push(cpu, bus, sr);
    cpu.set_flag(FLAG_INTERRUPT, true);
    let lo = bus.read(0xFFFA) as u16;
    let hi = bus.read(0xFFFB) as u16;
    cpu.set_pc(lo | (hi << 8));
}
```

## Summary

- The bus routes addresses to the correct hardware component
- RAM is mirrored — 2KB appears to fill 8KB of address space
- PPU registers are at $2000-$2007, mirrored every 8 bytes
- OAM DMA copies 256 bytes from CPU memory to PPU sprite memory
- Cartridge ROM wraps around for 16KB banks
- NMI interrupts the CPU at the start of vblank
