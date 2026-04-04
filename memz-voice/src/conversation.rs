//! Conversation history management for multi-turn NPC dialogue.
//!
//! Maintains a sliding window of recent exchanges so the LLM
//! has context of what was discussed. Without this, the NPC
//! forgets what was said 10 seconds ago.

/// A single exchange in a conversation (player says X, NPC says Y).
#[derive(Debug, Clone)]
pub struct Exchange {
    /// What the player said (transcribed text).
    pub player_text: String,
    /// What the NPC responded with.
    pub npc_text: String,
}

/// Sliding-window conversation history for a single NPC.
///
/// Keeps the last N exchanges in memory so the LLM can reference
/// prior context. Older exchanges are evicted automatically.
#[derive(Debug, Clone)]
pub struct ConversationHistory {
    /// The NPC this history belongs to (by UID or identifier).
    pub npc_id: u64,
    /// Recent exchanges, newest last.
    exchanges: Vec<Exchange>,
    /// Maximum number of exchanges to keep.
    max_exchanges: usize,
}

impl ConversationHistory {
    /// Create a new empty conversation history for an NPC.
    ///
    /// `max_exchanges`: How many recent exchanges to keep (default: 6).
    pub fn new(npc_id: u64, max_exchanges: usize) -> Self {
        Self {
            npc_id,
            exchanges: Vec::with_capacity(max_exchanges),
            max_exchanges,
        }
    }

    /// Create with default settings (6 exchanges).
    pub fn default_for(npc_id: u64) -> Self {
        Self::new(npc_id, 6)
    }

    /// Add a new exchange to the history.
    ///
    /// Evicts the oldest exchange if at capacity.
    pub fn add_exchange(&mut self, player_text: String, npc_text: String) {
        if self.exchanges.len() >= self.max_exchanges {
            self.exchanges.remove(0);
        }
        self.exchanges.push(Exchange {
            player_text,
            npc_text,
        });
    }

    /// Format the conversation history as LLM chat messages.
    ///
    /// Returns alternating user/assistant message pairs suitable
    /// for appending to an LLM prompt.
    pub fn to_prompt_messages(&self) -> Vec<(String, String)> {
        self.exchanges
            .iter()
            .map(|ex| (ex.player_text.clone(), ex.npc_text.clone()))
            .collect()
    }

    /// Format the history as a single context string for injection into prompts.
    pub fn to_context_string(&self) -> String {
        if self.exchanges.is_empty() {
            return String::new();
        }

        let mut ctx = String::from("Previous conversation:\n");
        for ex in &self.exchanges {
            ctx.push_str(&format!("Player: {}\n", ex.player_text));
            ctx.push_str(&format!("NPC: {}\n", ex.npc_text));
        }
        ctx
    }

    /// Number of exchanges currently stored.
    pub fn len(&self) -> usize {
        self.exchanges.len()
    }

    /// Check if history is empty.
    pub fn is_empty(&self) -> bool {
        self.exchanges.is_empty()
    }

    /// Clear all history (e.g., when player walks away).
    pub fn clear(&mut self) {
        self.exchanges.clear();
    }

    /// Get the last NPC response, if any.
    pub fn last_npc_response(&self) -> Option<&str> {
        self.exchanges.last().map(|ex| ex.npc_text.as_str())
    }
}

/// Registry managing conversation histories for all NPCs the player has talked to.
///
/// Automatically creates new histories on first interaction and evicts
/// stale conversations after a configurable limit.
#[derive(Debug)]
pub struct ConversationRegistry {
    histories: std::collections::HashMap<u64, ConversationHistory>,
    /// Maximum number of concurrent NPC conversation histories to keep.
    max_active: usize,
}

impl ConversationRegistry {
    /// Create a new registry.
    ///
    /// `max_active`: Maximum concurrent NPC conversations to maintain (default: 20).
    pub fn new(max_active: usize) -> Self {
        Self {
            histories: std::collections::HashMap::with_capacity(max_active),
            max_active,
        }
    }

    /// Get or create a conversation history for an NPC.
    pub fn get_or_create(&mut self, npc_id: u64) -> &mut ConversationHistory {
        // Evict oldest if at capacity
        if !self.histories.contains_key(&npc_id) && self.histories.len() >= self.max_active {
            // Remove the history with fewest exchanges (least important)
            if let Some((&evict_id, _)) = self
                .histories
                .iter()
                .min_by_key(|(_, h)| h.len())
            {
                self.histories.remove(&evict_id);
            }
        }

        self.histories
            .entry(npc_id)
            .or_insert_with(|| ConversationHistory::default_for(npc_id))
    }

    /// Get an existing conversation history (read-only).
    pub fn get(&self, npc_id: u64) -> Option<&ConversationHistory> {
        self.histories.get(&npc_id)
    }

    /// Clear a specific NPC's conversation history.
    pub fn clear_npc(&mut self, npc_id: u64) {
        self.histories.remove(&npc_id);
    }

    /// Clear all conversation histories.
    pub fn clear_all(&mut self) {
        self.histories.clear();
    }
}

impl Default for ConversationRegistry {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_retrieve() {
        let mut history = ConversationHistory::new(42, 3);
        history.add_exchange("Hello!".into(), "Greetings, traveler.".into());
        history.add_exchange("What do you sell?".into(), "Fine swords.".into());

        assert_eq!(history.len(), 2);
        assert_eq!(history.last_npc_response(), Some("Fine swords."));
    }

    #[test]
    fn test_eviction() {
        let mut history = ConversationHistory::new(42, 2);
        history.add_exchange("First".into(), "Reply 1".into());
        history.add_exchange("Second".into(), "Reply 2".into());
        history.add_exchange("Third".into(), "Reply 3".into());

        assert_eq!(history.len(), 2);
        // First exchange should be evicted
        let ctx = history.to_context_string();
        assert!(!ctx.contains("First"));
        assert!(ctx.contains("Second"));
        assert!(ctx.contains("Third"));
    }

    #[test]
    fn test_context_string() {
        let mut history = ConversationHistory::default_for(1);
        history.add_exchange("Hi".into(), "Hello!".into());

        let ctx = history.to_context_string();
        assert!(ctx.contains("Player: Hi"));
        assert!(ctx.contains("NPC: Hello!"));
    }

    #[test]
    fn test_registry() {
        let mut registry = ConversationRegistry::new(2);

        registry.get_or_create(1).add_exchange("A".into(), "B".into());
        registry.get_or_create(2).add_exchange("C".into(), "D".into());
        registry.get_or_create(3).add_exchange("E".into(), "F".into());

        // Registry should have max 2 entries
        assert!(registry.histories.len() <= 2);
        // NPC 3 should definitely be present (just created)
        assert!(registry.get(3).is_some());
    }
}
