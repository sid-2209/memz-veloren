//! Reputation Board — settlement-level reputation tracking (§14.2)
//!
//! Each settlement maintains a reputation board that aggregates NPC sentiment
//! into a public score visible to players. This creates a feedback loop where
//! player actions have visible, persistent consequences.
//!
//! Features:
//! - Per-settlement reputation derived from NPC dispositions
//! - Reputation tiers (Hero, Ally, Neutral, Outcast, Villain)
//! - Reputation decay over time (redemption is possible)
//! - Notable deeds tracking (visible on the board)

use serde::{Deserialize, Serialize};

use crate::types::{EntityId, GameTimestamp, SettlementId};

/// A settlement's reputation board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationBoard {
    /// Which settlement this board belongs to.
    pub settlement: SettlementId,
    /// Individual reputation entries (one per known entity).
    pub entries: Vec<ReputationEntry>,
    /// Notable deeds displayed on the board.
    pub notable_deeds: Vec<NotableDeed>,
    /// Maximum entries on the board.
    pub max_entries: usize,
    /// Maximum notable deeds to display.
    pub max_deeds: usize,
    /// Last time the board was refreshed.
    pub last_refresh: GameTimestamp,
}

/// A single entity's reputation within a settlement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEntry {
    /// Who this reputation is about.
    pub entity: EntityId,
    /// Current reputation score (-1.0 to +1.0).
    pub score: f32,
    /// Reputation tier derived from score.
    pub tier: ReputationTier,
    /// Number of NPCs who contributed to this score.
    pub contributor_count: u32,
    /// Last time this entry was updated.
    pub last_updated: GameTimestamp,
}

/// Reputation tiers visible to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReputationTier {
    /// Score > 0.8: Revered hero of the settlement.
    Hero,
    /// Score 0.4–0.8: Trusted ally.
    Ally,
    /// Score 0.1–0.4: Known and somewhat liked.
    Friendly,
    /// Score -0.1 to 0.1: Unknown or neutral.
    Neutral,
    /// Score -0.4 to -0.1: Somewhat disliked.
    Unfriendly,
    /// Score -0.8 to -0.4: Social outcast, unwelcome.
    Outcast,
    /// Score < -0.8: Active villain, may be attacked on sight.
    Villain,
}

impl ReputationTier {
    /// Classify a score into a tier.
    #[must_use]
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s > 0.8 => Self::Hero,
            s if s > 0.4 => Self::Ally,
            s if s > 0.1 => Self::Friendly,
            s if s > -0.1 => Self::Neutral,
            s if s > -0.4 => Self::Unfriendly,
            s if s > -0.8 => Self::Outcast,
            _ => Self::Villain,
        }
    }

    /// Get a human-readable description.
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::Hero => "Revered Hero — the settlement celebrates your deeds",
            Self::Ally => "Trusted Ally — you are welcome and respected here",
            Self::Friendly => "Known Friend — people recognise and like you",
            Self::Neutral => "Stranger — no one knows you yet",
            Self::Unfriendly => "Unwelcome — people eye you with suspicion",
            Self::Outcast => "Outcast — you are shunned and unwelcome",
            Self::Villain => "Villain — you may be attacked on sight",
        }
    }
}

/// A notable deed displayed on the settlement's reputation board.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotableDeed {
    /// Who performed the deed.
    pub actor: EntityId,
    /// Description of the deed.
    pub description: String,
    /// Emotional valence of the deed (-1.0 to +1.0).
    pub valence: f32,
    /// When the deed occurred.
    pub timestamp: GameTimestamp,
    /// How many NPCs witnessed or know about it.
    pub witness_count: u32,
}

impl ReputationBoard {
    /// Create a new empty reputation board.
    #[must_use]
    pub fn new(settlement: SettlementId, timestamp: GameTimestamp) -> Self {
        Self {
            settlement,
            entries: Vec::new(),
            notable_deeds: Vec::new(),
            max_entries: 100,
            max_deeds: 20,
            last_refresh: timestamp,
        }
    }

    /// Update reputation for an entity based on NPC sentiment reports.
    ///
    /// Each call from an NPC adds their individual sentiment.
    /// The board aggregates these into a single score.
    pub fn report_sentiment(
        &mut self,
        entity: EntityId,
        sentiment: f32,
        timestamp: GameTimestamp,
    ) {
        let sentiment = sentiment.clamp(-1.0, 1.0);

        if let Some(entry) = self.entries.iter_mut().find(|e| e.entity == entity) {
            // Running average
            let n = entry.contributor_count as f32;
            entry.score = (entry.score * n + sentiment) / (n + 1.0);
            entry.contributor_count += 1;
            entry.tier = ReputationTier::from_score(entry.score);
            entry.last_updated = timestamp;
        } else {
            // New entry
            self.entries.push(ReputationEntry {
                entity,
                score: sentiment,
                tier: ReputationTier::from_score(sentiment),
                contributor_count: 1,
                last_updated: timestamp,
            });

            // Enforce capacity
            if self.entries.len() > self.max_entries {
                // Drop lowest-contributor-count neutral entries
                self.entries.sort_by(|a, b| {
                    b.contributor_count
                        .cmp(&a.contributor_count)
                        .then(b.score.abs().partial_cmp(&a.score.abs()).unwrap_or(std::cmp::Ordering::Equal))
                });
                self.entries.truncate(self.max_entries);
            }
        }
    }

    /// Record a notable deed on the board.
    pub fn record_deed(&mut self, deed: NotableDeed) {
        self.notable_deeds.push(deed);

        // Keep only the most recent/impactful deeds
        if self.notable_deeds.len() > self.max_deeds {
            self.notable_deeds
                .sort_by(|a, b| {
                    b.valence
                        .abs()
                        .partial_cmp(&a.valence.abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            self.notable_deeds.truncate(self.max_deeds);
        }
    }

    /// Get an entity's reputation entry.
    #[must_use]
    pub fn get_reputation(&self, entity: EntityId) -> Option<&ReputationEntry> {
        self.entries.iter().find(|e| e.entity == entity)
    }

    /// Get an entity's reputation tier (defaults to Neutral if unknown).
    #[must_use]
    pub fn get_tier(&self, entity: EntityId) -> ReputationTier {
        self.get_reputation(entity)
            .map_or(ReputationTier::Neutral, |e| e.tier)
    }

    /// Decay all reputations toward neutral over time.
    ///
    /// This allows redemption — a villain can eventually become neutral
    /// if they stop committing crimes.
    pub fn decay_reputations(&mut self, decay_rate: f32, timestamp: GameTimestamp) {
        for entry in &mut self.entries {
            let days_since_update = {
                let delta = timestamp.tick.saturating_sub(entry.last_updated.tick);
                delta as f32 / 72_000.0 // ticks per game-day
            };
            let decay = (-decay_rate * days_since_update).exp();
            entry.score *= decay;
            entry.tier = ReputationTier::from_score(entry.score);
        }

        // Remove entries that have decayed to effectively neutral
        self.entries.retain(|e| e.score.abs() > 0.05);
        self.last_refresh = timestamp;
    }

    /// Get the top N most reputed entities (positive).
    #[must_use] 
    pub fn top_heroes(&self, count: usize) -> Vec<&ReputationEntry> {
        let mut sorted: Vec<&ReputationEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(count);
        sorted
    }

    /// Get the top N most notorious entities (negative).
    #[must_use] 
    pub fn top_villains(&self, count: usize) -> Vec<&ReputationEntry> {
        let mut sorted: Vec<&ReputationEntry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| {
            a.score
                .partial_cmp(&b.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.truncate(count);
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp, SettlementId};

    fn make_board() -> ReputationBoard {
        ReputationBoard::new(SettlementId::new(), GameTimestamp::now(36_000))
    }

    #[test]
    fn report_sentiment_creates_entry() {
        let mut board = make_board();
        let player = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        board.report_sentiment(player, 0.8, ts);

        let rep = board.get_reputation(player).expect("should exist");
        assert!((rep.score - 0.8).abs() < 0.01);
        assert_eq!(rep.tier, ReputationTier::Ally);
        assert_eq!(rep.contributor_count, 1);
    }

    #[test]
    fn multiple_sentiments_averaged() {
        let mut board = make_board();
        let player = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        board.report_sentiment(player, 0.8, ts);
        board.report_sentiment(player, 0.4, ts);
        board.report_sentiment(player, 0.6, ts);

        let rep = board.get_reputation(player).expect("should exist");
        // Running average: (0.8 + 0.4 + 0.6) / 3 = 0.6
        assert!((rep.score - 0.6).abs() < 0.01);
        assert_eq!(rep.contributor_count, 3);
    }

    #[test]
    fn reputation_tiers() {
        assert_eq!(ReputationTier::from_score(0.9), ReputationTier::Hero);
        assert_eq!(ReputationTier::from_score(0.5), ReputationTier::Ally);
        assert_eq!(ReputationTier::from_score(0.2), ReputationTier::Friendly);
        assert_eq!(ReputationTier::from_score(0.0), ReputationTier::Neutral);
        assert_eq!(ReputationTier::from_score(-0.2), ReputationTier::Unfriendly);
        assert_eq!(ReputationTier::from_score(-0.5), ReputationTier::Outcast);
        assert_eq!(ReputationTier::from_score(-0.9), ReputationTier::Villain);
    }

    #[test]
    fn reputation_decay() {
        let mut board = make_board();
        let player = EntityId::new();

        board.report_sentiment(player, 0.9, GameTimestamp::now(0));

        // Simulate time passing (72000 ticks = 1 game-day)
        board.decay_reputations(0.1, GameTimestamp::now(720_000)); // 10 days

        let rep = board.get_reputation(player);
        // After decay, score should be lower
        if let Some(rep) = rep {
            assert!(rep.score < 0.9);
        }
    }

    #[test]
    fn notable_deeds() {
        let mut board = make_board();
        let player = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        board.record_deed(NotableDeed {
            actor: player,
            description: "Defended the village from a dragon attack".to_string(),
            valence: 0.9,
            timestamp: ts,
            witness_count: 15,
        });

        assert_eq!(board.notable_deeds.len(), 1);
        assert_eq!(board.notable_deeds[0].witness_count, 15);
    }

    #[test]
    fn top_heroes_and_villains() {
        let mut board = make_board();
        let hero = EntityId::new();
        let villain = EntityId::new();
        let neutral = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        board.report_sentiment(hero, 0.9, ts);
        board.report_sentiment(villain, -0.8, ts);
        board.report_sentiment(neutral, 0.0, ts);

        let heroes = board.top_heroes(1);
        assert_eq!(heroes.len(), 1);
        assert_eq!(heroes[0].entity, hero);

        let villains = board.top_villains(1);
        assert_eq!(villains.len(), 1);
        assert_eq!(villains[0].entity, villain);
    }

    #[test]
    fn unknown_entity_is_neutral() {
        let board = make_board();
        let unknown = EntityId::new();

        assert_eq!(board.get_tier(unknown), ReputationTier::Neutral);
    }
}
