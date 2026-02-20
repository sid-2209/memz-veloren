//! Procedural Memory — "What I know how to do" (§8.7)
//!
//! Skills and behavioral routines that improve with practice.
//! Models motor-learning curves and skill transfer.
//!
//! Grounded in Anderson's ACT-R theory of procedural learning.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{GameTimestamp, MemoryId};

/// Proficiency level for a skill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProficiencyLevel {
    /// Just discovered / first exposure.
    Novice,
    /// Some practice, improving.
    Beginner,
    /// Competent but not automatic.
    Intermediate,
    /// Proficient, mostly automatic.
    Advanced,
    /// Fully automatic, can teach others.
    Expert,
}

impl ProficiencyLevel {
    /// Numeric value for comparison and interpolation.
    #[must_use]
    pub fn as_f32(self) -> f32 {
        match self {
            Self::Novice => 0.0,
            Self::Beginner => 0.25,
            Self::Intermediate => 0.5,
            Self::Advanced => 0.75,
            Self::Expert => 1.0,
        }
    }

    /// Determine proficiency level from a numeric value.
    #[must_use]
    pub fn from_score(score: f32) -> Self {
        match score {
            s if s < 0.15 => Self::Novice,
            s if s < 0.35 => Self::Beginner,
            s if s < 0.60 => Self::Intermediate,
            s if s < 0.85 => Self::Advanced,
            _ => Self::Expert,
        }
    }
}

/// A procedural memory — a learned skill or behavioral routine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProceduralMemory {
    /// Unique identifier.
    pub id: MemoryId,
    /// Skill name (e.g. "sword_fighting", "cooking_stew", "bargaining").
    pub skill: String,
    /// Current proficiency (0.0 to 1.0).
    pub proficiency: f32,
    /// Number of times this skill has been practiced.
    pub repetitions: u32,
    /// Last time the skill was practiced.
    pub last_practiced: GameTimestamp,
    /// Learning rate — how fast this NPC learns this skill.
    /// Influenced by personality traits and related skills.
    pub learning_rate: f32,
    /// Related skills that can transfer knowledge.
    pub related_skills: Vec<MemoryId>,
    /// Behavioral routine description (what the NPC *does* when performing this skill).
    pub routine_description: String,
    /// When this procedural memory was first formed.
    pub created_at: DateTime<Utc>,
}

impl ProceduralMemory {
    /// Create a new procedural memory for a skill.
    #[must_use]
    pub fn new(
        skill: impl Into<String>,
        timestamp: GameTimestamp,
        learning_rate: f32,
    ) -> Self {
        Self {
            id: MemoryId::new(),
            skill: skill.into(),
            proficiency: 0.0,
            repetitions: 0,
            last_practiced: timestamp,
            learning_rate: learning_rate.clamp(0.01, 2.0),
            related_skills: Vec::new(),
            routine_description: String::new(),
            created_at: Utc::now(),
        }
    }

    /// Practice the skill once — proficiency grows with diminishing returns.
    ///
    /// Uses a logarithmic learning curve:
    /// `proficiency = learning_rate × ln(1 + repetitions) / ln(1 + max_reps)`
    ///
    /// Where `max_reps` is the theoretical number of reps to reach expert.
    pub fn practice(&mut self, timestamp: GameTimestamp) {
        self.repetitions += 1;
        self.last_practiced = timestamp;

        // Power-law learning curve (matches cognitive science research)
        const MAX_REPS_TO_EXPERT: f32 = 1000.0;
        let progress = ((1.0 + self.repetitions as f32).ln())
            / ((1.0 + MAX_REPS_TO_EXPERT).ln());
        self.proficiency = (self.learning_rate * progress).clamp(0.0, 1.0);
    }

    /// Decay proficiency due to lack of practice.
    ///
    /// Skill decay is much slower than episodic memory decay.
    pub fn decay(&mut self, days_since_practice: f32) {
        // Procedural memory decays slowly — "you never forget how to ride a bike"
        // But complex skills do atrophy without practice.
        let decay_factor = (-days_since_practice / 365.0).exp(); // ~63% retained after 1 year
        self.proficiency *= decay_factor;
        self.proficiency = self.proficiency.max(0.0);
    }

    /// Get the current proficiency level.
    #[must_use]
    pub fn level(&self) -> ProficiencyLevel {
        ProficiencyLevel::from_score(self.proficiency)
    }

    /// Whether this NPC can teach this skill to another.
    #[must_use]
    pub fn can_teach(&self) -> bool {
        self.proficiency >= ProficiencyLevel::Advanced.as_f32()
    }

    /// Apply skill transfer from a related skill.
    pub fn apply_transfer(&mut self, related_proficiency: f32, transfer_rate: f32) {
        let boost = related_proficiency * transfer_rate * 0.1;
        self.proficiency = (self.proficiency + boost).clamp(0.0, 1.0);
    }
}
