//! LLM Request Queue — prioritised async request processing (§12.3)
//!
//! All LLM requests in MEMZ flow through a priority queue that ensures:
//! - Dialogue requests (player-facing) get highest priority
//! - Reflection and gossip requests are processed during idle time
//! - Back-pressure prevents overwhelming local LLM instances
//! - Requests that exceed their deadline are automatically cancelled
//!
//! Priority order (highest first):
//! 1. Dialogue generation (player is waiting)
//! 2. Injection validation (player submitted backstory)
//! 3. Reflection (NPC introspection)
//! 4. Gossip generation (NPC→NPC)
//! 5. Bard composition (background task)
//! 6. Memory summarisation (batch job)

use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Priority levels for LLM requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LlmPriority {
    /// Background batch jobs (lowest).
    Background = 0,
    /// Bard composition, memory summary.
    Low = 1,
    /// Gossip generation.
    Medium = 2,
    /// Reflection.
    High = 3,
    /// Injection validation (player waiting briefly).
    Urgent = 4,
    /// Dialogue (player actively waiting).
    Critical = 5,
}

/// A queued LLM request with priority and deadline.
#[derive(Debug)]
pub struct QueuedRequest {
    /// Unique request ID.
    pub id: u64,
    /// Priority level.
    pub priority: LlmPriority,
    /// System prompt.
    pub system_prompt: String,
    /// User prompt.
    pub user_prompt: String,
    /// Optional GBNF grammar for structured output.
    pub grammar: Option<String>,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature for generation.
    pub temperature: f32,
    /// When this request was enqueued.
    pub enqueued_at: Instant,
    /// Maximum time to wait in queue before cancelling.
    pub deadline: Duration,
}

impl QueuedRequest {
    /// Check if this request has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.enqueued_at.elapsed() > self.deadline
    }

    /// Remaining time before deadline.
    #[must_use]
    pub fn time_remaining(&self) -> Duration {
        self.deadline.saturating_sub(self.enqueued_at.elapsed())
    }
}

// BinaryHeap is a max-heap, so higher priority = dequeued first.
impl PartialEq for QueuedRequest {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueuedRequest {}

impl PartialOrd for QueuedRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: priority (higher first).
        // Secondary: FIFO (older first, so smaller enqueued_at = higher priority).
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.enqueued_at.cmp(&self.enqueued_at))
    }
}

/// Thread-safe LLM request queue.
pub struct LlmQueue {
    inner: Arc<Mutex<LlmQueueInner>>,
}

struct LlmQueueInner {
    heap: BinaryHeap<QueuedRequest>,
    next_id: u64,
    max_queue_size: usize,
    total_enqueued: u64,
    total_dropped: u64,
    total_expired: u64,
}

/// Statistics about the LLM queue.
#[derive(Debug, Clone)]
pub struct QueueStats {
    /// Current queue depth.
    pub depth: usize,
    /// Total requests enqueued.
    pub total_enqueued: u64,
    /// Total requests dropped (queue full).
    pub total_dropped: u64,
    /// Total requests expired (deadline exceeded).
    pub total_expired: u64,
}

impl LlmQueue {
    /// Create a new LLM queue with a maximum size.
    #[must_use]
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LlmQueueInner {
                heap: BinaryHeap::new(),
                next_id: 0,
                max_queue_size,
                total_enqueued: 0,
                total_dropped: 0,
                total_expired: 0,
            })),
        }
    }

    /// Enqueue a new LLM request.
    ///
    /// Returns the request ID, or `None` if the queue is full.
    pub fn enqueue(
        &self,
        priority: LlmPriority,
        system_prompt: String,
        user_prompt: String,
        grammar: Option<String>,
        max_tokens: u32,
        temperature: f32,
        deadline: Duration,
    ) -> Option<u64> {
        let mut inner = self.inner.lock();

        if inner.heap.len() >= inner.max_queue_size {
            inner.total_dropped += 1;
            return None;
        }

        let id = inner.next_id;
        inner.next_id += 1;
        inner.total_enqueued += 1;

        inner.heap.push(QueuedRequest {
            id,
            priority,
            system_prompt,
            user_prompt,
            grammar,
            max_tokens,
            temperature,
            enqueued_at: Instant::now(),
            deadline,
        });

        Some(id)
    }

    /// Dequeue the highest-priority non-expired request.
    ///
    /// Automatically skips and counts expired requests.
    pub fn dequeue(&self) -> Option<QueuedRequest> {
        let mut inner = self.inner.lock();

        loop {
            let request = inner.heap.pop()?;
            if request.is_expired() {
                inner.total_expired += 1;
                continue;
            }
            return Some(request);
        }
    }

    /// Peek at the highest-priority request without removing it.
    #[must_use]
    pub fn peek_priority(&self) -> Option<LlmPriority> {
        let inner = self.inner.lock();
        inner.heap.peek().map(|r| r.priority)
    }

    /// Current queue depth.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.lock().heap.len()
    }

    /// Whether the queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.lock().heap.is_empty()
    }

    /// Get queue statistics.
    #[must_use]
    pub fn stats(&self) -> QueueStats {
        let inner = self.inner.lock();
        QueueStats {
            depth: inner.heap.len(),
            total_enqueued: inner.total_enqueued,
            total_dropped: inner.total_dropped,
            total_expired: inner.total_expired,
        }
    }

    /// Purge all expired requests from the queue.
    pub fn purge_expired(&self) -> u64 {
        let mut inner = self.inner.lock();
        let before = inner.heap.len();

        let mut valid: Vec<QueuedRequest> = Vec::new();
        while let Some(req) = inner.heap.pop() {
            if req.is_expired() {
                inner.total_expired += 1;
            } else {
                valid.push(req);
            }
        }

        for r in valid {
            inner.heap.push(r);
        }

        (before - inner.heap.len()) as u64
    }
}

impl Clone for LlmQueue {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request(queue: &LlmQueue, priority: LlmPriority) -> Option<u64> {
        queue.enqueue(
            priority,
            "system".into(),
            "user".into(),
            None,
            100,
            0.7,
            Duration::from_secs(30),
        )
    }

    #[test]
    fn priority_ordering() {
        let queue = LlmQueue::new(100);

        make_request(&queue, LlmPriority::Low);
        make_request(&queue, LlmPriority::Critical);
        make_request(&queue, LlmPriority::Medium);

        let first = queue.dequeue().expect("should have request");
        assert_eq!(first.priority, LlmPriority::Critical);

        let second = queue.dequeue().expect("should have request");
        assert_eq!(second.priority, LlmPriority::Medium);

        let third = queue.dequeue().expect("should have request");
        assert_eq!(third.priority, LlmPriority::Low);
    }

    #[test]
    fn queue_full_drops_request() {
        let queue = LlmQueue::new(2);

        assert!(make_request(&queue, LlmPriority::Low).is_some());
        assert!(make_request(&queue, LlmPriority::Low).is_some());
        assert!(make_request(&queue, LlmPriority::Critical).is_none());

        let stats = queue.stats();
        assert_eq!(stats.total_dropped, 1);
    }

    #[test]
    fn expired_requests_skipped() {
        let queue = LlmQueue::new(100);

        // Enqueue with 0-duration deadline (instantly expired)
        queue.enqueue(
            LlmPriority::Critical,
            "system".into(),
            "user".into(),
            None,
            100,
            0.7,
            Duration::from_millis(0),
        );

        // This should skip the expired request
        std::thread::sleep(Duration::from_millis(1));
        assert!(queue.dequeue().is_none());

        let stats = queue.stats();
        assert_eq!(stats.total_expired, 1);
    }

    #[test]
    fn stats_tracking() {
        let queue = LlmQueue::new(100);

        make_request(&queue, LlmPriority::Low);
        make_request(&queue, LlmPriority::High);

        let stats = queue.stats();
        assert_eq!(stats.depth, 2);
        assert_eq!(stats.total_enqueued, 2);

        queue.dequeue();
        let stats = queue.stats();
        assert_eq!(stats.depth, 1);
    }

    #[test]
    fn clone_shares_state() {
        let queue1 = LlmQueue::new(100);
        let queue2 = queue1.clone();

        make_request(&queue1, LlmPriority::High);
        assert_eq!(queue2.len(), 1);
    }

    #[test]
    fn fifo_within_same_priority() {
        let queue = LlmQueue::new(100);

        let id1 = make_request(&queue, LlmPriority::Medium).expect("enqueue");
        let id2 = make_request(&queue, LlmPriority::Medium).expect("enqueue");

        let first = queue.dequeue().expect("dequeue");
        assert_eq!(first.id, id1, "FIFO: older request should come first");

        let second = queue.dequeue().expect("dequeue");
        assert_eq!(second.id, id2);
    }
}
