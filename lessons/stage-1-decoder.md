# Stage 1 — Bytecode, Opcodes, and a Decoder

## What "EVM bytecode" actually is

A deployed Ethereum contract is, physically, a flat byte array. Every byte is
either:

1. **An opcode** (e.g. `0x01` = ADD), or
2. **Immediate data** that belongs to a preceding PUSH opcode.

There is no framing, no length prefix, no separators. The interpreter walks the
array left-to-right with a single cursor called the **program counter (pc)**,
and at each step it reads one byte, decides what kind of instruction it is,
and advances `pc` by either 1 (most opcodes) or `1 + N` (PUSH1..PUSH32).

Because of this, you *cannot* statically index into bytecode at arbitrary
positions — you have to linearly scan it from the start. This is why tooling
like `JUMPDEST` analysis (Stage 3) exists: it pre-walks the bytecode once to
find all the legal jump targets so the interpreter doesn't have to re-scan on
every jump.

## Why PUSH is 32 different opcodes

The EVM word is **256 bits** (`U256`). Any value on the stack, in memory, or
in storage is conceptually a 256-bit integer. So when a contract wants to put
a constant onto the stack, how many bytes should the encoding read?

- If you always read 32 bytes, a contract that only needs `PUSH 1` wastes
  31 bytes. Deploy bytecode is expensive (gas is paid per byte), so this
  matters a lot.
- If you always read 1 byte, you can't express most constants (addresses,
  hashes, large amounts) without extra machinery.

The EVM's answer: reserve **32 opcodes** (`0x60..=0x7F`) for PUSH, where the
low nibble tells you how many immediate bytes to read. `PUSH1 0x05` is two
bytes total; `PUSH32 0x...` is 33 bytes total. The contract author picks the
tightest fit at compile time.

> **Aside:** EIP-3855 (Shanghai) added `PUSH0` at opcode `0x5F` — a PUSH with
> zero immediate bytes that always pushes the value 0. Before that, Solidity
> had to emit `PUSH1 0x00` (two bytes) for the common case of pushing zero.

## Why we use an `Instruction` enum

Revm, for performance reasons, does **not** do what we're doing. It keeps the
raw byte array and dispatches directly off the opcode byte in a giant match
in a hot inner loop, because allocating a `Vec<Instruction>` upfront would
cost cache locality and memory. What revm *does* do is a pre-pass that builds
a bitmap of which positions are valid jump destinations.

For a learning interpreter, a typed `Instruction` enum is much nicer: the
compiler enforces exhaustive matching, PUSH's immediate value is right there
as a `U256` instead of something you reconstruct from bytes on every execution
step, and unit tests read naturally.

When we move on to gas metering and jumps, we'll need to re-think this: gas
costs are per-opcode and jumps target byte offsets in the *original* bytecode,
not indices into our `Vec<Instruction>`. That's an interesting tension to
hold onto — it previews why revm's design is the way it is.

## The DUP/SWAP families

Like PUSH, DUP and SWAP are each 16 opcodes:

- `DUP1` (0x80) through `DUP16` (0x8F): copy the Nth item from the top of the
  stack and push it.
- `SWAP1` (0x90) through `SWAP16` (0x9F): exchange the top of stack with the
  Nth item below it.

In our decoder, we collapse each family into a single enum variant with a
small `u8` field: `Instruction::Dup(n)`. The opcode arithmetic `op - 0x7f`
recovers the 1-indexed position.

This is a pattern you'll see all over revm: a range of opcodes that differ
only in a numeric parameter gets decoded once, and the interpreter handles
them generically.

## The `ruint` crate and `U256`

`ruint` provides `Uint<BITS, LIMBS>`, a fixed-size big-unsigned-integer type
that stores its value as an array of `u64` limbs. `U256` is the alias for
`Uint<256, 4>`.

Two things worth understanding:

1. `U256` is `Copy`. It's a `[u64; 4]` under the hood, so passing it by value
   is essentially free — 32 bytes on the stack. You don't need references.
2. Arithmetic is checked by default in debug and wrapping in release for the
   `Wrapping*` methods. The EVM semantically uses **modular 2^256 arithmetic**
   for ADD/SUB/MUL, so we'll end up using `.wrapping_add`, `.wrapping_sub`,
   `.wrapping_mul` when we implement those in the interpreter.

Revm uses this exact crate, for the same reasons: no heap allocation, full
256-bit width, trait impls for all the arithmetic the EVM needs.

## Where we are

After this lesson, your repo has:

- A typed `Instruction` enum capturing the opcodes from Stages 1 and 2.
- A `decode(&[u8]) -> Vec<Instruction>` function, minus PUSH handling (that's
  your TODO).
- A `decode_hex(&str)` helper so tests can be written against human-readable
  bytecode strings.
- Tests that specify the *behavior* of the decoder — they will fail until the
  PUSH branch is filled in.

Next lesson (Stage 2): the interpreter itself — Stack, Memory, and an
execution loop that consumes our `Vec<Instruction>`.
