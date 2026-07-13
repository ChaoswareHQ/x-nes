# 2. Instruction Set & Addressing Modes

The 6502 has 56 documented instructions and 13 addressing modes. Not every instruction works with every mode, but the combination produces 256 possible opcodes (0x00-0xFF).

## 2.1 Addressing Modes

An instruction needs data to operate on. The **addressing mode** tells the CPU where to find that data. Each mode has its own machine code format and timing.

### 2.1.1 Immediate (#)

The value is embedded directly in the instruction. The operand byte IS the value.

```
Assembly:   LDA #$29      ; Load value 0x29 into A
Machine:    $A9 $29       ; Opcode $A9 = LDA immediate, next byte = value
Cycles:     2
```

```rust
pub fn lda_imm(cpu: &mut Cpu6502, bus: &mut Bus<'_>) -> u8 {
    let val = bus.read(cpu.pc());         // read the immediate value
    cpu.set_pc(cpu.pc().wrapping_add(1)); // advance past it
    cpu.set_a(val);
    cpu.set_sign(val);
    cpu.set_zero(val);
    2
}
```

### 2.1.2 Zero Page (zp)

The address is a single byte (0x00-0xFF), referring to the first page of memory. This is faster than full 16-bit addressing because only one byte needs to be read.

```
Assembly:   LDA $F4       ; Load from address $00F4
Machine:    $A5 $F4       ; Opcode = LDA zero page, address byte
Cycles:     3
```

```rust
pub fn lda_zp(cpu: &mut Cpu6502, bus: &mut Bus<'_>) -> u8 {
    let addr = bus.read(cpu.pc()) as u16;  // one byte address
    cpu.set_pc(cpu.pc().wrapping_add(1));
    let val = bus.read(addr);
    cpu.set_a(val); cpu.set_sign(val); cpu.set_zero(val);
    3
}
```

### 2.1.3 Zero Page, X (zpx)

The address is computed as `(zp_addr + X) & 0xFF`. The result wraps around the zero page — it never crosses into $0100.

```
Assembly:   LDA $20, X    ; Load from ($0020 + X)
Machine:    $B5 $20
Cycles:     4
```

The "wrap around" is important: `$FF + 2 = $01`, not $101.

### 2.1.4 Absolute (abs)

The full 16-bit address follows the opcode, low byte first.

```
Assembly:   LDA $31F6     ; Load from address $31F6
Machine:    $AD $F6 $31   ; Low byte first, then high byte
Cycles:     4
```

```rust
pub fn lda_abs(cpu: &mut Cpu6502, bus: &mut Bus<'_>) -> u8 {
    let lo = bus.read(cpu.pc()) as u16;
    let hi = bus.read(cpu.pc().wrapping_add(1)) as u16;
    cpu.set_pc(cpu.pc().wrapping_add(2));
    let addr = lo | (hi << 8);
    let val = bus.read(addr);
    cpu.set_a(val); cpu.set_sign(val); cpu.set_zero(val);
    4
}
```

### 2.1.5 Absolute, X (absx) and Absolute, Y (absy)

The address is computed as `base + X` (or `base + Y`). If the addition crosses a page boundary, an extra cycle is needed.

```
Assembly:   LDA $31F6, X
Machine:    $DD $F6 $31
Cycles:     4 (no page cross) or 5 (page cross)
```

Page-cross detection in x-nes is branchless:

```rust
pub fn absx(cpu: &Cpu6502, bus: &mut Bus<'_>) -> (u16, u8) {
    let base = abs(cpu, bus);
    let addr = base.wrapping_add(cpu.x() as u16);
    // XOR compares bits 8-15: same page = 0, different = 1
    (addr, ((base ^ addr) >> 8) as u8)
}
```

### 2.1.6 Indirect (for JMP only)

The instruction contains a pointer, and the actual destination is read from that pointer.

```
Assembly:   JMP ($215F)
Machine:    $6C $5F $21
Result:     Jump to the address stored at $215F-$2160
```

This is the only instruction that uses indirect addressing.

### 2.1.7 Indexed Indirect (indx) — Pre-indexed

First adds X to a zero-page address, then reads the effective address from that location.

```
LDA ($3E, X) where X = $05
1. Compute pointer: $3E + $05 = $43
2. Read address from $43-$44: $2415
3. Load value from $2415
```

```rust
pub fn indx(cpu: &Cpu6502, bus: &mut Bus<'_>) -> u16 {
    let ptr = bus.read(cpu.pc()).wrapping_add(cpu.x()) as u16;
    let lo = bus.read(ptr) as u16;
    let hi = bus.read(ptr.wrapping_add(1)) as u16;
    lo | (hi << 8)
}
```

### 2.1.8 Indirect Indexed (indy) — Post-indexed

First reads a base address from zero page, then adds Y.

```
LDA ($4C), Y where Y = $05
1. Read base from $4C-$4D: $2100
2. Add Y: $2100 + $05 = $2105
3. Load value from $2105
```

```rust
pub fn indy(cpu: &Cpu6502, bus: &mut Bus<'_>) -> (u16, u8) {
    let ptr = bus.read(cpu.pc()) as u16;
    let base = bus.read(ptr) as u16 | (bus.read(ptr.wrapping_add(1)) as u16) << 8;
    let addr = base.wrapping_add(cpu.y() as u16);
    (addr, ((base ^ addr) >> 8) as u8)  // page-cross penalty
}
```

### 2.1.9 Relative

Used for branch instructions. A signed byte displacement is added to PC if the branch is taken.

```
BEQ $A7  →  if Z flag is set, PC += $A7 (as signed -89 or +39)
```

The displacement range is -128 to +127 bytes from the instruction after the branch.

### 2.1.10 Implied

The operand is implied by the instruction itself. No additional bytes needed.

```
TAX   →   Transfer A to X (opcode $AA)
CLC   →   Clear carry flag (opcode $18)
```

### 2.1.11 Accumulator

The instruction operates on the accumulator directly, not on memory.

```
LSR A   →   Shift A right by one bit (opcode $4A)
ASL A   →   Shift A left by one bit (opcode $0A)
```

## 2.2 Instruction Categories

### 2.2.1 Data Movement

| Mnemonic | Operation | Flags |
|----------|-----------|-------|
| LDA | Load A from memory | N, Z |
| LDX | Load X from memory | N, Z |
| LDY | Load Y from memory | N, Z |
| STA | Store A to memory | — |
| STX | Store X to memory | — |
| STY | Store Y to memory | — |
| TAX | Transfer A to X | N, Z |
| TAY | Transfer A to Y | N, Z |
| TXA | Transfer X to A | N, Z |
| TYA | Transfer Y to A | N, Z |
| TSX | Transfer ST to X | N, Z |
| TXS | Transfer X to ST | — |

### 2.2.2 Arithmetic

| Mnemonic | Operation | Flags |
|----------|-----------|-------|
| ADC | Add with Carry | N, V, Z, C |
| SBC | Subtract with Borrow | N, V, Z, C |
| INC | Increment memory | N, Z |
| DEC | Decrement memory | N, Z |
| INX | Increment X | N, Z |
| DEX | Decrement X | N, Z |
| INY | Increment Y | N, Z |
| DEY | Decrement Y | N, Z |

### 2.2.3 Logical

| Mnemonic | Operation | Flags |
|----------|-----------|-------|
| AND | A = A & memory | N, Z |
| ORA | A = A \| memory | N, Z |
| EOR | A = A ^ memory | N, Z |
| BIT | Test bits of memory with A | N, V, Z |

### 2.2.4 Shifts and Rotates

| Mnemonic | Operation | Flags |
|----------|-----------|-------|
| ASL | Arithmetic shift left | N, Z, C |
| LSR | Logical shift right | N, Z, C |
| ROL | Rotate left through carry | N, Z, C |
| ROR | Rotate right through carry | N, Z, C |

### 2.2.5 Branches

| Mnemonic | Condition | Checked flag |
|----------|-----------|--------------|
| BPL | Branch if plus | N = 0 |
| BMI | Branch if minus | N = 1 |
| BVC | Branch if overflow clear | V = 0 |
| BVS | Branch if overflow set | V = 1 |
| BCC | Branch if carry clear | C = 0 |
| BCS | Branch if carry set | C = 1 |
| BNE | Branch if not equal | Z = 0 |
| BEQ | Branch if equal | Z = 1 |

### 2.2.6 Jumps and Subroutines

| Mnemonic | Operation |
|----------|-----------|
| JMP | Jump to address |
| JSR | Jump to subroutine (push return address) |
| RTS | Return from subroutine (pull return address) |
| BRK | Software interrupt |
| RTI | Return from interrupt |

### 2.2.7 Stack Operations

| Mnemonic | Operation |
|----------|-----------|
| PHA | Push A onto stack |
| PHP | Push status register onto stack |
| PLA | Pull A from stack |
| PLP | Pull status register from stack |

### 2.2.8 Flag Control

| Mnemonic | Operation |
|----------|-----------|
| CLC | Clear carry |
| SEC | Set carry |
| CLD | Clear decimal mode |
| SED | Set decimal mode |
| CLI | Clear interrupt disable |
| SEI | Set interrupt disable |
| CLV | Clear overflow |
| NOP | No operation |

## 2.3 The Jump Table

With 256 possible opcodes and 56 instructions across 13 modes, the 6502 instruction set maps onto exactly 256 byte values. x-nes encodes this mapping as a static array:

```rust
pub static TABLE: [Op; 256] = {
    use self::op::*;
    [
        // 0x00-0x0F
        brk, ora_indx, illegal, illegal,
        illegal, ora_zp, asl_zp, illegal,
        php, ora_imm, asl_a, illegal,
        illegal, ora_abs, asl_abs, illegal,
        // 0x10-0x1F
        bpl, ora_indy, illegal, illegal,
        // ... continues for all 256 entries ...
    ]
};
```

Each entry is a function pointer. `illegal` is used for undocumented opcodes (treated as NOP).

## 2.4 How ADC Works (Add with Carry)

ADC is the most complex arithmetic instruction. It adds memory + accumulator + carry:

```
result = A + M + C
```

In Rust:

```rust
fn adc_inner(cpu: &mut Cpu6502, operand: u8) {
    let a = cpu.a();
    let carry = cpu.get_flag(FLAG_CARRY) as u16;
    let result = a as u16 + operand as u16 + carry;
    let lo = result as u8;

    if cpu.get_flag(FLAG_DECIMAL) {
        // BCD mode: each nibble is a decimal digit (0-9)
        let low = (a & 0x0F) + (operand & 0x0F) + cpu.get_flag(FLAG_CARRY) as u8;
        let mut temp = result;
        if low > 9 { temp += 6; }          // adjust low digit
        cpu.set_flag(FLAG_CARRY, temp > 0x99);
        if temp > 0x99 { temp += 96; }     // adjust high digit
        cpu.set_a(temp as u8);
    } else {
        // Binary mode: standard 8-bit addition
        cpu.set_sign(lo);
        cpu.set_zero(lo);
        cpu.set_flag(FLAG_CARRY, result > 0xFF);
        cpu.set_flag(FLAG_OVERFLOW, !((a ^ operand) & 0x80) != 0 && ((a ^ lo) & 0x80) != 0);
        cpu.set_a(lo);
    }
}
```

How the overflow flag works:
- In signed arithmetic, overflow occurs when the sign of the result doesn't match the signs of the operands
- `(a ^ operand) & 0x80` checks if a and operand have different signs
- `(a ^ lo) & 0x80` checks if a and the result have different signs
- Both being true means a sign change that shouldn't have happened → overflow

## Summary

- The 6502 has 56 instructions and 13 addressing modes
- The jump table maps all 256 opcodes to their handlers
- Each addressing mode reads its operand differently
- ADC/SBC handle both binary and decimal (BCD) mode
- Branch instructions check status flags and add a signed displacement to PC
