# **`tinysetqueue`: A Stack-Allocated, Deduplicating FIFO Queue**

`tinysetqueue` fills a critical gap in the Rust ecosystem: a **stack-allocated, allocation-free FIFO queue with built-in membership tracking** for **dense integer domains**. Traditional queues (`VecDeque`, `heapless::Deque`) guarantee FIFO behavior but cannot prevent duplicate enqueueing without external bookkeeping. Set-like structures (`BitSet`, `HashSet`) track membership but provide no ordering guarantees. Many algorithms—BFS, flood fills, constraint propagation—require **both** properties simultaneously, especially in memory-constrained environments.

This crate codifies a proven pattern: a fixed-capacity ring buffer paired with a direct-mapped membership array, providing O(1) push/pop with automatic deduplication and **zero heap allocation**.

## **Core Use Cases**

* BFS and shortest-path on microcontrollers or in `no_std` kernels
* Sparse-update simulations and cellular automata avoiding duplicate work
* Topological frontier peeling with `Visited` semantics
* Any algorithm where IDs are dense integers (0..N) and memory is non-negotiable

## **Example: Flood Fill**

Map 2D coordinates to a dense integer index and guarantee each cell processes exactly once:

```rust
// (x, y) -> usize via y * WIDTH + x
let mut queue = TinySetQueue::new(&mut buf, &mut seen, MembershipMode::InQueue);
queue.push(start_id);

while let Some(id) = queue.pop() {
    // Process cell...
    for neighbor in neighbors(id) {
        queue.push(neighbor); // No duplicate work, no allocation
    }
}
```

## **Key Features**

* **Direct-Mapped Deduplication**: Uses `Into<usize>` for O(1) membership checks. **Best for dense integer keys** (array indices, entity IDs 0..N). Sparse IDs require proportional allocation.
* **Memory vs. Speed Trade-off**: v0.1 uses `[bool]` for the membership array, prioritizing CPU speed and code simplicity over bit-packing density (8x memory vs. a bitset).
* **Dual Membership Modes**:
  * `InQueue`: Re-enqueue after popping (standard BFS)
  * `Visited`: Permanent ban after first insert (topological peeling)
* **Zero Dependencies, `no_std` Compatible**: Works on stable Rust in any environment.

## **Corrected Implementation (v0.1)**

```rust
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PushResult {
    Inserted,
    AlreadyPresent,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MembershipMode {
    InQueue,
    Visited,
}

pub struct TinySetQueue<'a, T> {
    buf: &'a mut [T],
    in_queue: &'a mut [bool],
    mode: MembershipMode,
    head: usize,
    tail: usize,
    len: usize,
}

impl<'a, T> TinySetQueue<'a, T>
where
    T: Copy + Into<usize>,
{
    /// Creates a new queue. **Clears the `in_queue` slice** to ensure safety.
    pub fn new(
        buf: &'a mut [T],
        in_queue: &'a mut [bool],
        mode: MembershipMode,
    ) -> Self {
        // CRITICAL: Prevent false positives from garbage data
        for flag in in_queue.iter_mut() {
            *flag = false;
        }

        TinySetQueue {
            buf,
            in_queue,
            mode,
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    /// Resets the queue for reuse without reallocating buffers.
    pub fn clear(&mut self) {
        for flag in self.in_queue.iter_mut() {
            *flag = false;
        }
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }

    // ... push, pop, len, is_empty ...
}
```

## **Future Roadmap (v0.2)**

To address the density trade-off without breaking API compatibility, we will introduce a `SetBacking` trait:

```rust
pub trait SetBacking {
    fn contains(&self, index: usize) -> bool;
    fn insert(&mut self, index: usize);
    fn clear(&mut self);
}

// Implement for [bool] (default) and [u32] (bitset)
```

This allows users to opt into bit-level density if memory constraints demand it—no core logic changes required.

## **Why a Dedicated Crate?**

This pattern is **repeatedly reinvented** in embedded projects, competitive programming, and systems code. A standalone crate provides:

* **Tested, reusable abstraction** instead of ad-hoc, bug-prone rewrites
* **Clear semantics** distinguishing `InQueue` vs. `Visited` modes
* **Drop-in readiness** for `no_std` environments
* **Ergonomic safety** around initialization and reuse (`clear()`)

`tinysetqueue` is not a general-purpose queue—it is a **specialized primitive** for dense integer domains where allocation is forbidden and duplicate suppression is mandatory. It codifies an algorithmic pattern that is common, useful, and currently underserved.

**Publish v0.1. Fix `new`, keep it simple, and ship.**