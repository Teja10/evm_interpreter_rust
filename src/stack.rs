use ruint::aliases::U256;

/// A LIFO stack of 256-bit words. The real EVM caps this at 1024 entries;
/// we skip that check for the toy interpreter and will revisit it later.
#[derive(Debug, Default)]
pub struct Stack {
    items: Vec<U256>,
}

impl Stack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn push(&mut self, value: U256) {
        self.items.push(value);
    }

    pub fn pop(&mut self) -> U256 {
        self.items.pop().expect("stack underflow")
    }

    /// Peek at an item without removing it. `depth = 0` is the top of stack,
    /// `depth = 1` is the next item down, etc.
    pub fn peek(&self, depth: usize) -> U256 {
        let n = self.items.len();
        assert!(depth < n, "stack underflow on peek(depth={depth}, len={n})");
        self.items[n - 1 - depth]
    }

    /// DUP_n: duplicate the n-th item (1-indexed from the top) onto the stack.
    /// DUP1 copies the top; DUP2 copies the second-from-top; etc.
    pub fn dup(&mut self, n: usize) {
        assert!(n >= 1, "dup index must be >= 1");
        self.push(self.peek(n - 1));
    }

    /// SWAP_n: exchange the top of stack with the n-th item below it
    /// (1-indexed). SWAP1 swaps top with the one directly below, etc.
    pub fn swap(&mut self, n: usize) {
        assert!(n >= 1, "swap index must be >= 1");
        let len = self.items.len();
        assert!(len > n, "stack underflow on swap(n={n}, len={len})");
        self.items.swap(len - 1, len - 1 - n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ruint::uint;

    #[test]
    fn push_pop_roundtrip() {
        let mut s = Stack::new();
        s.push(uint!(42_U256));
        assert_eq!(s.len(), 1);
        assert_eq!(s.pop(), uint!(42_U256));
        assert!(s.is_empty());
    }

    #[test]
    fn peek_is_zero_indexed_from_top() {
        let mut s = Stack::new();
        s.push(uint!(1_U256));
        s.push(uint!(2_U256));
        s.push(uint!(3_U256));
        assert_eq!(s.peek(0), uint!(3_U256));
        assert_eq!(s.peek(1), uint!(2_U256));
        assert_eq!(s.peek(2), uint!(1_U256));
    }

    #[test]
    fn dup1_copies_top() {
        let mut s = Stack::new();
        s.push(uint!(7_U256));
        s.dup(1);
        assert_eq!(s.len(), 2);
        assert_eq!(s.peek(0), uint!(7_U256));
        assert_eq!(s.peek(1), uint!(7_U256));
    }

    #[test]
    fn swap1_exchanges_top_two() {
        let mut s = Stack::new();
        s.push(uint!(1_U256));
        s.push(uint!(2_U256));
        s.swap(1);
        assert_eq!(s.pop(), uint!(1_U256));
        assert_eq!(s.pop(), uint!(2_U256));
    }

    #[test]
    #[should_panic(expected = "stack underflow")]
    fn pop_empty_panics() {
        Stack::new().pop();
    }
}
