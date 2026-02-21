//! First 5 Minutes Experience — immediate engagement hook (§14.0)
//!
//! The first 5 minutes of gameplay are critical for player engagement.
//! This module orchestrates the "fuzzy seed NPC" encounter and starter
//! gossip boost that makes the world feel alive from the very first moment.
//!
//! ## Design (from §14.0)
//!
//! 1. **Fuzzy Seed NPC**: A pre-seeded NPC near the spawn point with
//!    rich memories and personality, ready for immediate deep interaction
//! 2. **Starter Gossip Boost**: Nearby NPCs have pre-seeded gossip about
//!    recent events, so players overhear interesting conversations immediately
//! 3. **First Meeting Protection**: The player's first interaction with any
//!    NPC creates a "first meeting" memory that never decays

use crate::memory::episodic::EpisodicMemory;
use crate::memory::injected::{InjectedMemory, InjectedPriority};
use crate::memory::semantic::SemanticMemory;
use crate::memory::social::SocialMemory;
use crate::memory::MemoryBank;
use crate::types::{EntityId, GameTimestamp, Location};

/// Configuration for the first-5-minutes experience.
#[derive(Debug, Clone)]
pub struct FirstFiveMinutesConfig {
    /// Number of seed memories for the fuzzy NPC.
    pub seed_npc_memory_count: usize,
    /// Number of gossip memories to inject into nearby NPCs.
    pub gossip_boost_count: usize,
    /// Radius (in world units) for gossip boost NPCs.
    pub gossip_radius: f32,
    /// Whether the seed NPC should have a "knows about player" memory.
    pub seed_npc_recognizes_player: bool,
}

impl Default for FirstFiveMinutesConfig {
    fn default() -> Self {
        Self {
            seed_npc_memory_count: 10,
            gossip_boost_count: 5,
            gossip_radius: 100.0,
            seed_npc_recognizes_player: false,
        }
    }
}

/// A seed NPC template for the first-5-minutes experience.
#[derive(Debug, Clone)]
pub struct SeedNpcTemplate {
    /// NPC's name.
    pub name: String,
    /// NPC's role/profession.
    pub profession: String,
    /// Pre-seeded backstory memories.
    pub backstory: Vec<String>,
    /// Pre-seeded knowledge (semantic memories).
    pub knowledge: Vec<String>,
    /// Pre-seeded gossip the NPC knows.
    pub gossip: Vec<SeedGossip>,
    /// Location where the NPC should be placed.
    pub location: Location,
}

/// A piece of pre-seeded gossip.
#[derive(Debug, Clone)]
pub struct SeedGossip {
    /// What entity the gossip is about.
    pub about_description: String,
    /// The gossip content.
    pub claim: String,
    /// How positive/negative the gossip is (-1.0 to +1.0).
    pub sentiment: f32,
}

/// Populate a memory bank for a seed NPC.
///
/// This creates a rich set of pre-existing memories that make the NPC
/// feel like they've been living in the world for a while.
#[must_use] 
pub fn populate_seed_npc(
    template: &SeedNpcTemplate,
    npc_id: EntityId,
    timestamp: GameTimestamp,
) -> MemoryBank {
    let mut bank = MemoryBank::new();

    // --- 1. Backstory memories (injected) ---
    for (i, backstory) in template.backstory.iter().enumerate() {
        let priority = if i < 2 {
            InjectedPriority::High
        } else if i < 5 {
            InjectedPriority::Normal
        } else {
            InjectedPriority::Low
        };

        let mut memory = InjectedMemory::new(
            backstory.clone(),
            0.5, // moderate emotional weight
            timestamp,
            priority,
        )
        .with_known_npcs(vec![npc_id]);
        memory.is_first_five_minutes = true;
        bank.injected.push(memory);
    }

    // --- 2. Knowledge (semantic memories) ---
    for knowledge in &template.knowledge {
        let semantic = SemanticMemory::new(
            knowledge.clone(),
            0.8, // high confidence (they "know" this)
            vec![],
            "world_knowledge",
            timestamp,
        );
        bank.semantic.push(semantic);
    }

    // --- 3. Gossip (social memories) ---
    for gossip in &template.gossip {
        let mut social = SocialMemory::new(
            EntityId::new(), // placeholder entity
            EntityId::new(), // placeholder source
            gossip.claim.clone(),
            0.7, // moderate trust
            0,   // first-hand (they "observed" this)
            timestamp,
        );
        social.sentiment = gossip.sentiment;
        bank.social.push(social);
    }

    // --- 4. Recent episodic memories (flavor) ---
    let flavor_events = [
        ("Had breakfast at the tavern this morning", 0.2, 0.1),
        ("Spoke with a traveling merchant about prices", 0.1, 0.2),
        ("Noticed the weather turning colder lately", 0.0, 0.1),
        ("Heard wolves howling near the forest edge last night", -0.2, 0.3),
    ];

    for (event, valence, importance) in &flavor_events {
        let episodic = EpisodicMemory::new(
            *event,
            vec![npc_id],
            template.location,
            timestamp,
            *valence,
            *importance,
        );
        bank.episodic.push(episodic);
    }

    bank
}

/// Inject starter gossip into nearby NPCs so they have something
/// interesting to talk about immediately.
pub fn inject_starter_gossip(
    banks: &mut [(EntityId, &mut MemoryBank)],
    gossip_items: &[SeedGossip],
    timestamp: GameTimestamp,
) {
    for (i, (_, bank)) in banks.iter_mut().enumerate() {
        // Each NPC gets a different subset of gossip
        for gossip in gossip_items.iter().skip(i % 2).take(3) {
            let mut social = SocialMemory::new(
                EntityId::new(),
                EntityId::new(),
                gossip.claim.clone(),
                0.6,
                1, // heard from someone
                timestamp,
            );
            social.sentiment = gossip.sentiment;
            bank.social.push(social);
        }
    }
}

/// Create a default seed NPC template for a blacksmith near the starter area.
#[must_use]
pub fn default_blacksmith_template() -> SeedNpcTemplate {
    SeedNpcTemplate {
        name: "Goran".to_string(),
        profession: "Blacksmith".to_string(),
        backstory: vec![
            "I've worked this forge for twenty years, since my father taught me the trade.".to_string(),
            "My wife passed during the winter plague three years ago. I still miss her.".to_string(),
            "I once forged a sword for a knight who went on to slay a great beast.".to_string(),
            "The mines to the north used to produce the finest iron, but they've been overrun.".to_string(),
            "I have a daughter studying alchemy in the capital city.".to_string(),
        ],
        knowledge: vec![
            "The best iron comes from the northern mines, though they're dangerous now.".to_string(),
            "A good sword requires at least three days of careful work.".to_string(),
            "The local guard captain is fair but strict about weapons in the market.".to_string(),
            "Wolves have been getting bolder near the village this season.".to_string(),
        ],
        gossip: vec![
            SeedGossip {
                about_description: "The merchant".to_string(),
                claim: "The merchant has been overcharging for healing potions lately.".to_string(),
                sentiment: -0.3,
            },
            SeedGossip {
                about_description: "The guard captain".to_string(),
                claim: "The guard captain drove off bandits from the south road last week.".to_string(),
                sentiment: 0.6,
            },
            SeedGossip {
                about_description: "Strange travelers".to_string(),
                claim: "Strange travelers from the east have been asking about ancient ruins nearby.".to_string(),
                sentiment: 0.0,
            },
        ],
        location: Location { x: 0.0, y: 0.0, z: 0.0 },
    }
}

/// Create a default seed NPC template for a tavern keeper.
#[must_use]
pub fn default_tavern_keeper_template() -> SeedNpcTemplate {
    SeedNpcTemplate {
        name: "Mira".to_string(),
        profession: "Tavern Keeper".to_string(),
        backstory: vec![
            "I inherited this tavern from my uncle who traveled to distant lands.".to_string(),
            "I hear everything that happens in this village — people talk over drinks.".to_string(),
            "I once served a legendary adventurer who left without paying. I still remember their face.".to_string(),
            "My specialty is honeyed mead, brewed with herbs from the eastern meadows.".to_string(),
        ],
        knowledge: vec![
            "The best rooms are upstairs, away from the noise of the common room.".to_string(),
            "Travelers usually arrive from the south road or by river from the west.".to_string(),
            "The local festival is coming up — it's the biggest event of the season.".to_string(),
        ],
        gossip: vec![
            SeedGossip {
                about_description: "The blacksmith".to_string(),
                claim: "The blacksmith Goran makes the finest blades in the region.".to_string(),
                sentiment: 0.5,
            },
            SeedGossip {
                about_description: "Mysterious figure".to_string(),
                claim: "A hooded figure was seen near the old ruins at midnight.".to_string(),
                sentiment: -0.2,
            },
        ],
        location: Location { x: 10.0, y: 0.0, z: 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp};

    #[test]
    fn seed_npc_has_memories() {
        let template = default_blacksmith_template();
        let npc_id = EntityId::new();
        let bank = populate_seed_npc(&template, npc_id, GameTimestamp::now(36_000));

        assert!(bank.injected.len() >= 4, "Should have backstory memories");
        assert!(bank.semantic.len() >= 3, "Should have knowledge");
        assert!(bank.social.len() >= 2, "Should have gossip");
        assert!(bank.episodic.len() >= 3, "Should have flavor memories");
    }

    #[test]
    fn seed_npc_backstory_marked_first_five() {
        let template = default_blacksmith_template();
        let bank = populate_seed_npc(&template, EntityId::new(), GameTimestamp::now(36_000));

        for injected in &bank.injected {
            assert!(injected.is_first_five_minutes);
        }
    }

    #[test]
    fn starter_gossip_distributed() {
        let ts = GameTimestamp::now(36_000);
        let mut bank1 = MemoryBank::new();
        let mut bank2 = MemoryBank::new();
        let e1 = EntityId::new();
        let e2 = EntityId::new();

        let gossip = vec![
            SeedGossip {
                about_description: "test".to_string(),
                claim: "Gossip A".to_string(),
                sentiment: 0.5,
            },
            SeedGossip {
                about_description: "test".to_string(),
                claim: "Gossip B".to_string(),
                sentiment: -0.3,
            },
            SeedGossip {
                about_description: "test".to_string(),
                claim: "Gossip C".to_string(),
                sentiment: 0.1,
            },
        ];

        inject_starter_gossip(
            &mut [(e1, &mut bank1), (e2, &mut bank2)],
            &gossip,
            ts,
        );

        assert!(!bank1.social.is_empty(), "NPC 1 should have gossip");
        assert!(!bank2.social.is_empty(), "NPC 2 should have gossip");
    }

    #[test]
    fn tavern_keeper_template() {
        let template = default_tavern_keeper_template();
        let bank = populate_seed_npc(&template, EntityId::new(), GameTimestamp::now(36_000));

        assert!(bank.total_count() > 5, "Tavern keeper should have many memories");
    }
}
