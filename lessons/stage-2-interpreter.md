# Stage 2 — The Interpreter: Stack, Memory, and the Fetch-Execute Loop

## What an interpreter actually is

Strip away all the Ethereum-specific vocabulary and an EVM interpreter is
just a **state machine driven by a linear program**. At any given moment, its
state is a tuple:

```
(stack, memory, ip, halted)
```

Each "step" of execution reads `instructions[ip]`, advances `ip`, and mutates
`stack` and `memory` according to what that instruction says. When the
instruction is `STOP`, `halted` flips to true and the loop ends. That's it.

The entire real EVM is this loop plus:
- Gas accounting (Stage 3)
- Jumps rewriting `ip` (Stage 3)
- Storage and a `Host` trait for world-state access (Stage 5)
- Sub-calls pushing a whole new interpreter onto a call stack (Stage 6)

Everything fancy about Reth and revm is *layers around* this core.

## The stack

EVM is a **stack machine**, not a register machine. There are no variables,
no named slots — just one global stack of 256-bit words. Every arithmetic
operation pops its operands off the top and pushes its result back.

### Pop order matters

Consider `SUB`. Solidity-level `a - b` compiles to:

```
PUSH <b>    ; pushes b
PUSH <a>    ; pushes a (so a is now on top)
SUB         ; pops a, pops b, pushes (a - b)
```

The *top* of the stack is the **first** operand, not the second. This feels
backwards coming from register machines, but it falls out of the compilation
strategy: to compute `f(x, y)` in a stack machine, you evaluate `y` first
(pushing its result), then `x` (pushing its result on top), then invoke `f`.
When `f` starts executing, it sees `x` on top.

This matters for non-commutative ops: `ADD` and `MUL` don't care about pop
order (they're commutative), but `SUB` and `DIV` do. Getting it wrong is the
single most common bug when hand-writing an interpreter.

### DUP and SWAP exist because the stack is the only addressing mode

Since there are no variables, if you want to use a value more than once you
must `DUP` it before consuming it. Want to swap two values? `SWAP1`. Want to
reach the fifth-from-top value? `DUP5` (copies it to the top).

Solidity's compiler spends a lot of its lowering logic on stack scheduling —
figuring out when to `DUP` a value vs. recompute it, when to `SWAP` to get an
operand into position. This is why compiled EVM bytecode has so many `DUP`
and `SWAP` instructions sprinkled in.

## Modular arithmetic

All EVM arithmetic is **modulo 2^256**. Adding two U256s that overflow wraps
around silently — there is no trap, no flag, no exception. This is the
semantics of `wrapping_add`, `wrapping_mul`, `wrapping_sub` in Rust.

This is why we chose `ruint` carefully: by default, `U256 + U256` in `ruint`
is **checked** in debug builds (panics on overflow) and **wrapping** in
release builds. If we wrote `self.stack.push(a + b)` and ran the tests in
debug mode, `MAX + 1` would panic instead of wrapping to 0. We want
deterministic EVM semantics regardless of build profile, so we explicitly
call `.wrapping_add()`.

### Division by zero

The EVM spec is very deliberate here: `x / 0 = 0`. No trap. This lets
compilers emit division instructions without having to guard every one with a
zero-check. It's also why `U256::checked_div` maps perfectly:

```rust
let result = a.checked_div(b).unwrap_or(U256::ZERO);
```

Same pattern holds for `MOD`, `ADDMOD`, `MULMOD` (Stage 3+).

## Memory

EVM memory is a linear, byte-addressed, zero-initialized buffer that grows
on demand. Three facts that matter:

1. **It's transient.** Memory exists only for the duration of a single call
   frame. When a contract returns or reverts, its memory is thrown away.
   Persistent state lives in **storage** (Stage 5), which is completely
   separate and much more expensive.

2. **Access auto-expands.** Writing one byte at offset 1,000,000 causes
   memory to grow to cover that offset. In the real EVM this costs gas
   quadratically in memory size (`3n + n²/512` per word), which is why
   smart contracts are careful about memory layout.

3. **Word-aligned growth.** Even though memory is byte-addressed, it grows
   in 32-byte chunks. Writing byte 33 forces memory to grow to 64 bytes,
   not 34. Our `Memory::expand_to` mirrors this with `div_ceil(32) * 32`.

### Why MSTORE8 exists separately

`MSTORE` writes a 32-byte word. `MSTORE8` writes a single byte (the low byte
of the U256 on the stack). Why both?

Because Solidity's byte-packed data structures (`bytes`, `string`) need to
write individual bytes without clobbering neighboring data. If `MSTORE` were
your only tool, writing one byte would require read-modify-write of a full
word — three instructions instead of one, plus more gas.

`MSTORE8` is a reminder that the EVM's opcode set is tuned for Solidity's
expected patterns. When you see a seemingly-niche opcode, it usually exists
because *some* common pattern would be painfully expensive without it.

## The instruction pointer vs. the program counter

You may have noticed we call it `ip` (instruction pointer) instead of the
EVM's traditional `pc` (program counter). Here's why:

- The EVM's `pc` indexes into **raw bytecode bytes**. A `PUSH32` instruction
  advances `pc` by 33.
- Our `ip` indexes into the **decoded `Vec<Instruction>`**. A `PUSH32`
  instruction advances `ip` by 1.

These are equivalent when there are no jumps — we emit one `Instruction` per
source opcode, in order. But the moment we add `JUMP`, we have a problem:
`JUMP` pops a destination byte-offset off the stack, but our `ip` doesn't
speak byte-offsets. We'll either need to build an offset→ip map during
decoding, or give up the typed enum and dispatch directly off bytecode bytes
(the revm approach).

We'll face this properly in Stage 3. For Stage 2, with no jumps, `ip` and
`pc` are isomorphic and it doesn't matter.

## What you're implementing

The Stack and Memory modules are complete and tested. What's missing is the
`step(inst)` function in `interpreter.rs`: the switch that turns an
`Instruction` into an actual state mutation.

Every test in `interpreter.rs::tests` specifies one handler's behavior. Work
through them top-down — `add` first, then `sub` (watch the operand order),
then the rest. Running `cargo test interpreter` will show you which are left.

Once they all pass, you have a working EVM interpreter. Small, but real.
