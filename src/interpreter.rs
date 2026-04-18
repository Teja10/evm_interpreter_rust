use ruint::aliases::U256;

use crate::memory::Memory;
use crate::opcode::Instruction;
use crate::stack::Stack;

/// A toy EVM interpreter: executes a decoded instruction list against a
/// Stack and Memory. No gas, no storage, no host — just raw compute.
#[derive(Debug)]
pub struct Interpreter {
    pub stack: Stack,
    pub memory: Memory,
    instructions: Vec<Instruction>,
    /// Index into `instructions`. Distinct from the EVM's `pc`, which indexes
    /// into the raw bytecode; we'll unify these in Stage 3 when jumps arrive.
    ip: usize,
    halted: bool,
}

impl Interpreter {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            stack: Stack::new(),
            memory: Memory::new(),
            instructions,
            ip: 0,
            halted: false,
        }
    }

    /// Run until STOP or end of instructions.
    pub fn run(&mut self) {
        while !self.halted && self.ip < self.instructions.len() {
            let inst = self.instructions[self.ip].clone();
            self.ip += 1;
            self.step(inst);
        }
    }

    fn step(&mut self, inst: Instruction) {
        // TODO(human): implement the remaining opcode handlers below.
        //
        // Semantics to get right:
        //
        //   Add / Mul / Sub: pop two operands (top of stack is the FIRST
        //     operand, per the EVM spec). Arithmetic is modular 2^256, so
        //     use U256's `wrapping_add` / `wrapping_mul` / `wrapping_sub`.
        //     SUB computes (top - second), NOT (second - top).
        //
        //   Div: pops a (top) then b, pushes a / b. Division by zero in the
        //     EVM does NOT trap — it pushes zero. U256's `checked_div` gives
        //     you `Option<U256>`, which maps naturally to this.
        //
        //   MLoad:  pop offset, push mem.read_word(offset).
        //   MStore: pop offset, pop value, mem.write_word(offset, value).
        //   MStore8: pop offset, pop value, write value.byte(0) at offset.
        //     (Ruint U256 → low byte: `value.as_limbs()[0] as u8`, or convert
        //      via `value.to::<u8>()` if it fits — be deliberate.)
        //
        //   Dup(n) / Swap(n): delegate to `self.stack.dup(n as usize)` and
        //     `self.stack.swap(n as usize)` — those methods already exist.
        //
        // For offsets coming off the stack, convert U256 → usize with
        // `.to::<usize>()` (ruint method). That will panic if the value
        // doesn't fit — fine for a toy; the real EVM would out-of-gas long
        // before an offset gets that large.
        match inst {
            Instruction::Stop => self.halted = true,
            Instruction::Push(v) => self.stack.push(v),
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Add => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                self.stack.push(a.wrapping_add(b));
            },
            Instruction::Mul => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                self.stack.push(a.wrapping_mul(b));
            }
            Instruction::Sub => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                self.stack.push(a.wrapping_sub(b));
            },
            Instruction::Div => {
                let a = self.stack.pop();
                let b = self.stack.pop();
                if b == U256::ZERO {
                    self.stack.push(U256::ZERO);
                } else {
                    self.stack.push(a.wrapping_div(b));
                }
            },
       
            Instruction::MLoad => {
                let offset = self.stack.pop().to::<usize>();
                self.stack.push(self.memory.read_word(offset));
            },
            Instruction::MStore => {
                let offset = self.stack.pop().to::<usize>();
                let value = self.stack.pop();
                self.memory.write_word(offset, value);
            },
            Instruction::MStore8 => {
                let offset = self.stack.pop().to::<usize>();
                let value = self.stack.pop().byte(0);
                self.memory.write_byte(offset, value);
            },
            Instruction::Dup(n) => {
                self.stack.dup(n as usize);
    
            },
            Instruction::Swap(n) => {
                self.stack.swap(n as usize);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::decode_hex;
    use ruint::aliases::U256;
    use ruint::uint;

    fn run(hex: &str) -> Interpreter {
        let mut vm = Interpreter::new(decode_hex(hex));
        vm.run();
        vm
    }

    #[test]
    fn add() {
        // PUSH1 2, PUSH1 3, ADD, STOP → top = 5
        let vm = run("6002 6003 01 00");
        assert_eq!(vm.stack.peek(0), uint!(5_U256));
    }

    #[test]
    fn add_wraps_modulo_2_to_the_256() {
        // PUSH32 MAX, PUSH1 1, ADD → 0 (wrap)
        let mut hex = String::from("7f");
        hex.push_str(&"ff".repeat(32));
        hex.push_str("60 01 01 00");
        let vm = run(&hex);
        assert_eq!(vm.stack.peek(0), U256::ZERO);
    }

    #[test]
    fn sub_is_top_minus_second() {
        // PUSH1 3, PUSH1 10, SUB → 10 - 3 = 7 (top minus what's below)
        let vm = run("6003 600a 03 00");
        assert_eq!(vm.stack.peek(0), uint!(7_U256));
    }

    #[test]
    fn mul() {
        // PUSH1 6, PUSH1 7, MUL → 42
        let vm = run("6006 6007 02 00");
        assert_eq!(vm.stack.peek(0), uint!(42_U256));
    }

    #[test]
    fn div_by_zero_is_zero() {
        // PUSH1 0, PUSH1 5, DIV → 5 / 0 = 0 (EVM semantics — no trap)
        let vm = run("6000 6005 04 00");
        assert_eq!(vm.stack.peek(0), U256::ZERO);
    }

    #[test]
    fn mstore_then_mload() {
        // PUSH1 0xbe, PUSH1 0, MSTORE, PUSH1 0, MLOAD → 0xbe
        let vm = run("60be 6000 52 6000 51 00");
        assert_eq!(vm.stack.peek(0), uint!(0xbe_U256));
    }

    #[test]
    fn dup1_duplicates_top() {
        // PUSH1 9, DUP1 → [9, 9]
        let vm = run("6009 80 00");
        assert_eq!(vm.stack.len(), 2);
        assert_eq!(vm.stack.peek(0), uint!(9_U256));
        assert_eq!(vm.stack.peek(1), uint!(9_U256));
    }

    #[test]
    fn swap1_exchanges_top_two() {
        // PUSH1 1, PUSH1 2, SWAP1 → top = 1, second = 2
        let vm = run("6001 6002 90 00");
        assert_eq!(vm.stack.peek(0), uint!(1_U256));
        assert_eq!(vm.stack.peek(1), uint!(2_U256));
    }

    #[test]
    fn pop_removes_top() {
        // PUSH1 1, PUSH1 2, POP → top = 1
        let vm = run("6001 6002 50 00");
        assert_eq!(vm.stack.len(), 1);
        assert_eq!(vm.stack.peek(0), uint!(1_U256));
    }

    #[test]
    fn dup3_copies_third_from_top() {
        let vm = run("6001 6002 6003 82 00");
        assert_eq!(vm.stack.len(), 4);
        assert_eq!(vm.stack.peek(0), uint!(1_U256));
    }

    #[test]
    fn swap2_non_adjacent() {
        let vm = run("6001 6002 6003 91 00");
        assert_eq!(vm.stack.peek(0), uint!(1_U256));
        assert_eq!(vm.stack.peek(2), uint!(3_U256));
    }

    #[test]
    fn mul_wraps() {
        let mut hex = String::from("7f");
        hex.push_str(&"ff".repeat(32));
        hex.push_str("6002 02 00");
        let vm = run(&hex);
        assert_eq!(vm.stack.peek(0), U256::MAX - uint!(1_U256));
    }

    #[test]
    fn sub_underflow_wraps() {
        let vm = run("6001 6000 03 00");
        assert_eq!(vm.stack.peek(0), U256::MAX);
    }

    #[test]
    fn div_truncates() {
        let vm =  run("6003 600a 04 00");
        assert_eq!(vm.stack.peek(0), uint!(3_U256));
    }

    #[test]
    fn div_small_by_large() {
        let vm = run("6010 6003 04 00");
        assert_eq!(vm.stack.peek(0), uint!(0_U256));
    }

    #[test]
    fn mstore8_ignores_high_bytes() {
        let vm = run("611234 6000 53 6000 51 00");
        assert_eq!(vm.stack.peek(0), uint!(0x34_U256) << 248);
    }

    #[test]
    fn compound_test() {
        let vm = run("6004 6003 6002 01 02 00");
        assert_eq!(vm.stack.peek(0), uint!(20_U256));
    }
    // TODO(human): add robustness tests below. The existing tests above only
    // hit the happy path for each opcode and won't catch several real bugs.
    // Priority order (most useful first):
    //
    //   1. dup3_copies_third_from_top
    //        PUSH1 1, PUSH1 2, PUSH1 3, DUP3, STOP
    //        expect: stack.len() == 4, stack.peek(0) == 1
    //        (This one catches a latent bug in DUP — write this first.)
    //
    //   2. swap2_non_adjacent
    //        PUSH1 1, PUSH1 2, PUSH1 3, SWAP2, STOP
    //        expect: peek(0) == 1, peek(2) == 3 (positions 0 and 2 exchanged)
    //
    //   3. mul_wraps
    //        PUSH32 MAX, PUSH1 2, MUL → MAX - 1
    //        (2 * (2^256 - 1) mod 2^256 = 2^256 - 2)
    //
    //   4. sub_underflow_wraps
    //        PUSH1 1, PUSH1 0, SUB → U256::MAX
    //
    //   5. div_truncates
    //        PUSH1 3, PUSH1 10, DIV → 3
    //
    //   6. div_small_by_large
    //        PUSH1 10, PUSH1 3, DIV → 0
    //
    //   7. mstore8_ignores_high_bytes
    //        PUSH2 0x1234, PUSH1 0, MSTORE8, PUSH1 0, MLOAD → 0x34 << 248
    //        (byte 0 holds 0x34 — the low byte of 0x1234; MLOAD reads
    //         big-endian, so that byte becomes the most-significant byte.)
    //
    //   8. compound_program
    //        compute (2 + 3) * 4 using PUSH/ADD/MUL; expect top = 20.
    //
    // Write them in any order; run `cargo test --lib interpreter` as you go.
    // Delete this TODO block when you're done.
}
