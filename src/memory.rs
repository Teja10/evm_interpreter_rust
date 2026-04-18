use ruint::aliases::U256;

/// EVM memory: a byte-addressed, auto-expanding buffer.
///
/// Key semantics:
/// - Reads and writes are allowed at any offset. If the access extends past
///   the current end of memory, the buffer grows (zero-filled) to cover it.
/// - The real EVM expands memory in 32-byte *words* and charges gas per
///   word. We preserve the word-aligned expansion here (it's cheap and keeps
///   the mental model right) but don't charge gas yet.
/// - MLOAD/MSTORE operate on 32-byte words stored big-endian.
#[derive(Debug, Default)]
pub struct Memory {
    bytes: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Grow memory so that byte index `end - 1` is addressable.
    /// Rounds up to the next 32-byte boundary, matching EVM semantics.
    fn expand_to(&mut self, end: usize) {
        if end > self.bytes.len() {
            let word_aligned = end.div_ceil(32) * 32;
            self.bytes.resize(word_aligned, 0);
        }
    }

    /// MLOAD: read a 32-byte word starting at `offset`, big-endian.
    pub fn read_word(&mut self, offset: usize) -> U256 {
        self.expand_to(offset + 32);
        U256::from_be_slice(&self.bytes[offset..offset + 32])
    }

    /// MSTORE: write a 32-byte word starting at `offset`, big-endian.
    pub fn write_word(&mut self, offset: usize, value: U256) {
        self.expand_to(offset + 32);
        let bytes: [u8; 32] = value.to_be_bytes();
        self.bytes[offset..offset + 32].copy_from_slice(&bytes);
    }

    /// MSTORE8: write a single low-order byte at `offset`.
    pub fn write_byte(&mut self, offset: usize, value: u8) {
        self.expand_to(offset + 1);
        self.bytes[offset] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruint::uint;

    #[test]
    fn write_then_read_word() {
        let mut m = Memory::new();
        m.write_word(0, uint!(0xdeadbeef_U256));
        assert_eq!(m.read_word(0), uint!(0xdeadbeef_U256));
    }

    #[test]
    fn expansion_is_word_aligned() {
        let mut m = Memory::new();
        // Touching byte 0..32 requires exactly 32 bytes.
        m.write_word(0, uint!(1_U256));
        assert_eq!(m.len(), 32);
        // Touching byte 40 forces a second word (bytes 32..64).
        m.write_byte(40, 0xff);
        assert_eq!(m.len(), 64);
    }

    #[test]
    fn reading_unwritten_region_is_zero_and_expands() {
        let mut m = Memory::new();
        assert_eq!(m.read_word(0), uint!(0_U256));
        assert_eq!(m.len(), 32);
    }

    #[test]
    fn mstore8_low_byte_only() {
        let mut m = Memory::new();
        // MSTORE8 should only write the least-significant byte, 0xab.
        m.write_byte(31, 0xab);
        let word = m.read_word(0);
        // Byte 31 is the least-significant byte of a big-endian word → value 0xab.
        assert_eq!(word, uint!(0xab_U256));
    }
}
