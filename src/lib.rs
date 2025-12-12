#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

/// Prelude re-exporting the most commonly used items.
pub mod prelude {
  #[cfg(feature = "pow2")]
  pub use super::TinySetQueuePow2;
  pub use super::{
    MembershipMode, ProcessingOrder, PushResult, SetBacking, TinySetQueue,
  };
}

mod private {
  pub trait Sealed {}
}

/// Behavior required from membership backings.
///
/// This trait is sealed; it can only be implemented by types provided by this
/// crate (currently `[bool]` and `[u64]`). Users opt into different behaviors
/// by passing these different slice types to [`TinySetQueue::new`].
pub trait SetBacking: private::Sealed {
  /// Number of representable entries in the membership domain.
  fn capacity(&self) -> usize;
  /// Returns `true` when the given index is present.
  fn contains(&self, index: usize) -> bool;
  /// Inserts the given index.
  fn insert(&mut self, index: usize);
  /// Removes the given index.
  fn remove(&mut self, index: usize);
  /// Clears all membership information.
  fn clear_all(&mut self);
}

impl private::Sealed for [bool] {}

impl SetBacking for [bool] {
  #[inline(always)]
  fn capacity(&self) -> usize {
    self.len()
  }

  #[inline(always)]
  fn contains(&self, index: usize) -> bool {
    self[index]
  }

  #[inline(always)]
  fn insert(&mut self, index: usize) {
    self[index] = true;
  }

  #[inline(always)]
  fn remove(&mut self, index: usize) {
    self[index] = false;
  }

  fn clear_all(&mut self) {
    self.fill(false);
  }
}

impl private::Sealed for [u64] {}

impl SetBacking for [u64] {
  #[inline(always)]
  fn capacity(&self) -> usize {
    self.len() << 6
  }

  #[inline(always)]
  fn contains(&self, index: usize) -> bool {
    let word = index >> 6;
    let bit = index & 63;
    (self[word] & (1u64 << bit)) != 0
  }

  #[inline(always)]
  fn insert(&mut self, index: usize) {
    let word = index >> 6;
    let bit = index & 63;
    self[word] |= 1u64 << bit;
  }

  #[inline(always)]
  fn remove(&mut self, index: usize) {
    let word = index >> 6;
    let bit = index & 63;
    self[word] &= !(1u64 << bit);
  }

  fn clear_all(&mut self) {
    self.fill(0);
  }
}

impl<const N: usize> private::Sealed for [bool; N] {}

impl<const N: usize> SetBacking for [bool; N] {
  #[inline(always)]
  fn capacity(&self) -> usize {
    N
  }

  #[inline(always)]
  fn contains(&self, index: usize) -> bool {
    self[index]
  }

  #[inline(always)]
  fn insert(&mut self, index: usize) {
    self[index] = true;
  }

  #[inline(always)]
  fn remove(&mut self, index: usize) {
    self[index] = false;
  }

  fn clear_all(&mut self) {
    self.fill(false);
  }
}

impl<const N: usize> private::Sealed for [u64; N] {}

impl<const N: usize> SetBacking for [u64; N] {
  #[inline(always)]
  fn capacity(&self) -> usize {
    N << 6
  }

  #[inline(always)]
  fn contains(&self, index: usize) -> bool {
    let word = index >> 6;
    let bit = index & 63;
    (self[word] & (1u64 << bit)) != 0
  }

  #[inline(always)]
  fn insert(&mut self, index: usize) {
    let word = index >> 6;
    let bit = index & 63;
    self[word] |= 1u64 << bit;
  }

  #[inline(always)]
  fn remove(&mut self, index: usize) {
    let word = index >> 6;
    let bit = index & 63;
    self[word] &= !(1u64 << bit);
  }

  fn clear_all(&mut self) {
    self.fill(0);
  }
}

/// Result of attempting to enqueue a value.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PushResult {
  /// The value was inserted into the queue.
  Inserted,
  /// The value was already present and was not enqueued again.
  AlreadyPresent,
}

/// Controls how membership is tracked when popping values.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MembershipMode {
  /// Membership is cleared upon popping, allowing the value to be enqueued again.
  InQueue,
  /// Membership persists after popping, preventing re-enqueueing.
  Visited,
}

/// Controls whether values are processed in FIFO or LIFO order.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ProcessingOrder {
  /// First-in, first-out processing (queue semantics).
  Fifo,
  /// Last-in, first-out processing (stack semantics).
  Lifo,
}

/// A fixed-capacity, allocation-free queue with direct-mapped membership tracking.
///
/// Values are converted to indices via [`Into<usize>`], so the queue works best when
/// keys are dense integers in the range `0..N`. Sparse identifiers (e.g. `{5, 1_000_000}`)
/// require a membership backing large enough to cover the full domain.
///
/// # Sizing
///
/// This queue never allocates and **never resizes** at runtime. If an index
/// exceeds the membership capacity, `push` returns an error.
pub struct TinySetQueue<'a, T, S>
where
  S: SetBacking + ?Sized,
{
  buf: &'a mut [T],
  in_queue: &'a mut S,
  mode: MembershipMode,
  order: ProcessingOrder,
  head: usize,
  tail: usize,
  len: usize,
}

impl<'a, T, S> TinySetQueue<'a, T, S>
where
  T: Copy + Into<usize>,
  S: SetBacking + ?Sized,
{
  /// Constructs a queue backed by caller-provided storage.
  ///
  /// * `buf` supplies the ring-buffer storage used for pending values.
  /// * `in_queue` is the direct-mapped membership backing (e.g. `[bool]`, `[u64]`).
  /// * `mode` determines whether membership clears on `pop`.
  /// * `order` selects FIFO or LIFO processing of queued values.
  ///
  /// `in_queue.capacity()` must exceed any index produced by `value.into()`. When the
  /// `clear_on_new` feature (enabled by default) is active, the backing is cleared to
  /// prevent stale membership flags.
  pub fn new(
    buf: &'a mut [T],
    in_queue: &'a mut S,
    mode: MembershipMode,
    order: ProcessingOrder,
  ) -> Self {
    #[cfg(feature = "clear_on_new")]
    in_queue.clear_all();
    TinySetQueue {
      buf,
      in_queue,
      mode,
      order,
      head: 0,
      tail: 0,
      len: 0,
    }
  }

  /// Clears the queue without freeing any backing storage.
  ///
  /// All membership flags are reset and the queue becomes empty.
  pub fn clear(&mut self) {
    self.in_queue.clear_all();
    self.head = 0;
    self.tail = 0;
    self.len = 0;
  }

  /// Returns the maximum number of pending items the queue can hold.
  #[inline]
  pub fn capacity(&self) -> usize {
    self.buf.len()
  }

  /// Returns the number of items currently enqueued.
  #[inline]
  pub fn len(&self) -> usize {
    self.len
  }

  /// Returns `true` when the queue is empty.
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len == 0
  }

  /// Returns `true` when the queue is at full capacity.
  #[inline]
  pub fn is_full(&self) -> bool {
    self.len == self.buf.len()
  }

  /// Pushes a value into the queue unless it is already present.
  ///
  /// # Errors
  ///
  /// Returns `Err(value)` if the queue is full or if `value.into()` exceeds the
  /// bounds of the membership backing.
  pub fn push(&mut self, value: T) -> Result<PushResult, T> {
    let idx: usize = value.into();

    if idx >= self.in_queue.capacity() {
      return Err(value);
    }

    if self.in_queue.contains(idx) {
      return Ok(PushResult::AlreadyPresent);
    }

    if self.is_full() {
      return Err(value);
    }

    self.buf[self.tail] = value;
    self.in_queue.insert(idx);

    self.tail = (self.tail + 1) % self.buf.len();
    self.len += 1;

    Ok(PushResult::Inserted)
  }

  /// Pops the next value according to the configured processing order, if any.
  ///
  /// Membership is cleared in [`MembershipMode::InQueue`] and retained in
  /// [`MembershipMode::Visited`].
  pub fn pop(&mut self) -> Option<T> {
    if self.is_empty() {
      return None;
    }

    let index = match self.order {
      ProcessingOrder::Fifo => {
        let idx = self.head;
        self.head = (self.head + 1) % self.buf.len();
        idx
      }
      ProcessingOrder::Lifo => {
        debug_assert!(self.buf.len() > 0);
        let idx = if self.tail == 0 {
          self.buf.len() - 1
        } else {
          self.tail - 1
        };
        self.tail = idx;
        idx
      }
    };

    let value = self.buf[index];
    let idx: usize = value.into();

    if matches!(self.mode, MembershipMode::InQueue) {
      self.in_queue.remove(idx);
    }

    self.len -= 1;

    Some(value)
  }
}

/// A power-of-two capacity variant that uses bit masking for wrap-around.
///
/// As with [`TinySetQueue`], membership is direct-mapped: the membership backing must be
/// large enough to cover the entire domain addressable by `T::into()`.
#[cfg(feature = "pow2")]
pub struct TinySetQueuePow2<'a, T, S>
where
  S: SetBacking + ?Sized,
{
  buf: &'a mut [T],
  in_queue: &'a mut S,
  mode: MembershipMode,
  order: ProcessingOrder,
  mask: usize,
  head: usize,
  tail: usize,
  len: usize,
}

#[cfg(feature = "pow2")]
impl<'a, T, S> TinySetQueuePow2<'a, T, S>
where
  T: Copy + Into<usize>,
  S: SetBacking + ?Sized,
{
  /// Constructs a queue backed by power-of-two-sized storage.
  ///
  /// # Panics
  ///
  /// Panics if `buf.len()` is not a power of two.
  pub fn new(
    buf: &'a mut [T],
    in_queue: &'a mut S,
    mode: MembershipMode,
    order: ProcessingOrder,
  ) -> Self {
    assert!(
      buf.len().is_power_of_two(),
      "buffer length must be a power of two"
    );
    #[cfg(feature = "clear_on_new")]
    in_queue.clear_all();
    let mask = buf.len() - 1;
    TinySetQueuePow2 {
      buf,
      in_queue,
      mode,
      order,
      mask,
      head: 0,
      tail: 0,
      len: 0,
    }
  }

  /// Clears the queue without freeing any backing storage.
  pub fn clear(&mut self) {
    self.in_queue.clear_all();
    self.head = 0;
    self.tail = 0;
    self.len = 0;
  }

  #[inline]
  pub fn capacity(&self) -> usize {
    self.buf.len()
  }

  #[inline]
  pub fn len(&self) -> usize {
    self.len
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len == 0
  }

  #[inline]
  pub fn is_full(&self) -> bool {
    self.len == self.buf.len()
  }

  pub fn push(&mut self, value: T) -> Result<PushResult, T> {
    let idx: usize = value.into();

    if idx >= self.in_queue.capacity() {
      return Err(value);
    }

    if self.in_queue.contains(idx) {
      return Ok(PushResult::AlreadyPresent);
    }

    if self.is_full() {
      return Err(value);
    }

    self.buf[self.tail] = value;
    self.in_queue.insert(idx);

    self.tail = (self.tail + 1) & self.mask;
    self.len += 1;

    Ok(PushResult::Inserted)
  }

  pub fn pop(&mut self) -> Option<T> {
    if self.is_empty() {
      return None;
    }

    let index = match self.order {
      ProcessingOrder::Fifo => {
        let idx = self.head;
        self.head = (self.head + 1) & self.mask;
        idx
      }
      ProcessingOrder::Lifo => {
        let idx = (self.tail.wrapping_sub(1)) & self.mask;
        self.tail = idx;
        idx
      }
    };

    let value = self.buf[index];
    let idx: usize = value.into();
    if matches!(self.mode, MembershipMode::InQueue) {
      self.in_queue.remove(idx);
    }

    self.len -= 1;

    Some(value)
  }
}

#[cfg(test)]
mod tests {
  use super::{MembershipMode, ProcessingOrder, PushResult, TinySetQueue};

  #[test]
  fn basic_push_pop_in_queue() {
    let mut buf = [0u8; 4];
    let mut membership = [false; 8];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert!(queue.is_empty());
    assert_eq!(queue.capacity(), 4);

    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::AlreadyPresent));
    assert_eq!(queue.len(), 1);

    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.len(), 0);

    // Membership cleared in InQueue mode -> can be inserted again.
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
  }

  #[test]
  fn visited_mode_prevents_requeue() {
    let mut buf = [0u8; 4];
    let mut membership = [false; 8];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::Visited,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(2), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.push(2), Ok(PushResult::AlreadyPresent));
  }

  #[test]
  fn lifo_order_pops_most_recent() {
    let mut buf = [0u8; 4];
    let mut membership = [false; 8];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Lifo,
    );

    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert_eq!(queue.push(2), Ok(PushResult::Inserted));
    assert_eq!(queue.push(3), Ok(PushResult::Inserted));
    assert_eq!(queue.len(), 3);

    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(1));
    assert!(queue.is_empty());
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
  }

  #[test]
  fn clear_resets_membership_and_indices() {
    let mut buf = [0u8; 2];
    let mut membership = [false; 4];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert!(queue.is_full());

    queue.clear();
    assert!(queue.is_empty());
    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
  }

  #[cfg(feature = "clear_on_new")]
  #[test]
  fn new_clears_membership_bitmap() {
    let mut buf = [0u8; 2];
    let mut membership = [true; 4];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert!(queue.is_empty());
    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
  }

  #[cfg(not(feature = "clear_on_new"))]
  #[test]
  fn new_preserves_membership_bitmap() {
    let mut buf = [0u8; 2];
    let mut membership = [true; 4];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert!(queue.is_empty());
    assert_eq!(queue.push(0), Ok(PushResult::AlreadyPresent));
  }

  #[test]
  fn push_rejects_out_of_range_index() {
    let mut buf = [0u8; 2];
    let mut membership = [false; 2];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(3), Err(3));
    assert!(queue.is_empty());
  }

  #[test]
  fn push_rejects_when_full() {
    let mut buf = [0u8; 2];
    let mut membership = [false; 4];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert!(queue.is_full());
    assert_eq!(queue.push(2), Err(2));
    assert_eq!(queue.len(), 2);
  }

  #[test]
  fn ring_buffer_wraparound_preserves_membership() {
    let mut buf = [0u8; 3];
    let mut membership = [false; 6];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert_eq!(queue.push(2), Ok(PushResult::Inserted));
    assert!(queue.is_full());

    assert_eq!(queue.pop(), Some(0));
    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(0));
    assert!(queue.is_empty());
  }

  #[test]
  fn zero_capacity_queue_behaves_consistently() {
    let mut buf: [u8; 0] = [];
    let mut membership = [false; 1];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.capacity(), 0);
    assert!(queue.is_empty());
    assert!(queue.is_full());
    assert_eq!(queue.push(0), Err(0));
    assert_eq!(queue.pop(), None);
  }

  #[test]
  fn bitset_backing_handles_high_indices() {
    let mut buf = [0u16; 4];
    let mut membership = [0u64; 2]; // capacity 128
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(63), Ok(PushResult::Inserted));
    assert_eq!(queue.push(63), Ok(PushResult::AlreadyPresent));
    assert_eq!(queue.push(64), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(0));
    assert_eq!(queue.push(0), Ok(PushResult::Inserted)); // membership cleared after pop
  }

  #[test]
  fn bitset_backing_enforces_capacity() {
    let mut buf = [0u8; 2];
    let mut membership = [0u64; 1]; // capacity 64
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(63), Ok(PushResult::Inserted));
    assert_eq!(queue.push(64), Err(64)); // out of range
  }

  #[test]
  fn bitset_visited_mode_persists_membership() {
    let mut buf = [0u8; 2];
    let mut membership = [0u64; 1];
    let mut queue = TinySetQueue::new(
      &mut buf,
      &mut membership,
      MembershipMode::Visited,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(10), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(10));
    assert_eq!(queue.push(10), Ok(PushResult::AlreadyPresent));
  }
}

#[cfg(all(test, feature = "pow2", feature = "std"))]
mod pow2_tests {
  use super::{MembershipMode, ProcessingOrder, PushResult, TinySetQueuePow2};

  #[test]
  fn rejects_non_power_of_two() {
    let mut buf = [0u8; 3];
    let mut membership = [false; 8];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      TinySetQueuePow2::new(
        &mut buf,
        &mut membership,
        MembershipMode::InQueue,
        ProcessingOrder::Fifo,
      );
    }));
    assert!(result.is_err());
  }

  #[test]
  fn push_pop_wraparound_uses_mask() {
    let mut buf = [0u8; 4];
    let mut membership = [false; 8];
    let mut queue = TinySetQueuePow2::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert_eq!(queue.push(2), Ok(PushResult::Inserted));
    assert_eq!(queue.push(3), Ok(PushResult::Inserted));
    assert!(queue.is_full());

    assert_eq!(queue.pop(), Some(0));
    assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.pop(), Some(2));
    assert_eq!(queue.pop(), Some(3));
    assert_eq!(queue.pop(), Some(0));
    assert!(queue.is_empty());
  }

  #[test]
  fn pow2_lifo_order_uses_tail() {
    let mut buf = [0u8; 4];
    let mut membership = [false; 32];
    let mut queue = TinySetQueuePow2::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Lifo,
    );

    assert_eq!(queue.push(10), Ok(PushResult::Inserted));
    assert_eq!(queue.push(11), Ok(PushResult::Inserted));
    assert_eq!(queue.push(12), Ok(PushResult::Inserted));

    assert_eq!(queue.pop(), Some(12));
    assert_eq!(queue.pop(), Some(11));
    assert_eq!(queue.pop(), Some(10));
    assert!(queue.is_empty());
    assert_eq!(queue.push(10), Ok(PushResult::Inserted));
  }

  #[test]
  fn pow2_supports_bitset_backing() {
    let mut buf = [0u8; 4];
    let mut membership = [0u64; 1];
    let mut queue = TinySetQueuePow2::new(
      &mut buf,
      &mut membership,
      MembershipMode::InQueue,
      ProcessingOrder::Fifo,
    );

    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
    assert_eq!(queue.push(17), Ok(PushResult::Inserted));
    assert_eq!(queue.pop(), Some(1));
    assert_eq!(queue.push(1), Ok(PushResult::Inserted));
  }
}
