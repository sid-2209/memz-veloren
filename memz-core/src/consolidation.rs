//! Memory Consolidation — "What I've internalised" (§8.8)
//!
//! Transforms memories from one type to another over time:
//!   - Episodic → Semantic: repeated experiences become general knowledge
//!   - Episodic → Procedural: repeated actions become skills
//!   - Multiple Episodic → Reflective: patterns noticed become insights
//!
//! Grounded in sleep-mediated memory consolidation research:
//!   - Stickgold, R. & Walker, M.P. (2013). "Sleep-Dependent Memory Consolidation."

use crate::memory::episodic::EpisodicMemory;
use crate::memory::procedural::ProceduralMemory;
use crate::memory::reflective::ReflectiveMemory;
use crate::memory::semantic::SemanticMemory;
use crate::types::{EntityId, GameTimestamp, MemoryId};

/// A consolidation task describes a pending memory transformation.
#[derive(Debug, Clone)]
pub struct ConsolidationTask {
    /// Source memory type.
    pub source_type: MemoryType,
    /// IDs of source memories to consolidate.
    pub source_ids: Vec<MemoryId>,
    /// Target memory type to produce.
    pub target_type: MemoryType,
    /// Priority (higher = process sooner).
    pub priority: f32,
    /// When this task was queued.
    pub created_at: GameTimestamp,
}

/// Types of memory (for consolidation routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Episodic memory type.
    Episodic,
    /// Semantic memory type.
    Semantic,
    /// Emotional memory type.
    Emotional,
    /// Social memory type.
    Social,
    /// Reflective memory type.
    Reflective,
    /// Procedural memory type.
    Procedural,
    /// Injected memory type.
    Injected,
}

/// Result of a consolidation operation.
#[derive(Debug)]
pub enum ConsolidationResult {
    /// A new semantic memory was distilled from episodic memories.
    NewSemantic(SemanticMemory),
    /// A new procedural memory was formed from repeated actions.
    NewProcedural(ProceduralMemory),
    /// A new reflective memory was generated from pattern recognition.
    NewReflective(ReflectiveMemory),
    /// No consolidation was possible (not enough data, too diverse, etc.).
    NoConsolidation {
        /// Why consolidation was not possible.
        reason: String,
    },
}

/// Attempt to consolidate a group of episodic memories into a semantic memory.
///
/// This is the rule-based fallback (Tier 0). The LLM path (Tier 1) produces
/// richer summaries but this works offline with zero latency.
///
/// Heuristic: If 3+ episodic memories share similar participants/location/theme,
/// they can be distilled into a general fact.
pub fn consolidate_episodic_to_semantic(
    memories: &[EpisodicMemory],
    current_time: GameTimestamp,
) -> ConsolidationResult {
    if memories.len() < 3 {
        return ConsolidationResult::NoConsolidation {
            reason: "Need at least 3 episodic memories to consolidate".to_string(),
        };
    }

    // Find common participants across memories.
    let first_participants = &memories[0].participants;
    let common_participants: Vec<_> = first_participants
        .iter()
        .filter(|p| memories.iter().all(|m| m.participants.contains(p)))
        .cloned()
        .collect();

    // Compute average emotional valence.
    let avg_valence: f32 = memories.iter().map(|m| m.emotional_valence).sum::<f32>()
        / memories.len() as f32;

    // Generate a summary fact.
    let fact = if !common_participants.is_empty() {
        let sentiment = if avg_valence > 0.3 {
            "generally positive"
        } else if avg_valence < -0.3 {
            "generally negative"
        } else {
            "mixed"
        };
        format!(
            "Based on {} experiences, interactions with entity {:?} have been {}.",
            memories.len(),
            common_participants[0],
            sentiment,
        )
    } else {
        format!(
            "After {} recent experiences, the general pattern seems to be {}.",
            memories.len(),
            if avg_valence > 0.0 { "positive" } else { "challenging" },
        )
    };

    let derived_from: Vec<MemoryId> = memories.iter().map(|m| m.id).collect();
    let confidence = (memories.len() as f32 / 10.0).clamp(0.3, 0.9); // more evidence → higher confidence

    let semantic = SemanticMemory::new(
        fact,
        confidence,
        derived_from,
        "person_knowledge",
        current_time,
    );

    ConsolidationResult::NewSemantic(semantic)
}

/// Attempt to consolidate repeated actions into a procedural memory (skill).
///
/// If an NPC has performed the same type of action N times, they start
/// developing a procedural memory for it.
pub fn consolidate_to_procedural(
    skill_name: &str,
    repetition_count: u32,
    current_time: GameTimestamp,
    learning_rate: f32,
) -> ConsolidationResult {
    if repetition_count < 3 {
        return ConsolidationResult::NoConsolidation {
            reason: "Need at least 3 repetitions to form a procedural memory".to_string(),
        };
    }

    let mut procedural = ProceduralMemory::new(skill_name, current_time, learning_rate);
    // Apply all past repetitions at once.
    for _ in 0..repetition_count {
        procedural.practice(current_time);
    }

    ConsolidationResult::NewProcedural(procedural)
}

/// Identify consolidation opportunities from a set of episodic memories.
///
/// Returns a list of consolidation tasks ordered by priority.
pub fn identify_consolidation_tasks(
    episodic: &[EpisodicMemory],
    current_time: &GameTimestamp,
) -> Vec<ConsolidationTask> {
    let mut tasks = Vec::new();

    // Group by common participants.
    let mut participant_groups: std::collections::HashMap<EntityId, Vec<MemoryId>> =
        std::collections::HashMap::new();

    for memory in episodic {
        for participant in &memory.participants {
            participant_groups
                .entry(*participant)
                .or_default()
                .push(memory.id);
        }
    }

    // Create consolidation tasks for groups with 3+ memories.
    for (_participant_id, memory_ids) in &participant_groups {
        if memory_ids.len() >= 3 {
            tasks.push(ConsolidationTask {
                source_type: MemoryType::Episodic,
                source_ids: memory_ids.clone(),
                target_type: MemoryType::Semantic,
                priority: memory_ids.len() as f32 / 10.0, // more memories → higher priority
                created_at: *current_time,
            });
        }
    }

    // Sort by priority (highest first).
    tasks.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    tasks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp, Location};

    fn make_episodic(event: &str, participants: Vec<EntityId>, valence: f32) -> EpisodicMemory {
        EpisodicMemory::new(
            event,
            participants,
            Location::default(),
            GameTimestamp::now(36_000),
            valence,
            0.5,
        )
    }

    #[test]
    fn consolidates_positive_experiences() {
        let entity = EntityId::new();
        let memories = vec![
            make_episodic("helped with task", vec![entity], 0.7),
            make_episodic("shared a meal", vec![entity], 0.5),
            make_episodic("defended from bandits", vec![entity], 0.9),
        ];

        let result = consolidate_episodic_to_semantic(
            &memories,
            GameTimestamp::now(0),
        );

        match result {
            ConsolidationResult::NewSemantic(mem) => {
                assert!(mem.fact.contains("positive"));
                assert!(mem.derived_from.len() == 3);
                assert!(mem.confidence > 0.0);
            }
            _ => panic!("Expected NewSemantic"),
        }
    }

    #[test]
    fn needs_minimum_memories() {
        let entity = EntityId::new();
        let memories = vec![
            make_episodic("one event", vec![entity], 0.5),
            make_episodic("two events", vec![entity], 0.5),
        ];

        let result = consolidate_episodic_to_semantic(
            &memories,
            GameTimestamp::now(0),
        );

        assert!(matches!(result, ConsolidationResult::NoConsolidation { .. }));
    }

    #[test]
    fn procedural_requires_repetitions() {
        let result = consolidate_to_procedural(
            "sword_fighting",
            2,
            GameTimestamp::now(0),
            1.0,
        );
        assert!(matches!(result, ConsolidationResult::NoConsolidation { .. }));

        let result = consolidate_to_procedural(
            "sword_fighting",
            10,
            GameTimestamp::now(0),
            1.0,
        );
        match result {
            ConsolidationResult::NewProcedural(mem) => {
                assert_eq!(mem.skill, "sword_fighting");
                assert_eq!(mem.repetitions, 10);
                assert!(mem.proficiency > 0.0);
            }
            _ => panic!("Expected NewProcedural"),
        }
    }

    #[test]
    fn identifies_consolidation_tasks() {
        let e1 = EntityId::new();
        let e2 = EntityId::new();
        let e3 = EntityId::new();
        let memories = vec![
            make_episodic("event1", vec![e1, e2], 0.5),
            make_episodic("event2", vec![e1], 0.3),
            make_episodic("event3", vec![e1, e3], 0.7),
            make_episodic("event4", vec![e2], 0.2),
        ];

        let tasks = identify_consolidation_tasks(
            &memories,
            &GameTimestamp::now(0),
        );

        // Entity 1 appears in 3 memories → should produce a task.
        assert!(!tasks.is_empty());
        assert!(tasks[0].source_ids.len() >= 3);
    }
}
