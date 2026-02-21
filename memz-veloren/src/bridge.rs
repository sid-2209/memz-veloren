//! Bridge module — maps between Veloren and MEMZ type systems (§12.2).
//!
//! Veloren uses its own type system for NPCs:
//! - `common::rtsim::Personality` (Big Five: OCEAN, u8 0–255)
//! - `rtsim::data::sentiment::Sentiments` (i8 positivity per target)
//! - `common::rtsim::Actor` / `NpcId` / `CharacterId` (entity identity)
//!
//! MEMZ uses:
//! - `PersonalityTraits` (credulity, openness, `gossip_tendency`, `emotional_volatility`, bravery; f32 0–1)
//! - `PADState` (Pleasure-Arousal-Dominance; f32 -1..1)
//! - `EntityId` (UUID)
//!
//! This module provides bidirectional mappings so MEMZ can read Veloren state
//! and Veloren can act on MEMZ outputs without either knowing the other's internals.

use memz_core::types::{EntityId, GameTimestamp, Location, PADState, PersonalityTraits};

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Personality Mapping
// ---------------------------------------------------------------------------

/// Map Veloren's OCEAN personality (u8 0–255) to MEMZ `PersonalityTraits` (f32 0–1).
///
/// The mapping is:
/// - `openness` → `openness` (direct)
/// - `agreeableness` → `credulity` (trusting people = believing gossip)
/// - `extraversion` → `gossip_tendency` (extroverts share more)
/// - `neuroticism` → `emotional_volatility` (neurotic = more reactive)
/// - Derived: `bravery` = (1 - neuroticism) * 0.5 + extraversion * 0.5 (brave = calm + outgoing)
///
/// This is a heuristic mapping — the exact coefficients are tunable.
#[must_use]
pub fn veloren_personality_to_memz(
    openness: u8,
    conscientiousness: u8,
    extraversion: u8,
    agreeableness: u8,
    neuroticism: u8,
) -> PersonalityTraits {
    let norm = |v: u8| f32::from(v) / 255.0;

    let o = norm(openness);
    let _c = norm(conscientiousness);
    let e = norm(extraversion);
    let a = norm(agreeableness);
    let n = norm(neuroticism);

    PersonalityTraits {
        openness: o,
        credulity: a * 0.7 + o * 0.3, // agreeable + open → credulous
        gossip_tendency: e * 0.6 + a * 0.2 + o * 0.2, // extroverted + sociable → gossips
        emotional_volatility: n * 0.8 + (1.0 - _c) * 0.2, // neurotic + unconscientious → volatile
        bravery: (1.0 - n) * 0.5 + e * 0.3 + _c * 0.2, // calm + outgoing + disciplined → brave
    }
}

/// Map MEMZ `PersonalityTraits` back to Veloren's OCEAN (u8 0–255).
///
/// This is the reverse of `veloren_personality_to_memz`, but imprecise
/// because the mapping is many-to-one. We produce a "best guess" OCEAN.
#[must_use]
pub fn memz_personality_to_veloren(traits: &PersonalityTraits) -> [u8; 5] {
    let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;

    let openness = to_u8(traits.openness);
    let agreeableness = to_u8(traits.credulity * 0.8 + traits.gossip_tendency * 0.2);
    let extraversion = to_u8(traits.gossip_tendency * 0.7 + traits.bravery * 0.3);
    let neuroticism = to_u8(traits.emotional_volatility);
    let conscientiousness = to_u8(1.0 - traits.emotional_volatility * 0.3 + traits.bravery * 0.2);

    [openness, conscientiousness, extraversion, agreeableness, neuroticism]
}

// ---------------------------------------------------------------------------
// Sentiment Mapping
// ---------------------------------------------------------------------------

/// Veloren sentiment thresholds mapped to descriptive strings for MEMZ.
///
/// Veloren's Sentiment is a single i8 (`positivity` in -126..126, exposed as f32 -1..1).
/// MEMZ uses richer emotional models, but we bridge via a textual description
/// that the behavior/dialogue system can consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentimentLevel {
    /// Extremely positive (≥ 0.8) — hero-worship.
    Hero,
    /// Very positive (≥ 0.6) — deep friendship.
    Friend,
    /// Positive (≥ 0.3) — ally.
    Ally,
    /// Mildly positive (≥ 0.1) — warm.
    Positive,
    /// Neutral (-0.1 .. 0.1).
    Neutral,
    /// Mildly negative (≤ -0.1) — cool.
    Negative,
    /// Negative (≤ -0.3) — rival.
    Rival,
    /// Very negative (≤ -0.6) — enemy.
    Enemy,
    /// Extremely negative (≤ -0.8) — villain.
    Villain,
}

impl SentimentLevel {
    /// Classify a Veloren sentiment f32 value into a `SentimentLevel`.
    #[must_use]
    pub fn from_value(value: f32) -> Self {
        if value >= 0.8 {
            Self::Hero
        } else if value >= 0.6 {
            Self::Friend
        } else if value >= 0.3 {
            Self::Ally
        } else if value >= 0.1 {
            Self::Positive
        } else if value > -0.1 {
            Self::Neutral
        } else if value > -0.3 {
            Self::Negative
        } else if value > -0.6 {
            Self::Rival
        } else if value > -0.8 {
            Self::Enemy
        } else {
            Self::Villain
        }
    }

    /// Convert to a human-readable description for prompt context.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Hero => "deeply admired hero",
            Self::Friend => "close friend",
            Self::Ally => "trusted ally",
            Self::Positive => "somewhat liked",
            Self::Neutral => "neither liked nor disliked",
            Self::Negative => "somewhat disliked",
            Self::Rival => "distrusted rival",
            Self::Enemy => "despised enemy",
            Self::Villain => "hated villain",
        }
    }

    /// Map to a PAD emotional state modifier when encountering this target.
    #[must_use]
    pub fn to_pad_modifier(self) -> PADState {
        match self {
            Self::Hero => PADState::new(0.8, 0.5, -0.2),
            Self::Friend => PADState::new(0.6, 0.3, 0.0),
            Self::Ally => PADState::new(0.3, 0.1, 0.1),
            Self::Positive => PADState::new(0.1, 0.0, 0.0),
            Self::Neutral => PADState::NEUTRAL,
            Self::Negative => PADState::new(-0.1, 0.1, 0.0),
            Self::Rival => PADState::new(-0.3, 0.3, -0.1),
            Self::Enemy => PADState::new(-0.6, 0.5, -0.2),
            Self::Villain => PADState::new(-0.8, 0.7, -0.4),
        }
    }

    /// Map to an emotional valence for memory creation.
    #[must_use]
    pub fn to_valence(self) -> f32 {
        match self {
            Self::Hero => 0.9,
            Self::Friend => 0.7,
            Self::Ally => 0.4,
            Self::Positive => 0.2,
            Self::Neutral => 0.0,
            Self::Negative => -0.2,
            Self::Rival => -0.4,
            Self::Enemy => -0.7,
            Self::Villain => -0.9,
        }
    }
}

// ---------------------------------------------------------------------------
// Entity ID Mapping
// ---------------------------------------------------------------------------

/// Bidirectional map between Veloren NPC/Character IDs and MEMZ `EntityId`s.
///
/// Veloren uses `NpcId` (slotmap index) and `CharacterId(i64)` while MEMZ
/// uses `EntityId(Uuid)`. This registry maintains stable mappings across
/// save/load cycles.
#[derive(Debug, Clone, Default)]
pub struct EntityRegistry {
    /// Veloren NPC numeric seed → MEMZ `EntityId`.
    npc_to_memz: HashMap<u64, EntityId>,
    /// MEMZ `EntityId` → Veloren NPC numeric seed.
    memz_to_npc: HashMap<EntityId, u64>,
    /// Character (player) ID → MEMZ `EntityId`.
    character_to_memz: HashMap<i64, EntityId>,
    /// MEMZ `EntityId` → Character (player) ID.
    memz_to_character: HashMap<EntityId, i64>,
}

impl EntityRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Get or create a MEMZ `EntityId` for a Veloren NPC (identified by UID).
    pub fn npc_entity(&mut self, npc_uid: u64) -> EntityId {
        *self
            .npc_to_memz
            .entry(npc_uid)
            .or_insert_with(|| {
                let id = EntityId::new();
                self.memz_to_npc.insert(id, npc_uid);
                id
            })
    }

    /// Get or create a MEMZ `EntityId` for a player character.
    pub fn character_entity(&mut self, character_id: i64) -> EntityId {
        *self
            .character_to_memz
            .entry(character_id)
            .or_insert_with(|| {
                let id = EntityId::new();
                self.memz_to_character.insert(id, character_id);
                id
            })
    }

    /// Look up a Veloren NPC UID from a MEMZ `EntityId`.
    #[must_use]
    pub fn lookup_npc(&self, entity: &EntityId) -> Option<u64> {
        self.memz_to_npc.get(entity).copied()
    }

    /// Look up a character ID from a MEMZ `EntityId`.
    #[must_use]
    pub fn lookup_character(&self, entity: &EntityId) -> Option<i64> {
        self.memz_to_character.get(entity).copied()
    }

    /// Total registered entities.
    #[must_use]
    pub fn count(&self) -> usize {
        self.npc_to_memz.len() + self.character_to_memz.len()
    }
}

// ---------------------------------------------------------------------------
// Location Mapping
// ---------------------------------------------------------------------------

/// Convert Veloren `Vec3<f32>` world position to MEMZ `Location`.
#[must_use]
pub fn veloren_pos_to_location(x: f32, y: f32, z: f32) -> Location {
    Location { x, y, z }
}

/// Convert MEMZ `Location` to a Veloren-style position tuple.
#[must_use]
pub fn location_to_veloren_pos(loc: &Location) -> (f32, f32, f32) {
    (loc.x, loc.y, loc.z)
}

// ---------------------------------------------------------------------------
// Time Mapping
// ---------------------------------------------------------------------------

/// Convert Veloren game tick + real Time to MEMZ `GameTimestamp`.
///
/// Veloren's `common::resources::Time(f64)` counts seconds since server start.
/// We convert to a monotonic tick for MEMZ, using Veloren's tick counter.
#[must_use]
pub fn veloren_time_to_timestamp(tick: u64) -> GameTimestamp {
    GameTimestamp::now(tick)
}

// ---------------------------------------------------------------------------
// Dialogue Context Builder
// ---------------------------------------------------------------------------

/// A structured context snapshot for memory-aware dialogue generation.
///
/// This is assembled from both Veloren state and MEMZ memories, then
/// fed to the prompt template engine.
#[derive(Debug, Clone)]
pub struct DialogueContext {
    /// The NPC's name.
    pub npc_name: String,
    /// The NPC's profession/role.
    pub npc_profession: String,
    /// The settlement they're in.
    pub settlement_name: String,
    /// Personality description (human-readable).
    pub personality_description: String,
    /// Current PAD state.
    pub pad_state: PADState,
    /// Sentiment level toward the dialogue partner.
    pub sentiment: SentimentLevel,
    /// Top-K retrieved memories (pre-ranked by retrieval engine).
    pub top_memories: Vec<MemorySnippet>,
    /// What the player said/did.
    pub player_action: String,
    /// Overall context description.
    pub context_description: String,
}

/// A simplified memory representation for dialogue prompts.
#[derive(Debug, Clone)]
pub struct MemorySnippet {
    /// The memory type (e.g., "episodic", "social", "emotional").
    pub memory_type: String,
    /// Human-readable summary.
    pub summary: String,
    /// Memory strength (0.0–1.0).
    pub strength: f32,
    /// Age in game-days.
    pub age_days: f32,
}

impl DialogueContext {
    /// Render personality traits as a human-readable description.
    #[must_use]
    pub fn describe_personality(traits: &PersonalityTraits) -> String {
        let mut parts = Vec::new();

        if traits.openness > 0.7 {
            parts.push("curious and open-minded");
        } else if traits.openness < 0.3 {
            parts.push("traditional and cautious");
        }

        if traits.credulity > 0.7 {
            parts.push("trusting");
        } else if traits.credulity < 0.3 {
            parts.push("skeptical");
        }

        if traits.gossip_tendency > 0.7 {
            parts.push("talkative and social");
        } else if traits.gossip_tendency < 0.3 {
            parts.push("reserved and private");
        }

        if traits.emotional_volatility > 0.7 {
            parts.push("emotional and reactive");
        } else if traits.emotional_volatility < 0.3 {
            parts.push("calm and composed");
        }

        if traits.bravery > 0.7 {
            parts.push("brave and confrontational");
        } else if traits.bravery < 0.3 {
            parts.push("cautious and avoidant");
        }

        if parts.is_empty() {
            "balanced and unremarkable".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Build the template variables map for prompt rendering.
    #[must_use]
    pub fn to_template_vars(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        vars.insert("npc_name".to_string(), self.npc_name.clone());
        vars.insert("npc_profession".to_string(), self.npc_profession.clone());
        vars.insert("settlement_name".to_string(), self.settlement_name.clone());
        vars.insert(
            "personality_description".to_string(),
            self.personality_description.clone(),
        );
        vars.insert(
            "pad_state".to_string(),
            format!(
                "P={:.1} A={:.1} D={:.1}",
                self.pad_state.pleasure, self.pad_state.arousal, self.pad_state.dominance
            ),
        );
        vars.insert("pleasure".to_string(), format!("{:.2}", self.pad_state.pleasure));
        vars.insert("arousal".to_string(), format!("{:.2}", self.pad_state.arousal));
        vars.insert("dominance".to_string(), format!("{:.2}", self.pad_state.dominance));
        vars.insert(
            "overall_sentiment".to_string(),
            self.sentiment.description().to_string(),
        );
        vars.insert("player_action".to_string(), self.player_action.clone());
        vars.insert(
            "context_description".to_string(),
            self.context_description.clone(),
        );

        // Build top_memories as a formatted block
        let memories_text: String = self
            .top_memories
            .iter()
            .map(|m| {
                format!(
                    "- [{}] {} (strength: {:.2}, age: {:.1} days)",
                    m.memory_type, m.summary, m.strength, m.age_days
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        vars.insert("top_memories".to_string(), memories_text);

        vars
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personality_mapping_round_trip() {
        let ocean = [128u8, 200, 50, 180, 100]; // mid open, high conscientious, low extrav, high agree, mid neuro
        let traits = veloren_personality_to_memz(ocean[0], ocean[1], ocean[2], ocean[3], ocean[4]);

        // Check ranges are valid
        assert!((0.0..=1.0).contains(&traits.openness));
        assert!((0.0..=1.0).contains(&traits.credulity));
        assert!((0.0..=1.0).contains(&traits.gossip_tendency));
        assert!((0.0..=1.0).contains(&traits.emotional_volatility));
        assert!((0.0..=1.0).contains(&traits.bravery));

        // High agreeableness → high credulity
        assert!(traits.credulity > 0.5);
        // Low extraversion → low gossip tendency
        assert!(traits.gossip_tendency < 0.5);
    }

    #[test]
    fn personality_extreme_values() {
        // All maxed out
        let traits = veloren_personality_to_memz(255, 255, 255, 255, 255);
        assert!((0.0..=1.0).contains(&traits.openness));
        assert!((0.0..=1.0).contains(&traits.credulity));
        assert!((0.0..=1.0).contains(&traits.gossip_tendency));
        assert!((0.0..=1.0).contains(&traits.emotional_volatility));
        assert!((0.0..=1.0).contains(&traits.bravery));

        // All zeroed
        let traits = veloren_personality_to_memz(0, 0, 0, 0, 0);
        assert!((0.0..=1.0).contains(&traits.bravery));
    }

    #[test]
    fn sentiment_level_classification() {
        assert_eq!(SentimentLevel::from_value(0.9), SentimentLevel::Hero);
        assert_eq!(SentimentLevel::from_value(0.6), SentimentLevel::Friend);
        assert_eq!(SentimentLevel::from_value(0.3), SentimentLevel::Ally);
        assert_eq!(SentimentLevel::from_value(0.0), SentimentLevel::Neutral);
        assert_eq!(SentimentLevel::from_value(-0.5), SentimentLevel::Rival);
        assert_eq!(SentimentLevel::from_value(-0.9), SentimentLevel::Villain);
    }

    #[test]
    fn sentiment_pad_modifiers_are_valid() {
        for level in [
            SentimentLevel::Hero,
            SentimentLevel::Friend,
            SentimentLevel::Ally,
            SentimentLevel::Positive,
            SentimentLevel::Neutral,
            SentimentLevel::Negative,
            SentimentLevel::Rival,
            SentimentLevel::Enemy,
            SentimentLevel::Villain,
        ] {
            let pad = level.to_pad_modifier();
            assert!((-1.0..=1.0).contains(&pad.pleasure));
            assert!((-1.0..=1.0).contains(&pad.arousal));
            assert!((-1.0..=1.0).contains(&pad.dominance));
        }
    }

    #[test]
    fn entity_registry_npc() {
        let mut reg = EntityRegistry::new();
        let id1 = reg.npc_entity(42);
        let id2 = reg.npc_entity(42);
        assert_eq!(id1, id2); // Same NPC → same EntityId

        let id3 = reg.npc_entity(99);
        assert_ne!(id1, id3); // Different NPC → different EntityId

        assert_eq!(reg.lookup_npc(&id1), Some(42));
        assert_eq!(reg.count(), 2);
    }

    #[test]
    fn entity_registry_character() {
        let mut reg = EntityRegistry::new();
        let id = reg.character_entity(1001);
        assert_eq!(reg.lookup_character(&id), Some(1001));
    }

    #[test]
    fn dialogue_context_template_vars() {
        let ctx = DialogueContext {
            npc_name: "Goran".to_string(),
            npc_profession: "Blacksmith".to_string(),
            settlement_name: "Ironhaven".to_string(),
            personality_description: "gruff but fair".to_string(),
            pad_state: PADState::new(0.3, -0.1, 0.5),
            sentiment: SentimentLevel::Ally,
            top_memories: vec![MemorySnippet {
                memory_type: "episodic".to_string(),
                summary: "Player helped defend the forge".to_string(),
                strength: 0.8,
                age_days: 2.5,
            }],
            player_action: "greeted the blacksmith".to_string(),
            context_description: "Player approaches Goran at his forge".to_string(),
        };

        let vars = ctx.to_template_vars();
        assert_eq!(vars.get("npc_name").unwrap(), "Goran");
        assert!(vars.get("top_memories").unwrap().contains("episodic"));
        assert!(vars.get("overall_sentiment").unwrap().contains("ally"));
    }

    #[test]
    fn describe_personality_traits() {
        let brave_open = PersonalityTraits {
            openness: 0.9,
            credulity: 0.5,
            gossip_tendency: 0.2,
            emotional_volatility: 0.1,
            bravery: 0.9,
        };
        let desc = DialogueContext::describe_personality(&brave_open);
        assert!(desc.contains("curious"));
        assert!(desc.contains("brave"));
        assert!(desc.contains("calm"));
        assert!(desc.contains("reserved"));
    }

    #[test]
    fn location_round_trip() {
        let loc = veloren_pos_to_location(100.5, 200.3, 50.0);
        let (x, y, z) = location_to_veloren_pos(&loc);
        assert!((x - 100.5).abs() < f32::EPSILON);
        assert!((y - 200.3).abs() < f32::EPSILON);
        assert!((z - 50.0).abs() < f32::EPSILON);
    }
}
