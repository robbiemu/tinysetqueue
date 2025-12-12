# tinysetqueue

`tinysetqueue` is a stack-allocated FIFO queue with built-in membership tracking for dense integer domains. It eliminates duplicate work while keeping latency and memory usage predictable—ideal for embedded, `no_std`, and data-structure heavy workloads such as BFS, frontier expansion, and constraint propagation.

## Highlights
- Allocation-free API uses caller-provided ring-buffer storage
- Direct-mapped membership bitmap deduplicates enqueues in O(1)
- Two membership modes: `InQueue` (requeue after pop) and `Visited` (ban after first insert)
- `no_std` by default; opt into the `std` feature when desired
- Works with `[bool]` backings for speed or `[u64]` bitsets for dense domains
- Zero external dependencies and zero unsafe code
- Configurable FIFO (default) or LIFO processing order

## Quick Start

```rust
use tinysetqueue::{MembershipMode, PushResult, TinySetQueue};

const CAPACITY: usize = 16;
const DOMAIN: usize = 64;

let mut buf = [0u16; CAPACITY];
let mut membership = [false; DOMAIN];
let mut queue =
  TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

assert_eq!(queue.push(4), Ok(PushResult::Inserted));
assert_eq!(queue.push(4), Ok(PushResult::AlreadyPresent));
assert_eq!(queue.pop(), Some(4));
assert_eq!(queue.push(4), Ok(PushResult::Inserted)); // membership cleared by pop
```

## Choosing a Backing

`TinySetQueue` accepts any membership storage that implements its sealed `SetBacking` trait. Supply the backing that best matches your constraints:

```rust
use tinysetqueue::{MembershipMode, TinySetQueue};

let mut buf = [0u16; 4];

// Fast path: 1 byte per element.
let mut bitmap = [false; 32];
let mut queue = TinySetQueue::new(&mut buf, &mut bitmap, MembershipMode::InQueue);

// Memory-dense path: 1 bit per element (requires domain <= 64 * backing.len()).
let mut bitset = [0u64; 1];
let mut dense_queue = TinySetQueue::new(&mut buf, &mut bitset, MembershipMode::InQueue);
```

Both queues share the same API; the compiler infers the correct backing behavior from the slice you pass.

## Usage Notes

- This is a **direct-mapped** queue: keys must map densely into `0..DOMAIN`. If you push `id.into() == 1_000_000`, your `in_queue` slice must be at least that long. For sparse identifiers, consider remapping or using a different data structure such as `HashSet`.
- By default `TinySetQueue::new` clears the membership bitmap for you (feature `clear_on_new`). Disable it if you need to preserve pre-seeded membership data.
- `MembershipMode::Visited` keeps membership markers set after popping. This makes the queue behave like a hybrid queue/set that only schedules each element once.
- Reuse the queue by calling `clear` to reset membership and indices without reallocating.
- By default the crate compiles in `no_std` mode. Enable the `std` feature to integrate with standard-library environments without needing `#![no_std]` in your binary.

## When to Reach for tinysetqueue

- BFS, Dijkstra-lite, IDA*, or flood-fill algorithms on microcontrollers
- Cellular automata or constraint propagation where duplicate work must be suppressed
- Graph traversals keyed by dense integer handles (entity/component IDs, array offsets)
- Simulation step scheduling where memory predictability matters as much as time

## Feature Flags

- `std` *(default)* — Pulls in the standard library so the crate can be used without a `#![no_std]` consumer.
- `clear_on_new` *(default)* — Automatically zeroes the membership bitmap inside `TinySetQueue::new`. Disable to keep caller-supplied membership state.
- `pow2` — Enables the bit-masking `TinySetQueuePow2` variant for power-of-two capacities.

## Power-of-Two Variant

For workloads that need the lowest possible overhead on fixed-length buffers, enable the `pow2` feature and use `TinySetQueuePow2`. This variant requires the queue capacity to be a power of two and replaces the `%` arithmetic with very fast bit masking. The feature gate keeps the default build lean for users who do not need the specialized path; flip it on when you opt into the stricter buffer requirement and the extra code is worth the saved cycles. Activate it with `--features pow2` when building or testing.

```rust
# #[cfg(feature = "pow2")]
# {
use tinysetqueue::{MembershipMode, TinySetQueuePow2};

let mut buf = [0u8; 8]; // power-of-two length
let mut membership = [false; 16];
let mut queue =
  TinySetQueuePow2::new(&mut buf, &mut membership, MembershipMode::InQueue);
# }
```

If the buffer length is not a power of two, the constructor panics.

## License

Licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate is licensed as described above.

When adding changes, please run tests in both configurations to cover the `clear_on_new` feature toggle:

```bash
cargo test
cargo test --no-default-features --features std
cargo test --features pow2
cargo test --no-default-features --features "std pow2"
```
