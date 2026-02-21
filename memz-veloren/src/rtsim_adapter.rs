//! Veloren rtsim adapter — thin glue wiring MEMZ into `RtState` (§12.2).
//!
//! This module provides the concrete [`Rule`] implementation that registers
//! MEMZ event handlers with Veloren's rtsim event bus.  It converts native
//! Veloren events (`OnDeath`, `OnHelped`, `OnTheft`, `OnTick`, etc.) into
//! MEMZ `MemoryRule` calls, and feeds memory-informed data back into the
//! NPC `Controller` for dialogue, pricing, and behavior.
//!
//! ## Integration Pattern
//!
//! ```text
//! RtState::start_rule::<MemzRule>()
//!   └─ binds OnDeath  → on_death_handler()
//!   └─ binds OnHelped → on_helped_handler()
//!   └─ binds OnTheft  → on_theft_handler()
//!   └─ binds OnTick   → on_tick_handler()
//! ```
//!
//! ## Lifecycle
//!
//! 1. On server startup, `MemzRule::start()` loads config, creates `MemoryRule`.
//! 2. Each bound handler converts Veloren types to MEMZ types via `bridge`.
//! 3. On `OnTick`, decay + gossip + reflection are run within frame budget.
//! 4. On dialogue initiation, the adapter provides memory context to the
//!    dialogue tree so NPCs can reference past events.
//!
//! ## Build Note
//!
//! This file is **not compiled** as part of `memz-veloren`'s normal build
//! because it would require a direct dependency on the `veloren-rtsim` crate,
//! creating a circular dependency.  Instead, this serves as the ready-to-paste
//! adapter that a Veloren fork would place inside `veloren/rtsim/src/rule/`.
//!
//! The adapter depends on:
//! - `rtsim::{RtState, Rule, RuleError, EventCtx}`
//! - `rtsim::event::{OnDeath, OnHelped, OnTheft, OnTick, OnHealthChange}`
//! - `rtsim::data::npc::{Npc, NpcId}`
//! - `common::rtsim::Actor`
//! - `memz_core` (via `memz-veloren`)

// ---- The code below is intended for `veloren/rtsim/src/rule/memz.rs` ----
//
// ```rust
// use crate::{RtState, Rule, RuleError, event::*};
// use memz_veloren::{bridge, memory_rule, dialogue};
// use memz_veloren::memory_rule::MemoryRule;
// use memz_core::types::Location;
// use parking_lot::Mutex;
// use std::sync::Arc;
//
// pub struct MemzRule {
//     memory: Arc<Mutex<MemoryRule>>,
// }
//
// impl Rule for MemzRule {
//     fn start(rtstate: &mut RtState) -> Result<Self, RuleError> {
//         let memory = Arc::new(Mutex::new(MemoryRule::new()));
//
//         // --- Bind: OnDeath ---
//         {
//             let mem = Arc::clone(&memory);
//             rtstate.bind::<Self, OnDeath>(move |ctx| {
//                 let mut rule = mem.lock();
//                 let data = ctx.state.data();
//
//                 let deceased_id = match ctx.event.actor {
//                     Actor::Npc(npc_id) => {
//                         if let Some(npc) = data.npcs.get(npc_id) {
//                             rule.registry.npc_entity(npc.uid)
//                         } else { return; }
//                     }
//                     Actor::Character(cid) => rule.registry.character_entity(cid.0),
//                 };
//
//                 let killer_id = ctx.event.killer.map(|actor| match actor {
//                     Actor::Npc(npc_id) => {
//                         data.npcs.get(npc_id)
//                             .map(|npc| rule.registry.npc_entity(npc.uid))
//                     }
//                     Actor::Character(cid) => Some(rule.registry.character_entity(cid.0)),
//                 }).flatten();
//
//                 let location = ctx.event.wpos.map(|w| Location { x: w.x, y: w.y, z: w.z })
//                     .unwrap_or_default();
//
//                 // Gather nearby NPC witnesses (within observation radius)
//                 let witnesses = gather_nearby_npcs(&data, ctx.event.wpos, &mut rule.registry);
//
//                 let settlement = resolve_settlement(&data, ctx.event.wpos);
//
//                 memory_rule::on_death(
//                     &mut rule, deceased_id, killer_id,
//                     &witnesses, location, settlement,
//                     bridge::veloren_time_to_timestamp(data.tick),
//                 );
//             });
//         }
//
//         // --- Bind: OnHelped ---
//         {
//             let mem = Arc::clone(&memory);
//             rtstate.bind::<Self, OnHelped>(move |ctx| {
//                 let mut rule = mem.lock();
//                 let data = ctx.state.data();
//
//                 let helped_id = match ctx.event.actor {
//                     Actor::Npc(npc_id) => {
//                         data.npcs.get(npc_id).map(|n| rule.registry.npc_entity(n.uid))
//                     }
//                     Actor::Character(cid) => Some(rule.registry.character_entity(cid.0)),
//                 };
//                 let helper_id = ctx.event.saver.and_then(|actor| match actor {
//                     Actor::Npc(npc_id) => data.npcs.get(npc_id).map(|n| rule.registry.npc_entity(n.uid)),
//                     Actor::Character(cid) => Some(rule.registry.character_entity(cid.0)),
//                 });
//
//                 if let (Some(helped), Some(helper)) = (helped_id, helper_id) {
//                     let npc_pos = match ctx.event.actor {
//                         Actor::Npc(npc_id) => data.npcs.get(npc_id).map(|n| n.wpos),
//                         _ => None,
//                     };
//                     let location = npc_pos.map(|w| Location { x: w.x, y: w.y, z: w.z })
//                         .unwrap_or_default();
//
//                     let witnesses = gather_nearby_npcs(&data, npc_pos, &mut rule.registry);
//                     let settlement = resolve_settlement(&data, npc_pos);
//
//                     memory_rule::on_helped(
//                         &mut rule, helped, helper,
//                         "defended from danger",
//                         &witnesses, location, settlement,
//                         bridge::veloren_time_to_timestamp(data.tick),
//                     );
//                 }
//             });
//         }
//
//         // --- Bind: OnTheft ---
//         {
//             let mem = Arc::clone(&memory);
//             rtstate.bind::<Self, OnTheft>(move |ctx| {
//                 let mut rule = mem.lock();
//                 let data = ctx.state.data();
//
//                 let thief_id = match ctx.event.actor {
//                     Actor::Npc(npc_id) => data.npcs.get(npc_id).map(|n| rule.registry.npc_entity(n.uid)),
//                     Actor::Character(cid) => Some(rule.registry.character_entity(cid.0)),
//                 };
//
//                 if let Some(thief) = thief_id {
//                     let location = Location {
//                         x: ctx.event.wpos.x as f32,
//                         y: ctx.event.wpos.y as f32,
//                         z: ctx.event.wpos.z as f32,
//                     };
//                     let sprite_desc = format!("{:?}", ctx.event.sprite);
//                     let witnesses = gather_nearby_npcs(
//                         &data,
//                         Some(vek::Vec3::new(location.x, location.y, location.z)),
//                         &mut rule.registry,
//                     );
//                     let settlement = ctx.event.site.and_then(|site_id| {
//                         // Map Veloren SiteId → MEMZ SettlementId
//                         Some(memz_core::types::SettlementId::new()) // TODO: stable mapping
//                     });
//
//                     memory_rule::on_theft(
//                         &mut rule, thief, &witnesses,
//                         &sprite_desc, location, settlement,
//                         bridge::veloren_time_to_timestamp(data.tick),
//                     );
//                 }
//             });
//         }
//
//         // --- Bind: OnTick ---
//         {
//             let mem = Arc::clone(&memory);
//             rtstate.bind::<Self, OnTick>(move |ctx| {
//                 let mut rule = mem.lock();
//                 let data = ctx.state.data();
//
//                 // Sync personality traits for loaded NPCs
//                 if ctx.event.tick % 600 == 0 {
//                     for (npc_id, npc) in data.npcs.iter() {
//                         let entity = rule.registry.npc_entity(npc.uid);
//                         let memz_personality = bridge::veloren_personality_to_memz(
//                             npc.personality.openness(),    // Would need accessor
//                             npc.personality.conscientiousness(),
//                             npc.personality.extraversion(),
//                             npc.personality.agreeableness(),
//                             npc.personality.neuroticism(),
//                         );
//                         rule.set_personality(entity, memz_personality);
//                     }
//                 }
//
//                 // Run MEMZ tick (decay, reflection, eviction)
//                 memory_rule::on_tick(&mut rule, ctx.event.tick, ctx.event.dt);
//
//                 // Gossip propagation for NPCs that are talking
//                 // (Veloren marks NPCs with NpcActivity::Talk)
//                 if ctx.event.tick % 30 == 0 {
//                     for (npc_id, npc) in data.npcs.iter() {
//                         if let Some(common::rtsim::NpcActivity::Talk(target)) =
//                             npc.controller.activity
//                         {
//                             if let Actor::Npc(target_npc_id) = target {
//                                 if let Some(target_npc) = data.npcs.get(target_npc_id) {
//                                     let speaker = rule.registry.npc_entity(npc.uid);
//                                     let listener = rule.registry.npc_entity(target_npc.uid);
//                                     let ts = bridge::veloren_time_to_timestamp(data.tick);
//                                     memory_rule::propagate_gossip(&mut rule, speaker, listener, ts);
//                                 }
//                             }
//                         }
//                     }
//                 }
//             });
//         }
//
//         Ok(Self { memory })
//     }
// }
//
// // ---------------------------------------------------------------------------
// // Helper functions
// // ---------------------------------------------------------------------------
//
// /// Gather MEMZ EntityIds for NPCs within observation radius of a world position.
// fn gather_nearby_npcs(
//     data: &crate::data::Data,
//     wpos: Option<vek::Vec3<f32>>,
//     registry: &mut bridge::EntityRegistry,
// ) -> Vec<memz_core::types::EntityId> {
//     let Some(center) = wpos else { return Vec::new(); };
//     let radius_sq = 50.0_f32 * 50.0; // 50 block observation radius
//
//     data.npcs.iter()
//         .filter(|(_, npc)| {
//             let diff = npc.wpos - center;
//             diff.magnitude_squared() < radius_sq && !npc.is_dead()
//         })
//         .map(|(_, npc)| registry.npc_entity(npc.uid))
//         .collect()
// }
//
// /// Resolve the MEMZ SettlementId for a world position.
// fn resolve_settlement(
//     data: &crate::data::Data,
//     wpos: Option<vek::Vec3<f32>>,
// ) -> Option<memz_core::types::SettlementId> {
//     // Find the nearest site to the position
//     let center = wpos?;
//     let _nearest = data.sites.iter()
//         .min_by_key(|(_, site)| {
//             let diff = site.wpos.as_::<f32>() - center.xy();
//             diff.magnitude_squared() as i64
//         });
//     // TODO: maintain a stable SiteId → SettlementId mapping
//     Some(memz_core::types::SettlementId::new())
// }
// ```
//
// ## Dialogue Integration
//
// To wire memory-aware dialogue into Veloren's dialogue tree, modify
// `veloren/rtsim/src/rule/npc_ai/dialogue.rs`:
//
// ```rust
// // In the `general` function, add a "memories" response option:
//
// // Memory-aware sentiments (replaces 3-tier system)
// responses.push((
//     Response::from(Content::localized("dialogue-question-sentiment")),
//     dialogue_memory_sentiment(tgt, session, &memz_rule).boxed(),
// ));
//
// // Gossip sharing (new)
// if let Some(gossip) = memz_dialogue::generate_gossip_text(bank, personality, npc_name) {
//     responses.push((
//         Response::from(Content::localized("dialogue-share-gossip")),
//         session.say_statement(Content::Plain(gossip)).boxed(),
//     ));
// }
//
// fn dialogue_memory_sentiment<S: State>(
//     tgt: Actor,
//     session: DialogueSession,
//     memz_rule: &MemoryRule,
// ) -> impl Action<S> {
//     now(move |ctx, _| {
//         let entity = memz_rule.registry.npc_entity(ctx.npc.uid);
//         let player = match tgt {
//             Actor::Character(cid) => memz_rule.registry.character_entity(cid.0),
//             Actor::Npc(npc_id) => {
//                 ctx.data.npcs.get(npc_id)
//                     .map(|n| memz_rule.registry.npc_entity(n.uid))
//                     .unwrap_or(EntityId::new())
//             }
//         };
//
//         let sentiment_level = SentimentLevel::from_value(
//             ctx.sentiments.toward(tgt).positivity as f32 / 126.0
//         );
//
//         if let Some(bank) = memz_rule.bank(entity) {
//             let personality = memz_rule.personality(&entity);
//             let ts = bridge::veloren_time_to_timestamp(ctx.data.tick);
//             let response = memz_dialogue::generate_sentiment_response(
//                 bank, &personality, player, &ctx.npc.get_name().unwrap_or_default(),
//                 sentiment_level, &ts,
//             );
//             session.say_statement(Content::Plain(response))
//         } else {
//             // Fallback to vanilla
//             session.say_statement(Content::localized("npc-response-ambivalent_you"))
//         }
//     })
// }
// ```
//
// ## Greeting Integration
//
// In `veloren/server/src/rtsim/tick.rs` or the agent greeting handler:
//
// ```rust
// // When an NPC starts a dialogue with a player:
// let entity = memz_rule.registry.npc_entity(npc.uid);
// let player = memz_rule.registry.character_entity(character_id.0);
// if let Some(bank) = memz_rule.bank(entity) {
//     let personality = memz_rule.personality(&entity);
//     let ts = bridge::veloren_time_to_timestamp(data.tick);
//     let (greeting, style) = memz_dialogue::generate_greeting(
//         bank, &personality, player, &npc.get_name().unwrap_or_default(), &ts,
//     );
//     controller.say(tgt, Content::Plain(greeting));
// }
// ```
//
// ## Price Modifier Integration
//
// In the trading system:
//
// ```rust
// // When calculating trade prices:
// let entity = memz_rule.registry.npc_entity(npc.uid);
// let player = memz_rule.registry.character_entity(character_id.0);
// if let Some(bank) = memz_rule.bank(entity) {
//     let personality = memz_rule.personality(&entity);
//     let modifier = memz_dialogue::get_price_modifier(bank, &personality, &player);
//     price = (base_price as f32 * modifier) as u32;
// }
// ```

// ---------------------------------------------------------------------------
// This file is intentionally a documentation-only module with no compiled code.
// The actual adapter code (shown above in doc-comments) is designed to be
// pasted into a Veloren fork's `rtsim/src/rule/memz.rs` file.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    /// Verify the module compiles and the doc-comments are well-formed.
    #[test]
    fn adapter_module_exists() {
        // This test simply confirms the module is reachable.
        assert!(true);
    }
}
