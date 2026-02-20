//! Emotional Memory — "How I feel" (§8.3)
//!
//! Persistent emotional associations with entities, places, or concepts.
//! Uses the PAD (Pleasure-Arousal-Dominance) model from Russell & Mehrabian (1977).

use serde::{Deserialize, Serialize};

use crate::types::{EntityId, GameTimestamp, MemoryId, PADState};

/// An emotional memory — a persistent feeling toward a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalMemory {
    /// Unique identifier.
    pub id: MemoryId,
    /// What or who the emotion is about.
    pub target: EntityId,
    /// Primary emotion label (trust, fear, admiration, resentment, etc.).
    pub emotion: String,
    /// Intensity of the emotion (0.0 to 1.0).
    pub intensity: f32,
    /// Full PAD emotional state toward this target.
    pub pad_state: PADState,
    /// Trajectory: is the feeling getting stronger, weaker, or stable?
    pub trajectory: EmotionTrajectory,
    /// Episodic memories that form the basis of this emotion.
    pub basis: Vec<MemoryId>,
    /// When this emotional association was last updated.
    pub last_updated: GameTimestamp,
}

/// Direction an emotional association is trending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmotionTrajectory {
    /// Getting stronger over time.
    Increasing,
    /// Roughly stable.
    Stable,
    /// Fading over time.
    Decreasing,
}

impl EmotionalMemory {
    /// Create a new emotional memory.
    #[must_use]
    pub fn new(
        target: EntityId,
        emotion: impl Into<String>,
        intensity: f32,
        pad_state: PADState,
        basis: Vec<MemoryId>,
        timestamp: GameTimestamp,
    ) -> Self {
        Self {
            id: MemoryId::new(),
            target,
            emotion: emotion.into(),
            intensity: intensity.clamp(0.0, 1.0),
            pad_state,
            trajectory: EmotionTrajectory::Stable,
            basis,
            last_updated: timestamp,
        }
    }

    /// Update the emotion with a new event, shifting intensity and PAD state.
    pub fn update(
        &mut self,
        valence_shift: f32,
        arousal_shift: f32,
        new_basis: MemoryId,
        now: GameTimestamp,
    ) {
        let old_intensity = self.intensity;
        self.intensity = (self.intensity + valence_shift.abs() * 0.1).clamp(0.0, 1.0);
        self.pad_state = PADState::new(
            self.pad_state.pleasure + valence_shift * 0.2,
            self.pad_state.arousal + arousal_shift * 0.2,
            self.pad_state.dominance,
        );
        self.basis.push(new_basis);
        self.last_updated = now;

        // Update trajectory.
        if self.intensity > old_intensity + 0.05 {
            self.trajectory = EmotionTrajectory::Increasing;
        } else if self.intensity < old_intensity - 0.05 {
            self.trajectory = EmotionTrajectory::Decreasing;
        } else {
            self.trajectory = EmotionTrajectory::Stable;
        }
    }
}
