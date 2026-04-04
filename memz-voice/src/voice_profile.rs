//! NPC voice profile system.
//!
//! Maps NPC attributes (profession, personality, body type) to TTS
//! voice parameters so each NPC sounds distinct and appropriate.

/// Voice characteristics for a specific NPC.
#[derive(Debug, Clone)]
pub struct VoiceProfile {
    /// TTS voice/speaker ID (maps to Kokoro voice presets).
    pub voice_id: String,
    /// Speech rate multiplier (0.7 = slow, 1.0 = normal, 1.3 = fast).
    pub speed: f32,
    /// Pitch shift in semitones (-3.0 = deeper, 0.0 = normal, 3.0 = higher).
    pub pitch_shift: f32,
    /// Energy/volume modifier (0.5 = quiet, 1.0 = normal, 1.5 = loud).
    pub energy: f32,
    /// Name for logging/display.
    pub description: String,
}

impl VoiceProfile {
    /// Create a voice profile for an NPC based on their profession and personality traits.
    ///
    /// Maps Veloren professions to distinct voice characteristics:
    /// - Guards: deep, authoritative, measured pace
    /// - Merchants: warm, animated, slightly fast
    /// - Blacksmiths: deep, gruff, deliberate
    /// - Herbalists: soft, calm, gentle
    /// - Pirates: rough, loud, fast
    /// - Captains: clear, commanding
    /// etc.
    pub fn from_npc(
        profession: &str,
        personality_extraversion: f32,
        personality_neuroticism: f32,
    ) -> Self {
        let mut profile = match profession.to_lowercase().as_str() {
            "guard" => Self {
                voice_id: "af_sky".to_string(), // Kokoro American female "sky" as default
                speed: 0.95,
                pitch_shift: -1.5,
                energy: 1.1,
                description: "Guard: authoritative, measured".to_string(),
            },
            "merchant" | "trader" => Self {
                voice_id: "af_bella".to_string(),
                speed: 1.05,
                pitch_shift: 0.5,
                energy: 1.1,
                description: "Merchant: warm, animated".to_string(),
            },
            "blacksmith" => Self {
                voice_id: "am_adam".to_string(), // American male
                speed: 0.90,
                pitch_shift: -2.0,
                energy: 1.2,
                description: "Blacksmith: deep, gruff".to_string(),
            },
            "herbalist" => Self {
                voice_id: "bf_emma".to_string(), // British female
                speed: 0.90,
                pitch_shift: 1.0,
                energy: 0.85,
                description: "Herbalist: soft, calm".to_string(),
            },
            "chef" | "alchemist" => Self {
                voice_id: "af_nicole".to_string(),
                speed: 1.0,
                pitch_shift: 0.0,
                energy: 1.0,
                description: "Artisan: balanced, friendly".to_string(),
            },
            "pirate" => Self {
                voice_id: "am_michael".to_string(),
                speed: 1.10,
                pitch_shift: -1.0,
                energy: 1.3,
                description: "Pirate: rough, loud".to_string(),
            },
            "captain" => Self {
                voice_id: "bm_george".to_string(), // British male
                speed: 1.0,
                pitch_shift: -0.5,
                energy: 1.15,
                description: "Captain: commanding, clear".to_string(),
            },
            "adventurer" => Self {
                voice_id: "af_heart".to_string(),
                speed: 1.05,
                pitch_shift: 0.0,
                energy: 1.05,
                description: "Adventurer: energetic, confident".to_string(),
            },
            "farmer" | "hunter" => Self {
                voice_id: "am_adam".to_string(),
                speed: 0.90,
                pitch_shift: -0.5,
                energy: 0.95,
                description: "Villager: hearty, grounded".to_string(),
            },
            "cultist" => Self {
                voice_id: "bf_emma".to_string(),
                speed: 0.85,
                pitch_shift: -1.0,
                energy: 0.80,
                description: "Cultist: sinister, quiet".to_string(),
            },
            _ => Self::default(),
        };

        // Modulate based on personality traits
        // High extraversion → slightly faster, more energy
        profile.speed += (personality_extraversion - 0.5) * 0.1;
        profile.energy += (personality_extraversion - 0.5) * 0.15;

        // High neuroticism → slightly faster, higher pitch
        profile.speed += (personality_neuroticism - 0.5) * 0.05;
        profile.pitch_shift += (personality_neuroticism - 0.5) * 0.5;

        // Clamp to reasonable ranges
        profile.speed = profile.speed.clamp(0.7, 1.4);
        profile.pitch_shift = profile.pitch_shift.clamp(-3.0, 3.0);
        profile.energy = profile.energy.clamp(0.5, 1.5);

        profile
    }
}

impl Default for VoiceProfile {
    fn default() -> Self {
        Self {
            voice_id: "af_heart".to_string(),
            speed: 1.0,
            pitch_shift: 0.0,
            energy: 1.0,
            description: "Default: neutral".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_voice() {
        let profile = VoiceProfile::from_npc("guard", 0.4, 0.3);
        assert!(profile.pitch_shift < 0.0, "Guards should have deep voices");
        assert!(profile.speed < 1.0, "Guards should speak deliberately");
    }

    #[test]
    fn test_merchant_voice() {
        let profile = VoiceProfile::from_npc("merchant", 0.8, 0.4);
        assert!(profile.speed > 1.0, "Merchants should speak quickly");
        assert!(profile.energy > 1.0, "Merchants should be energetic");
    }

    #[test]
    fn test_personality_modulation() {
        let introverted = VoiceProfile::from_npc("guard", 0.1, 0.5);
        let extroverted = VoiceProfile::from_npc("guard", 0.9, 0.5);
        assert!(extroverted.speed > introverted.speed);
        assert!(extroverted.energy > introverted.energy);
    }

    #[test]
    fn test_unknown_profession() {
        let profile = VoiceProfile::from_npc("unknown_role", 0.5, 0.5);
        assert_eq!(profile.speed, 1.0);
    }

    #[test]
    fn test_clamping() {
        // Extreme personality values should still produce reasonable output
        let profile = VoiceProfile::from_npc("pirate", 1.0, 1.0);
        assert!(profile.speed >= 0.7 && profile.speed <= 1.4);
        assert!(profile.energy >= 0.5 && profile.energy <= 1.5);
    }
}
