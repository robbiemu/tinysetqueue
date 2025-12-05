# tinysetqueue

`tinysetqueue` is a stack-allocated FIFO queue with built-in membership tracking for dense integer domains. It eliminates duplicate work while keeping latency and memory usage predictable—ideal for embedded, `no_std`, and data-structure heavy workloads such as BFS, frontier expansion, and constraint propagation.

## Highlights
- Allocation-free API uses caller-provided ring-buffer storage
- Direct-mapped membership bitmap deduplicates enqueues in O(1)
- Two membership modes: `InQueue` (requeue after pop) and `Visited` (ban after first insert)
- `no_std` by default; opt into the `std` feature when desired
- Zero external dependencies and zero unsafe code

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

## Usage Notes

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

## License

Licensed under either of

- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate is licensed as described above.
