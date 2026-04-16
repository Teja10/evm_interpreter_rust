use ruint::aliases::U256;

/// A decoded EVM instruction.
///
/// Note: the EVM's on-the-wire form is a flat `&[u8]` where each byte is an
/// opcode, except for PUSH1..PUSH32 which are immediately followed by 1..32
/// bytes of inline data. We decode that into a typed enum here so the
/// interpreter can pattern-match instead of reading raw bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    Stop,
    Add,
    Mul,
    Sub,
    Div,
    Pop,
    MLoad,
    MStore,
    MStore8,
    /// PUSH1..PUSH32 all decode to the same variant — the distinction only
    /// matters at encode time (how many bytes of immediate to read).
    Push(U256),
    /// DUP1..DUP16. Field is the 1-indexed stack position to duplicate.
    Dup(u8),
    /// SWAP1..SWAP16. Field is the 1-indexed depth to swap with top of stack.
    Swap(u8),
}

/// Decode a byte slice of EVM bytecode into a flat list of instructions.
pub fn decode(bytecode: &[u8]) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut pc = 0;
    while pc < bytecode.len() {
        let op = bytecode[pc];
        match op {
            0x00 => {
                out.push(Instruction::Stop);
                pc += 1;
            }
            0x01 => {
                out.push(Instruction::Add);
                pc += 1;
            }
            0x02 => {
                out.push(Instruction::Mul);
                pc += 1;
            }
            0x03 => {
                out.push(Instruction::Sub);
                pc += 1;
            }
            0x04 => {
                out.push(Instruction::Div);
                pc += 1;
            }
            0x50 => {
                out.push(Instruction::Pop);
                pc += 1;
            }
            0x51 => {
                out.push(Instruction::MLoad);
                pc += 1;
            }
            0x52 => {
                out.push(Instruction::MStore);
                pc += 1;
            }
            0x53 => {
                out.push(Instruction::MStore8);
                pc += 1;
            }
            0x60..=0x7f => {
                let n = (op - 0x5f) as usize;
                assert!(pc + 1 + n <= bytecode.len(), "push exceeds bytecode length");
                let value = U256::from_be_slice(&bytecode[pc + 1..pc + 1 + n]);
                out.push(Instruction::Push(value));
                pc += 1 + n;
            }
            0x80..=0x8f => {
                out.push(Instruction::Dup(op - 0x7f));
                pc += 1;
            }
            0x90..=0x9f => {
                out.push(Instruction::Swap(op - 0x8f));
                pc += 1;
            }
            _ => panic!("unknown opcode: 0x{:02x} at pc={}", op, pc),
        }
    }
    out
}

/// Decode a hex string (with or without `0x` prefix) into instructions.
pub fn decode_hex(hex: &str) -> Vec<Instruction> {
    decode(&hex_to_bytes(hex))
}

fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    let hex = hex.strip_prefix("0x").unwrap_or(&hex);
    assert!(hex.len() % 2 == 0, "hex string has odd length");
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("invalid hex"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruint::uint;

    #[test]
    fn decodes_arithmetic_sequence() {
        // PUSH1 0x02, PUSH1 0x03, ADD, STOP
        let program = decode_hex("6002600301 00");
        assert_eq!(
            program,
            vec![
                Instruction::Push(uint!(2_U256)),
                Instruction::Push(uint!(3_U256)),
                Instruction::Add,
                Instruction::Stop,
            ]
        );
    }

    #[test]
    fn decodes_push32_full_width() {
        // PUSH32 0x00..01 (32 bytes, value = 1)
        let mut hex = String::from("7f");
        hex.push_str(&"00".repeat(31));
        hex.push_str("01");
        assert_eq!(decode_hex(&hex), vec![Instruction::Push(uint!(1_U256))]);
    }

    #[test]
    fn decodes_dup_and_swap_indices() {
        // DUP1, DUP16, SWAP1, SWAP16
        assert_eq!(
            decode_hex("80 8f 90 9f"),
            vec![
                Instruction::Dup(1),
                Instruction::Dup(16),
                Instruction::Swap(1),
                Instruction::Swap(16),
            ]
        );
    }

    #[test]
    fn decodes_memory_and_pop() {
        // PUSH1 0x20, MLOAD, PUSH1 0x00, MSTORE, POP, STOP
        assert_eq!(
            decode_hex("6020 51 6000 52 50 00"),
            vec![
                Instruction::Push(uint!(0x20_U256)),
                Instruction::MLoad,
                Instruction::Push(uint!(0_U256)),
                Instruction::MStore,
                Instruction::Pop,
                Instruction::Stop,
            ]
        );
    }

    #[test]
    #[should_panic(expected = "unknown opcode")]
    fn unknown_opcode_panics() {
        decode_hex("fe");
    }
}
