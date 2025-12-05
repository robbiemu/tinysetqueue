#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

/// Prelude re-exporting the most commonly used items.
pub mod prelude {
    #[cfg(feature = "pow2")]
    pub use super::TinySetQueuePow2;
    pub use super::{MembershipMode, PushResult, TinySetQueue};
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

/// A fixed-capacity, allocation-free FIFO queue with direct-mapped membership tracking.
///
/// Values are converted to indices via [`Into<usize>`], so the queue works best when
/// keys are dense integers in the range `0..N`. Sparse identifiers (e.g. `{5, 1_000_000}`)
/// require an `in_queue` slice large enough to cover the full domain.
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
    /// Constructs a queue backed by caller-provided storage.
    ///
    /// * `buf` supplies the ring-buffer storage used for FIFO ordering.
    /// * `in_queue` is the direct-mapped membership bitmap.
    /// * `mode` determines whether membership clears on `pop`.
    ///
    /// `in_queue.len()` must be larger than any index produced by `value.into()`.
    /// When the `clear_on_new` feature (enabled by default) is active, the membership
    /// bitmap is cleared on construction to prevent stale flags.
    pub fn new(buf: &'a mut [T], in_queue: &'a mut [bool], mode: MembershipMode) -> Self {
        #[cfg(feature = "clear_on_new")]
        in_queue.fill(false);
        TinySetQueue {
            buf,
            in_queue,
            mode,
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    /// Clears the queue without freeing any backing storage.
    ///
    /// All membership flags are reset and the queue becomes empty.
    pub fn clear(&mut self) {
        self.in_queue.fill(false);
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
    /// bounds of the membership bitmap.
    pub fn push(&mut self, value: T) -> Result<PushResult, T> {
        let idx: usize = value.into();

        if idx >= self.in_queue.len() {
            return Err(value);
        }

        if self.in_queue[idx] {
            return Ok(PushResult::AlreadyPresent);
        }

        if self.is_full() {
            return Err(value);
        }

        self.buf[self.tail] = value;
        self.in_queue[idx] = true;

        self.tail = (self.tail + 1) % self.buf.len();
        self.len += 1;

        Ok(PushResult::Inserted)
    }

    /// Pops the oldest value from the queue, if any.
    ///
    /// Membership is cleared in [`MembershipMode::InQueue`] and retained in
    /// [`MembershipMode::Visited`].
    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.buf[self.head];
        let idx: usize = value.into();

        if matches!(self.mode, MembershipMode::InQueue) {
            self.in_queue[idx] = false;
        }

        self.head = (self.head + 1) % self.buf.len();
        self.len -= 1;

        Some(value)
    }
}

/// A power-of-two capacity variant that uses bit masking for wrap-around.
///
/// As with [`TinySetQueue`], membership is direct-mapped: the membership bitmap must be
/// large enough to cover the entire domain addressable by `T::into()`.
#[cfg(feature = "pow2")]
pub struct TinySetQueuePow2<'a, T> {
    buf: &'a mut [T],
    in_queue: &'a mut [bool],
    mode: MembershipMode,
    mask: usize,
    head: usize,
    tail: usize,
    len: usize,
}

#[cfg(feature = "pow2")]
impl<'a, T> TinySetQueuePow2<'a, T>
where
    T: Copy + Into<usize>,
{
    /// Constructs a queue backed by power-of-two-sized storage.
    ///
    /// # Panics
    ///
    /// Panics if `buf.len()` is not a power of two.
    pub fn new(buf: &'a mut [T], in_queue: &'a mut [bool], mode: MembershipMode) -> Self {
        assert!(
            buf.len().is_power_of_two(),
            "buffer length must be a power of two"
        );
        #[cfg(feature = "clear_on_new")]
        in_queue.fill(false);
        let mask = buf.len() - 1;
        TinySetQueuePow2 {
            buf,
            in_queue,
            mode,
            mask,
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    /// Clears the queue without freeing any backing storage.
    pub fn clear(&mut self) {
        self.in_queue.fill(false);
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

        if idx >= self.in_queue.len() {
            return Err(value);
        }

        if self.in_queue[idx] {
            return Ok(PushResult::AlreadyPresent);
        }

        if self.is_full() {
            return Err(value);
        }

        self.buf[self.tail] = value;
        self.in_queue[idx] = true;

        self.tail = (self.tail + 1) & self.mask;
        self.len += 1;

        Ok(PushResult::Inserted)
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.buf[self.head];
        let idx: usize = value.into();
        if matches!(self.mode, MembershipMode::InQueue) {
            self.in_queue[idx] = false;
        }

        self.head = (self.head + 1) & self.mask;
        self.len -= 1;

        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::{MembershipMode, PushResult, TinySetQueue};

    #[test]
    fn basic_push_pop_in_queue() {
        let mut buf = [0u8; 4];
        let mut membership = [false; 8];
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

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
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::Visited);

        assert_eq!(queue.push(2), Ok(PushResult::Inserted));
        assert_eq!(queue.pop(), Some(2));
        assert_eq!(queue.push(2), Ok(PushResult::AlreadyPresent));
    }

    #[test]
    fn clear_resets_membership_and_indices() {
        let mut buf = [0u8; 2];
        let mut membership = [false; 4];
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

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
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

        assert!(queue.is_empty());
        assert_eq!(queue.push(0), Ok(PushResult::Inserted));
    }

    #[cfg(not(feature = "clear_on_new"))]
    #[test]
    fn new_preserves_membership_bitmap() {
        let mut buf = [0u8; 2];
        let mut membership = [true; 4];
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

        assert!(queue.is_empty());
        assert_eq!(queue.push(0), Ok(PushResult::AlreadyPresent));
    }

    #[test]
    fn push_rejects_out_of_range_index() {
        let mut buf = [0u8; 2];
        let mut membership = [false; 2];
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

        assert_eq!(queue.push(3), Err(3));
        assert!(queue.is_empty());
    }

    #[test]
    fn push_rejects_when_full() {
        let mut buf = [0u8; 2];
        let mut membership = [false; 4];
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

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
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

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
        let mut queue = TinySetQueue::new(&mut buf, &mut membership, MembershipMode::InQueue);

        assert_eq!(queue.capacity(), 0);
        assert!(queue.is_empty());
        assert!(queue.is_full());
        assert_eq!(queue.push(0), Err(0));
        assert_eq!(queue.pop(), None);
    }
}

#[cfg(all(test, feature = "pow2"))]
mod pow2_tests {
    use super::{MembershipMode, PushResult, TinySetQueuePow2};

    #[test]
    fn rejects_non_power_of_two() {
        let mut buf = [0u8; 3];
        let mut membership = [false; 8];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            TinySetQueuePow2::new(&mut buf, &mut membership, MembershipMode::InQueue);
        }));
        assert!(result.is_err());
    }

    #[test]
    fn push_pop_wraparound_uses_mask() {
        let mut buf = [0u8; 4];
        let mut membership = [false; 8];
        let mut queue = TinySetQueuePow2::new(&mut buf, &mut membership, MembershipMode::InQueue);

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
}
