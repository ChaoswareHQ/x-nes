# 1. The 6502 CPU

The 6502 is an 8-bit microprocessor designed by MOS Technology in 1975. It powered the NES, Commodore 64, Apple II, and countless other systems. Understanding its architecture is the first step to building an emulator.

## 1.1 CPU Registers

The 6502 has six registers. In the real chip, these are physical storage locations inside the processor. In our emulator, they are fields in a struct.

```
Register   Size    Name
─────────────────────────────
PC         16-bit  Program Counter
A          8-bit   Accumulator
X          8-bit   Index Register X
Y          8-bit   Index Register Y
ST         8-bit   Stack Pointer
SR         8-bit   Status Register (flags)
```

### 1.1.1 Program Counter (PC)

The PC holds the address of the next instruction to execute. When the CPU finishes one instruction, it reads from the address in PC, executes, and advances PC past that instruction.

```
Start:    PC → $C000  (read opcode here)
After:    PC → $C001  (PC advanced by 1)
```

In x-nes, PC is stored as two bytes (little-endian) at the start of our CPU struct:

```rust
pub struct CpuRp2a03 {
    bytes: [u8; 7],
    // [0..2] = PC (low byte at index 0, high byte at index 1)
    // [2]    = A
    // [3]    = X
    // [4]    = Y
    // [5]    = ST
    // [6]    = SR
}
```

The PC value is reconstructed from the two bytes:

```rust
pub fn pc(&self) -> u16 {
    u16::from_le_bytes([self.bytes[0], self.bytes[1]])
}
```

Why 7 bytes? The 6502's registers total exactly 7 bytes: one 16-bit value (PC) and five 8-bit values (A, X, Y, ST, SR). By storing them as raw bytes, we avoid alignment issues and keep the struct compact.

### 1.1.2 Accumulator (A)

The accumulator is the primary working register. All arithmetic (addition, subtraction) and logical operations (AND, OR, XOR) use it. Most data movement involves loading values into A or storing A to memory.

| Instruction | What it does |
|-------------|--------------|
| `LDA #$05` | Load value 5 into A |
| `ADC #$03` | Add 3 to A (with carry) |
| `STA $0200` | Store A to address $0200 |

### 1.1.3 Index Registers (X and Y)

X and Y serve multiple purposes:
- **Indexing**: Adding to memory addresses for array access
- **Counting**: Loop counters (DEX, DEY, INX, INY)
- **Copying**: Transferring values to/from A (TAX, TAY, TXA, TYA)

X and Y are similar but not identical — some instructions work only with X, others only with Y.

### 1.1.4 Stack Pointer (ST)

The 6502 uses a hardware stack at $0100-$01FF. The stack pointer holds the low byte of the current stack address. It starts at $FF (meaning the first push goes to $01FF) and grows downward.

```
Stack operations:
  PHA  → Push A onto stack (ST decrements)
  PLA  → Pull value from stack into A (ST increments)
  JSR  → Push return address, jump to subroutine
  RTS  → Pull return address from stack, return from subroutine
```

In x-nes, push and pull are straightforward:

```rust
fn push(cpu: &mut CpuRp2a03, bus: &mut Bus<'_>, val: u8) {
    bus.write(0x0100 | cpu.st() as u16, val);
    cpu.set_st(cpu.st().wrapping_sub(1));
}

fn pull(cpu: &mut CpuRp2a03, bus: &mut Bus<'_>) -> u8 {
    cpu.set_st(cpu.st().wrapping_add(1));
    bus.read(0x0100 | cpu.st() as u16)
}
```

The stack page ($0100) is fixed in hardware. `0x0100 | cpu.st() as u16` assembles the full address.

### 1.1.5 Status Register (SR)

The status register contains 8 flag bits that change based on instruction results:

```
Bit  7     6     5     4     3     2     1     0
    ┌─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┐
    │  N  │  V  │  -  │  B  │  D  │  I  │  Z  │  C  │
    └─────┴─────┴─────┴─────┴─────┴─────┴─────┴─────┘
```

| Bit | Name | Meaning |
|-----|------|---------|
| 0 | C | Carry — set if arithmetic result exceeds 255 (or borrow in subtraction) |
| 1 | Z | Zero — set if result is zero |
| 2 | I | Interrupt Disable — if set, IRQ is ignored |
| 3 | D | Decimal Mode — if set, ADC/SBC use BCD arithmetic |
| 4 | B | Break — set when BRK instruction executes |
| 5 | — | Unused (always reads as 1 on real hardware) |
| 6 | V | Overflow — set if signed arithmetic overflows |
| 7 | N | Negative — set if bit 7 of result is 1 |

In x-nes, flags are stored as bit constants:

```rust
pub const FLAG_CARRY: u8    = 0b0000_0001;
pub const FLAG_ZERO: u8     = 0b0000_0010;
pub const FLAG_INTERRUPT: u8 = 0b0000_0100;
pub const FLAG_DECIMAL: u8  = 0b0000_1000;
pub const FLAG_BREAK: u8    = 0b0001_0000;
pub const FLAG_OVERFLOW: u8 = 0b0100_0000;
pub const FLAG_NEGATIVE: u8 = 0b1000_0000;
```

Reading and setting individual flags:

```rust
pub fn get_flag(&self, flag: u8) -> bool {
    self.bytes[6] & flag != 0
}

pub fn set_flag(&mut self, flag: u8, set: bool) {
    let mask = set as u8;
    self.bytes[6] = (self.bytes[6] & !flag) | (mask * flag);
}
```

When `set` is true, `mask` is 1 and `mask * flag` equals `flag` — the flag bit becomes 1. When `set` is false, `mask` is 0 and `mask * flag` is 0 — the flag bit becomes 0. No `if` statement needed.

## 1.2 The NES Memory Model

The CPU sees a 16-bit address space (64KB). The NES divides this into regions:

```
Address Range    Size    Region
──────────────────────────────────────
$0000-$07FF     2KB     Internal CPU RAM
$0800-$1FFF     —       RAM Mirrors (repeats every 2KB)
$2000-$3FFF     —       PPU Registers (repeats every 8 bytes)
$4000-$401F     32B     APU & I/O Registers
$4020-$FFFF     —       Cartridge PRG-ROM
```

### 1.2.1 RAM and Mirroring

The NES has only 2KB of CPU RAM, but it appears to occupy 8KB of address space ($0000-$1FFF). Addresses $0800-$1FFF are **mirrors** — they reference the same physical RAM cells. Our emulator handles this with a bitmask:

```rust
pub fn read(&mut self, addr: u16) -> u8 {
    let top = (addr >> 12) as u8;
    match top {
        0 | 1 => self.ram[(addr & 0x07FF) as usize],
        // $0000-$1FFF: all redirected to 2KB RAM via mask
```

`addr & 0x07FF` maps all addresses in the $0000-$1FFF range to 0-2047.

### 1.2.2 Cartridge ROM

Games are stored on cartridges containing ROM chips. The PRG-ROM is mapped to $4020-$FFFF. Most NES games use 16KB or 32KB of PRG-ROM. For 16KB cartridges, the ROM is mirrored — it appears at both $8000-$BFFF and $C000-$FFFF.

```rust
fn read_prg(&self, addr: u16) -> u8 {
    if addr < 0x8000 || self.prg.is_empty() {
        return 0;
    }
    self.prg[((addr - 0x8000) as usize) % self.prg.len()]
}
```

The modulo automatically handles mirroring: a 16KB ROM (16384 bytes) at address $C000 gives `$C000 - $8000 = $4000 = 16384`, and `16384 % 16384 = 0` — it wraps to the start.

## 1.3 How Instructions Execute

Every instruction follows the same three-step cycle:

```
1. FETCH   — Read the opcode from the address in PC
2. DECODE  — Look up which instruction it is
3. EXECUTE — Run the instruction, return cycle count
```

In x-nes, this is implemented in `lib.rs`:

```rust
pub fn tick(cpu: &mut CpuRp2a03, bus: &mut Bus<'_>) -> u8 {
    // 1. FETCH: read the opcode byte
    let opcode = bus.read(cpu.pc());

    // 2. DECODE + ADVANCE: move PC past the opcode
    cpu.set_pc(cpu.pc().wrapping_add(1));

    // 3. EXECUTE: jump to the correct handler
    let cycles = TABLE[opcode as usize](cpu, bus);

    // Advance PPU and APU in sync
    bus.ppu.tick_batch((cycles as u16) * 3);
    bus.apu.tick(cycles);

    cycles
}
```

The decode step uses a **jump table** — an array of 256 function pointers, one per possible opcode:

```rust
type Op = for<'a> fn(&mut CpuRp2a03, &mut Bus<'a>) -> u8;

pub static TABLE: [Op; 256] = [
    brk, ora_indx, illegal, illegal, ...
];
```

`TABLE[opcode as usize]` selects the right function in constant time. No searching, no matching — just a lookup and a call.

## 1.4 The Complete CPU State

Putting it all together, the CPU struct contains everything needed to represent the 6502's state:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CpuRp2a03 {
    bytes: [u8; 7],
}
```

With accessor methods for each register:

| Method | Reads | Writes |
|--------|-------|--------|
| `pc()` / `set_pc()` | bytes[0..2] as u16 | bytes[0..2] from u16 |
| `a()` / `set_a()` | bytes[2] | bytes[2] |
| `x()` / `set_x()` | bytes[3] | bytes[3] |
| `y()` / `set_y()` | bytes[4] | bytes[4] |
| `st()` / `set_st()` | bytes[5] | bytes[5] |
| `sr()` / `set_sr()` | bytes[6] | bytes[6] |

Plus flag helpers:
- `get_flag(flag)` — test a specific flag
- `set_flag(flag, set)` — set or clear a specific flag
- `set_sign(val)` — set N flag based on bit 7 of val
- `set_zero(val)` — set Z flag if val is zero

## Summary

- The 6502 has 6 registers totaling 7 bytes
- The NES memory map has RAM, PPU registers, APU registers, and cartridge ROM
- Instruction execution follows fetch-decode-execute
- Flags track the results of operations
- The stack lives at $0100-$01FF and grows downward
