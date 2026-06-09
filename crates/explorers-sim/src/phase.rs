//! Autonomous phase functions for the tick loop.
//!
//! Each function takes agent state (and world parameters where needed),
//! mutates state in place, and returns events recording what happened.
//! These are free functions, not methods on World.

use crate::event::{Event, EventKind};
use crate::spatial::SpatialGrid;
use crate::{
    Agent, Carcass, Connection, FUNCTIONAL_TRAIT_COUNT, FUNCTIONAL_TRAIT_INDICES, TraitVector,
    WorldParameters,
};
use rand::Rng;
use rand_distr::{Distribution, Normal, Poisson};

/// Photosynthesise: agents with nonzero effective photosynthetic absorption
/// absorb energy from local solar flux into reserve. Light competition splits
/// flux proportionally among co-located producers via spatial grid query.
pub fn photosynthesise(
    agents: &mut [Agent],
    grid: &SpatialGrid,
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();
    let k = params.wear_degradation_steepness;
    let flux = params.solar_flux_magnitude;
    let competition_radius = params.light_competition_radius;
    let extent = params.world_extent;

    for i in 0..agents.len() {
        let eff_photo = agents[i].effective_trait_with_steepness(0, k);
        if eff_photo <= 0.0 {
            continue;
        }

        // Light share weighted by effective_autotrophy × structure (body size).
        // Bigger producers shade smaller ones. Zero structure means zero light.
        let my_weight = eff_photo * agents[i].structure;
        if my_weight <= 0.0 {
            continue;
        }

        // Compute total weight among co-located producers.
        // Use grid query but deduplicate results (grid can return duplicates
        // when query radius exceeds half the world extent).
        let mut total_weight = my_weight;
        let mut seen = std::collections::HashSet::new();
        seen.insert(i as u64);
        for j_id in grid.query_radius(agents[i].position, competition_radius) {
            let j = j_id as usize;
            if !seen.insert(j_id) {
                continue;
            }
            let other_eff = agents[j].effective_trait_with_steepness(0, k);
            if other_eff <= 0.0 {
                continue;
            }
            let other_weight = other_eff * agents[j].structure;
            if other_weight <= 0.0 {
                continue;
            }
            if crate::toroidal_distance(agents[i].position, agents[j].position, extent)
                < competition_radius
            {
                total_weight += other_weight;
            }
        }
        let light_share = my_weight / total_weight;
        let income = flux * light_share;

        agents[i].reserve += income;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Photosynthesized,
            source: agents[i].id,
            target: None,
            energy_delta: income,
            position: Some(agents[i].position),
            target_was_carcass: false,
        });
    }
    events
}

/// Absorb nutrients: uptake from the local nutrient grid cell at each agent's
/// position, proportional sharing within each cell when demand exceeds supply.
pub fn absorb_nutrients(
    agents: &mut [Agent],
    nutrient_grid: &mut crate::spatial::NutrientGrid,
    params: &WorldParameters,
) -> Vec<Event> {
    let mut events = Vec::new();

    let k = params.wear_degradation_steepness;

    // Group agents by nutrient grid cell, with their demand.
    let mut cell_agents: std::collections::HashMap<usize, Vec<(usize, f32)>> =
        std::collections::HashMap::new();

    for (i, agent) in agents.iter().enumerate() {
        let demand = agent.effective_trait_with_steepness(0, k);
        if demand <= 0.0 {
            continue;
        }
        let cell_idx = nutrient_grid.cell_index_for(agent.position);
        cell_agents.entry(cell_idx).or_default().push((i, demand));
    }

    // Process each cell independently
    for (cell_idx, agent_demands) in &cell_agents {
        let cell_pool = nutrient_grid.cell_mut(*cell_idx);
        if *cell_pool <= 0.0 {
            continue;
        }

        let total_demand: f32 = agent_demands.iter().map(|(_, d)| *d).sum();
        if total_demand <= 0.0 {
            continue;
        }

        let available = *cell_pool;
        for &(i, demand) in agent_demands {
            let uptake = if total_demand <= available {
                demand
            } else {
                demand / total_demand * available
            };
            // Credit the uptake, split by kappa (ADR-0003): the kappa share feeds
            // the free store (the growth build-permit), the (1 - kappa) share the
            // reproductive-nutrient earmark. Unlike energy there is no retention
            // buffer — nutrient has no per-tick holding cost, so the whole uptake
            // flow is split. The same route-agnostic split feeds consumption
            // nutrient (ADR-0004).
            agents[i].credit_nutrient(uptake);
            *cell_pool -= uptake;
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::NutrientAbsorbed,
                source: agents[i].id,
                target: None,
                energy_delta: uptake,
                position: Some(agents[i].position),
                target_was_carcass: false,
            });
        }
    }
    events
}

/// Metabolise: fixed costs only. Base rate + trait maintenance (including
/// mobility maintenance) + structure maintenance. Per-distance movement cost
/// is in the movement phase. Somatic maintenance is governed by kappa
/// allocation in the grow phase.
pub fn metabolise(agents: &mut [Agent], params: &WorldParameters) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;

    let exp = params.maintenance_cost_exponent;
    for agent in agents.iter_mut() {
        let cost = params.base_metabolic_rate
            + agent.traits.photosynthetic_absorption.powf(exp) * params.photo_maintenance_cost
            + agent.traits.heterotrophy.powf(exp) * params.heterotrophy_maintenance_cost
            + agent.traits.mobility.powf(exp) * params.mobility_maintenance_cost
            + agent.traits.asexual_propensity.powf(exp)
                * params.asexual_propensity_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient;

        agent.reserve -= cost;
        total_dissipated += cost;
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Metabolized,
            source: agent.id,
            target: None,
            energy_delta: cost,
            position: Some(agent.position),
            target_was_carcass: false,
        });
    }
    (events, total_dissipated)
}

/// The reserve an agent holds back to cover near-future metabolism: its per-tick
/// metabolic cost times `growth_retention_multiplier` (the same buffer the grow
/// phase mobilises surplus above). Reserve above this is the agent's surplus.
fn retention_buffer(agent: &Agent, params: &WorldParameters) -> f32 {
    let exp = params.maintenance_cost_exponent;
    let metabolic_cost = params.base_metabolic_rate
        + agent.traits.photosynthetic_absorption.powf(exp) * params.photo_maintenance_cost
        + agent.traits.heterotrophy.powf(exp) * params.heterotrophy_maintenance_cost
        + agent.traits.mobility.powf(exp) * params.mobility_maintenance_cost
        + agent.traits.asexual_propensity.powf(exp) * params.asexual_propensity_maintenance_cost
        + agent.structure * params.structure_maintenance_coefficient;
    metabolic_cost * params.growth_retention_multiplier
}

/// Form network connections (flow 5): each surplus-carrying agent builds a
/// connection to a contacted neighbour it is not already linked to, paying the
/// creation cost from reserve, up to the per-agent connection cap. Inert while the
/// network is disabled (zero cap), so genesis and existing recipes never run it.
///
/// - **Surplus-gated:** an agent builds only if it can pay the creation cost while
///   staying above its retention buffer (`reserve ≥ buffer + creation_cost`) — so
///   well-established agents become hubs and the cost never starves the builder.
/// - **Surface-contact eligibility:** the two agents are within
///   `contact_range_coefficient` on the surface (the trait-free base physical reach
///   that feeding and mate reach build on). The link then persists independent of
///   surface distance — it is never removed here, only added.
/// - **Unilateral, de-duplicated:** the builder owns the link; at most one link
///   exists between any pair (a builder skips a partner it is already linked to in
///   either direction).
/// - **Deterministic:** agents are processed in slice order; when more eligible
///   partners exist than free cap slots, the lowest partner ids are taken first.
///
/// Returns the total creation cost dissipated to heat (energy conserved: the
/// builder's reserve drop is matched by this dissipation).
pub fn form_connections(
    agents: &mut [Agent],
    connections: &mut Vec<Connection>,
    grid: &SpatialGrid,
    params: &WorldParameters,
) -> f32 {
    if params.network_connection_cap == 0 {
        return 0.0;
    }
    let cap = params.network_connection_cap as usize;
    let contact = params.contact_range_coefficient;
    let creation = params.network_creation_cost;
    let mut dissipated = 0.0_f32;

    for i in 0..agents.len() {
        let builder_id = agents[i].id;
        let mut built = connections
            .iter()
            .filter(|c| c.builder == builder_id)
            .count();
        if built >= cap {
            continue;
        }

        // Eligible partners: contacted, not already linked (either direction),
        // ordered by ascending id (the deterministic cap tie-break).
        let mut candidates: Vec<(u64, usize)> = grid
            .query_radius(agents[i].position, contact)
            .into_iter()
            .map(|idx| idx as usize)
            .filter(|&j| j != i)
            .map(|j| (agents[j].id, j))
            .filter(|&(pid, _)| {
                !connections.iter().any(|c| {
                    (c.builder == builder_id && c.partner == pid)
                        || (c.builder == pid && c.partner == builder_id)
                })
            })
            .collect();
        // Order by ascending partner id (the deterministic cap tie-break) and
        // dedupe: query_radius can return an id more than once under toroidal cell
        // wrapping, and a partner must be considered at most once per builder.
        candidates.sort_by_key(|&(pid, _)| pid);
        candidates.dedup_by_key(|&mut (pid, _)| pid);

        for (partner_id, _) in candidates {
            if built >= cap {
                break;
            }
            // Surplus gate: pay creation without dipping below the retention buffer.
            if agents[i].reserve < retention_buffer(&agents[i], params) + creation {
                break;
            }
            agents[i].reserve -= creation;
            dissipated += creation;
            connections.push(Connection {
                builder: builder_id,
                partner: partner_id,
            });
            built += 1;
        }
    }
    dissipated
}

/// Network redistribution (flow 5) — the coordinated pass that moves energy and
/// nutrient along live [`Connection`]s from surplus to deficit, between
/// consumption and reproduction in the tick. It is **inert** while the network is
/// disabled (`network_connection_cap == 0`) or no connection exists, returning no
/// events and no dissipation, so default worlds are bit-unchanged. The
/// per-currency gradient flow, transfer loss, and connection lifecycle are
/// introduced by later slices (#410 energy, #411 nutrient, #412–#413 formation
/// and persistence); this is the skeleton that threads the pass into the loop.
pub fn redistribute(
    agents: &mut [Agent],
    connections: &[Connection],
    params: &WorldParameters,
) -> (Vec<Event>, f32) {
    if params.network_connection_cap == 0 || connections.is_empty() {
        return (Vec::new(), 0.0);
    }
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    let rate = params.network_redistribution_rate;
    let efficiency = params.network_transfer_efficiency;

    for conn in connections {
        // Both endpoints must be present and live. Death cleanup (#413) removes
        // stale connections; this guard keeps the pass robust regardless.
        let (Some(a), Some(b)) = (
            agents.iter().position(|x| x.id == conn.builder),
            agents.iter().position(|x| x.id == conn.partner),
        ) else {
            continue;
        };

        // Energy flows down the reserve gradient: the richer endpoint is the
        // donor. A fraction `rate` of the gradient moves, reduced by the flat
        // cooperative efficiency on receipt; the loss dissipates to heat (flow 7).
        // The transfer is capped at the *equalising* amount — the `sent` that
        // lands donor and recipient level after the loss — so a single tick never
        // overshoots (flips) the gradient, however large the rate. This is the
        // damping the network exists to provide; an overshoot would instead make
        // a connection oscillate. The nutrient flow below is independent.
        let gradient = (agents[a].reserve - agents[b].reserve).abs();
        let (donor, recipient) = if agents[a].reserve >= agents[b].reserve {
            (a, b)
        } else {
            (b, a)
        };
        let equalising = gradient / (1.0 + efficiency);
        let sent = (rate * gradient).min(equalising);
        if sent > 0.0 {
            let received = sent * efficiency;
            agents[donor].reserve -= sent;
            agents[recipient].reserve += received;
            dissipated += sent - received;
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Redistributed,
                source: agents[donor].id,
                target: Some(agents[recipient].id),
                energy_delta: received,
                position: Some(agents[donor].position),
                target_was_carcass: false,
            });
        }

        // Nutrient redistribution (flow 5): the free store flows down its own
        // gradient, independent of the energy flow (so a producer can send energy
        // one way while receiving nutrient the other). Nutrient is matter — it is
        // *conserved*, never reduced by the transfer efficiency — so the equalising
        // cap is the lossless midpoint (gradient / 2). The reproductive-nutrient
        // earmark and bound structure are untouched; only the free store moves, and
        // it is not re-split by kappa (this is a transfer between free stores, not
        // fresh acquisition).
        let n_gradient = (agents[a].nutrient - agents[b].nutrient).abs();
        let (n_donor, n_recipient) = if agents[a].nutrient >= agents[b].nutrient {
            (a, b)
        } else {
            (b, a)
        };
        let n_sent = (rate * n_gradient).min(n_gradient / 2.0);
        if n_sent > 0.0 {
            agents[n_donor].nutrient -= n_sent;
            agents[n_recipient].nutrient += n_sent;
        }
    }
    (events, dissipated)
}

/// Grow: surplus energy (reserve above metabolic retention) is split by kappa.
/// - kappa fraction → soma: growth (reserve → structure, lossy) and wear repair
/// - (1 - kappa) fraction → repro_reserve: accumulates across ticks
pub fn grow(agents: &mut [Agent], params: &WorldParameters) -> (Vec<Event>, f32) {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;

    let exp = params.maintenance_cost_exponent;
    for agent in agents.iter_mut() {
        if agent.reserve <= 0.0 {
            continue;
        }
        // Retain enough reserve for next tick's metabolism
        let metabolic_cost = params.base_metabolic_rate
            + agent.traits.photosynthetic_absorption.powf(exp) * params.photo_maintenance_cost
            + agent.traits.heterotrophy.powf(exp) * params.heterotrophy_maintenance_cost
            + agent.traits.mobility.powf(exp) * params.mobility_maintenance_cost
            + agent.traits.asexual_propensity.powf(exp)
                * params.asexual_propensity_maintenance_cost
            + agent.structure * params.structure_maintenance_coefficient;
        let retention = metabolic_cost * params.growth_retention_multiplier;
        // Reserve above the buffer is mobilisable; of that excess only a bounded
        // fraction `reserve_mobilisation_rate` is mobilised this tick (DEB energy
        // conductance — flow 9). The remainder stays in reserve as a feast-famine
        // cushion. At rate 1.0 the whole excess is mobilised (historical no-op).
        let excess = (agent.reserve - retention).max(0.0);
        let surplus = excess * params.reserve_mobilisation_rate;
        if surplus <= 0.0 {
            continue;
        }

        let kappa = agent.traits.kappa.clamp(0.0, 1.0);
        let soma_fraction = surplus * kappa;
        let repro_fraction = surplus - soma_fraction; // (1-kappa) * surplus

        // Deduct entire surplus from reserve
        agent.reserve -= surplus;

        // Repair gets priority from soma budget: counteract accumulated wear
        let decay = params.repair_decay;
        let mut repair_energy_spent = 0.0_f32;
        if soma_fraction > 0.0 && decay > 0.0 {
            let base_repair = kappa;
            for ft in 0..crate::FUNCTIONAL_TRAIT_COUNT {
                if agent.wear[ft] <= 0.0 {
                    continue;
                }
                let effective_repair = base_repair * (-decay * agent.wear[ft]).exp();
                let repair = effective_repair.min(agent.wear[ft]);
                let cost = repair; // 1:1 energy-to-repair
                if repair_energy_spent + cost > soma_fraction {
                    let remaining = soma_fraction - repair_energy_spent;
                    let capped_repair = remaining.min(agent.wear[ft]);
                    agent.wear[ft] -= capped_repair;
                    repair_energy_spent = soma_fraction;
                    break;
                }
                agent.wear[ft] -= repair;
                repair_energy_spent += cost;
            }
        }
        total_dissipated += repair_energy_spent;

        // Remainder of soma fraction → growth
        let growth_budget = soma_fraction - repair_energy_spent;
        let efficiency = params.growth_efficiency;
        if efficiency > 0.0 && growth_budget > 0.0 {
            // Energy alone would build this much structure...
            let energy_limited = growth_budget * efficiency;
            // ...but structure is matter, so building it binds free nutrient into
            // the body at the stoichiometric demand (ADR-0003 embodiment). Growth
            // is co-limited (Liebig's law of the minimum): the structure built is
            // the smaller of what the energy affords and what the *unearmarked*
            // free store supports (free / ratio). Growth never touches the
            // reproductive-nutrient earmark, so it cannot starve reproduction
            // (#269). Energy that cannot be matched with nutrient stays in reserve
            // rather than burning, and the matched nutrient is consumed into the
            // body, released only when the structure is grazed or returned to a
            // carcass at death.
            let ratio = crate::stoichiometric_demand(&agent.traits, 1.0, params);
            let nutrient_limited = if ratio > 0.0 {
                (agent.nutrient / ratio).max(0.0)
            } else {
                f32::INFINITY
            };
            let to_structure = energy_limited.min(nutrient_limited);
            // Energy actually spent on that structure; the rest could not be
            // matched by nutrient, so it stays in reserve rather than burning.
            let energy_spent = to_structure / efficiency;
            let dissipated = energy_spent - to_structure;
            agent.structure += to_structure;
            // Growth is the only flow that increases structure: advance the
            // peak high-water mark the death threshold is measured against.
            agent.record_peak_structure();
            agent.reserve += growth_budget - energy_spent;
            // Bind the matched free nutrient into the new structure.
            agent.nutrient -= to_structure * ratio;
            total_dissipated += dissipated;

            if to_structure > 0.0 {
                events.push(Event {
                    tick: 0,
                    seq: 0,
                    kind: EventKind::Grew,
                    source: agent.id,
                    target: None,
                    energy_delta: to_structure,
                    position: Some(agent.position),
                    target_was_carcass: false,
                });
            }
        } else if growth_budget > 0.0 {
            // No growth efficiency: remaining soma is dissipated
            total_dissipated += growth_budget;
        }

        // Repro fraction → repro_reserve (accumulates across ticks)
        agent.repro_reserve += repro_fraction;
    }
    (events, total_dissipated)
}

/// Apply wear: baseline + use-dependent accumulation per functional trait.
/// Accumulation only — repair is funded from soma energy in `grow()`.
///
/// `usage` maps agent id → per-functional-trait usage amounts:
///   [0] = energy captured (autotrophy), [1] = energy drained (heterotrophy),
///   [2] = distance moved (mobility).
pub fn apply_wear(
    agents: &mut [Agent],
    params: &WorldParameters,
    usage: &std::collections::HashMap<u64, [f32; FUNCTIONAL_TRAIT_COUNT]>,
) -> Vec<Event> {
    let mut events = Vec::new();
    let baseline_rate = params.wear_rate;
    let use_rate = params.use_wear_rate;

    for agent in agents.iter_mut() {
        let mut total_wear_delta = 0.0_f32;

        let agent_usage = usage
            .get(&agent.id)
            .copied()
            .unwrap_or([0.0; FUNCTIONAL_TRAIT_COUNT]);

        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            let nominal = agent.traits.get(FUNCTIONAL_TRAIT_INDICES[ft]);
            let baseline = baseline_rate * nominal.max(0.0);
            let use_dependent = use_rate * agent_usage[ft].max(0.0);
            let accumulation = baseline + use_dependent;
            agent.wear[ft] += accumulation;
            total_wear_delta += accumulation;
        }

        if total_wear_delta > 0.0 {
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Wore,
                source: agent.id,
                target: None,
                energy_delta: total_wear_delta,
                position: Some(agent.position),
                target_was_carcass: false,
            });
        }
    }
    events
}

/// Result of drain resolution phase.
pub struct DrainResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    pub dead_agents: Vec<u64>,
    pub new_carcasses: Vec<Carcass>,
}

/// Consumption (feeding) reach for a consumer with the given effective
/// heterotrophy and structure. Feeding reach has two physical solutions, like
/// reproductive reach: move the organism (the contact-range term) or extend the
/// body through the substrate (the body-extent term — a mycelium foraging by
/// growth). `sqrt(structure)` makes the body term sublinear (a disc's radius
/// scales with the square root of its area), so reach grows with body size
/// without running away. The whole reach is modulated by `eff_heterotrophy`, so
/// the foraging body is the *heterotrophic* body: a large autotroph gains no
/// carcass reach from its bulk, while a growing heterotroph (a mycelium) does.
/// Both the living-target and carcass passes call this so they cannot drift
/// apart (they diverged before — the #303/#293 index/id drain bug).
pub(crate) fn consumption_reach(
    eff_heterotrophy: f32,
    structure: f32,
    params: &WorldParameters,
) -> f32 {
    eff_heterotrophy
        * (params.contact_range_coefficient
            + params.body_reach_coefficient * structure.max(0.0).sqrt())
}

/// Resolve drains: coordinated pass 1. For each potential target (living agent
/// or carcass), gather consumers within contact range, compute demand, apply
/// proportional split when demand exceeds supply, transfer energy with trophic
/// loss, transfer nutrient with stoichiometric mismatch. Check death thresholds
/// on drained living agents. New carcasses from kills this tick are NOT available
/// for decomposition (no re-entrant processing).
pub fn resolve_drains(
    agents: &mut [Agent],
    carcasses: &mut [Carcass],
    grid: &SpatialGrid,
    params: &WorldParameters,
    nutrient_grid: &mut crate::spatial::NutrientGrid,
) -> DrainResult {
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    let mut dead_agents: Vec<u64> = Vec::new();
    let mut new_carcasses: Vec<Carcass> = Vec::new();
    let k = params.wear_degradation_steepness;
    let extent = params.world_extent;

    // Max query radius: largest consumer consumption reach across all agents.
    // Reach now includes the structure-derived body-extent term, so the grid
    // query must use the same `consumption_reach` helper as the per-consumer
    // checks below — otherwise a large-bodied consumer's far reach would be
    // truncated to the contact-range-only query radius.
    let max_contact_range = agents
        .iter()
        .map(|a| consumption_reach(a.effective_trait_with_steepness(1, k), a.structure, params))
        .fold(0.0_f32, f32::max);

    // --- Pass over living targets ---
    // For each agent that has structure, find consumers in range. The spatial
    // grid is keyed by slice index (see `World::step`), so query results are
    // indices into `agents`, used directly below.

    // Collect per-target drain info before mutating
    struct TargetDrain {
        target_idx: usize,
        consumers: Vec<(usize, f32, f32)>, // (consumer_idx, demand, trophic_eff)
    }

    let mut living_drains: Vec<TargetDrain> = Vec::new();

    for target_idx in 0..agents.len() {
        if agents[target_idx].structure <= 0.0 {
            continue;
        }
        let target_pos = agents[target_idx].position;

        // Find consumers within their individual contact range
        let mut consumers = Vec::new();
        let nearby = grid.query_radius(target_pos, max_contact_range);
        let mut seen = std::collections::HashSet::new();
        for neighbor_id in nearby {
            if !seen.insert(neighbor_id) {
                continue;
            }
            // The spatial grid is keyed by slice index (see `World::step`), not by
            // agent id — index and id diverge as soon as any agent dies. Treat the
            // query result as the index it is.
            let consumer_idx = neighbor_id as usize;
            if consumer_idx < agents.len() {
                if consumer_idx == target_idx {
                    continue; // can't consume yourself
                }
                let eff_heterotrophy = agents[consumer_idx].effective_trait_with_steepness(1, k); // index 1 = heterotrophy
                if eff_heterotrophy <= 0.0 {
                    continue;
                }
                let consumer_contact_range =
                    consumption_reach(eff_heterotrophy, agents[consumer_idx].structure, params);
                if crate::toroidal_distance(agents[consumer_idx].position, target_pos, extent)
                    > consumer_contact_range
                {
                    continue;
                }
                // Binary-reach drain (#380): while the target is within feeding
                // reach, the consumer drains it at its effective heterotrophy each
                // tick — no warm-up, no contact-duration ramp. Predation vs grazing
                // emerges downstream from the size of this drain relative to the
                // target's structural death threshold, not from contact duration.
                let demand = eff_heterotrophy;
                let trophic_eff = crate::trophic_transfer_efficiency(
                    &agents[consumer_idx].traits,
                    &agents[target_idx].traits,
                    params,
                );
                consumers.push((consumer_idx, demand, trophic_eff));
            }
        }

        if !consumers.is_empty() {
            living_drains.push(TargetDrain {
                target_idx,
                consumers,
            });
        }
    }

    // Apply living target drains
    for drain in &living_drains {
        let available = agents[drain.target_idx].structure;
        let total_demand: f32 = drain.consumers.iter().map(|(_, d, _)| *d).sum();

        for &(consumer_idx, demand, trophic_eff) in &drain.consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let energy_gained = actual_drain * trophic_eff;
            let energy_lost = actual_drain - energy_gained;

            agents[drain.target_idx].structure -= actual_drain;
            agents[consumer_idx].reserve += energy_gained;
            dissipated += energy_lost;

            // Nutrient transfer: grazing releases only the nutrient *bound in the
            // structure removed* (ADR-0003 embodiment). The structure decrement
            // above already lowered the target's bound nutrient; that released
            // matter (actual_drain * target_ratio) is what transfers. The target's
            // free store and reproductive-nutrient earmark are never touched —
            // they stay with it until death.
            let target_ratio =
                crate::stoichiometric_demand(&agents[drain.target_idx].traits, 1.0, params);
            let nutrient_released = actual_drain * target_ratio;
            if nutrient_released > 0.0 {
                // Consumer retains up to stoichiometric demand
                let consumer_demand = crate::stoichiometric_demand(
                    &agents[consumer_idx].traits,
                    agents[consumer_idx].structure,
                    params,
                );
                let consumer_nutrient_need = consumer_demand * energy_gained;
                let retained = nutrient_released.min(consumer_nutrient_need);
                let excreted = nutrient_released - retained;

                // Credit the retained nutrient, split by kappa (ADR-0004): a
                // heterotroph funds reproduction from ingested nutrient, exactly as
                // a producer does from autotrophic pool uptake (flow 2).
                agents[consumer_idx].credit_nutrient(retained);
                // Excrete excess nutrient to the local cell at the target's position
                *nutrient_grid.at_position(agents[drain.target_idx].position) += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(agents[drain.target_idx].id),
                energy_delta: actual_drain,
                position: Some(agents[drain.target_idx].position),
                target_was_carcass: false,
            });
        }
    }

    // Check death thresholds on living agents that were drained
    for drain in &living_drains {
        let agent = &agents[drain.target_idx];
        let threshold = crate::death_threshold(&agent.traits, agent.peak_structure);
        let dies = agent.reserve <= 0.0
            || (agent.structure > 0.0 && agent.structure < threshold)
            || agent.structure <= 0.0;

        if dies && !dead_agents.contains(&agent.id) {
            dead_agents.push(agent.id);
            new_carcasses.push(Carcass {
                id: agent.id,
                position: agent.position,
                energy: agent.structure.max(0.0),
                nutrient: agent.nutrient_total(params),
                traits: agent.traits,
            });
        }
    }

    // --- Pass over carcass targets ---
    for carcass_idx in 0..carcasses.len() {
        if carcasses[carcass_idx].energy <= 0.0 {
            continue;
        }
        let carcass_pos = carcasses[carcass_idx].position;
        let nearby = grid.query_radius(carcass_pos, max_contact_range);
        let mut consumers: Vec<(usize, f32, f32)> = Vec::new(); // (idx, demand, trophic_eff)
        let mut seen = std::collections::HashSet::new();

        for neighbor_id in nearby {
            if !seen.insert(neighbor_id) {
                continue;
            }
            // Grid results are slice indices, not agent ids (see the living-target
            // pass above and `World::step`). Conflating the two made this pass find
            // no consumers once any death had reindexed `agents` — which is exactly
            // when carcasses exist, so carcasses accumulated unconsumed forever.
            let consumer_idx = neighbor_id as usize;
            if consumer_idx < agents.len() {
                // Must not be dead this tick
                if dead_agents.contains(&agents[consumer_idx].id) {
                    continue;
                }
                let eff_heterotrophy = agents[consumer_idx].effective_trait_with_steepness(1, k); // index 1 = heterotrophy
                if eff_heterotrophy <= 0.0 {
                    continue;
                }
                let consumer_contact_range =
                    consumption_reach(eff_heterotrophy, agents[consumer_idx].structure, params);
                if crate::toroidal_distance(agents[consumer_idx].position, carcass_pos, extent)
                    > consumer_contact_range
                {
                    continue;
                }
                // Binary-reach drain (#380): a carcass within feeding reach is
                // drained at the consumer's effective heterotrophy each tick — no
                // contact-duration ramp.
                let demand = eff_heterotrophy;
                let trophic_eff = crate::trophic_transfer_efficiency(
                    &agents[consumer_idx].traits,
                    &carcasses[carcass_idx].traits,
                    params,
                );
                consumers.push((consumer_idx, demand, trophic_eff));
            }
        }

        if consumers.is_empty() {
            continue;
        }

        let available = carcasses[carcass_idx].energy;
        let total_demand: f32 = consumers.iter().map(|(_, d, _)| *d).sum();

        for &(consumer_idx, demand, trophic_eff) in &consumers {
            let actual_drain = if total_demand <= available {
                demand
            } else {
                (demand / total_demand) * available
            };

            let energy_gained = actual_drain * trophic_eff;
            let energy_lost = actual_drain - energy_gained;

            carcasses[carcass_idx].energy -= actual_drain;
            agents[consumer_idx].reserve += energy_gained;
            dissipated += energy_lost;

            // Nutrient transfer from carcass
            let carcass_nutrient = carcasses[carcass_idx].nutrient;
            if available > 0.0 {
                let nutrient_fraction = actual_drain / available;
                let nutrient_transferred = carcass_nutrient * nutrient_fraction;

                let consumer_demand = crate::stoichiometric_demand(
                    &agents[consumer_idx].traits,
                    agents[consumer_idx].structure,
                    params,
                );
                let consumer_nutrient_need = consumer_demand * energy_gained;
                let retained = nutrient_transferred.min(consumer_nutrient_need);
                let excreted = nutrient_transferred - retained;

                carcasses[carcass_idx].nutrient -= nutrient_transferred;
                // Credit the retained nutrient, split by kappa (ADR-0004): the
                // detrital chain funds decomposer reproduction from carcass
                // nutrient, the same route-agnostic split as living prey and uptake.
                agents[consumer_idx].credit_nutrient(retained);
                // Excrete excess nutrient to the local cell at the carcass's position
                *nutrient_grid.at_position(carcass_pos) += excreted;
            }

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Consumed,
                source: agents[consumer_idx].id,
                target: Some(carcasses[carcass_idx].id),
                energy_delta: actual_drain,
                position: Some(carcass_pos),
                target_was_carcass: true,
            });
        }
    }

    DrainResult {
        events,
        dissipated,
        dead_agents,
        new_carcasses,
    }
}

/// Check death thresholds: reserve depletion or structure below
/// complexity-dependent threshold produces carcass.
/// Returns (events, carcasses, dissipated) — dissipated includes reserve and
/// repro_reserve energy that doesn't transfer to the carcass.
pub fn check_death_thresholds(
    agents: &mut [Agent],
    params: &WorldParameters,
) -> (Vec<Event>, Vec<Carcass>, f32) {
    let mut events = Vec::new();
    let mut carcasses = Vec::new();
    let mut dissipated = 0.0_f32;

    for agent in agents.iter_mut() {
        let threshold = crate::death_threshold(&agent.traits, agent.peak_structure);
        let dies = agent.reserve <= 0.0 || (agent.structure > 0.0 && agent.structure < threshold);

        if dies {
            let carcass_energy = agent.structure.max(0.0);
            // Energy in reserve and repro_reserve is dissipated at death
            let lost = agent.reserve.max(0.0) + agent.repro_reserve.max(0.0);
            dissipated += lost;
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Died,
                source: agent.id,
                target: None,
                energy_delta: 0.0,
                position: Some(agent.position),
                target_was_carcass: false,
            });
            carcasses.push(Carcass {
                id: agent.id,
                position: agent.position,
                energy: carcass_energy,
                nutrient: agent.nutrient_total(params),
                traits: agent.traits,
            });
            // Mark for removal by setting reserve to 0. Nutrient (free store and
            // earmark) has already been transferred to the carcass above.
            agent.reserve = 0.0;
            agent.repro_reserve = 0.0;
            agent.repro_nutrient = 0.0;
            agent.nutrient = 0.0;
            agent.repro_nutrient = 0.0;
        }
    }
    (events, carcasses, dissipated)
}

/// Result of the movement phase.
pub struct MoveResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    /// Per-agent sensing throughput: number of agents + carcasses detected.
    /// Indexed by position in the agents slice.
    pub sensing_throughput: Vec<f32>,
    /// Per-agent distance actually moved this tick (mobility use). Indexed by
    /// position in the agents slice. Movement runs before wear, so this is
    /// folded into the same tick's mobility use-wear.
    pub move_distance: Vec<f32>,
}

/// Move agents: the final repositioning phase of the tick loop, running after
/// all energy-affecting phases but before wear and the death check. Agents reposition based on
/// spatial context (nearby agents and carcasses), traits (chemotaxis, consumption,
/// scavenging, mobility), and a random walk component. Movement costs energy
/// proportional to distance.
pub fn move_agents(
    agents: &mut [Agent],
    carcasses: &[Carcass],
    grid: &SpatialGrid,
    params: &WorldParameters,
    run_seed: u64,
    tick: u64,
) -> MoveResult {
    let mut events = Vec::new();
    let mut total_dissipated = 0.0_f32;
    let mut sensing_throughput = vec![0.0_f32; agents.len()];
    let mut move_distance = vec![0.0_f32; agents.len()];
    let k = params.wear_degradation_steepness;
    let extent = params.world_extent;

    for i in 0..agents.len() {
        let eff_mobility = agents[i].effective_trait_with_steepness(2, k);
        // Sensing range derived from mobility: mobile agents perceive farther
        let eff_sensing = eff_mobility * params.sensing_range_coefficient;
        // Chemotaxis strength derived from mobility (no separate trait)
        let eff_chemotaxis = eff_mobility;
        let eff_heterotrophy = agents[i].effective_trait_with_steepness(1, k);

        if eff_mobility <= 0.0 {
            // Sessile: no locomotion this tick.
            continue;
        }

        // Query spatial grid for nearby agents within sensing range
        let mut dir_x = 0.0_f32;
        let mut dir_y = 0.0_f32;
        let mut detected_count = 0.0_f32;

        if eff_sensing > 0.0 {
            // Detect nearby living agents
            let nearby = grid.query_radius(agents[i].position, eff_sensing);
            let mut seen = std::collections::HashSet::new();
            seen.insert(i as u64);
            for neighbor_id in nearby {
                if !seen.insert(neighbor_id) {
                    continue;
                }
                let j = neighbor_id as usize;
                if j >= agents.len() {
                    continue;
                }
                let dist = crate::toroidal_distance(agents[i].position, agents[j].position, extent);
                if dist < 1e-6 {
                    detected_count += 1.0;
                    continue;
                }
                let (dx, dy) =
                    crate::toroidal_displacement(agents[i].position, agents[j].position, extent);
                // Attraction weighted by chemotaxis * heterotrophy (toward living agents)
                let weight = eff_chemotaxis * eff_heterotrophy / dist;
                dir_x += dx * weight;
                dir_y += dy * weight;
                detected_count += 1.0;
            }

            // Detect nearby carcasses
            for carcass in carcasses.iter() {
                let dist = crate::toroidal_distance(agents[i].position, carcass.position, extent);
                if dist > eff_sensing {
                    continue;
                }
                detected_count += 1.0;
                if dist < 1e-6 {
                    continue;
                }
                let (dx, dy) =
                    crate::toroidal_displacement(agents[i].position, carcass.position, extent);
                // Attraction weighted by chemotaxis * heterotrophy (toward carcasses)
                let weight = eff_chemotaxis * eff_heterotrophy / dist;
                dir_x += dx * weight;
                dir_y += dy * weight;
            }
        }

        sensing_throughput[i] = detected_count;

        // Random walk component. Keyed-stateless: a fresh local stream seeded
        // from this agent's stable id + tick + the movement phase tag, so the
        // jitter is a pure function of identity and is independent of where this
        // agent sits in the iteration order (see `keyed_rng`).
        let mut rng = crate::keyed_rng::agent_rng(
            run_seed,
            agents[i].id,
            tick,
            crate::keyed_rng::PhaseTag::Movement,
        );
        let angle: f32 = rng.random::<f32>() * std::f32::consts::TAU;
        let random_magnitude: f32 = rng.random::<f32>();
        dir_x += angle.cos() * random_magnitude;
        dir_y += angle.sin() * random_magnitude;

        // Normalize direction and scale by effective mobility
        let dir_mag = (dir_x * dir_x + dir_y * dir_y).sqrt();
        if dir_mag < 1e-6 {
            continue;
        }

        let distance = eff_mobility;
        let move_x = (dir_x / dir_mag) * distance;
        let move_y = (dir_y / dir_mag) * distance;

        // Deduct energy cost (capped at available reserve — agent cannot
        // spend energy it does not have)
        let raw_cost = distance * params.movement_cost_coefficient * agents[i].structure;
        let cost = raw_cost.min(agents[i].reserve.max(0.0));
        agents[i].reserve -= cost;
        total_dissipated += cost;

        // Update position with toroidal wrapping
        let new_pos = crate::wrap_position(
            (agents[i].position.0 + move_x, agents[i].position.1 + move_y),
            extent,
        );

        let moved = (new_pos.0 - agents[i].position.0).abs() > 1e-6
            || (new_pos.1 - agents[i].position.1).abs() > 1e-6;

        if moved {
            agents[i].position = new_pos;
            move_distance[i] = distance;
        }

        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Moved,
            source: agents[i].id,
            target: None,
            energy_delta: cost,
            position: Some(new_pos),
            target_was_carcass: false,
        });
    }

    MoveResult {
        events,
        dissipated: total_dissipated,
        sensing_throughput,
        move_distance,
    }
}

/// Result of the reproduction resolution phase.
pub struct ReproductionResult {
    pub events: Vec<Event>,
    pub dissipated: f32,
    pub offspring: Vec<Agent>,
}

/// Resolve reproduction: coordinated pass 2.
///
/// Each eligible agent first rolls against its `asexual_propensity`. On success
/// the agent reproduces alone — offspring are clones with mutation applied (no
/// crossover). On failure the agent falls through to sexual mate-finding: pairs
/// by closest trait-space distance within spatial proximity, invests energy from
/// both parents, produces offspring with uniform crossover + Gaussian mutation.
///
/// Events distinguish asexual (target=None) from sexual (target=Some(mate_id)).
pub fn resolve_reproduction(
    agents: &mut [Agent],
    dead_ids: &std::collections::HashSet<u64>,
    grid: &SpatialGrid,
    params: &WorldParameters,
    run_seed: u64,
    tick: u64,
) -> ReproductionResult {
    let mut events = Vec::new();
    let mut dissipated = 0.0_f32;
    // Offspring are collected with a world-state-derived sort key so that their
    // final ids can be assigned in a canonical order, independent of the order
    // in which parents are iterated. Each entry is `(key, agent)` where
    // `key = (k0, k1, birth_slot)`:
    //   * asexual: `(parent_id, SINGLE_AGENT_SENTINEL, slot)`
    //   * sexual:  `(min_pair_id, max_pair_id, slot)`
    // `birth_slot` is the offspring's position within a single parent/pair's
    // brood (the brood's draw order off its own local stream — deterministic).
    // Without canonical assignment, a shuffled reproduction loop would mint the
    // same offspring with different ids, diverging every downstream keyed stream.
    let mut keyed_offspring: Vec<((u64, u64, usize), Agent)> = Vec::new();
    let extent = params.world_extent;

    // Build eligible set: alive, with both reproductive earmarks above their
    // thresholds. Reproduction draws from the earmarks (repro_reserve and
    // repro_nutrient), never from the free reserve or free nutrient store, so
    // growth cannot pin an agent on either gate. The old body-support nutrient
    // gate (`nutrient >= structure * demand`) is gone.
    let eligible: std::collections::HashSet<usize> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            // The energy gate is the flat reproduction floor. #310's extra
            // "minimum viable investment" gate is gone: with the peak-relative
            // death threshold (#313), offspring are born above their own
            // threshold by construction (birth structure == peak structure), so
            // there is no longer a doomed-on-arrival brood to gate against.
            !dead_ids.contains(&a.id)
                && a.repro_reserve >= params.reproduction_energy_threshold
                && a.repro_nutrient >= params.reproduction_nutrient_threshold
        })
        .map(|(i, _)| i)
        .collect();

    // Track which agents have already reproduced this tick (at most once each).
    let mut reproduced = std::collections::HashSet::new();

    // Next agent ID: find max existing ID + 1. Offspring receive canonical ids
    // from this counter *after* being sorted by their world-state key below.
    let next_id = agents.iter().map(|a| a.id).max().unwrap_or(0) + 1;

    let sensing_coeff = params.sensing_range_coefficient;
    let dispersal_reach_coeff = params.dispersal_reach_coefficient;
    let wear_steepness = params.wear_degradation_steepness;

    // ---------- Phase A: Asexual reproduction attempts ----------
    // Each eligible agent rolls against its asexual_propensity. On success it
    // reproduces alone; on failure it will attempt sexual reproduction below.
    let eligible_sorted: Vec<usize> = {
        let mut v: Vec<usize> = eligible.iter().copied().collect();
        v.sort(); // deterministic ordering for reproducibility
        v
    };

    for &i in &eligible_sorted {
        // Keyed-stateless: a fresh local stream keyed on this agent's stable id
        // + tick + the asexual phase tag. Every asexual draw for this agent —
        // the propensity roll, the Poisson fecundity, per-offspring mutation and
        // dispersal — comes off this one stream, so the outcome is a pure
        // function of identity, independent of iteration order. A distinct
        // phase tag from the sexual path firewalls the two streams (#376).
        let parent_id = agents[i].id;
        let mut rng = crate::keyed_rng::agent_rng(
            run_seed,
            parent_id,
            tick,
            crate::keyed_rng::PhaseTag::AsexualReproduction,
        );

        let propensity = agents[i].traits.asexual_propensity.clamp(0.0, 1.0);
        if propensity <= 0.0 || rng.random::<f32>() >= propensity {
            continue; // asexual attempt failed — will try sexual below
        }

        // Asexual reproduction succeeds.
        reproduced.insert(i);

        let parent_traits = agents[i].traits;
        let parent_repro = agents[i].repro_reserve;
        let parent_repro_nutrient = agents[i].repro_nutrient.max(0.0);
        let parent_pos = agents[i].position;

        let investment = parent_repro.max(0.0);
        if investment <= 0.0 {
            continue;
        }

        // Offspring count from Poisson distribution (single parent's fecundity).
        // A zero draw is reproductive failure (world-rules flow 4): the committed
        // energy is still consumed but no offspring result.
        let fecundity = parent_traits.fecundity.max(0.1);
        let poisson = Poisson::new(fecundity as f64).unwrap();
        let offspring_count = poisson.sample(&mut rng) as usize;

        // The committed investment is always consumed from the parent.
        agents[i].repro_reserve -= investment;

        if offspring_count == 0 {
            // Reproductive failure: no offspring carry the investment, so the
            // entire committed energy dissipates. Nutrient is not donated and
            // remains with the parent.
            dissipated += investment;

            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Reproduced,
                source: parent_id,
                target: None,
                energy_delta: investment,
                position: Some(parent_pos),
                target_was_carcass: false,
            });
            continue;
        }

        let post_efficiency = investment * params.reproduction_efficiency;
        // Dispersal propagule cost: part of the reproductive budget is spent
        // building propagule structures before the remainder is divided among
        // offspring. Superlinear in the parent's dispersal trait; the spent
        // energy dissipates (energy conserved), so higher dispersal leaves less
        // to provision each offspring. Charged only here, never per tick.
        let propagule_fraction =
            crate::dispersal_propagule_cost_fraction(parent_traits.dispersal, params);
        let propagule_cost = post_efficiency * propagule_fraction;
        let offspring_total_energy = post_efficiency - propagule_cost;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Per-offspring reserve/structure split is computed per child below: the
        // structure share of the per-offspring energy is converted via
        // growth_efficiency (same lossy conversion as in-life growth), but the
        // structure built is co-limited by the offspring's donated nutrient
        // (ADR-0003 embodiment) — birth structure binds `structure * demand`, so
        // a nutrient-starved offspring is born smaller, with the unmatched
        // structure-energy staying in its reserve rather than burning.
        let struct_fraction = params.offspring_structure_fraction.clamp(0.0, 1.0);
        let structure_energy_share = energy_per_offspring * struct_fraction;

        // Nutrient donation: the parent donates its entire reproductive-nutrient
        // earmark to the offspring, divided by the realised offspring count
        // (mirroring how repro_reserve energy is divided). The free store is
        // never touched. The donation provisions both the offspring's bound
        // birth nutrient and its starting free store.
        let nutrient_donated = parent_repro_nutrient;
        let nutrient_per_offspring = nutrient_donated / offspring_count as f32;

        // Deduct the donated earmark from the parent.
        agents[i].repro_nutrient -= nutrient_donated;

        // Dispersal: sigma proportional to parent's dispersal trait
        let dispersal_radius = parent_traits.dispersal;

        for birth_slot in 0..offspring_count {
            // Asexual offspring: parent traits + mutation only (no crossover)
            let mut child_traits = parent_traits;
            for dim in 0..TraitVector::NUM_DIMS {
                if params.mutation_rate > 0.0 && rng.random::<f32>() < params.mutation_rate {
                    let normal = Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                    let mutated = (child_traits.get(dim) + normal.sample(&mut rng)).max(0.0);
                    // Clamp kappa and asexual_propensity to [0, 1]
                    let val = if dim == 3 || dim == 5 {
                        mutated.clamp(0.0, 1.0)
                    } else {
                        mutated
                    };
                    child_traits.set(dim, val);
                }
            }

            // Dispersal position
            let (dx, dy) = if dispersal_radius > 0.0 {
                let normal = Normal::new(0.0_f32, dispersal_radius).unwrap();
                (normal.sample(&mut rng), normal.sample(&mut rng))
            } else {
                (0.0, 0.0)
            };
            let pos = crate::wrap_position((parent_pos.0 + dx, parent_pos.1 + dy), extent);

            // Provision the offspring's structure, reserve, and nutrient, with
            // birth structure co-limited by the donated nutrient (ADR-0003).
            let prov = crate::provision_offspring(
                &child_traits,
                structure_energy_share,
                energy_per_offspring,
                nutrient_per_offspring,
                params,
            );
            dissipated += prov.heat;

            let child = Agent {
                // Placeholder id; the canonical id is assigned after all
                // offspring are sorted by their world-state key below.
                id: 0,
                position: pos,
                reserve: prov.reserve,
                structure: prov.structure,
                // Birth structure is the offspring's peak high-water mark, so it
                // is born above its own peak-relative death threshold (#310).
                peak_structure: prov.structure,
                nutrient: prov.free_nutrient,
                traits: child_traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 0.0,
                repro_nutrient: 0.0,
            };
            // Sort key: single parent id, sentinel high slot, brood position.
            keyed_offspring.push((
                (
                    parent_id,
                    crate::keyed_rng::SINGLE_AGENT_SENTINEL,
                    birth_slot,
                ),
                child,
            ));
        }

        // Asexual event: target is None (no mate)
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Reproduced,
            source: parent_id,
            target: None,
            energy_delta: investment,
            position: Some(parent_pos),
            target_was_carcass: false,
        });
    }

    // ---------- Phase B: Sexual reproduction (fallback for non-asexual) ----------
    // Only eligible agents that have not already reproduced asexually participate.
    let sexual_eligible: std::collections::HashSet<usize> = eligible
        .iter()
        .copied()
        .filter(|i| !reproduced.contains(i))
        .collect();

    let compatibility_distance = params.reproductive_compatibility_distance;

    // Iterate eligible agents in **stable-id** order, and resolve every
    // pairing tie on agent ids rather than slice indices (#376). Slice indices
    // are not stable identity — a shuffled `agents` slice gives the same agent a
    // different index — so keying mate-finding on indices makes the resolved
    // pair set (and the RNG streams it drives) depend on iteration order, the
    // very leak this refactor removes. Sorting by id makes the pair set a pure
    // function of world state. (This subsumes #343's index tiebreak: ids are the
    // stable key; #343's HashSet-order fix was the symptom, not the invariant.)
    let sexual_eligible_sorted: Vec<usize> = {
        let mut v: Vec<usize> = sexual_eligible.iter().copied().collect();
        v.sort_by_key(|&i| agents[i].id);
        v
    };

    // Candidate tuple carries both the (low, high) slice indices used to drive
    // the resolution, and the (low, high) ids used as the order-independent
    // tiebreak.
    let mut pair_candidates: Vec<(usize, usize, f32, u64, u64)> = Vec::new();

    for &i in &sexual_eligible_sorted {
        let agent_i = &agents[i];
        // Mate-finding reach is the spatial eligibility axis of mating. The
        // mobility term uses effective (wear-adjusted) mobility, matching how
        // perception/movement derive their range, so a worn mobile agent's reach
        // shrinks with age. The dispersal term lets a sessile broadcaster (or any
        // agent) extend reach by scattering gametes; dispersal does not wear, so
        // this contribution is age-stable. A pair forms when *either* agent's
        // reach spans the gap (the OR-style search below), so a wide-broadcasting
        // producer can pollinate a near-sessile compatible neighbour. Reach gates
        // eligibility only — it does not move offspring (placement is governed
        // independently by the dispersal trait at the birth step).
        let eff_mobility = agent_i.effective_trait_with_steepness(2, wear_steepness);
        let reach = eff_mobility * sensing_coeff + agent_i.traits.dispersal * dispersal_reach_coeff;

        let nearby = grid.query_radius(agent_i.position, reach);
        // Best mate: closest in trait space, tiebroken on the candidate's stable
        // id (not slice index), so co-located clonal mates at equal trait
        // distance resolve to the same choice regardless of iteration order.
        let mut best: Option<(usize, f32, u64)> = None;
        let mut seen = std::collections::HashSet::new();
        seen.insert(i); // exclude self

        for neighbor_id in nearby {
            let j = neighbor_id as usize;
            if !seen.insert(j) {
                continue;
            }
            if j >= agents.len() || !sexual_eligible.contains(&j) {
                continue;
            }
            let dist_spatial =
                crate::toroidal_distance(agent_i.position, agents[j].position, extent);
            if dist_spatial > reach {
                continue;
            }
            let trait_dist = agent_i.traits.distance(&agents[j].traits);
            // Check reproductive compatibility: trait distance must be within world param threshold
            if compatibility_distance > 0.0 && trait_dist > compatibility_distance {
                continue;
            }
            let j_id = agents[j].id;
            let better = match best {
                None => true,
                Some((_, best_dist, best_id)) => {
                    trait_dist < best_dist || (trait_dist == best_dist && j_id < best_id)
                }
            };
            if better {
                best = Some((j, trait_dist, j_id));
            }
        }

        if let Some((j, dist, _)) = best {
            let (a, b) = if i < j { (i, j) } else { (j, i) };
            let id_lo = agents[a].id.min(agents[b].id);
            let id_hi = agents[a].id.max(agents[b].id);
            pair_candidates.push((a, b, dist, id_lo, id_hi));
        }
    }

    // Sort by trait distance (closest pairs first), tiebroken on the (low, high)
    // agent **ids**. Clonal producers share traits, so many candidate pairs tie
    // on trait distance; keying the tiebreak on stable ids (not slice indices)
    // makes the greedy assignment below a pure function of world state, invariant
    // under any agent-slice permutation (#376).
    pair_candidates.sort_by(|a, b| {
        a.2.partial_cmp(&b.2)
            .unwrap()
            .then(a.3.cmp(&b.3))
            .then(a.4.cmp(&b.4))
    });

    // Greedily assign pairs: each agent can only reproduce once per tick
    let mut paired = std::collections::HashSet::new();
    let mut final_pairs: Vec<(usize, usize)> = Vec::new();

    for (a, b, _, _, _) in &pair_candidates {
        if !paired.contains(a) && !paired.contains(b) {
            paired.insert(*a);
            paired.insert(*b);
            final_pairs.push((*a, *b));
        }
    }

    // Resolve each sexual pair
    for (a_idx, b_idx) in &final_pairs {
        // Copy needed values before mutating
        let a_traits = agents[*a_idx].traits;
        let b_traits = agents[*b_idx].traits;
        let a_repro = agents[*a_idx].repro_reserve;
        let b_repro = agents[*b_idx].repro_reserve;
        let a_repro_nutrient = agents[*a_idx].repro_nutrient.max(0.0);
        let b_repro_nutrient = agents[*b_idx].repro_nutrient.max(0.0);
        let a_pos = agents[*a_idx].position;
        let b_pos = agents[*b_idx].position;
        let a_id = agents[*a_idx].id;
        let b_id = agents[*b_idx].id;

        // Keyed-stateless: a fresh local stream keyed on the symmetric ordered
        // pair (min id, max id) + tick + the sexual phase tag. Every sexual draw
        // for this pair — Poisson fecundity, the seed-parent coin, per-offspring
        // crossover, mutation and dispersal — comes off this one stream, so the
        // pair's outcome (including which parent seeds placement) is a pure
        // function of `(min_id, max_id, tick)`, independent of iteration order
        // and of which parent is `a` vs `b`. A distinct phase tag from the
        // asexual path firewalls the two streams (#376).
        let mut rng = crate::keyed_rng::pair_rng(
            run_seed,
            a_id,
            b_id,
            tick,
            crate::keyed_rng::PhaseTag::SexualReproduction,
        );
        let pair_key_lo = a_id.min(b_id);
        let pair_key_hi = a_id.max(b_id);

        // Investment: each parent invests their entire repro_reserve
        let invest_a = a_repro.max(0.0);
        let invest_b = b_repro.max(0.0);
        let total_investment = invest_a + invest_b;

        if total_investment <= 0.0 {
            continue;
        }

        // Offspring count from Poisson distribution. A zero draw is reproductive
        // failure (world-rules flow 4): both parents' energy is still committed
        // but no offspring result.
        let avg_fecundity = ((a_traits.fecundity + b_traits.fecundity) / 2.0).max(0.1);
        let poisson = Poisson::new(avg_fecundity as f64).unwrap();
        let offspring_count = poisson.sample(&mut rng) as usize;

        // The committed investment is always consumed from both parents.
        agents[*a_idx].repro_reserve -= invest_a;
        agents[*b_idx].repro_reserve -= invest_b;

        if offspring_count == 0 {
            // Reproductive failure: no offspring carry the investment, so the
            // entire combined committed energy dissipates. Nutrient is not
            // donated and remains with the parents.
            dissipated += total_investment;

            let (dx, dy) = crate::toroidal_displacement(a_pos, b_pos, extent);
            let mid_pos = crate::wrap_position((a_pos.0 + dx / 2.0, a_pos.1 + dy / 2.0), extent);
            events.push(Event {
                tick: 0,
                seq: 0,
                kind: EventKind::Reproduced,
                source: a_id,
                target: Some(b_id),
                energy_delta: total_investment,
                position: Some(mid_pos),
                target_was_carcass: false,
            });
            continue;
        }

        let post_efficiency = total_investment * params.reproduction_efficiency;
        // Dispersal propagule cost (sexual): keyed on the parents' average
        // dispersal trait — both parents contribute gametes to the event, so the
        // provisioning cost reflects their joint dispersal investment. This is
        // the cost framing only; offspring *placement* is governed separately by
        // the seed parent's own kernel (issue #283). Superlinear; the spent
        // energy dissipates, leaving less to provision each offspring. Charged
        // only at the event, never per tick.
        let avg_dispersal = (a_traits.dispersal + b_traits.dispersal) / 2.0;
        let propagule_fraction = crate::dispersal_propagule_cost_fraction(avg_dispersal, params);
        let propagule_cost = post_efficiency * propagule_fraction;
        let offspring_total_energy = post_efficiency - propagule_cost;
        let energy_per_offspring = offspring_total_energy / offspring_count as f32;
        let tick_dissipated = total_investment - offspring_total_energy;
        dissipated += tick_dissipated;

        // Per-offspring reserve/structure split is computed per child below
        // (same lossy-growth conversion as asexual reproduction and in-life
        // growth), with birth structure co-limited by the offspring's donated
        // nutrient (ADR-0003 embodiment).
        let struct_fraction = params.offspring_structure_fraction.clamp(0.0, 1.0);
        let structure_energy_share = energy_per_offspring * struct_fraction;

        // Nutrient donation: each parent donates its entire reproductive-nutrient
        // earmark; the combined donation is split equally among offspring
        // (mirroring how the combined repro_reserve energy is divided). Free
        // stores are untouched. Offspring receive the donation as their starting
        // free nutrient store.
        let nutrient_a = a_repro_nutrient;
        let nutrient_b = b_repro_nutrient;
        let nutrient_per_offspring = (nutrient_a + nutrient_b) / offspring_count as f32;

        // Deduct the donated earmarks from each parent.
        agents[*a_idx].repro_nutrient -= nutrient_a;
        agents[*b_idx].repro_nutrient -= nutrient_b;

        // Seed-parent placement (issue #283): offspring originate at one
        // parent's position, chosen once per event, and are scattered by *that*
        // parent's offspring-dispersal kernel. This keeps offspring anchored to
        // a real organism no matter how far apart the mates are — mate-finding
        // reach (#285) can pair spatially distant agents via gamete broadcast,
        // but that reach geometry must not leak into placement (offspring must
        // not land in the empty space between distant parents). Co-located
        // parents are unaffected: either seed sits at effectively the same spot.
        // The coin must select between the two parents by *stable id*, not by
        // which one happens to be the lower slice index (`a` vs `b`): under a
        // permuted agents slice the same pair can arrive with `a`/`b` swapped,
        // and binding the coin's branches to slice order would then flip which
        // physical parent seeds placement — an iteration-order leak (#376). Bind
        // the `true` branch to the low-id parent so the choice, like the coin
        // itself, is a pure function of `(min_id, max_id, tick)`.
        let (lo_pos, lo_disp, hi_pos, hi_disp) = if a_id < b_id {
            (a_pos, a_traits.dispersal, b_pos, b_traits.dispersal)
        } else {
            (b_pos, b_traits.dispersal, a_pos, a_traits.dispersal)
        };
        let (seed_pos, seed_dispersal) = if rng.random::<bool>() {
            (lo_pos, lo_disp)
        } else {
            (hi_pos, hi_disp)
        };

        // Dispersal: sigma = the seed parent's own dispersal trait, independent
        // of the mate's dispersal and of the inter-parent distance.
        let dispersal_radius = seed_dispersal;

        // Crossover, like the seed coin, draws each allele from one of the two
        // parents on a pair-keyed coin. Bind the `true` branch to the low-id
        // parent (not slice order) so a permuted slice can't flip which parent
        // donates a given dimension (#376).
        let (lo_traits, hi_traits) = if a_id < b_id {
            (a_traits, b_traits)
        } else {
            (b_traits, a_traits)
        };

        for birth_slot in 0..offspring_count {
            // Trait crossover: each dimension from one parent (uniform crossover)
            let mut child_traits = TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: 0.0,
                mobility: 0.0,
                kappa: 0.0,
                fecundity: 0.0,
                asexual_propensity: 0.0,
                dispersal: 0.0,
            };
            for dim in 0..TraitVector::NUM_DIMS {
                let parent_val = if rng.random::<bool>() {
                    lo_traits.get(dim)
                } else {
                    hi_traits.get(dim)
                };
                // Mutation
                let mut val =
                    if params.mutation_rate > 0.0 && rng.random::<f32>() < params.mutation_rate {
                        let normal = Normal::new(0.0_f32, params.mutation_magnitude).unwrap();
                        (parent_val + normal.sample(&mut rng)).max(0.0)
                    } else {
                        parent_val
                    };
                // Clamp kappa and asexual_propensity to [0, 1]
                if dim == 3 || dim == 5 {
                    val = val.clamp(0.0, 1.0);
                }
                child_traits.set(dim, val);
            }

            // Dispersal position
            let (dx, dy) = if dispersal_radius > 0.0 {
                let normal = Normal::new(0.0_f32, dispersal_radius).unwrap();
                (normal.sample(&mut rng), normal.sample(&mut rng))
            } else {
                (0.0, 0.0)
            };
            let pos = crate::wrap_position((seed_pos.0 + dx, seed_pos.1 + dy), extent);

            // Provision the offspring's structure, reserve, and nutrient, with
            // birth structure co-limited by the donated nutrient (ADR-0003).
            let prov = crate::provision_offspring(
                &child_traits,
                structure_energy_share,
                energy_per_offspring,
                nutrient_per_offspring,
                params,
            );
            dissipated += prov.heat;

            let child = Agent {
                // Placeholder id; the canonical id is assigned after all
                // offspring are sorted by their world-state key below.
                id: 0,
                position: pos,
                reserve: prov.reserve,
                structure: prov.structure,
                // Birth structure is the offspring's peak high-water mark, so it
                // is born above its own peak-relative death threshold (#310).
                peak_structure: prov.structure,
                nutrient: prov.free_nutrient,
                traits: child_traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 0.0, // offspring born with zero repro_reserve
                repro_nutrient: 0.0,
            };
            // Sort key: symmetric ordered pair (min id, max id), brood position.
            keyed_offspring.push(((pair_key_lo, pair_key_hi, birth_slot), child));
        }

        // Sexual event: target is Some(mate_id)
        events.push(Event {
            tick: 0,
            seq: 0,
            kind: EventKind::Reproduced,
            source: a_id,
            target: Some(b_id),
            energy_delta: total_investment,
            // The event is anchored at the seed parent — the same location the
            // offspring originate from (issue #283).
            position: Some(seed_pos),
            target_was_carcass: false,
        });
    }

    // ---------- Canonical newborn-id assignment (#376) ----------
    // Sort every offspring produced this tick (asexual + sexual) by its
    // world-state-derived key — `(asexual_parent_id | min_pair_id, sentinel |
    // max_pair_id, birth_slot)`. Then hand out sequential ids from `next_id` in
    // that sorted order. The key is a pure function of world state, so the same
    // brood gets the same ids regardless of the order parents were iterated;
    // ids stay compact, monotonic and unique. The keys collide across the two
    // phases only if an asexual parent shares a `(parent_id, SENTINEL)` with a
    // sexual pair's `(min, max)` — impossible, since SENTINEL = u64::MAX is not
    // a reachable id, so the asexual/sexual orderings never interleave on a tie.
    keyed_offspring.sort_by(|a, b| a.0.cmp(&b.0));
    let mut next_id = next_id;
    let offspring: Vec<Agent> = keyed_offspring
        .into_iter()
        .map(|(_, mut child)| {
            child.id = next_id;
            next_id += 1;
            child
        })
        .collect();

    ReproductionResult {
        events,
        dissipated,
        offspring,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FUNCTIONAL_TRAIT_COUNT, TraitVector, WorldParameters};

    fn zero_traits() -> TraitVector {
        TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.0,
            mobility: 0.0,
            kappa: 0.0,
            fecundity: 0.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        }
    }

    fn test_params() -> WorldParameters {
        WorldParameters {
            solar_flux_magnitude: 10.0,
            base_metabolic_rate: 0.1,
            sensing_range_coefficient: 10.0,
            base_trophic_efficiency: 0.5,
            trophic_distance_decay: 0.0,
            reproduction_efficiency: 0.7,
            movement_cost_coefficient: 0.0,
            reproduction_energy_threshold: 50.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 5.0,
            world_extent: 100.0,
            initial_population_size: 0,
            light_competition_radius: 1000.0,
            photo_maintenance_cost: 0.0,
            heterotrophy_maintenance_cost: 0.0,
            initial_nutrient_pool: 0.0,
            growth_efficiency: 0.0,
            wear_rate: 0.0,
            wear_degradation_steepness: 0.0,
            somatic_maintenance_cost_coefficient: 0.0,
            use_wear_rate: 0.0,
            structure_maintenance_coefficient: 0.0,
            repair_decay: 0.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            reproductive_compatibility_distance: 2.0,
            mobility_maintenance_cost: 0.0,
            maintenance_cost_exponent: 1.0,
            nutrient_grid_cell_size: 10.0,
            growth_retention_multiplier: 2.0,
            reserve_mobilisation_rate: 1.0,
            offspring_structure_fraction: 0.2,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            dispersal_reach_coefficient: 0.0,
            body_reach_coefficient: 0.0,
            network_connection_cap: 0,
            network_creation_cost: 0.0,
            network_maintenance_cost: 0.0,
            network_redistribution_rate: 0.0,
            network_transfer_efficiency: 0.0,
        }
    }

    fn make_agent(id: u64, position: (f32, f32), reserve: f32, traits: TraitVector) -> Agent {
        Agent::new(id, position, reserve, 0.0, 0.0, traits)
    }

    // --- Network formation (flow 5) ---

    #[test]
    fn form_connections_links_a_surplus_agent_to_a_contacted_neighbour() {
        // Flow 5 (#412): a surplus-carrying agent builds a connection to a
        // contacted neighbour and pays the creation cost from reserve. Building is
        // unilateral and de-duplicated — only one link forms between the pair.
        let mut params = test_params();
        params.network_connection_cap = 2;
        params.network_creation_cost = 1.0;
        params.contact_range_coefficient = 5.0;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, zero_traits()),
            make_agent(2, (1.0, 0.0), 100.0, zero_traits()),
        ];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut connections = Vec::new();

        let dissipated = form_connections(&mut agents, &mut connections, &grid, &params);

        assert_eq!(connections.len(), 1, "one link forms between the pair");
        assert_eq!(
            connections[0].builder, 1,
            "first surplus agent is the builder"
        );
        assert_eq!(connections[0].partner, 2);
        assert!(
            (dissipated - 1.0).abs() < 1e-5,
            "one creation cost dissipated"
        );
        assert!(
            (agents[0].reserve - 99.0).abs() < 1e-5,
            "builder paid the creation cost: {}",
            agents[0].reserve
        );
    }

    #[test]
    fn form_connections_requires_surplus_and_contact() {
        let mut params = test_params();
        params.network_connection_cap = 2;
        params.network_creation_cost = 1.0;
        params.contact_range_coefficient = 5.0;

        // Marginal agents (cannot pay creation above their retention buffer) → no link.
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.5, zero_traits()),
            make_agent(2, (1.0, 0.0), 0.5, zero_traits()),
        ];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut conns = Vec::new();
        form_connections(&mut agents, &mut conns, &grid, &params);
        assert!(conns.is_empty(), "no surplus → no connection");

        // Surplus agents out of contact → no link.
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, zero_traits()),
            make_agent(2, (50.0, 0.0), 100.0, zero_traits()),
        ];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut conns = Vec::new();
        form_connections(&mut agents, &mut conns, &grid, &params);
        assert!(conns.is_empty(), "out of contact → no connection");
    }

    #[test]
    fn form_connections_caps_and_breaks_ties_by_partner_id() {
        // Flow 5 (#412): when more eligible partners exist than free cap slots, the
        // lowest partner ids are taken first — a deterministic tie-break.
        let mut params = test_params();
        params.network_connection_cap = 1;
        params.network_creation_cost = 1.0;
        params.contact_range_coefficient = 5.0;
        let mut agents = vec![
            make_agent(5, (0.0, 0.0), 100.0, zero_traits()), // sole surplus builder
            make_agent(3, (1.0, 0.0), 0.0, zero_traits()),
            make_agent(2, (1.0, 1.0), 0.0, zero_traits()),
        ];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        for (i, a) in agents.iter().enumerate() {
            grid.insert(i as u64, a.position);
        }
        let mut conns = Vec::new();
        form_connections(&mut agents, &mut conns, &grid, &params);

        let from5: Vec<_> = conns.iter().filter(|c| c.builder == 5).collect();
        assert_eq!(from5.len(), 1, "cap 1 respected");
        assert_eq!(from5[0].partner, 2, "lowest partner id chosen");
    }

    #[test]
    fn formed_connection_persists_when_agents_separate() {
        // Flow 5 (#412): the wire outlasts the touch — once built, a connection
        // survives the agents drifting out of contact, and is not rebuilt/duplicated.
        let mut params = test_params();
        params.network_connection_cap = 2;
        params.network_creation_cost = 1.0;
        params.contact_range_coefficient = 5.0;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, zero_traits()),
            make_agent(2, (1.0, 0.0), 100.0, zero_traits()),
        ];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let mut conns = Vec::new();
        form_connections(&mut agents, &mut conns, &grid, &params);
        assert_eq!(conns.len(), 1);

        // The agents drift far apart; a second formation pass leaves the existing
        // link intact and forms no duplicate.
        agents[1].position = (90.0, 0.0);
        let mut grid2 = SpatialGrid::new(100.0, 10.0);
        grid2.insert(0, agents[0].position);
        grid2.insert(1, agents[1].position);
        form_connections(&mut agents, &mut conns, &grid2, &params);
        assert_eq!(conns.len(), 1, "the link persists and is not duplicated");
    }

    // --- Network redistribution (flow 5) ---

    #[test]
    fn redistribute_moves_energy_down_the_gradient_lossily() {
        // Flow 5 (#410): energy flows from the higher-reserve endpoint to the
        // lower along a connection — a bounded fraction of the gradient, reduced
        // by the flat transfer efficiency. The Redistributed event records the net
        // (received) amount; the donor's transfer loss is returned as dissipation.
        let mut params = test_params();
        params.network_connection_cap = 1;
        params.network_redistribution_rate = 0.5;
        params.network_transfer_efficiency = 0.8;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, zero_traits()),
            make_agent(2, (0.0, 0.0), 2.0, zero_traits()),
        ];
        let connections = vec![Connection {
            builder: 1,
            partner: 2,
        }];

        let (events, dissipated) = redistribute(&mut agents, &connections, &params);

        // gradient 8 → sent = 0.5·8 = 4, received = 4·0.8 = 3.2, loss = 0.8.
        assert!(
            (agents[0].reserve - 6.0).abs() < 1e-5,
            "donor reserve {}",
            agents[0].reserve
        );
        assert!(
            (agents[1].reserve - 5.2).abs() < 1e-5,
            "recipient reserve {}",
            agents[1].reserve
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Redistributed);
        assert_eq!(events[0].source, 1);
        assert_eq!(events[0].target, Some(2));
        assert!((events[0].energy_delta - 3.2).abs() < 1e-5);
        assert!((dissipated - 0.8).abs() < 1e-5);
    }

    #[test]
    fn redistribute_flows_toward_the_poorer_endpoint_regardless_of_builder() {
        // Flow 5 (#410): the donor is the higher-reserve endpoint, not the
        // builder — the flow follows the gradient. Here the builder is poorer, so
        // energy flows partner → builder.
        let mut params = test_params();
        params.network_connection_cap = 1;
        params.network_redistribution_rate = 1.0;
        params.network_transfer_efficiency = 1.0;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 1.0, zero_traits()), // builder, poorer
            make_agent(2, (0.0, 0.0), 9.0, zero_traits()), // partner, richer
        ];
        let connections = vec![Connection {
            builder: 1,
            partner: 2,
        }];

        let (events, dissipated) = redistribute(&mut agents, &connections, &params);

        // rate 1, efficiency 1 → fully equalise at the midpoint, lossless.
        assert!((agents[0].reserve - 5.0).abs() < 1e-5);
        assert!((agents[1].reserve - 5.0).abs() < 1e-5);
        assert_eq!(events[0].source, 2, "richer partner is the donor");
        assert_eq!(events[0].target, Some(1));
        assert!(dissipated.abs() < 1e-5, "lossless at efficiency 1");
    }

    #[test]
    fn redistribute_moves_free_nutrient_down_the_gradient_conserved() {
        // Flow 5 (#411): free-store nutrient flows down its own gradient, capped at
        // equalisation, and is *conserved* — the energy transfer efficiency does
        // NOT apply to nutrient (nutrient is matter, never lost to heat). With equal
        // reserves no energy moves, isolating the nutrient flow.
        let mut params = test_params();
        params.network_connection_cap = 1;
        params.network_redistribution_rate = 0.5;
        params.network_transfer_efficiency = 0.8; // energy-only; must not touch nutrient
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 5.0, zero_traits()),
            make_agent(2, (0.0, 0.0), 5.0, zero_traits()), // equal reserve → no energy flow
        ];
        agents[0].nutrient = 8.0;
        agents[1].nutrient = 0.0;
        let connections = vec![Connection {
            builder: 1,
            partner: 2,
        }];

        let (events, dissipated) = redistribute(&mut agents, &connections, &params);

        // gradient 8, rate 0.5 → sent = min(4, 8/2) = 4, equalise at 4 each, lossless.
        assert!(
            (agents[0].nutrient - 4.0).abs() < 1e-5,
            "{}",
            agents[0].nutrient
        );
        assert!(
            (agents[1].nutrient - 4.0).abs() < 1e-5,
            "{}",
            agents[1].nutrient
        );
        assert_eq!(agents[0].reserve, 5.0, "no energy moves at equal reserve");
        assert!(
            events.is_empty(),
            "no energy event when only nutrient flows"
        );
        assert!(
            dissipated.abs() < 1e-5,
            "nutrient is conserved, no dissipation"
        );
    }

    #[test]
    fn redistribute_exchanges_energy_and_nutrient_in_opposite_directions() {
        // Flow 5 (#411): the two currencies flow down their *own* gradients, so a
        // complementary pair trades both at once — the producer↔decomposer
        // carbon-for-nutrient exchange, emergent from two independent gradient
        // flows in a single tick.
        let mut params = test_params();
        params.network_connection_cap = 1;
        params.network_redistribution_rate = 0.5;
        params.network_transfer_efficiency = 1.0; // lossless for clean numbers
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, zero_traits()), // energy-rich, nutrient-poor
            make_agent(2, (0.0, 0.0), 0.0, zero_traits()),  // energy-poor, nutrient-rich
        ];
        agents[0].nutrient = 0.0;
        agents[1].nutrient = 10.0;
        let connections = vec![Connection {
            builder: 1,
            partner: 2,
        }];

        let (events, _) = redistribute(&mut agents, &connections, &params);

        // Energy flows 1 → 2 (down the reserve gradient); nutrient flows 2 → 1
        // (down the nutrient gradient). Each equalises at 5.
        assert!((agents[0].reserve - 5.0).abs() < 1e-5);
        assert!((agents[1].reserve - 5.0).abs() < 1e-5);
        assert!((agents[0].nutrient - 5.0).abs() < 1e-5);
        assert!((agents[1].nutrient - 5.0).abs() < 1e-5);
        assert_eq!(events[0].source, 1, "energy donor is the energy-rich agent");
        assert_eq!(events[0].target, Some(2));
    }

    // --- Photosynthesise ---

    #[test]
    fn photosynthesise_adds_energy_proportional_to_absorption() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].structure = 5.0; // nonzero structure required for light capture
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Photosynthesized);
        // Isolated producer: light_share=1.0, income = flux * light_share = 10 * 1.0 = 10.0
        assert!((events[0].energy_delta - 10.0).abs() < 1e-3);
        assert!((agents[0].reserve - 20.0).abs() < 1e-3);
    }

    #[test]
    fn photosynthesise_splits_light_among_colocated_producers() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        // Equal structure: equal light share
        agents[0].structure = 5.0;
        agents[1].structure = 5.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 2);
        // Each gets half the light share: income = 10 * 0.5 = 5.0
        assert!((events[0].energy_delta - 5.0).abs() < 1e-3);
        assert!((events[1].energy_delta - 5.0).abs() < 1e-3);
    }

    #[test]
    fn photosynthesise_skips_agents_without_absorption() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];
        let grid = SpatialGrid::new(100.0, 10.0);

        let events = photosynthesise(&mut agents, &grid, &params);
        assert!(events.is_empty());
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
    }

    #[test]
    fn photosynthesise_larger_producer_captures_more_light() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        // Agent 1 is 3x bigger than agent 2
        agents[0].structure = 9.0;
        agents[1].structure = 3.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        assert_eq!(events.len(), 2);
        // weight_0 = 0.5 * 9.0 = 4.5, weight_1 = 0.5 * 3.0 = 1.5, total = 6.0
        // share_0 = 4.5 / 6.0 = 0.75, share_1 = 1.5 / 6.0 = 0.25
        // income_0 = 10.0 * 0.75 = 7.5, income_1 = 10.0 * 0.25 = 2.5
        assert!(
            (events[0].energy_delta - 7.5).abs() < 1e-3,
            "larger producer should get 7.5, got {}",
            events[0].energy_delta
        );
        assert!(
            (events[1].energy_delta - 2.5).abs() < 1e-3,
            "smaller producer should get 2.5, got {}",
            events[1].energy_delta
        );
        // Total flux conserved
        let total: f32 = events.iter().map(|e| e.energy_delta).sum();
        assert!(
            (total - 10.0).abs() < 1e-3,
            "total flux should be conserved"
        );
    }

    #[test]
    fn photosynthesise_zero_structure_excluded_from_competition() {
        // A newborn (zero structure) among established producers gets nothing;
        // established producers split the full flux between themselves.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits), // newborn, structure=0
            make_agent(2, (1.0, 0.0), 10.0, traits), // established
            make_agent(3, (2.0, 0.0), 10.0, traits), // established
        ];
        agents[1].structure = 4.0;
        agents[2].structure = 4.0;
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (2.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        // Only 2 events (the two established producers)
        assert_eq!(events.len(), 2, "newborn should not photosynthesise");
        // Each established producer gets half the flux
        assert!((events[0].energy_delta - 5.0).abs() < 1e-3);
        assert!((events[1].energy_delta - 5.0).abs() < 1e-3);
        // Newborn reserve unchanged
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
    }

    #[test]
    fn photosynthesise_zero_structure_gets_zero_light() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // make_agent creates agents with structure=0.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let events = photosynthesise(&mut agents, &grid, &params);

        // With zero structure, agent should get zero light
        assert!(
            events.is_empty(),
            "agent with zero structure should produce no photosynthesis event"
        );
        assert!(
            (agents[0].reserve - 10.0).abs() < 1e-6,
            "reserve should be unchanged"
        );
    }

    // --- Absorb nutrients ---

    #[test]
    fn absorb_nutrients_splits_uptake_by_kappa() {
        // Each tick's nutrient uptake is split by kappa: the (1 - kappa) share
        // goes to the repro_nutrient earmark, the kappa share to the free store.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            kappa: 0.3,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        absorb_nutrients(&mut agents, &mut grid, &params);

        // Total uptake = effective autotrophy = 1.0.
        let total = agents[0].nutrient + agents[0].repro_nutrient;
        assert!(
            (total - 1.0).abs() < 1e-3,
            "total uptake should be 1.0, got {total}"
        );
        // kappa share (0.3) to free store, (1 - kappa) share (0.7) to earmark.
        assert!(
            (agents[0].nutrient - 0.3).abs() < 1e-3,
            "free store should get kappa share 0.3, got {}",
            agents[0].nutrient
        );
        assert!(
            (agents[0].repro_nutrient - 0.7).abs() < 1e-3,
            "earmark should get (1-kappa) share 0.7, got {}",
            agents[0].repro_nutrient
        );
    }

    #[test]
    fn absorb_nutrients_demand_equals_effective_autotrophy() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = eff_autotrophy = 0.5 (no wear degradation on fresh agent)
        let expected = 0.5;
        assert!(
            (agents[0].nutrient_total(&params) - expected).abs() < 1e-3,
            "uptake should equal effective autotrophy, got {}",
            agents[0].nutrient_total(&params)
        );
        assert!((grid.total() - (100.0 - expected)).abs() < 1e-3);
    }

    #[test]
    fn absorb_nutrients_shares_proportionally_when_demand_exceeds_pool() {
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, traits),
            make_agent(2, (1.0, 0.0), 10.0, traits),
        ];
        // Place all nutrient in the cell where agents are
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        *grid.at_position((0.0, 0.0)) = 0.1;

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 2);
        // Both have same demand, so each gets half
        let total_uptake = agents[0].nutrient_total(&params) + agents[1].nutrient_total(&params);
        assert!((total_uptake - 0.1).abs() < 1e-3);
        assert!(
            (agents[0].nutrient_total(&params) - agents[1].nutrient_total(&params)).abs() < 1e-3
        );
    }

    #[test]
    fn absorb_nutrients_is_not_contact_gated() {
        // Nutrient uptake is driven by autotrophy and local nutrient availability,
        // not by any contact/stillness gate — a freshly-placed agent absorbs on
        // its first tick. (Unviability of mobile autotrophs emerges downstream from
        // superlinear maintenance costs, not from a feeding-duration gate.)
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);
        assert_eq!(
            events.len(),
            1,
            "agent with autotrophy should absorb nutrients on its first tick"
        );
        assert!(
            agents[0].nutrient_total(&params) > 0.0,
            "nutrient uptake should be positive"
        );
    }

    #[test]
    fn absorb_nutrients_derives_uptake_from_autotrophy() {
        // Nutrient uptake is coupled to autotrophy — the same sessile infrastructure
        // that captures light also extracts nutrients.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::NutrientAbsorbed);
        // demand = effective_autotrophy = 0.5
        let expected = 0.5;
        assert!(
            (agents[0].nutrient_total(&params) - expected).abs() < 1e-3,
            "autotrophy-derived uptake expected {expected}, got {}",
            agents[0].nutrient_total(&params)
        );
    }

    #[test]
    fn absorb_nutrients_zero_autotrophy_gets_zero_uptake() {
        // Agents with zero autotrophy cannot absorb nutrients from the available pool —
        // they must obtain nutrient through consumption.
        let params = test_params();
        let traits = TraitVector {
            heterotrophy: 0.8,
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);

        let events = absorb_nutrients(&mut agents, &mut grid, &params);

        assert!(
            events.is_empty(),
            "zero-autotrophy agent should get no nutrient uptake events"
        );
        assert!(
            (agents[0].nutrient_total(&params)).abs() < 1e-6,
            "zero-autotrophy agent should have zero nutrient"
        );
        assert!(
            (grid.total() - 100.0).abs() < 1e-6,
            "pool should be unchanged when no uptake occurs"
        );
    }

    #[test]
    fn absorb_nutrients_agents_in_different_cells_see_independent_pools() {
        // Two agents in different cells: each draws from its own local cell only.
        // Place a nutrient-rich cell at one position and a nutrient-poor cell elsewhere.
        let params = test_params();
        let traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        // Agent A at (-40, 0), Agent B at (40, 0) — in different cells (cell_size=10)
        let mut agents = vec![
            make_agent(1, (-40.0, 0.0), 10.0, traits),
            make_agent(2, (40.0, 0.0), 10.0, traits),
        ];
        let mut grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        // Give only agent A's cell nutrient
        *grid.at_position((-40.0, 0.0)) = 5.0;
        // Agent B's cell has zero

        let _events = absorb_nutrients(&mut agents, &mut grid, &params);

        // Agent A should have absorbed nutrient
        assert!(
            agents[0].nutrient_total(&params) > 0.0,
            "agent in nutrient-rich cell should absorb, got {}",
            agents[0].nutrient_total(&params)
        );
        // Agent B should have absorbed nothing
        assert!(
            (agents[1].nutrient_total(&params)).abs() < 1e-6,
            "agent in nutrient-poor cell should absorb nothing, got {}",
            agents[1].nutrient_total(&params)
        );
        // Total nutrient conserved
        let total =
            grid.total() + agents[0].nutrient_total(&params) + agents[1].nutrient_total(&params);
        assert!(
            (total - 5.0).abs() < 1e-3,
            "total nutrient should be conserved, got {}",
            total
        );
    }

    // --- Metabolise ---

    #[test]
    fn metabolise_deducts_base_rate_from_reserve() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let (events, dissipated) = metabolise(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Metabolized);
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_trait_maintenance_costs() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_asexual_propensity_maintenance() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 2.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            maintenance_cost_exponent: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5^1 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_asexual_propensity_maintenance_is_superlinear() {
        // The cost is raised to maintenance_cost_exponent before scaling,
        // matching the other maintenance terms.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 2.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, _) = metabolise(&mut agents, &params);

        // cost = 0.5^2 * 2.0 = 0.5  (vs 1.0 if it were linear)
        assert!((events[0].energy_delta - 0.5).abs() < 1e-6);
    }

    #[test]
    fn metabolise_higher_asexual_propensity_costs_strictly_more() {
        // Selection gradient: more machinery, more standing cost.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 1.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let mut low = vec![make_agent(
            1,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.2,
                ..zero_traits()
            },
        )];
        let mut high = vec![make_agent(
            2,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.8,
                ..zero_traits()
            },
        )];

        let (low_events, _) = metabolise(&mut low, &params);
        let (high_events, _) = metabolise(&mut high, &params);

        assert!(high_events[0].energy_delta > low_events[0].energy_delta);
        assert!(high[0].reserve < low[0].reserve);
    }

    #[test]
    fn metabolise_charges_no_asexual_maintenance_when_cost_zero() {
        // Zero cost must leave behaviour unchanged: propensity is free.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(
            1,
            (0.0, 0.0),
            10.0,
            TraitVector {
                asexual_propensity: 0.9,
                ..zero_traits()
            },
        )];

        let (events, dissipated) = metabolise(&mut agents, &params);

        assert!((events[0].energy_delta - 0.0).abs() < 1e-6);
        assert!((agents[0].reserve - 10.0).abs() < 1e-6);
        assert!((dissipated - 0.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_structure_maintenance() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            structure_maintenance_coefficient: 0.1,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];
        agents[0].structure = 20.0;

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 20.0 * 0.1 = 2.0
        assert!((events[0].energy_delta - 2.0).abs() < 1e-6);
        assert!((agents[0].reserve - 8.0).abs() < 1e-6);
        assert!((dissipated - 2.0).abs() < 1e-6);
    }

    #[test]
    fn metabolise_charges_mobility_maintenance_even_when_stationary() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let (events, dissipated) = metabolise(&mut agents, &params);

        // cost = 0.5 * 2.0 = 1.0
        assert!((events[0].energy_delta - 1.0).abs() < 1e-6);
        assert!((agents[0].reserve - 9.0).abs() < 1e-6);
        assert!((dissipated - 1.0).abs() < 1e-6);
    }

    #[test]
    fn grow_retention_reflects_mobility_maintenance_not_movement_cost() {
        // An agent with mobility should retain reserve based on mobility_maintenance_cost
        // (trait maintenance), not movement_cost_coefficient (per-distance cost).
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 100.0, // high, but should NOT affect retention
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // retention = metabolic_cost * 2 = (0 + 0 + 0 + 1.0*1.0 + 0) * 2 = 2.0
        // surplus = reserve - retention = 10.0 - 2.0 = 8.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (_events, _dissipated) = grow(&mut agents, &params);

        // If retention used movement_cost_coefficient (100.0), retention would be 200.0
        // and surplus would be 0, so no growth would occur and reserve stays at 10.0.
        // With mobility_maintenance_cost (1.0), retention is 2.0, surplus is 8.0,
        // and with kappa=1.0 and growth_efficiency=1.0, all goes to structure.
        assert!(
            (agents[0].reserve - 2.0).abs() < 1e-6,
            "reserve should equal retention"
        );
        assert!(
            (agents[0].structure - 8.0).abs() < 1e-6,
            "surplus should become structure"
        );
    }

    #[test]
    fn grow_retention_reflects_asexual_propensity_maintenance() {
        // Grow's retention buffer must account for the asexual_propensity
        // maintenance the next metabolise tick will charge, mirroring the
        // metabolise cost exactly so growth never over-allocates surplus.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            asexual_propensity_maintenance_cost: 1.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            maintenance_cost_exponent: 1.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            asexual_propensity: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // metabolic_cost = 1.0^1 * 1.0 = 1.0; retention = 1.0 * 2 = 2.0
        // surplus = 10.0 - 2.0 = 8.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (_events, _dissipated) = grow(&mut agents, &params);

        assert!(
            (agents[0].reserve - 2.0).abs() < 1e-6,
            "reserve should equal retention"
        );
        assert!(
            (agents[0].structure - 8.0).abs() < 1e-6,
            "surplus should become structure"
        );
    }

    // --- Grow ---

    #[test]
    fn grow_consumes_free_nutrient_into_structure() {
        // Embodiment (ADR-0003): growth binds free nutrient into the body. The
        // structure built this tick is co-limited — min(what energy affords,
        // what the free store supports, free / ratio) — and building it spends
        // `built * ratio` from the free store. Energy that cannot be matched
        // with nutrient stays in reserve rather than being burned.
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 1.0,
            specification_nutrient_coefficient: 0.0, // ratio = base_nutrient_ratio = 0.1
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 1.0,
            ..zero_traits()
        }; // all surplus to soma
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 1.0; // ratio 0.1 -> supports at most 10.0 structure

        let (events, dissipated) = grow(&mut agents, &params);

        // retention = 1.0 * 2.0 = 2.0; surplus = 98.0; efficiency 1.0 would give
        // 98.0 structure, but the free store (1.0 / 0.1 = 10.0) is the binding
        // constraint, so only 10.0 structure is built.
        assert_eq!(events.len(), 1);
        assert!(
            (agents[0].structure - 10.0).abs() < 1e-3,
            "structure co-limited by free nutrient supply, got {}",
            agents[0].structure
        );
        // Building 10.0 structure binds 10.0 * 0.1 = 1.0 free nutrient — the whole
        // free store is consumed into the body.
        assert!(
            agents[0].nutrient.abs() < 1e-3,
            "growth consumes the free store into structure, got {}",
            agents[0].nutrient
        );
        // Unspent growth energy (88.0) returns to reserve on top of retention.
        assert!(
            (agents[0].reserve - 90.0).abs() < 1e-3,
            "leftover growth energy stays in reserve, got {}",
            agents[0].reserve
        );
        // No energy dissipated at efficiency 1.0.
        assert!(
            dissipated.abs() < 1e-3,
            "no dissipation at efficiency 1.0, got {dissipated}"
        );
    }

    #[test]
    fn grow_without_nutrient_builds_no_structure() {
        // With no nutrient, an energy-rich agent cannot lay down structure at all
        // (nutrient is the binding constraint). The growth energy is retained in
        // reserve rather than burned.
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        // make_agent leaves nutrient at 0.0.

        let (events, dissipated) = grow(&mut agents, &params);

        assert!(events.is_empty(), "no Grew event when nutrient-starved");
        assert!(
            agents[0].structure.abs() < 1e-6,
            "no structure built without nutrient"
        );
        // Surplus (98.0) was not burned: retention (2.0) + returned growth energy.
        assert!(
            (agents[0].reserve - 100.0).abs() < 1e-3,
            "growth energy stays in reserve, got {}",
            agents[0].reserve
        );
        assert!(
            dissipated.abs() < 1e-6,
            "nothing dissipated, got {dissipated}"
        );
    }

    #[test]
    fn grow_never_draws_the_repro_nutrient_earmark() {
        // The reproductive-nutrient earmark is off-limits to growth (ADR-0003).
        // An agent with an empty free store but a full earmark builds no
        // structure and the earmark is left untouched — growth can never starve
        // reproduction of nutrient.
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 0.0; // empty free store
        agents[0].repro_nutrient = 50.0; // ample earmark

        let (events, _dissipated) = grow(&mut agents, &params);

        assert!(events.is_empty(), "no growth when the free store is empty");
        assert!(
            agents[0].structure.abs() < 1e-6,
            "growth cannot bind the earmark, so no structure is built"
        );
        assert!(
            (agents[0].repro_nutrient - 50.0).abs() < 1e-6,
            "the reproductive-nutrient earmark is untouched, got {}",
            agents[0].repro_nutrient
        );
    }

    #[test]
    fn grow_converts_surplus_reserve_to_structure() {
        let params = WorldParameters {
            base_metabolic_rate: 1.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 1.0,
            ..zero_traits()
        }; // all surplus to soma
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (events, dissipated) = grow(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Grew);
        // retention = 1.0 * 2.0 = 2.0
        // surplus = 100.0 - 2.0 = 98.0
        // kappa=1.0: soma_fraction = 98.0, repro_fraction = 0.0
        // to_structure = 98.0 * 0.8 = 78.4
        // dissipated = 98.0 - 78.4 = 19.6
        assert!((agents[0].structure - 78.4).abs() < 1e-3);
        assert!((agents[0].reserve - 2.0).abs() < 1e-3);
        assert!((dissipated - 19.6).abs() < 1e-3);
        assert!(
            (agents[0].repro_reserve).abs() < 1e-6,
            "kappa=1 should send nothing to repro"
        );
    }

    #[test]
    fn growth_retention_multiplier_default_is_two() {
        // The retention buffer multiplier defaults to 2.0 to preserve historical behaviour.
        let params = test_params();
        assert!((params.growth_retention_multiplier - 2.0).abs() < 1e-6);
    }

    #[test]
    fn growth_retention_multiplier_scales_retention() {
        // Overriding the multiplier should change how much reserve grow retains
        // before computing surplus. With multiplier=5.0 and metabolic_cost=1.0,
        // retention=5.0 (vs. default 2.0), so surplus shrinks from 9.0 to 5.0.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 0.0,
            growth_efficiency: 1.0,
            growth_retention_multiplier: 5.0,
            offspring_structure_fraction: 0.2,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 1.0,
            ..zero_traits()
        };
        // metabolic_cost = 1.0; retention = 1.0 * 5.0 = 5.0
        // surplus = 10.0 - 5.0 = 5.0; kappa=1, growth_efficiency=1
        // -> structure = 5.0; reserve = 5.0
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here
        let (_events, _dissipated) = grow(&mut agents, &params);
        assert!(
            (agents[0].reserve - 5.0).abs() < 1e-6,
            "reserve should equal retention (5.0) under multiplier=5.0"
        );
        assert!(
            (agents[0].structure - 5.0).abs() < 1e-6,
            "surplus (5.0) should become structure"
        );
    }

    #[test]
    fn reserve_mobilisation_rate_leaves_a_cushion_above_retention() {
        // Flow 9: at a reserve mobilisation rate < 1.0, grow mobilises only
        // `rate * (reserve - retention)` per tick instead of the whole excess,
        // so reserve is drawn down to retention + (1 - rate) * excess rather than
        // all the way to the bare retention buffer. This is DEB's energy
        // conductance: a large reserve mobilises a fraction per tick, never all
        // at once. metabolic_cost = 1.0, retention = 2.0 (default multiplier),
        // reserve = 12.0, excess = 10.0. At rate 0.25, mobilised = 2.5, so
        // reserve ends at 12.0 - 2.5 = 9.5 (a 7.5 cushion above retention).
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 0.0,
            growth_efficiency: 0.0, // surplus → repro_reserve only; reserve drawdown is what matters
            reserve_mobilisation_rate: 0.25,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.0, // all mobilised surplus → repro_reserve, none to growth/repair
            ..zero_traits()
        };
        // metabolic_cost = 1.0; retention = 1.0 * 2.0 = 2.0; excess = 12.0 - 2.0 = 10.0
        // mobilised = 0.25 * 10.0 = 2.5; reserve ends at 12.0 - 2.5 = 9.5
        let mut agents = vec![make_agent(1, (0.0, 0.0), 12.0, traits)];
        let (_events, _dissipated) = grow(&mut agents, &params);
        assert!(
            (agents[0].reserve - 9.5).abs() < 1e-5,
            "reserve should keep a cushion above retention (expected 9.5, got {})",
            agents[0].reserve
        );
        assert!(
            (agents[0].repro_reserve - 2.5).abs() < 1e-5,
            "only the mobilised flow (2.5) is split by kappa into repro_reserve, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn reserve_mobilisation_rate_default_is_one_and_liquidates_whole_excess() {
        // Default f = 1.0 reproduces the historical one-tick liquidation: the
        // entire above-buffer excess is mobilised, leaving reserve at the bare
        // retention buffer. metabolic_cost = 1.0, retention = 2.0, reserve = 12.0
        // -> whole excess (10.0) mobilised, reserve ends at 2.0.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            mobility_maintenance_cost: 1.0,
            movement_cost_coefficient: 0.0,
            growth_efficiency: 0.0,
            ..test_params()
        };
        assert!(
            (params.reserve_mobilisation_rate - 1.0).abs() < 1e-6,
            "default reserve mobilisation rate must be 1.0 (no-op)"
        );
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 12.0, traits)];
        let (_events, _dissipated) = grow(&mut agents, &params);
        assert!(
            (agents[0].reserve - 2.0).abs() < 1e-5,
            "at f=1.0 reserve is drawn down to the bare retention buffer (2.0), got {}",
            agents[0].reserve
        );
    }

    #[test]
    fn grow_does_nothing_when_reserve_below_retention() {
        let params = WorldParameters {
            base_metabolic_rate: 100.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let (events, dissipated) = grow(&mut agents, &params);

        assert!(events.is_empty());
        assert!((agents[0].structure).abs() < 1e-6);
        assert!((dissipated).abs() < 1e-6);
    }

    #[test]
    fn grow_does_nothing_when_efficiency_is_zero() {
        let params = WorldParameters {
            growth_efficiency: 0.0,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, _) = grow(&mut agents, &params);
        assert!(events.is_empty());
    }

    #[test]
    fn grow_kappa_splits_surplus_between_soma_and_repro() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0, // perfect conversion for easy math
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 0.6,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].nutrient = 1_000.0; // ample: growth is energy-limited here

        let (_events, _dissipated) = grow(&mut agents, &params);

        // surplus = 100.0, kappa=0.6
        // soma = 60.0 -> all to structure (efficiency=1.0, dissipated=0)
        // repro = 40.0 -> to repro_reserve
        assert!(
            (agents[0].structure - 60.0).abs() < 1e-3,
            "kappa fraction should go to structure, got {}",
            agents[0].structure
        );
        assert!(
            (agents[0].repro_reserve - 40.0).abs() < 1e-3,
            "1-kappa fraction should go to repro_reserve, got {}",
            agents[0].repro_reserve
        );
        assert!(
            (agents[0].reserve).abs() < 1e-3,
            "all surplus should be consumed, got {}",
            agents[0].reserve
        );
    }

    #[test]
    fn grow_zero_kappa_sends_all_surplus_to_repro_reserve() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 0.8,
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 50.0, traits)];

        let (_events, _dissipated) = grow(&mut agents, &params);

        // kappa=0: everything to repro
        assert!(
            (agents[0].structure).abs() < 1e-6,
            "zero kappa should not grow"
        );
        assert!(
            (agents[0].repro_reserve - 50.0).abs() < 1e-3,
            "all surplus to repro_reserve, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn grow_repro_reserve_accumulates_across_ticks() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            kappa: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 20.0, traits)];

        // First tick: surplus=20, repro=10
        grow(&mut agents, &params);
        let after_first = agents[0].repro_reserve;
        assert!((after_first - 10.0).abs() < 1e-3);

        // Give more reserve for second tick
        agents[0].reserve = 20.0;
        grow(&mut agents, &params);
        // Should accumulate: 10.0 + 10.0 = 20.0
        assert!(
            (agents[0].repro_reserve - 20.0).abs() < 1e-3,
            "repro_reserve should accumulate, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn reproduction_draws_from_repro_reserve_not_reserve() {
        let params = WorldParameters {
            reproduction_efficiency: 1.0,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        agents[0].repro_reserve = 20.0;
        agents[0].repro_nutrient = 20.0;
        agents[1].repro_reserve = 20.0;
        agents[1].repro_nutrient = 20.0;
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(!result.offspring.is_empty(), "should reproduce");
        // Reserve should be unchanged
        assert!(
            (agents[0].reserve - 50.0).abs() < 1e-3,
            "reserve should be untouched, got {}",
            agents[0].reserve
        );
        // Repro_reserve should be zero
        assert!(
            agents[0].repro_reserve.abs() < 1e-3,
            "repro_reserve should be spent, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn offspring_born_with_zero_repro_reserve() {
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        for child in &result.offspring {
            assert!(
                (child.repro_reserve).abs() < 1e-6,
                "offspring should have zero repro_reserve, got {}",
                child.repro_reserve
            );
        }
    }

    // #310 end-to-end: the example6 decomposer (mobility variant) funded past the
    // flat reproduction floor produces a brood that SURVIVES the same-tick death
    // check — refuting "births=1, deaths=1, offspring dies the tick it is born".
    // Wires resolve_reproduction -> check_death_thresholds exactly as
    // World::step does. With the peak-relative death threshold (#313), offspring
    // are born above their own threshold by construction (birth structure ==
    // peak structure), so no extra investment gate is needed.
    #[test]
    fn example6_decomposer_offspring_survives_its_birth_tick() {
        let params = WorldParameters {
            growth_efficiency: 0.3,
            reproduction_efficiency: 0.7,
            offspring_structure_fraction: 0.2,
            reproduction_energy_threshold: 15.0,
            reproduction_nutrient_threshold: 1.0,
            base_nutrient_ratio: 0.1,
            specification_nutrient_coefficient: 0.2,
            ..test_params()
        };
        // The example6 decomposer with the issue's mobility=0.2 lift: a fragile,
        // low-fecundity generalist (death threshold ~2.14).
        let decomposer = TraitVector {
            photosynthetic_absorption: 0.45,
            heterotrophy: 0.5,
            mobility: 0.2,
            kappa: 0.5,
            fecundity: 0.2,
            asexual_propensity: 1.0,
            dispersal: 0.1,
        };
        // Low fecundity means most reproduction draws are empty; iterate seeds to
        // find the first realised brood, then assert every newborn survives. Fund
        // generously past the flat floor so even an above-expected brood clears —
        // the point under test is that newborns survive the death check, not the
        // funding level.
        let mut produced = false;
        for seed in 0..200u64 {
            let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, decomposer)];
            agents[0].structure = 10.0; // an established, well-grown parent
            agents[0].repro_reserve = 100.0;
            agents[0].repro_nutrient = 100.0;
            agents[0].nutrient = 100.0;

            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));

            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, seed, 0);
            if result.offspring.is_empty() {
                continue;
            }
            produced = true;
            let mut offspring = result.offspring;
            let structures: Vec<f32> = offspring.iter().map(|o| o.structure).collect();
            let (_events, carcasses, _diss) = check_death_thresholds(&mut offspring, &params);
            assert!(
                carcasses.is_empty(),
                "decomposer offspring must survive its birth tick (seed {seed}): \
                 {} of {} died; structures {:?}",
                carcasses.len(),
                structures.len(),
                structures
            );
            break;
        }
        assert!(
            produced,
            "decomposer should produce a brood within 200 seeds"
        );
    }

    // WR-16: "Offspring are born with zero wear." Parents accumulate wear over
    // their lifetime; a newborn must start its own wear accumulator at zero on
    // every functional trait, regardless of how worn the parent(s) are.

    #[test]
    fn asexual_offspring_born_with_zero_wear() {
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0, // force asexual path
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        // Both parents are heavily worn on every functional trait.
        for agent in agents.iter_mut() {
            agent.wear = [0.9; FUNCTIONAL_TRAIT_COUNT];
            agent.repro_reserve = 15.0;
            agent.repro_nutrient = 15.0;
            agent.nutrient = 10.0;
        }

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            !result.offspring.is_empty(),
            "asexual parents should reproduce"
        );
        for child in &result.offspring {
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                assert_eq!(
                    child.wear[ft], 0.0,
                    "asexual offspring must be born with zero wear on trait {ft}, got {}",
                    child.wear[ft]
                );
            }
        }
    }

    #[test]
    fn sexual_offspring_born_with_zero_wear() {
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 4.0,          // high mean so the Poisson draw yields offspring
            asexual_propensity: 0.0, // force sexual path
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 50.0, traits),
            make_agent(2, (1.0, 0.0), 50.0, traits),
        ];
        // Both parents are heavily worn on every functional trait.
        for agent in agents.iter_mut() {
            agent.wear = [0.9; FUNCTIONAL_TRAIT_COUNT];
            agent.repro_reserve = 15.0;
            agent.repro_nutrient = 15.0;
            agent.nutrient = 10.0;
        }

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // Confirm we actually exercised the sexual path (target = mate id).
        assert!(
            result
                .events
                .iter()
                .any(|e| e.kind == EventKind::Reproduced && e.target.is_some()),
            "expected a sexual reproduction event with a mate target"
        );
        assert!(
            !result.offspring.is_empty(),
            "sexual pair should produce offspring"
        );
        for child in &result.offspring {
            for ft in 0..FUNCTIONAL_TRAIT_COUNT {
                assert_eq!(
                    child.wear[ft], 0.0,
                    "sexual offspring must be born with zero wear on trait {ft}, got {}",
                    child.wear[ft]
                );
            }
        }
    }

    // --- Apply wear ---

    #[test]
    fn apply_wear_accumulates_baseline_wear() {
        let params = WorldParameters {
            wear_rate: 0.1,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let no_usage = std::collections::HashMap::new();
        let events = apply_wear(&mut agents, &params, &no_usage);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Wore);
        // ft 0 (photo): 0.1 * 0.5 = 0.05
        assert!((agents[0].wear[0] - 0.05).abs() < 1e-6);
    }

    #[test]
    fn apply_wear_accumulates_without_repair() {
        let params = WorldParameters {
            wear_rate: 0.1,
            repair_decay: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.7,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];
        agents[0].wear[0] = 1.0; // pre-existing wear

        let no_usage = std::collections::HashMap::new();
        let _events = apply_wear(&mut agents, &params, &no_usage);

        // Accumulation only: 0.1 * 0.5 = 0.05, wear goes to 1.05
        assert!(
            (agents[0].wear[0] - 1.05).abs() < 1e-6,
            "apply_wear should only accumulate (repair is in grow), got {}",
            agents[0].wear[0]
        );
    }

    #[test]
    fn grow_repairs_wear_from_soma_budget() {
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            growth_efficiency: 1.0,
            repair_decay: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.7,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].wear[0] = 1.0;

        let (_events, _dissipated) = grow(&mut agents, &params);

        assert!(
            agents[0].wear[0] < 1.0,
            "grow should repair wear from soma budget, got {}",
            agents[0].wear[0]
        );
        assert!(agents[0].wear[0] > 0.0, "wear should still be positive");
    }

    #[test]
    fn apply_wear_no_event_when_all_traits_zero() {
        let params = WorldParameters {
            wear_rate: 0.1,
            ..test_params()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, zero_traits())];

        let no_usage = std::collections::HashMap::new();
        let events = apply_wear(&mut agents, &params, &no_usage);
        assert!(events.is_empty());
    }

    // --- Use-dependent wear ---

    #[test]
    fn use_wear_autotrophy_increases_with_photosynthesis() {
        // An agent that photosynthesises should accumulate extra autotrophy wear
        // proportional to energy captured, on top of baseline wear.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        // No usage: only baseline wear
        let no_usage = std::collections::HashMap::new();
        let _events_baseline = apply_wear(&mut agents, &params, &no_usage);
        let baseline_wear = agents[0].wear[0];
        assert!(baseline_wear > 0.0, "baseline wear should accumulate");

        // Reset and apply with photosynthesis usage
        agents[0].wear = [0.0; FUNCTIONAL_TRAIT_COUNT];
        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [5.0_f32, 0.0, 0.0]); // 5.0 energy captured via photosynthesis
        let _events_use = apply_wear(&mut agents, &params, &usage);
        let use_wear = agents[0].wear[0];

        // Use-dependent wear should exceed baseline alone
        assert!(
            use_wear > baseline_wear,
            "use-dependent wear ({use_wear}) should exceed baseline ({baseline_wear})"
        );
        // Extra wear = use_wear_rate * energy_captured = 0.05 * 5.0 = 0.25
        let expected_extra = 0.05 * 5.0;
        let actual_extra = use_wear - baseline_wear;
        assert!(
            (actual_extra - expected_extra).abs() < 1e-6,
            "extra wear should be {expected_extra}, got {actual_extra}"
        );
    }

    #[test]
    fn use_wear_heterotrophy_increases_with_consumption() {
        // An agent that consumes should accumulate extra heterotrophy wear
        // proportional to energy drained.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            heterotrophy: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        // With heterotrophy usage
        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [0.0_f32, 3.0, 0.0]); // 3.0 energy drained
        let _events = apply_wear(&mut agents, &params, &usage);

        // Baseline: 0.1 * 0.8 = 0.08
        // Use-dependent: 0.05 * 3.0 = 0.15
        // Total heterotrophy wear (index 1): 0.08 + 0.15 = 0.23
        let expected = 0.08 + 0.15;
        assert!(
            (agents[0].wear[1] - expected).abs() < 1e-6,
            "heterotrophy wear should be {expected}, got {}",
            agents[0].wear[1]
        );
    }

    #[test]
    fn use_wear_mobility_increases_with_distance_moved() {
        // An agent that moves should accumulate extra mobility wear
        // proportional to distance traveled.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            mobility: 0.6,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let mut usage = std::collections::HashMap::new();
        usage.insert(1_u64, [0.0_f32, 0.0, 2.0]); // 2.0 distance moved
        let _events = apply_wear(&mut agents, &params, &usage);

        // Baseline: 0.1 * 0.6 = 0.06
        // Use-dependent: 0.05 * 2.0 = 0.10
        // Total mobility wear (index 2): 0.06 + 0.10 = 0.16
        let expected = 0.06 + 0.10;
        assert!(
            (agents[0].wear[2] - expected).abs() < 1e-6,
            "mobility wear should be {expected}, got {}",
            agents[0].wear[2]
        );
    }

    #[test]
    fn use_wear_zero_usage_equals_baseline_only() {
        // An agent with no usage should only have baseline wear.
        let params = WorldParameters {
            wear_rate: 0.1,
            use_wear_rate: 0.05,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            heterotrophy: 0.3,
            mobility: 0.4,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 10.0, traits)];

        let no_usage = std::collections::HashMap::new();
        let _events = apply_wear(&mut agents, &params, &no_usage);

        // Only baseline wear: rate * nominal for each trait
        assert!((agents[0].wear[0] - 0.05).abs() < 1e-6); // 0.1 * 0.5
        assert!((agents[0].wear[1] - 0.03).abs() < 1e-6); // 0.1 * 0.3
        assert!((agents[0].wear[2] - 0.04).abs() < 1e-6); // 0.1 * 0.4
    }

    // --- Check death thresholds ---

    #[test]
    fn check_death_kills_agent_with_zero_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 0.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert_eq!(carcasses[0].id, 1);
    }

    #[test]
    fn check_death_kills_agent_with_negative_reserve() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), -5.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(carcasses.len(), 1);
    }

    #[test]
    fn check_death_kills_agent_with_structure_below_threshold() {
        let params = test_params();
        // Generalist traits spread across many dimensions -> high threshold
        let traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.1,
            mobility: 0.1,
            kappa: 0.1,
            fecundity: 0.1,
            asexual_propensity: 0.1,
            dispersal: 0.1,
        };
        // The threshold is a fraction of *peak* structure: an agent that grew to
        // a peak of 10.0 and was then worn below its peak-relative threshold dies.
        let peak = 10.0;
        let threshold = crate::death_threshold(&traits, peak);
        assert!(threshold > 0.0, "generalist should have nonzero threshold");

        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        agents[0].peak_structure = peak;
        agents[0].structure = threshold * 0.5; // below threshold

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, EventKind::Died);
        assert_eq!(carcasses.len(), 1);
        assert!((carcasses[0].energy - threshold * 0.5).abs() < 1e-6);
    }

    #[test]
    fn check_death_preserves_healthy_agents() {
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, zero_traits())];

        let (events, carcasses, _dissipated) = check_death_thresholds(&mut agents, &params);

        assert!(events.is_empty());
        assert!(carcasses.is_empty());
        assert!((agents[0].reserve - 100.0).abs() < 1e-6);
    }

    #[test]
    fn check_death_carcass_inherits_free_earmark_and_bound_nutrient() {
        // Death converts the whole agent into a carcass: carcass.nutrient =
        // free store + reproductive earmark + nutrient bound in structure
        // (structure * demand). (ADR-0003 embodiment.)
        let params = test_params();
        let mut agents = vec![make_agent(1, (0.0, 0.0), 0.0, zero_traits())];
        agents[0].nutrient = 5.0; // free store
        agents[0].repro_nutrient = 2.0; // earmark
        agents[0].structure = 10.0; // bound = 10.0 * 0.1 = 1.0 (zero traits)

        let (_, carcasses, _) = check_death_thresholds(&mut agents, &params);

        assert_eq!(carcasses.len(), 1);
        // free(5.0) + earmark(2.0) + bound(1.0) = 8.0
        assert!(
            (carcasses[0].nutrient - 8.0).abs() < 1e-6,
            "carcass inherits free + earmark + bound, got {}",
            carcasses[0].nutrient
        );
        assert!((carcasses[0].energy - 10.0).abs() < 1e-6);
    }

    #[test]
    fn carcass_nutrient_scales_with_body_size_at_death() {
        // A larger body binds more nutrient, so it leaves a more nutrient-rich
        // carcass — even with identical free store and earmark.
        let params = test_params();
        let make = |structure: f32| {
            let mut a = make_agent(1, (0.0, 0.0), 0.0, zero_traits());
            a.nutrient = 1.0;
            a.structure = structure;
            a
        };
        let mut small = vec![make(2.0)];
        let mut large = vec![make(20.0)];

        let (_, small_carcasses, _) = check_death_thresholds(&mut small, &params);
        let (_, large_carcasses, _) = check_death_thresholds(&mut large, &params);

        assert!(
            large_carcasses[0].nutrient > small_carcasses[0].nutrient,
            "bigger body -> more bound nutrient -> richer carcass: small={}, large={}",
            small_carcasses[0].nutrient,
            large_carcasses[0].nutrient
        );
        // small: free(1.0) + bound(2.0 * 0.1 = 0.2) = 1.2
        // large: free(1.0) + bound(20.0 * 0.1 = 2.0) = 3.0
        assert!((small_carcasses[0].nutrient - 1.2).abs() < 1e-6);
        assert!((large_carcasses[0].nutrient - 3.0).abs() < 1e-6);
    }

    // --- Resolve drains ---

    #[test]
    fn drain_single_consumer_drains_living_target_structure() {
        // Consumer with consumption_rate near target with structure.
        // Consumer drains target's structure; consumer gains energy (with trophic loss).
        let params = test_params(); // consumption_efficiency = 0.5, contact_radius = 5.0
        let consumer_traits = TraitVector {
            heterotrophy: 0.4,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Consumer at (0,0), target at (1,0) — within contact_radius=5.0
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 20.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Consumer demand = effective consumption_rate = 0.4
        // Target has 20.0 structure, demand < supply, so full drain of 0.4
        // Target structure: 20.0 - 0.4 = 19.6
        // Consumer reserve: 10.0 + 0.4 * 0.5 = 10.2 (trophic efficiency)
        // Dissipated: 0.4 * 0.5 = 0.2
        assert!(
            (agents[1].structure - 19.6).abs() < 1e-3,
            "target structure should be 19.6, got {}",
            agents[1].structure
        );
        assert!(
            (agents[0].reserve - 10.2).abs() < 1e-3,
            "consumer reserve should be 10.2, got {}",
            agents[0].reserve
        );
        assert!(
            (result.dissipated - 0.2).abs() < 1e-3,
            "dissipated should be 0.2, got {}",
            result.dissipated
        );
        assert!(result.dead_agents.is_empty());
        assert!(result.new_carcasses.is_empty());
    }

    #[test]
    fn drain_proportional_split_two_consumers() {
        // Two consumers target the same agent with demands 3.0 and 1.0.
        // Target has 2.0 available structure.
        // They receive 1.5 and 0.5 respectively (proportional to demand).
        let params = test_params(); // consumption_efficiency = 0.5, contact_radius = 5.0
        let consumer_a_traits = TraitVector {
            heterotrophy: 3.0,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // All within contact_radius=5.0
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_a_traits), // demand 3.0
            make_agent(2, (1.0, 0.0), 10.0, consumer_b_traits), // demand 1.0
            make_agent(3, (0.5, 0.0), 10.0, target_traits),     // target
        ];
        agents[2].structure = 2.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Total demand = 4.0, available = 2.0
        // Consumer A gets 3.0/4.0 * 2.0 = 1.5
        // Consumer B gets 1.0/4.0 * 2.0 = 0.5
        // Consumer A reserve: 10.0 + 1.5 * 0.5 = 10.75
        // Consumer B reserve: 10.0 + 0.5 * 0.5 = 10.25
        // Target structure: 2.0 - 2.0 = 0.0
        assert!(
            (agents[0].reserve - 10.75).abs() < 1e-3,
            "consumer A reserve should be 10.75, got {}",
            agents[0].reserve
        );
        assert!(
            (agents[1].reserve - 10.25).abs() < 1e-3,
            "consumer B reserve should be 10.25, got {}",
            agents[1].reserve
        );
        assert!(
            agents[2].structure.abs() < 1e-3,
            "target structure should be 0.0, got {}",
            agents[2].structure
        );
        // Dissipated: 1.5 * 0.5 + 0.5 * 0.5 = 1.0
        assert!(
            (result.dissipated - 1.0).abs() < 1e-3,
            "dissipated should be 1.0, got {}",
            result.dissipated
        );
    }

    #[test]
    fn drain_death_marking_produces_carcass() {
        // Consumer drains target below death threshold -> target dies, carcass produced.
        let params = test_params();
        // Target is a generalist with high death threshold
        let generalist_traits = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.1,
            mobility: 0.1,
            kappa: 0.1,
            fecundity: 0.1,
            asexual_propensity: 0.1,
            dispersal: 0.1,
        };
        let consumer_traits = TraitVector {
            heterotrophy: 5.0, // high demand to drain target
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, generalist_traits),
        ];
        // The target grew to a peak of 10.0; its peak-relative threshold is a
        // fraction of that. Give it structure just above threshold, so draining
        // pushes it below and it dies.
        let peak = 10.0;
        let threshold = crate::death_threshold(&generalist_traits, peak);
        assert!(threshold > 0.0, "generalist should have nonzero threshold");
        agents[1].peak_structure = peak;
        agents[1].structure = threshold * 1.5;
        agents[1].nutrient = 3.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Consumer demand = 5.0, target structure = threshold * 1.5
        // If demand > structure, drain is capped at structure
        // After drain, structure should be below threshold -> death
        assert!(!result.dead_agents.is_empty(), "target should be dead");
        assert_eq!(result.dead_agents[0], 2);
        assert_eq!(result.new_carcasses.len(), 1);
        assert_eq!(result.new_carcasses[0].id, 2);
    }

    #[test]
    fn drain_no_reentrant_processing_new_carcass_not_decomposed() {
        // When a living target is killed this tick, the resulting carcass is NOT
        // available for decomposition in the same tick.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 100.0, // high heterotrophy drains both living and carcass
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 5.0;

        // No pre-existing carcasses
        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Target should be killed (drained to 0)
        assert!(!result.dead_agents.is_empty(), "target should be dead");
        assert_eq!(result.new_carcasses.len(), 1);

        // The new carcass was NOT available for scavenging.
        // Consumer only got energy from the living-target drain, not from
        // decomposing the new carcass. The carcass energy is in new_carcasses,
        // not consumed further.
        // Consumer reserve = 10.0 + 5.0 * 0.5 = 12.5 (only from living drain)
        assert!(
            (agents[0].reserve - 12.5).abs() < 1e-3,
            "consumer should only get energy from living drain, got {}",
            agents[0].reserve
        );
    }

    #[test]
    fn drain_heterotrophy_drains_both_living_and_carcass_same_tick() {
        // An agent with heterotrophy can drain a living target AND a carcass
        // in the same tick — the same trait drives both, target state determines
        // which efficiency applies.
        let params = test_params(); // consumption_efficiency=0.5, decomposition_efficiency=0.5
        let heterotroph_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, heterotroph_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 20.0;

        // Pre-existing carcass at nearby position
        let mut carcasses = vec![Carcass {
            id: 99,
            position: (2.0, 0.0), // within contact_radius=5.0
            energy: 10.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // From living target: drain 2.0, gain 2.0 * 0.5 = 1.0 (consumption_efficiency)
        // From carcass: drain 2.0, gain 2.0 * 0.5 = 1.0 (decomposition_efficiency)
        // Consumer reserve: 10.0 + 1.0 + 1.0 = 12.0
        assert!(
            (agents[0].reserve - 12.0).abs() < 1e-3,
            "consumer should gain from both living and carcass, got {}",
            agents[0].reserve
        );
        // Living target structure: 20.0 - 2.0 = 18.0
        assert!(
            (agents[1].structure - 18.0).abs() < 1e-3,
            "target structure should be 18.0, got {}",
            agents[1].structure
        );
        // Carcass energy: 10.0 - 2.0 = 8.0
        assert!(
            (carcasses[0].energy - 8.0).abs() < 1e-3,
            "carcass energy should be 8.0, got {}",
            carcasses[0].energy
        );
        // Events should include both living and carcass consumption
        assert!(
            result.events.len() >= 2,
            "should have events for both consumption types"
        );
        // The raw interaction fact: which Consumed events drained a carcass.
        let living = result
            .events
            .iter()
            .find(|e| e.kind == EventKind::Consumed && e.target == Some(2))
            .expect("a Consumed event targeting the living agent");
        assert!(
            !living.target_was_carcass,
            "draining a living agent is not decomposition"
        );
        let carcass = result
            .events
            .iter()
            .find(|e| e.kind == EventKind::Consumed && e.target == Some(99))
            .expect("a Consumed event targeting the carcass");
        assert!(
            carcass.target_was_carcass,
            "draining a carcass is decomposition"
        );
    }

    // --- Move agents ---

    #[test]
    fn move_direction_attracted_to_nearby_living_agent_by_heterotrophy() {
        // Agent with heterotrophy and chemotaxis should move toward a nearby living agent.
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0; // isolate direction test
        let mover_traits = TraitVector {
            heterotrophy: 0.5,
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        // Mover at origin, target at (20, 0)
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, mover_traits),
            make_agent(1, (20.0, 0.0), 100.0, target_traits),
        ];
        agents[1].structure = 10.0;
        let carcasses = vec![];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (20.0, 0.0));

        let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Agent should have moved in the +x direction (toward target)
        assert!(
            agents[0].position.0 > 0.0,
            "agent should move toward target in +x, got x={}",
            agents[0].position.0
        );
        assert!(!result.events.is_empty());
        assert_eq!(result.events[0].kind, EventKind::Moved);
    }

    #[test]
    fn move_direction_attracted_to_carcass_by_heterotrophy() {
        // Agent with heterotrophy and chemotaxis should move toward a nearby carcass.
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let mover_traits = TraitVector {
            heterotrophy: 0.5,
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, mover_traits)];
        let carcasses = vec![Carcass {
            id: 99,
            position: (0.0, 20.0),
            energy: 10.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Agent should have moved in the +y direction (toward carcass)
        assert!(
            agents[0].position.1 > 0.0,
            "agent should move toward carcass in +y, got y={}",
            agents[0].position.1
        );
    }

    #[test]
    fn move_zero_structure_pays_zero_movement_cost() {
        // Newborns (structure=0) should move for free regardless of coefficient.
        let mut params = test_params();
        params.movement_cost_coefficient = 2.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        // structure is 0.0 by default from make_agent
        assert_eq!(agents[0].structure, 0.0);
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Zero structure means zero movement cost
        assert!(
            (agents[0].reserve - 100.0).abs() < 1e-3,
            "zero-structure agent should pay zero movement cost, reserve={}",
            agents[0].reserve
        );
        assert!(
            (result.dissipated).abs() < 1e-3,
            "zero dissipation for zero-structure agent, got {}",
            result.dissipated
        );
    }

    #[test]
    fn move_large_agent_pays_more_than_small_agent() {
        // Two agents with same mobility but different structure: the larger one
        // should pay proportionally more movement cost.
        let mut params = test_params();
        params.movement_cost_coefficient = 1.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };

        // Small agent (structure=2)
        let mut small = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        small[0].structure = 2.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        let result_small = move_agents(&mut small, &carcasses, &grid, &params, 0, 0);

        // Large agent (structure=10)
        let mut large = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        large[0].structure = 10.0;
        let result_large = move_agents(&mut large, &carcasses, &grid, &params, 0, 0);

        // Large agent should pay 5x more (10/2)
        let small_cost = 100.0 - small[0].reserve;
        let large_cost = 100.0 - large[0].reserve;
        assert!(
            large_cost > small_cost,
            "large agent cost ({}) should exceed small agent cost ({})",
            large_cost,
            small_cost
        );
        assert!(
            (large_cost / small_cost - 5.0).abs() < 1e-3,
            "cost ratio should be 5.0, got {}",
            large_cost / small_cost
        );
        assert!(
            (result_large.dissipated / result_small.dissipated - 5.0).abs() < 1e-3,
            "dissipation ratio should be 5.0"
        );
    }

    #[test]
    fn move_energy_cost_proportional_to_distance_and_structure() {
        let mut params = test_params();
        params.movement_cost_coefficient = 2.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };
        let mut agents = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        agents[0].structure = 3.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // eff_mobility = 0.5 (no wear, k=0 so exp(0)=1)
        // cost = distance * coefficient * structure = 0.5 * 2.0 * 3.0 = 3.0
        let expected_cost = 0.5 * 2.0 * 3.0;
        assert!(
            (agents[0].reserve - (100.0 - expected_cost)).abs() < 1e-3,
            "reserve should be {}, got {}",
            100.0 - expected_cost,
            agents[0].reserve
        );
        assert!((result.dissipated - expected_cost).abs() < 1e-3);
    }

    #[test]
    fn move_toroidal_wrapping_applied() {
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        // Agent near the edge with mobility — should wrap around
        let traits = TraitVector {
            mobility: 5.0,
            ..zero_traits()
        };
        // Place at edge of world (extent=100, so range is -50..50)
        // With enough mobility, the position wraps
        let mut agents = vec![make_agent(0, (48.0, 0.0), 100.0, traits)];
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let _result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Position should be within bounds after wrapping
        let extent = params.world_extent;
        let half = extent / 2.0;
        assert!(
            agents[0].position.0 >= -half && agents[0].position.0 <= half,
            "x position should be within bounds, got {}",
            agents[0].position.0
        );
        assert!(
            agents[0].position.1 >= -half && agents[0].position.1 <= half,
            "y position should be within bounds, got {}",
            agents[0].position.1
        );
    }

    #[test]
    fn move_deterministic_with_seeded_rng() {
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            mobility: 0.5,
            ..zero_traits()
        };

        // Run twice with same seed
        let mut agents1 = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        let mut agents2 = vec![make_agent(0, (0.0, 0.0), 100.0, traits)];
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let _r1 = move_agents(&mut agents1, &carcasses, &grid, &params, 0, 0);
        let _r2 = move_agents(&mut agents2, &carcasses, &grid, &params, 0, 0);

        assert!(
            (agents1[0].position.0 - agents2[0].position.0).abs() < 1e-6,
            "positions should be identical with same seed"
        );
        assert!((agents1[0].position.1 - agents2[0].position.1).abs() < 1e-6);
    }

    #[test]
    fn move_sensing_throughput_counts_detected_entities() {
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            mobility: 5.0, // sensing range = 5.0 * 10.0 = 50.0
            heterotrophy: 0.1,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, traits),
            make_agent(1, (10.0, 0.0), 100.0, target_traits),
            make_agent(2, (5.0, 5.0), 100.0, target_traits),
        ];
        agents[1].structure = 5.0;
        agents[2].structure = 5.0;
        let carcasses = vec![Carcass {
            id: 99,
            position: (3.0, 0.0),
            energy: 5.0,
            nutrient: 0.0,
            traits: zero_traits(),
        }];
        let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (10.0, 0.0));
        grid.insert(2, (5.0, 5.0));

        let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Agent 0 should detect 2 living agents + 1 carcass = 3
        assert!(
            (result.sensing_throughput[0] - 3.0).abs() < 1e-6,
            "sensing throughput should be 3.0, got {}",
            result.sensing_throughput[0]
        );
    }

    #[test]
    fn drain_releases_bound_nutrient_retaining_up_to_stoichiometric_demand() {
        // Embodiment (ADR-0003): grazing releases only the nutrient *bound in the
        // structure removed* (actual_drain * target_ratio). The target's free
        // store is never touched. The consumer retains up to its stoichiometric
        // demand; the excess is excreted to the available pool.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        // High-demand target so the bound released exceeds the consumer's need,
        // forcing excretion: target_ratio = 0.1 + 0.2 * 4.0 = 0.9 per structure.
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        // Consumer demand = structure * (base_ratio + spec_coeff * spec_sum)
        //                 = 5.0 * (0.1 + 0.2 * 2.0) = 5.0 * 0.5 = 2.5
        agents[0].structure = 5.0;
        agents[1].structure = 10.0;
        agents[1].nutrient = 20.0; // free store — must be left untouched

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Drain = 2.0 (demand <= supply of 10.0)
        // target_ratio = 0.1 + 0.2 * 4.0 = 0.9
        // bound released = actual_drain * target_ratio = 2.0 * 0.9 = 1.8
        // energy_gained = 2.0 * 0.5 = 1.0
        // consumer_nutrient_need = demand(2.5) * energy_gained(1.0) = 2.5
        // retained = min(1.8, 2.5) = 1.8; excreted = 0.0
        // kappa = 0 here, so the retained nutrient lands entirely in the
        // reproductive-nutrient earmark (ADR-0004). This test asserts the
        // retention magnitude, so check the total retained across both stores.
        let retained = agents[0].nutrient + agents[0].repro_nutrient;
        assert!(
            (retained - 1.8).abs() < 1e-3,
            "consumer should retain the 1.8 bound nutrient released, got {retained}"
        );
        assert!(
            (nutrient_grid.total()).abs() < 1e-3,
            "consumer need exceeds release, so nothing is excreted, got {}",
            nutrient_grid.total()
        );
        assert!(
            (agents[1].nutrient - 20.0).abs() < 1e-3,
            "grazing never touches the target's free store, got {}",
            agents[1].nutrient
        );
    }

    #[test]
    fn drain_excretes_bound_nutrient_excess_to_available_pool() {
        // When the bound nutrient released by grazing exceeds the consumer's
        // stoichiometric demand, the excess is excreted to the available pool —
        // the target's free store still stays untouched.
        let params = test_params();
        // Low-demand consumer (tiny structure) so the released bound exceeds need.
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0, // ratio = 0.9 per structure
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[0].structure = 0.1; // demand = 0.1 * 0.5 = 0.05
        agents[1].structure = 10.0;
        agents[1].nutrient = 20.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // bound released = 2.0 * 0.9 = 1.8; consumer need = 0.05 * 1.0 = 0.05
        // retained = 0.05; excreted = 1.75
        // kappa = 0 routes the retained nutrient to the earmark (ADR-0004); the
        // retention magnitude is the total across both stores.
        let retained = agents[0].nutrient + agents[0].repro_nutrient;
        assert!(
            (retained - 0.05).abs() < 1e-3,
            "consumer retains its demand 0.05, got {retained}"
        );
        assert!(
            (nutrient_grid.total() - 1.75).abs() < 1e-3,
            "excess bound nutrient 1.75 is excreted, got {}",
            nutrient_grid.total()
        );
        assert!(
            (agents[1].nutrient - 20.0).abs() < 1e-3,
            "target's free store untouched, got {}",
            agents[1].nutrient
        );
    }

    #[test]
    fn pure_heterotroph_earmarks_reproductive_nutrient_from_prey_alone() {
        // ADR-0004 headline: a pure heterotroph (zero autotrophy) draws nothing
        // from the available pool (flow 2), so before this change its
        // reproductive-nutrient earmark could never grow and it could never clear
        // reproduction_nutrient_threshold — predation sustained a body but never a
        // lineage. With consumption nutrient split by kappa, ingested prey now
        // funds the earmark.
        let params = test_params();
        let consumer_traits = TraitVector {
            photosynthetic_absorption: 0.0, // pure heterotroph: no pool uptake
            heterotrophy: 2.0,
            kappa: 0.5,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[0].structure = 5.0;
        agents[1].structure = 10.0;

        // A nutrient-rich pool the consumer cannot tap (zero autotrophy).
        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 100.0);
        absorb_nutrients(&mut agents, &mut nutrient_grid, &params);
        assert!(
            (agents[0].repro_nutrient).abs() < 1e-6,
            "pure heterotroph draws no earmark from pool uptake, got {}",
            agents[0].repro_nutrient
        );

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Consumption now feeds the earmark: released 1.8, retained 1.8, half to
        // the earmark = 0.9, which clears the test threshold of 1.0? No — 0.9 < 1.0
        // here, but the earmark is non-zero and accumulates, which is the point.
        assert!(
            agents[0].repro_nutrient > 0.0,
            "heterotroph should earmark reproductive nutrient from prey, got {}",
            agents[0].repro_nutrient
        );
    }

    #[test]
    fn drain_living_target_splits_retained_nutrient_by_kappa() {
        // ADR-0004: nutrient retained from consuming a living target is split by
        // kappa, mirroring the autotrophic uptake split (flow 2) and the energy
        // side. The kappa share feeds the free store; the (1 - kappa) share feeds
        // the reproductive-nutrient earmark. The cap on how much is retained is
        // unchanged — only the routing of the retained amount changes.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            kappa: 0.25,
            ..zero_traits()
        };
        // High-demand target so the bound released (1.8) is below the consumer's
        // need (2.5), forcing the whole release to be retained and then split.
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[0].structure = 5.0; // demand = 5.0 * 0.5 = 2.5
        agents[1].structure = 10.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // released = 2.0 * 0.9 = 1.8; need = 2.5; retained = 1.8; excreted = 0.0.
        // kappa share (0.25) -> free store = 0.45; (1 - kappa) share -> earmark = 1.35.
        assert!(
            (agents[0].nutrient - 0.45).abs() < 1e-3,
            "free store should get kappa share 0.45, got {}",
            agents[0].nutrient
        );
        assert!(
            (agents[0].repro_nutrient - 1.35).abs() < 1e-3,
            "earmark should get (1-kappa) share 1.35, got {}",
            agents[0].repro_nutrient
        );
        assert!(
            (nutrient_grid.total()).abs() < 1e-3,
            "release is below need, so nothing is excreted, got {}",
            nutrient_grid.total()
        );
    }

    #[test]
    fn non_lethal_graze_leaves_victim_free_store_and_repro_earmark_intact() {
        // Issue #274: grazing a living target transfers only the nutrient *bound
        // in the structure removed*. The victim's free nutrient store AND its
        // reproductive-nutrient earmark must survive a non-lethal graze; the
        // consumer keeps up to its stoichiometric demand and the excess is
        // excreted to the local pool. This locks in the embodiment behaviour
        // (#273) against regressions that would touch the free store/earmark or
        // stop excreting.
        let params = test_params();
        // Low-demand consumer (tiny structure) so the released bound exceeds the
        // consumer's stoichiometric need, forcing a non-zero excretion.
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0, // ratio = 0.1 + 0.2 * 4.0 = 0.9 per structure
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 10.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[0].structure = 0.1; // demand = 0.1 * 0.5 = 0.05
        agents[1].structure = 10.0; // graze removes 2.0 -> 8.0, well above death threshold
        agents[1].nutrient = 20.0; // free store — must be left untouched
        agents[1].repro_nutrient = 7.0; // reproductive earmark — must be left untouched

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // The graze is non-lethal: nothing died this tick.
        assert!(
            result.dead_agents.is_empty(),
            "graze should be non-lethal, but agents died: {:?}",
            result.dead_agents
        );

        // bound released = actual_drain(2.0) * target_ratio(0.9) = 1.8
        // consumer need = demand(0.05) * energy_gained(1.0) = 0.05
        // retained = 0.05; excreted = 1.75
        // kappa = 0 routes the consumer's retained nutrient to the earmark
        // (ADR-0004); the magnitude retained is the total across both stores.
        let retained = agents[0].nutrient + agents[0].repro_nutrient;
        assert!(
            (retained - 0.05).abs() < 1e-3,
            "consumer retains its stoichiometric demand 0.05, got {retained}"
        );
        assert!(
            nutrient_grid.total() > 0.0,
            "excess bound nutrient must be excreted to the pool, got {}",
            nutrient_grid.total()
        );
        assert!(
            (nutrient_grid.total() - 1.75).abs() < 1e-3,
            "excess bound nutrient 1.75 is excreted, got {}",
            nutrient_grid.total()
        );

        // The victim keeps BOTH its free store and its reproductive earmark.
        assert!(
            (agents[1].nutrient - 20.0).abs() < 1e-3,
            "grazing never touches the victim's free store, got {}",
            agents[1].nutrient
        );
        assert!(
            (agents[1].repro_nutrient - 7.0).abs() < 1e-3,
            "grazing never touches the victim's reproductive earmark, got {}",
            agents[1].repro_nutrient
        );
    }

    #[test]
    fn drain_nutrient_excretion_returns_to_local_cell() {
        // Nutrient excreted during consumption goes to the nutrient grid cell
        // at the target's position, not to a global pool.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 4.0, // high demand -> released bound exceeds need
            ..zero_traits()
        };
        // Place consumer and target far apart but within contact range
        // (contact_radius is 5.0 in test_params)
        let target_pos = (40.0, 0.0);
        let consumer_pos = (41.0, 0.0); // 1.0 away, within contact radius
        let mut agents = vec![
            make_agent(1, consumer_pos, 10.0, consumer_traits),
            make_agent(2, target_pos, 10.0, target_traits),
        ];
        agents[0].structure = 0.1; // low demand so the released bound is excreted
        agents[1].structure = 10.0;
        agents[1].nutrient = 20.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, consumer_pos);
        grid.insert(1, target_pos);

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Excreted nutrient should be in the cell at the target's position
        let target_cell_nutrient = *nutrient_grid.at_position(target_pos);
        assert!(
            target_cell_nutrient > 0.0,
            "excreted nutrient should be in target's cell, got {}",
            target_cell_nutrient
        );

        // A distant cell should have zero
        let distant_cell = *nutrient_grid.at_position((-40.0, -40.0));
        assert!(
            (distant_cell).abs() < 1e-6,
            "distant cell should be unaffected, got {}",
            distant_cell
        );

        // Total excreted should match what the nutrient grid received
        assert!(
            (nutrient_grid.total() - target_cell_nutrient).abs() < 1e-6,
            "all excreted nutrient should be in target's cell"
        );
    }

    #[test]
    fn drain_carcass_excreted_nutrient_conserves_into_available_pool() {
        // Direct positive routing test (WR-25): when a low-demand consumer
        // decomposes a nutrient-rich carcass, the nutrient drained from the
        // carcass splits exactly into (a) nutrient retained by the consumer and
        // (b) excess excreted to the NutrientGrid at the carcass's cell. Nothing
        // dissipates: consumer gain + grid delta == total drained from target.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            ..zero_traits()
        };
        let carcass_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Low stoichiometric demand consumer (small structure), placed within
        // contact range (contact_radius = 5.0 in test_params).
        let carcass_pos = (40.0, 0.0);
        let consumer_pos = (41.0, 0.0);
        let mut agents = vec![make_agent(1, consumer_pos, 10.0, consumer_traits)];
        agents[0].structure = 5.0;

        let mut carcasses = vec![Carcass {
            id: 99,
            position: carcass_pos,
            energy: 10.0,
            nutrient: 20.0, // nutrient-rich target
            traits: carcass_traits,
        }];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, consumer_pos); // grid key is the consumer's slice index

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);

        // Record before-state. kappa = 0 routes retained nutrient to the earmark
        // (ADR-0004), so the consumer's holdings span both stores.
        let consumer_nutrient_before = agents[0].nutrient + agents[0].repro_nutrient;
        let carcass_nutrient_before = carcasses[0].nutrient;
        let grid_total_before = nutrient_grid.total();

        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Deltas across the single consumption tick.
        let consumer_gain =
            (agents[0].nutrient + agents[0].repro_nutrient) - consumer_nutrient_before;
        let drained_from_carcass = carcass_nutrient_before - carcasses[0].nutrient;
        let excreted_to_grid = nutrient_grid.total() - grid_total_before;

        // Routing must split the drained nutrient with no loss.
        assert!(consumer_gain > 0.0, "consumer should retain some nutrient");
        assert!(
            excreted_to_grid > 0.0,
            "excess should reach the available pool"
        );
        assert!(
            (consumer_gain + excreted_to_grid - drained_from_carcass).abs() < 1e-4,
            "consumer gain ({consumer_gain}) + excreted ({excreted_to_grid}) must equal \
             nutrient drained from carcass ({drained_from_carcass})"
        );

        // The excreted nutrient lands at the carcass's cell specifically.
        let carcass_cell = *nutrient_grid.at_position(carcass_pos);
        assert!(
            (carcass_cell - excreted_to_grid).abs() < 1e-6,
            "all excreted nutrient should be in the carcass's cell"
        );
    }

    #[test]
    fn drain_carcass_splits_retained_nutrient_by_kappa() {
        // ADR-0004: nutrient retained from decomposing a carcass is split by
        // kappa, exactly like the living-target route and autotrophic uptake. The
        // detrital chain funds decomposer reproduction from ingested nutrient.
        let params = test_params();
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            kappa: 0.25,
            ..zero_traits()
        };
        let carcass_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let carcass_pos = (40.0, 0.0);
        let consumer_pos = (41.0, 0.0);
        let mut agents = vec![make_agent(1, consumer_pos, 10.0, consumer_traits)];
        agents[0].structure = 5.0; // demand = 5.0 * 0.5 = 2.5

        let mut carcasses = vec![Carcass {
            id: 99,
            position: carcass_pos,
            energy: 10.0,
            nutrient: 20.0,
            traits: carcass_traits,
        }];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, consumer_pos); // grid key is the consumer's slice index

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // actual_drain = 2.0; energy_gained = 2.0 * 0.5 = 1.0; need = 2.5 * 1.0 = 2.5.
        // transferred = 20.0 * (2.0/10.0) = 4.0; retained = min(4.0, 2.5) = 2.5.
        // kappa share (0.25) -> free store = 0.625; (1 - kappa) -> earmark = 1.875.
        assert!(
            (agents[0].nutrient - 0.625).abs() < 1e-3,
            "free store should get kappa share 0.625, got {}",
            agents[0].nutrient
        );
        assert!(
            (agents[0].repro_nutrient - 1.875).abs() < 1e-3,
            "earmark should get (1-kappa) share 1.875, got {}",
            agents[0].repro_nutrient
        );
        // Excretion is unchanged by the split: 4.0 - 2.5 = 1.5 reaches the pool.
        assert!(
            (nutrient_grid.total() - 1.5).abs() < 1e-3,
            "excess 1.5 still excreted to the pool, got {}",
            nutrient_grid.total()
        );
    }

    // --- Distance-dependent trophic efficiency in drains ---

    #[test]
    fn drain_similar_consumer_gains_more_than_distant_consumer() {
        // Two consumers drain the same target. The one with traits closer to
        // the target gets more energy per unit drained.
        let params = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 2.0,
            ..test_params()
        };
        // Target is a producer
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        // Consumer A: similar to target (also photosynthetic + heterotroph)
        let similar_consumer = TraitVector {
            photosynthetic_absorption: 0.4,
            heterotrophy: 1.0,
            ..zero_traits()
        };
        // Consumer B: very different from target (high mobility, no photo)
        let distant_consumer = TraitVector {
            heterotrophy: 1.0,
            mobility: 2.0,
            ..zero_traits()
        };

        // Run two separate drain resolutions to compare energy gained
        let run_drain = |consumer_traits: TraitVector| -> f32 {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 0.0, consumer_traits),
                make_agent(2, (1.0, 0.0), 10.0, target_traits),
            ];
            agents[1].structure = 100.0; // plenty of structure
            let mut carcasses: Vec<Carcass> = Vec::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (1.0, 0.0));
            let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
            let _result = resolve_drains(
                &mut agents,
                &mut carcasses,
                &grid,
                &params,
                &mut nutrient_grid,
            );
            agents[0].reserve // energy gained
        };

        let gained_similar = run_drain(similar_consumer);
        let gained_distant = run_drain(distant_consumer);

        assert!(
            gained_similar > gained_distant,
            "similar consumer should gain more: similar={}, distant={}",
            gained_similar,
            gained_distant
        );
        assert!(gained_similar > 0.0, "similar consumer should gain energy");
        assert!(
            gained_distant > 0.0,
            "distant consumer should still gain some energy"
        );
    }

    #[test]
    fn drain_carcass_uses_original_trait_vector_for_distance() {
        // A carcass retains the dead agent's trait vector. Efficiency depends
        // on trait distance between consumer and the carcass's original traits.
        let params = WorldParameters {
            base_trophic_efficiency: 0.8,
            trophic_distance_decay: 2.0,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 0.5,
            photosynthetic_absorption: 0.3,
            ..zero_traits()
        };
        // Carcass from a similar organism
        let similar_carcass_traits = TraitVector {
            photosynthetic_absorption: 0.4,
            ..zero_traits()
        };
        // Carcass from a very different organism
        let distant_carcass_traits = TraitVector {
            mobility: 3.0,
            ..zero_traits()
        };

        let run_carcass_drain = |carcass_traits: TraitVector| -> f32 {
            // Grid key is the slice index; here agent id 0 sits at index 0.
            let mut agents = vec![make_agent(0, (0.0, 0.0), 0.0, consumer_traits)];
            let mut carcasses = vec![Carcass {
                id: 99,
                position: (0.0, 0.0), // co-located with consumer
                energy: 100.0,
                nutrient: 0.0,
                traits: carcass_traits,
            }];
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, agents[0].position);
            let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
            let _result = resolve_drains(
                &mut agents,
                &mut carcasses,
                &grid,
                &params,
                &mut nutrient_grid,
            );
            agents[0].reserve
        };

        let gained_similar = run_carcass_drain(similar_carcass_traits);
        let gained_distant = run_carcass_drain(distant_carcass_traits);

        assert!(
            gained_similar > gained_distant,
            "decomposing similar carcass should yield more: similar={}, distant={}",
            gained_similar,
            gained_distant
        );
    }

    #[test]
    fn drain_energy_conservation_with_distance_dependent_efficiency() {
        // Energy drained from target = energy gained by consumer + dissipated.
        // This must hold regardless of distance-dependent efficiency.
        let params = WorldParameters {
            base_trophic_efficiency: 0.7,
            trophic_distance_decay: 1.5,
            ..test_params()
        };
        let consumer_traits = TraitVector {
            heterotrophy: 2.0,
            mobility: 1.0,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_traits),
            make_agent(2, (1.0, 0.0), 10.0, target_traits),
        ];
        agents[1].structure = 50.0;

        let pre_target_structure = agents[1].structure;
        let pre_consumer_reserve = agents[0].reserve;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        let drained = pre_target_structure - agents[1].structure;
        let gained = agents[0].reserve - pre_consumer_reserve;
        let dissipated = result.dissipated;

        assert!(drained > 0.0, "something should have been drained");
        assert!(
            (drained - gained - dissipated).abs() < 1e-4,
            "energy conservation: drained={}, gained={}, dissipated={}, diff={}",
            drained,
            gained,
            dissipated,
            drained - gained - dissipated
        );
    }

    // --- Resolve reproduction ---

    #[test]
    fn reproduction_two_compatible_agents_produce_offspring_with_correct_energy() {
        // This test focuses on the reproduction_efficiency loss only;
        // structure provisioning is exercised by the conservation tests.
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0, // sensing range = 1.0 * 10.0 = 10.0
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        // Set repro_reserve above threshold (10.0)
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        // Each parent invests entire repro_reserve (15.0)
        // Total = 30.0, offspring energy = 30.0 * 0.7 = 21.0
        // Dissipated = 30.0 - 21.0 = 9.0
        assert!(!result.offspring.is_empty(), "should produce offspring");
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!(
            (total_offspring_energy - 21.0).abs() < 1e-3,
            "offspring energy should be 21.0, got {}",
            total_offspring_energy
        );
        assert!(
            (result.dissipated - 9.0).abs() < 1e-3,
            "dissipated should be 9.0, got {}",
            result.dissipated
        );
        // Parents' reserve should be unchanged (investment came from repro_reserve)
        assert!(
            (agents[0].reserve - 100.0).abs() < 1e-3,
            "parent A reserve should be unchanged, got {}",
            agents[0].reserve
        );
        // Parents' repro_reserve should be 0
        assert!(
            agents[0].repro_reserve.abs() < 1e-3,
            "parent A repro_reserve should be 0, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn reproduction_dead_agents_excluded() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        // Mark agent 1 as dead (killed in drain pass)
        let mut dead_ids = std::collections::HashSet::new();
        dead_ids.insert(1u64);

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            result.offspring.is_empty(),
            "dead agent should not reproduce"
        );
        assert!(
            (agents[0].reserve - 100.0).abs() < 1e-6,
            "no investment should occur"
        );
    }

    #[test]
    fn reproduction_below_energy_threshold_excluded() {
        let params = WorldParameters {
            reproduction_energy_threshold: 200.0, // higher than agent reserve
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            result.offspring.is_empty(),
            "agents below energy threshold should not reproduce"
        );
    }

    #[test]
    fn reproduction_blocked_when_repro_nutrient_earmark_below_threshold() {
        // The reproduction gate reads the reproductive-nutrient earmark, not the
        // free store. An agent with an ample free store but an earmark below the
        // threshold cannot reproduce.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 5.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].structure = 10.0;
        agents[1].structure = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;
        // Ample free store, but earmark below the 5.0 threshold.
        agents[0].nutrient = 100.0;
        agents[1].nutrient = 100.0;
        agents[0].repro_nutrient = 1.0;
        agents[1].repro_nutrient = 1.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            result.offspring.is_empty(),
            "agents with earmark below threshold should not reproduce"
        );
    }

    #[test]
    fn reproduction_proceeds_on_earmark_despite_depleted_free_store() {
        // The old body-support gate (`nutrient >= structure * demand`) is gone:
        // a near-empty free store no longer blocks reproduction so long as the
        // reproductive-nutrient earmark clears its threshold.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 5.0,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 2.0,
            asexual_propensity: 1.0, // force asexual path
            ..zero_traits()
        };
        let mut agents = vec![make_agent(1, (0.0, 0.0), 100.0, traits)];
        // Large structure → old body-support demand (10 * 0.2 = 2.0) far exceeds
        // the tiny free store, which would have blocked reproduction before.
        agents[0].structure = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[0].nutrient = 0.01; // free store nearly empty
        agents[0].repro_nutrient = 20.0; // earmark well above threshold

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            !result.offspring.is_empty(),
            "earmark above threshold should permit reproduction despite empty free store"
        );
    }

    #[test]
    fn reproduction_invests_entire_repro_reserve() {
        let params = WorldParameters {
            reproduction_efficiency: 0.7,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 15.0, traits),
            make_agent(2, (1.0, 0.0), 15.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 20.0;
        agents[0].repro_nutrient = 20.0;
        agents[1].repro_reserve = 20.0;
        agents[1].repro_nutrient = 20.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        // Each parent invests entire repro_reserve: 20.0 each
        // Total = 40.0, offspring energy = 40.0 * 0.7 = 28.0
        assert!(!result.offspring.is_empty());
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!(
            (total_offspring_energy - 28.0).abs() < 1e-3,
            "offspring energy should be 28.0, got {}",
            total_offspring_energy
        );
        // Parents' repro_reserve should be 0
        assert!(
            agents[0].repro_reserve.abs() < 1e-3,
            "parent A should have 0 repro_reserve, got {}",
            agents[0].repro_reserve
        );
        assert!(
            agents[1].repro_reserve.abs() < 1e-3,
            "parent B should have 0 repro_reserve, got {}",
            agents[1].repro_reserve
        );
        // Parents' reserve should be unchanged
        assert!(
            (agents[0].reserve - 15.0).abs() < 1e-3,
            "parent A reserve should be unchanged, got {}",
            agents[0].reserve
        );
    }

    #[test]
    fn reproduction_mate_pairing_selects_closest_trait_distance() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        // Agent A and B have identical traits (distance=0), C is different
        let traits_ab = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_c = TraitVector {
            photosynthetic_absorption: 0.1,
            heterotrophy: 0.4,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        // All three within sensing range of each other
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_ab),
            make_agent(2, (2.0, 0.0), 100.0, traits_ab),
            make_agent(3, (4.0, 0.0), 100.0, traits_c),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;
        agents[2].nutrient = 10.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (2.0, 0.0));
        grid.insert(2, (4.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // A and B should pair (trait distance=0), C has no mate
        assert_eq!(result.events.len(), 1, "should produce exactly one pair");
        let ev = &result.events[0];
        // The pair should be agents 1 and 2 (ids)
        let paired_ids = [ev.source, ev.target.unwrap()];
        assert!(
            paired_ids.contains(&1) && paired_ids.contains(&2),
            "agents 1 and 2 should pair, got {:?}",
            paired_ids
        );
        // Agent C (id=3) should not have invested
        assert!(
            (agents[2].reserve - 100.0).abs() < 1e-6,
            "agent C should not have reproduced"
        );
    }

    #[test]
    fn reproduction_offspring_traits_are_crossover_of_parents() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0, // no mutation to isolate crossover
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits_a = TraitVector {
            photosynthetic_absorption: 0.8,
            heterotrophy: 0.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 0.8,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_a),
            make_agent(2, (1.0, 0.0), 100.0, traits_b),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(!result.offspring.is_empty());
        let child = &result.offspring[0];
        // With no mutation, each trait dimension comes from one parent.
        // For photosynthetic_absorption: either 0.8 (from A) or 0.0 (from B)
        // For consumption_rate: either 0.0 (from A) or 0.8 (from B)
        // No budget normalization — traits retain raw crossover values.
        // Each dimension should come from exactly one parent (no averaging)
        for dim in 0..TraitVector::NUM_DIMS {
            let val = child.traits.get(dim);
            assert!(val >= 0.0, "trait dimension {} should be non-negative", dim);
        }
    }

    #[test]
    fn reproduction_fecundity_controls_offspring_count() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        // High fecundity -> more offspring
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 5.0, // Poisson mean=5
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 20.0;
        agents[1].nutrient = 20.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // With Poisson(5.0), expect multiple offspring (statistically unlikely to get 1)
        assert!(
            result.offspring.len() > 1,
            "high fecundity should produce multiple offspring, got {}",
            result.offspring.len()
        );
        // Total offspring energy should equal total_investment * efficiency
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        let total_investment = 30.0; // 15.0 + 15.0 (entire repro_reserve of each parent)
        let expected_total = total_investment * 0.7;
        assert!(
            (total_offspring_energy - expected_total).abs() < 1e-3,
            "total offspring energy should be {}, got {}",
            expected_total,
            total_offspring_energy
        );
    }

    #[test]
    fn reproduction_offspring_zero_wear_and_dispersed() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 3.0,
            dispersal: 5.0, // high dispersal -> wider offspring placement
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            // Zero wear
            for w in &child.wear {
                assert!(*w == 0.0, "offspring should have zero wear");
            }
            // Zero structure
            assert!(
                child.structure == 0.0,
                "offspring should have zero structure"
            );
            // Position within world bounds
            let half = params.world_extent / 2.0;
            assert!(
                child.position.0 >= -half && child.position.0 <= half,
                "offspring x should be within bounds"
            );
            assert!(
                child.position.1 >= -half && child.position.1 <= half,
                "offspring y should be within bounds"
            );
        }
        // With high dispersal trait, offspring should be placed at different positions
        // (not all at the exact same position)
        if result.offspring.len() > 1 {
            let positions: Vec<(f32, f32)> = result.offspring.iter().map(|o| o.position).collect();
            let all_same = positions.iter().all(|p| {
                (p.0 - positions[0].0).abs() < 1e-6 && (p.1 - positions[0].1).abs() < 1e-6
            });
            assert!(
                !all_same,
                "offspring should be dispersed to different positions"
            );
        }
    }

    #[test]
    fn reproduction_emits_reproduced_events() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind, EventKind::Reproduced);
        assert!(
            result.events[0].target.is_some(),
            "Reproduced event should have target (second parent)"
        );
    }

    #[test]
    fn reproduction_deterministic_with_seeded_rng() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.5,
            mutation_magnitude: 0.1,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 3.0,
            ..zero_traits()
        };

        let run = |seed: u64| -> (Vec<(f32, f32)>, Vec<f32>) {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 100.0, traits),
                make_agent(2, (1.0, 0.0), 100.0, traits),
            ];
            agents[0].nutrient = 10.0;
            agents[1].nutrient = 10.0;
            agents[0].repro_reserve = 15.0;
            agents[0].repro_nutrient = 15.0;
            agents[1].repro_reserve = 15.0;
            agents[1].repro_nutrient = 15.0;

            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (1.0, 0.0));

            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, seed, 0);
            let positions: Vec<(f32, f32)> = result.offspring.iter().map(|o| o.position).collect();
            let energies: Vec<f32> = result.offspring.iter().map(|o| o.reserve).collect();
            (positions, energies)
        };

        let (pos1, en1) = run(42);
        let (pos2, en2) = run(42);
        assert_eq!(
            pos1.len(),
            pos2.len(),
            "same seed should produce same count"
        );
        for i in 0..pos1.len() {
            assert!((pos1[i].0 - pos2[i].0).abs() < 1e-6);
            assert!((pos1[i].1 - pos2[i].1).abs() < 1e-6);
            assert!((en1[i] - en2[i]).abs() < 1e-6);
        }
    }

    #[test]
    fn reproduction_is_lossy_energy_dissipated() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.5, // 50% efficient
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            offspring_structure_fraction: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        // Total investment = 15.0 + 15.0 = 30.0 (entire repro_reserve)
        // Offspring energy = 30.0 * 0.5 = 15.0
        // Dissipated = 30.0 - 15.0 = 15.0
        let total_offspring_energy: f32 = result.offspring.iter().map(|o| o.reserve).sum();
        assert!(
            (total_offspring_energy - 15.0).abs() < 1e-3,
            "offspring energy should be 15.0, got {}",
            total_offspring_energy
        );
        assert!(
            (result.dissipated - 15.0).abs() < 1e-3,
            "dissipated should be 15.0, got {}",
            result.dissipated
        );
        // Conservation: investment = offspring + dissipated
        assert!(
            (total_offspring_energy + result.dissipated - 30.0).abs() < 1e-3,
            "energy should be conserved"
        );
    }

    #[test]
    fn asexual_zero_poisson_draw_consumes_energy_yields_no_offspring() {
        // World-rules flow 4: reproductive failure (zero offspring despite energy
        // investment) must be possible. With fecundity at its floor (0.1) the
        // Poisson mean is 0.1, so a zero draw is overwhelmingly likely. The energy
        // committed to reproduction is still consumed and dissipated.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 0.0, // floored to 0.1 mean -> P(0) ~ 90%
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // Zero offspring produced (reproductive failure).
        assert!(
            result.offspring.is_empty(),
            "zero Poisson draw should yield no offspring, got {}",
            result.offspring.len()
        );
        // Energy was still committed: parent's repro_reserve is consumed.
        assert!(
            agents[0].repro_reserve < 1e-6,
            "parent repro_reserve should be consumed even on failure, got {}",
            agents[0].repro_reserve
        );
        // Conservation: the entire committed investment (20.0) is dissipated since
        // no offspring carry any of it.
        assert!(
            (result.dissipated - 20.0).abs() < 1e-3,
            "full investment should dissipate on zero-offspring failure, got {}",
            result.dissipated
        );
    }

    #[test]
    fn sexual_zero_poisson_draw_consumes_energy_yields_no_offspring() {
        // Sexual reproductive failure: both parents commit their repro_reserve but
        // a zero Poisson draw yields no offspring. The full combined investment is
        // dissipated.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 0.0,          // floored to 0.1 mean -> P(0) ~ 90%
            asexual_propensity: 0.0, // force sexual path
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // Zero offspring (reproductive failure).
        assert!(
            result.offspring.is_empty(),
            "zero Poisson draw should yield no offspring, got {}",
            result.offspring.len()
        );
        // Both parents' repro_reserve consumed.
        assert!(
            agents[0].repro_reserve < 1e-6 && agents[1].repro_reserve < 1e-6,
            "both parents' repro_reserve should be consumed, got {} and {}",
            agents[0].repro_reserve,
            agents[1].repro_reserve
        );
        // Full combined investment (15 + 15 = 30) dissipates.
        assert!(
            (result.dissipated - 30.0).abs() < 1e-3,
            "full investment should dissipate on zero-offspring failure, got {}",
            result.dissipated
        );
    }

    // --- Derived sensing range ---

    #[test]
    fn move_sensing_range_derived_from_mobility() {
        // Sensing range = mobility * sensing_range_coefficient.
        // Agent with high mobility detects agents further away.
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        params.sensing_range_coefficient = 10.0;
        // Low mobility -> sensing range = 0.2 * 10 = 2.0
        let low_mobility = TraitVector {
            mobility: 0.2,
            heterotrophy: 0.1,
            ..zero_traits()
        };
        // High mobility -> sensing range = 2.0 * 10 = 20.0
        let high_mobility = TraitVector {
            mobility: 2.0,
            heterotrophy: 0.1,
            ..zero_traits()
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };

        // Target at distance 5.0 — within high-mobility sensing but outside low-mobility
        let run = |mover_traits: TraitVector| -> f32 {
            let mut agents = vec![
                make_agent(0, (0.0, 0.0), 100.0, mover_traits),
                make_agent(1, (5.0, 0.0), 100.0, target_traits),
            ];
            agents[1].structure = 5.0;
            let carcasses = vec![];
            let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (5.0, 0.0));

            let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);
            result.sensing_throughput[0]
        };

        let low_detected = run(low_mobility);
        let high_detected = run(high_mobility);
        assert_eq!(
            low_detected, 0.0,
            "low-mobility agent should not detect target at distance 5 with sensing range 2"
        );
        assert!(
            high_detected >= 1.0,
            "high-mobility agent should detect target at distance 5 with sensing range 20"
        );
    }

    #[test]
    fn move_zero_mobility_gets_zero_sensing() {
        // An agent with zero mobility has zero sensing range and detects nothing.
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        let traits = TraitVector {
            heterotrophy: 0.5,
            ..zero_traits() // mobility = 0
        };
        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(0, (0.0, 0.0), 100.0, traits),
            make_agent(1, (1.0, 0.0), 100.0, target_traits),
        ];
        agents[1].structure = 5.0;
        let carcasses = vec![];
        let grid = crate::spatial::SpatialGrid::new(100.0, 10.0);

        let result = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);

        // Zero mobility -> stationary, no sensing
        assert_eq!(
            result.sensing_throughput[0], 0.0,
            "zero-mobility agent should detect nothing"
        );
    }

    // --- Reproductive compatibility distance ---

    #[test]
    fn reproduction_uses_world_param_compatibility_distance() {
        // Agents whose trait-space distance exceeds reproductive_compatibility_distance
        // cannot mate, regardless of spatial proximity.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 0.5, // tight threshold
            ..test_params()
        };
        // Two agents with trait distance > 0.5
        let traits_a = TraitVector {
            photosynthetic_absorption: 1.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 1.0,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        let dist = traits_a.distance(&traits_b);
        assert!(
            dist > 0.5,
            "trait distance should exceed compatibility threshold: {}",
            dist
        );

        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits_a),
            make_agent(2, (1.0, 0.0), 100.0, traits_b),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            result.offspring.is_empty(),
            "agents beyond compatibility distance should not reproduce"
        );
    }

    #[test]
    fn reproduction_compatible_within_world_param_distance() {
        // Agents whose trait-space distance is within reproductive_compatibility_distance
        // can mate.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0, // generous threshold
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits()
        };
        // Identical traits -> distance = 0, well within threshold
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (1.0, 0.0), 100.0, traits),
        ];
        agents[0].nutrient = 10.0;
        agents[1].nutrient = 10.0;
        agents[0].repro_reserve = 15.0;
        agents[0].repro_nutrient = 15.0;
        agents[1].repro_reserve = 15.0;
        agents[1].repro_nutrient = 15.0;

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(
            !result.offspring.is_empty(),
            "agents within compatibility distance should reproduce"
        );
    }

    // --- Reproductive reach (dispersal contributes to mate-finding) ---

    #[test]
    fn sessile_agent_with_dispersal_finds_mate() {
        // A mobility-0 (sessile) agent has zero mobility-derived reach, but a
        // sufficient dispersal trait extends its mate-search radius so it can
        // pair with a compatible neighbour. Reach = dispersal * coefficient.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0,
            sensing_range_coefficient: 10.0,
            dispersal_reach_coefficient: 10.0, // dispersal 0.5 -> reach 5.0
            ..test_params()
        };
        // Both sessile (mobility 0) but with dispersal that gives reach 5.0.
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            dispersal: 0.5,
            ..zero_traits() // mobility = 0
        };
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, traits),
            make_agent(2, (3.0, 0.0), 100.0, traits), // gap 3.0 < reach 5.0
        ];
        for a in agents.iter_mut() {
            a.nutrient = 10.0;
            a.repro_reserve = 15.0;
            a.repro_nutrient = 15.0;
        }

        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (3.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(
            !result.offspring.is_empty(),
            "sessile agents with sufficient dispersal reach should reproduce sexually"
        );
    }

    #[test]
    fn mate_reach_uses_effective_mobility_not_nominal() {
        // The mobility term of mate-finding reach is wear-adjusted. Two mobile,
        // zero-dispersal agents sit at a gap that falls inside their *nominal*
        // reach but outside their *effective* reach once mobility wear is applied.
        // With wear, they must fail to pair — proving reach uses effective mobility.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0,
            sensing_range_coefficient: 10.0,
            wear_degradation_steepness: 1.0,
            ..test_params()
        };
        // Nominal reach = 1.0 * 10 = 10.0. Gap is 8.0 (inside nominal).
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits() // dispersal = 0
        };

        let run = |mobility_wear: f32| -> bool {
            let mut agents = vec![
                make_agent(1, (0.0, 0.0), 100.0, traits),
                make_agent(2, (8.0, 0.0), 100.0, traits),
            ];
            for a in agents.iter_mut() {
                a.nutrient = 10.0;
                a.repro_reserve = 15.0;
                a.repro_nutrient = 15.0;
                a.wear[2] = mobility_wear; // index 2 = mobility
            }
            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (8.0, 0.0));
            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);
            !result.offspring.is_empty()
        };

        // No wear: effective reach = nominal 10.0 > gap 8.0 -> pair.
        assert!(
            run(0.0),
            "unworn mobile agents within nominal reach should pair"
        );
        // Heavy wear: effective mobility = 1.0 * exp(-1.0 * 1.0) ~ 0.368,
        // reach ~ 3.68 < gap 8.0 -> no pair.
        assert!(
            !run(1.0),
            "worn mobile agents should fail to pair as effective reach shrinks below the gap"
        );
    }

    #[test]
    fn mate_reach_asymmetric_wide_reach_pairs_zero_reach_neighbour() {
        // OR-style pairing: a wide-reach agent pairs with a compatible neighbour
        // that has zero reach of its own (mobility 0, dispersal 0), as long as the
        // neighbour falls within the wide agent's reach.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0,
            sensing_range_coefficient: 10.0,
            dispersal_reach_coefficient: 10.0,
            ..test_params()
        };
        // Broadcaster: dispersal 1.0 -> reach 10.0. Neighbour: sessile, no dispersal.
        let broadcaster = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            dispersal: 1.0,
            ..zero_traits()
        };
        let neighbour = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            ..zero_traits() // mobility 0, dispersal 0 -> reach 0
        };
        // Gap 6.0: outside the neighbour's reach (0) but inside the broadcaster's (10).
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 100.0, broadcaster),
            make_agent(2, (6.0, 0.0), 100.0, neighbour),
        ];
        for a in agents.iter_mut() {
            a.nutrient = 10.0;
            a.repro_reserve = 15.0;
            a.repro_nutrient = 15.0;
        }
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (6.0, 0.0));
        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);
        assert!(
            !result.offspring.is_empty(),
            "a wide-reach broadcaster should pair with a zero-reach neighbour inside its reach"
        );
    }

    #[test]
    fn mate_reach_does_not_move_offspring() {
        // Reach is the eligibility axis only: it decides whether a pair forms,
        // not where offspring land. Offspring placement is governed independently
        // by the dispersal trait around the seed parent (issue #283). Running the
        // same scenario with different dispersal_reach_coefficient values must
        // yield identical offspring positions.
        let base = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            reproductive_compatibility_distance: 5.0,
            sensing_range_coefficient: 10.0,
            ..test_params()
        };
        // Mobile (reach via mobility, so the pair forms regardless of the
        // dispersal-reach coefficient) but dispersal trait 0 -> offspring land
        // exactly on the seed parent.
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 5.0,  // high mean so the Poisson draw is reliably nonzero
            ..zero_traits()  // dispersal = 0 -> no offspring scatter
        };

        let run = |reach_coeff: f32| -> Vec<(f32, f32)> {
            let params = WorldParameters {
                dispersal_reach_coefficient: reach_coeff,
                ..base
            };
            let mut agents = vec![
                make_agent(1, (10.0, 10.0), 100.0, traits),
                make_agent(2, (14.0, 10.0), 100.0, traits),
            ];
            for a in agents.iter_mut() {
                a.nutrient = 10.0;
                a.repro_reserve = 15.0;
                a.repro_nutrient = 15.0;
            }
            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (10.0, 10.0));
            grid.insert(1, (14.0, 10.0));
            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);
            result.offspring.iter().map(|o| o.position).collect()
        };

        let no_reach = run(0.0);
        let wide_reach = run(50.0);
        assert!(!no_reach.is_empty(), "scenario should produce offspring");
        assert_eq!(
            no_reach, wide_reach,
            "offspring positions must be unaffected by the mate-reach coefficient"
        );
        // With dispersal trait 0, offspring land exactly on the seed parent —
        // one of the two parent positions (10,10) or (14,10), never between.
        for pos in &no_reach {
            let at_a = (pos.0 - 10.0).abs() < 1e-4 && (pos.1 - 10.0).abs() < 1e-4;
            let at_b = (pos.0 - 14.0).abs() < 1e-4 && (pos.1 - 10.0).abs() < 1e-4;
            assert!(
                at_a || at_b,
                "offspring should land on a parent (seed), got {:?}",
                pos
            );
        }
    }

    // --- Chemotaxis derived from mobility ---

    #[test]
    fn move_chemotaxis_proportional_to_mobility() {
        // Chemotaxis strength is derived from mobility. Higher mobility agents
        // have stronger directional bias toward detected signals.
        let mut params = test_params();
        params.movement_cost_coefficient = 0.0;
        params.sensing_range_coefficient = 100.0; // ensure both can sense

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };

        // Strong chemotaxis signal: a close, attractive target and high
        // heterotrophy so the deterministic directional bias dominates the
        // bounded (unit-magnitude) random-walk jitter. The keyed-stateless RNG
        // (#376) makes the jitter a fixed per-agent vector, so the test must
        // ensure chemotaxis clearly outweighs it rather than relying on a
        // particular seed's jitter happening to point the right way.
        let run = |mob: f32| -> f32 {
            let mover = TraitVector {
                heterotrophy: 5.0,
                mobility: mob,
                ..zero_traits()
            };
            let mut agents = vec![
                make_agent(0, (0.0, 0.0), 100.0, mover),
                make_agent(1, (1.0, 0.0), 100.0, target_traits),
            ];
            agents[1].structure = 5.0;
            let carcasses = vec![];
            let mut grid = crate::spatial::SpatialGrid::new(100.0, 10.0);
            grid.insert(0, (0.0, 0.0));
            grid.insert(1, (1.0, 0.0));

            let _ = move_agents(&mut agents, &carcasses, &grid, &params, 0, 0);
            agents[0].position.0 // x position after move
        };

        let low_mob_x = run(0.1);
        let high_mob_x = run(1.0);
        // Higher mobility -> larger movement distance AND stronger chemotaxis bias
        assert!(
            high_mob_x > low_mob_x,
            "higher mobility should move further toward target: low={}, high={}",
            low_mob_x,
            high_mob_x
        );
    }

    // --- Trait vector dimensions ---

    #[test]
    fn trait_vector_has_six_dimensions() {
        assert_eq!(TraitVector::NUM_DIMS, 7);
    }

    #[test]
    fn functional_trait_count_is_three() {
        assert_eq!(crate::FUNCTIONAL_TRAIT_COUNT, 3);
        assert_eq!(crate::FUNCTIONAL_TRAIT_INDICES, [0, 1, 2]);
    }

    // --- Asexual reproduction tests ---

    #[test]
    fn asexual_reproduction_succeeds_with_high_propensity() {
        // An agent with asexual_propensity=1.0 should always reproduce asexually.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 5.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // Should have reproduced
        assert!(
            !result.events.is_empty(),
            "should produce reproduction event"
        );
        assert_eq!(result.events[0].kind, EventKind::Reproduced);
        // Asexual: target is None
        assert_eq!(
            result.events[0].target, None,
            "asexual reproduction should have target=None"
        );
        assert_eq!(result.events[0].source, 1);
        // Should have offspring
        assert!(!result.offspring.is_empty(), "should produce offspring");
        // Parent's repro_reserve should be depleted
        assert!(
            agents[0].repro_reserve < 1e-6,
            "parent repro_reserve should be depleted, got {}",
            agents[0].repro_reserve
        );
    }

    #[test]
    fn asexual_offspring_have_parent_traits_no_crossover() {
        // With mutation_rate=0, asexual offspring should have exact parent traits.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.7,
            heterotrophy: 0.2,
            mobility: 0.1,
            kappa: 0.6,
            fecundity: 5.0,
            asexual_propensity: 1.0,
            dispersal: 0.0,
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(
                child.traits.photosynthetic_absorption,
                traits.photosynthetic_absorption
            );
            assert_eq!(child.traits.heterotrophy, traits.heterotrophy);
            assert_eq!(child.traits.mobility, traits.mobility);
            assert_eq!(child.traits.kappa, traits.kappa);
            assert_eq!(child.traits.asexual_propensity, traits.asexual_propensity);
        }
    }

    #[test]
    fn asexual_failure_falls_through_to_sexual() {
        // Two eligible agents with asexual_propensity=0.0 should use sexual reproduction.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: 20.0,
            },
            Agent {
                id: 2,
                position: (1.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        // Should have sexual reproduction event with target=Some(mate_id)
        assert!(
            !result.events.is_empty(),
            "should produce reproduction event"
        );
        assert!(
            result.events[0].target.is_some(),
            "sexual reproduction should have target=Some(mate_id)"
        );
        assert!(!result.offspring.is_empty(), "should produce offspring");
    }

    #[test]
    fn asexual_reproduction_energy_from_single_parent() {
        // Verify that asexual reproduction draws from single parent's repro_reserve only.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.5,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        let initial_repro = 30.0;
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: initial_repro,
            repro_nutrient: 10.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        // Parent's repro_reserve should be 0
        assert!(
            agents[0].repro_reserve.abs() < 1e-6,
            "parent repro_reserve should be depleted"
        );
        // Total offspring energy + dissipated = initial investment
        let offspring_energy: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let total = offspring_energy + result.dissipated;
        assert!(
            (total - initial_repro).abs() < 1e-3,
            "offspring energy ({}) + dissipated ({}) should equal investment ({})",
            offspring_energy,
            result.dissipated,
            initial_repro
        );
        // Event energy_delta should equal the investment
        assert!((result.events[0].energy_delta - initial_repro).abs() < 1e-3);
    }

    /// Helper: run a single-parent asexual reproduction with the given dispersal
    /// trait and propagule-cost coefficient, returning total energy provisioned
    /// into offspring (reserve + structure).
    fn asexual_offspring_energy(dispersal: f32, propagule_coeff: f32) -> f32 {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            growth_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            dispersal_propagule_cost_coefficient: propagule_coeff,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 3.0,
            asexual_propensity: 1.0,
            dispersal,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            // Funded above the #310 viability gate (fecundity 3 × generalist
            // death threshold), identical across dispersal values so the
            // propagule-cost comparison stays valid.
            repro_reserve: 400.0,
            repro_nutrient: 40.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);
        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);
        assert!(
            !result.offspring.is_empty(),
            "fixture should produce offspring"
        );
        result
            .offspring
            .iter()
            .map(|c| c.reserve + c.structure)
            .sum()
    }

    #[test]
    fn higher_dispersal_reduces_offspring_provisioning() {
        // With a propagule-cost coefficient active, an agent with higher dispersal
        // spends more of its reproductive budget on propagule structures, leaving
        // less to provision offspring.
        let low = asexual_offspring_energy(0.5, 0.5);
        let high = asexual_offspring_energy(1.0, 0.5);
        assert!(
            high < low,
            "higher dispersal should provision less offspring energy: low={low}, high={high}"
        );
    }

    /// Helper: run a sexual reproduction between two identical parents with the
    /// given dispersal trait and propagule coefficient; returns total offspring
    /// energy provisioned (reserve + structure).
    fn sexual_offspring_energy(dispersal: f32, propagule_coeff: f32) -> f32 {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            growth_efficiency: 1.0,
            reproductive_compatibility_distance: 5.0,
            sensing_range_coefficient: 50.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            dispersal_propagule_cost_coefficient: propagule_coeff,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.5,
            kappa: 0.5,
            fecundity: 3.0,
            asexual_propensity: 0.0, // force sexual
            dispersal,
            ..zero_traits()
        };
        let mk = |id: u64, pos: (f32, f32)| Agent {
            id,
            position: pos,
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            // Funded above the #310 viability gate, identical across dispersal
            // values so the propagule-cost comparison stays valid.
            repro_reserve: 400.0,
            repro_nutrient: 40.0,
        };
        let mut agents = vec![mk(1, (0.0, 0.0)), mk(2, (1.0, 0.0))];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);
        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);
        assert!(
            !result.offspring.is_empty(),
            "fixture should produce offspring"
        );
        result
            .offspring
            .iter()
            .map(|c| c.reserve + c.structure)
            .sum()
    }

    #[test]
    fn higher_dispersal_reduces_sexual_offspring_provisioning() {
        // The propagule cost applies on the sexual path too, keyed on the
        // parents' average dispersal trait.
        let low = sexual_offspring_energy(0.5, 0.5);
        let high = sexual_offspring_energy(1.0, 0.5);
        assert!(
            high < low,
            "higher dispersal should provision less sexual offspring energy: low={low}, high={high}"
        );
    }

    #[test]
    fn dispersal_adds_no_per_tick_maintenance_cost() {
        // A non-reproducing agent's metabolism must be independent of its
        // dispersal value, even with the propagule-cost coefficient active.
        // Dispersal is a reproduction-event cost, never a standing maintenance cost.
        let params = WorldParameters {
            maintenance_cost_exponent: 2.0,
            dispersal_propagule_cost_coefficient: 1.0,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let mut low = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                dispersal: 0.0,
                ..zero_traits()
            },
        )];
        let mut high = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                dispersal: 5.0,
                ..zero_traits()
            },
        )];
        let (_, lost_low) = metabolise(&mut low, &params);
        let (_, lost_high) = metabolise(&mut high, &params);
        assert!(
            (lost_low - lost_high).abs() < 1e-6,
            "metabolism must not depend on dispersal: low={lost_low}, high={lost_high}"
        );
        assert!(
            (low[0].reserve - high[0].reserve).abs() < 1e-6,
            "reserve after metabolise must not depend on dispersal"
        );
    }

    #[test]
    fn dispersal_propagule_cost_is_superlinear() {
        // The provisioning reduction must grow faster than linearly in dispersal.
        // Baseline (no cost) provisioning, then the lost energy at dispersal d and 2d.
        let baseline = asexual_offspring_energy(0.0, 0.5);
        let lost_at_d = baseline - asexual_offspring_energy(0.5, 0.5);
        let lost_at_2d = baseline - asexual_offspring_energy(1.0, 0.5);
        assert!(
            lost_at_d > 0.0 && lost_at_2d > 0.0,
            "cost should be positive"
        );
        // Superlinear: doubling dispersal more than doubles the lost energy.
        assert!(
            lost_at_2d > 2.0 * lost_at_d,
            "doubling dispersal should more than double the cost: lost_d={lost_at_d}, lost_2d={lost_at_2d}"
        );
    }

    #[test]
    fn zero_asexual_propensity_never_reproduces_alone() {
        // An agent with asexual_propensity=0.0 and no mate should not reproduce.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            result.events.is_empty(),
            "should not reproduce without mate or asexual propensity"
        );
        assert!(result.offspring.is_empty());
        assert!(
            (agents[0].repro_reserve - 20.0).abs() < 1e-6,
            "repro_reserve should be unchanged"
        );
    }

    #[test]
    fn asexual_propensity_is_heritable() {
        // Offspring of asexual reproduction should inherit asexual_propensity.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 5.0,
            asexual_propensity: 0.8,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(
                child.traits.asexual_propensity, 0.8,
                "offspring should inherit parent's asexual_propensity"
            );
        }
    }

    #[test]
    fn sexual_offspring_use_crossover_not_clone() {
        // Two parents with different traits: sexual offspring should have mixed traits
        // (from crossover), not a clone of either parent.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };
        let traits_a = TraitVector {
            photosynthetic_absorption: 1.0,
            heterotrophy: 0.0,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let traits_b = TraitVector {
            photosynthetic_absorption: 0.0,
            heterotrophy: 1.0,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 1.0,
            asexual_propensity: 0.0,
            dispersal: 0.0,
        };
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits: traits_a,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: 20.0,
            },
            Agent {
                id: 2,
                position: (1.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits: traits_b,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: 20.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 1, 0);

        assert!(!result.offspring.is_empty());
        // Sexual event should have target
        assert!(result.events[0].target.is_some());
        // At least one offspring should have traits from both parents
        // (with crossover each dim is independently chosen from one parent)
        for child in &result.offspring {
            let photo = child.traits.photosynthetic_absorption;
            let hetero = child.traits.heterotrophy;
            // Each should be either 0.0 or 1.0 (from one parent, no mutation)
            assert!(
                photo == 0.0 || photo == 1.0,
                "photo should be from one parent: {}",
                photo
            );
            assert!(
                hetero == 0.0 || hetero == 1.0,
                "hetero should be from one parent: {}",
                hetero
            );
        }
    }

    #[test]
    fn asexual_nutrient_from_single_parent() {
        // Verify nutrient donation comes from single parent in asexual reproduction.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 5.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        // Donation comes from the reproductive-nutrient earmark, not the free
        // store. The free store is left untouched by reproduction.
        let initial_free = 50.0;
        let initial_earmark = 20.0;
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: initial_free,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: initial_earmark,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            !result.offspring.is_empty(),
            "parent should have reproduced"
        );
        // The free store is untouched by reproduction.
        assert!(
            (agents[0].nutrient - initial_free).abs() < 1e-6,
            "free store should be untouched, got {}",
            agents[0].nutrient
        );
        // The entire earmark is donated to offspring, split by count.
        assert!(
            agents[0].repro_nutrient.abs() < 1e-6,
            "parent earmark should be depleted, got {}",
            agents[0].repro_nutrient
        );
        // Offspring receive the donated earmark as their free nutrient store.
        let offspring_nutrient: f32 = result.offspring.iter().map(|c| c.nutrient).sum();
        assert!(
            (offspring_nutrient - initial_earmark).abs() < 1e-3,
            "offspring should collectively receive the donated earmark {}, got {}",
            initial_earmark,
            offspring_nutrient
        );
        // Per-offspring share is equal.
        let n = result.offspring.len() as f32;
        for child in &result.offspring {
            assert!(
                (child.nutrient - initial_earmark / n).abs() < 1e-3,
                "each offspring should get an equal earmark share"
            );
            assert!(
                child.repro_nutrient.abs() < 1e-6,
                "offspring born with zero earmark, got {}",
                child.repro_nutrient
            );
        }
    }

    #[test]
    fn sexual_nutrient_donated_from_both_earmarks() {
        // Both parents donate their entire reproductive-nutrient earmark; the
        // combined donation is split equally among offspring. Free stores are
        // untouched.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            reproductive_compatibility_distance: 10.0,
            mutation_rate: 0.0,
            asexual_propensity_maintenance_cost: 0.0,
            dispersal_propagule_cost_coefficient: 0.0,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 4.0,
            asexual_propensity: 0.0, // force sexual path
            ..zero_traits()
        };
        let earmark_a = 12.0;
        let earmark_b = 8.0;
        let free_store = 50.0;
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: free_store,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: earmark_a,
            },
            Agent {
                id: 2,
                position: (1.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: free_store,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 20.0,
                repro_nutrient: earmark_b,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(
            !result.offspring.is_empty(),
            "the pair should have reproduced"
        );
        // Free stores untouched.
        assert!(
            (agents[0].nutrient - free_store).abs() < 1e-6,
            "parent A free store changed"
        );
        assert!(
            (agents[1].nutrient - free_store).abs() < 1e-6,
            "parent B free store changed"
        );
        // Both earmarks fully donated.
        assert!(
            agents[0].repro_nutrient.abs() < 1e-6,
            "parent A earmark not depleted"
        );
        assert!(
            agents[1].repro_nutrient.abs() < 1e-6,
            "parent B earmark not depleted"
        );
        // Offspring collectively receive both earmarks.
        let offspring_nutrient: f32 = result.offspring.iter().map(|c| c.nutrient).sum();
        assert!(
            (offspring_nutrient - (earmark_a + earmark_b)).abs() < 1e-3,
            "offspring should receive both earmarks {}, got {}",
            earmark_a + earmark_b,
            offspring_nutrient
        );
    }

    // --- Dispersal trait tests ---

    #[test]
    fn dispersal_trait_controls_offspring_spread() {
        // Higher dispersal trait produces wider offspring placement.
        // Compare mean distance from parent position for low vs high dispersal.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };

        let run = |dispersal_val: f32| -> f32 {
            let traits = TraitVector {
                photosynthetic_absorption: 0.5,
                kappa: 0.5,
                fecundity: 10.0, // many offspring for statistical signal
                asexual_propensity: 1.0,
                dispersal: dispersal_val,
                ..zero_traits()
            };
            let mut agents = vec![Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 50.0,
                repro_nutrient: 50.0,
            }];
            let dead_ids = std::collections::HashSet::new();
            let grid = SpatialGrid::new(100.0, 10.0);

            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);
            // Mean distance from parent position (0,0)
            let total_dist: f32 = result
                .offspring
                .iter()
                .map(|o| (o.position.0 * o.position.0 + o.position.1 * o.position.1).sqrt())
                .sum();
            total_dist / result.offspring.len() as f32
        };

        let low_spread = run(0.5);
        let high_spread = run(5.0);
        assert!(
            high_spread > low_spread,
            "higher dispersal should produce wider spread: low={}, high={}",
            low_spread,
            high_spread
        );
    }

    #[test]
    fn zero_dispersal_places_offspring_at_parent_position() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 5.0,
            asexual_propensity: 1.0,
            dispersal: 0.0, // zero dispersal
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (10.0, 10.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert!(
                (child.position.0 - 10.0).abs() < 1e-6,
                "zero dispersal: offspring should be at parent x, got {}",
                child.position.0
            );
            assert!(
                (child.position.1 - 10.0).abs() < 1e-6,
                "zero dispersal: offspring should be at parent y, got {}",
                child.position.1
            );
        }
    }

    #[test]
    fn sexual_reproduction_scatter_widens_with_dispersal() {
        // Seed-parent placement (issue #283): offspring are scattered by the
        // seed parent's own dispersal kernel. With both parents sharing the same
        // dispersal trait, a wider trait must produce a wider scatter regardless
        // of which parent is seeded.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            contact_range_coefficient: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };

        let run = |disp_a: f32, disp_b: f32| -> f32 {
            let traits_a = TraitVector {
                photosynthetic_absorption: 0.5,
                mobility: 0.3,
                kappa: 0.5,
                fecundity: 10.0,
                asexual_propensity: 0.0,
                dispersal: disp_a,
                ..zero_traits()
            };
            let traits_b = TraitVector {
                photosynthetic_absorption: 0.5,
                mobility: 0.3,
                kappa: 0.5,
                fecundity: 10.0,
                asexual_propensity: 0.0,
                dispersal: disp_b,
                ..zero_traits()
            };
            let mut agents = vec![
                Agent {
                    id: 1,
                    position: (0.0, 0.0),
                    reserve: 50.0,
                    structure: 5.0,
                    peak_structure: 5.0,
                    nutrient: 100.0,
                    traits: traits_a,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                    repro_reserve: 30.0,
                    repro_nutrient: 30.0,
                },
                Agent {
                    id: 2,
                    position: (1.0, 0.0),
                    reserve: 50.0,
                    structure: 5.0,
                    peak_structure: 5.0,
                    nutrient: 100.0,
                    traits: traits_b,
                    wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                    repro_reserve: 30.0,
                    repro_nutrient: 30.0,
                },
            ];
            let dead_ids = std::collections::HashSet::new();
            let mut grid = SpatialGrid::new(100.0, 10.0);
            grid.insert(0, agents[0].position);
            grid.insert(1, agents[1].position);

            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);
            // Scatter is measured from whichever parent each offspring is
            // closest to — the seed anchor is a real parent position, so the
            // distance to the nearest parent is the scatter magnitude.
            let parents = [(0.0_f32, 0.0_f32), (1.0_f32, 0.0_f32)];
            let total_dist: f32 = result
                .offspring
                .iter()
                .map(|o| {
                    parents
                        .iter()
                        .map(|p| {
                            let dx = o.position.0 - p.0;
                            let dy = o.position.1 - p.1;
                            (dx * dx + dy * dy).sqrt()
                        })
                        .fold(f32::INFINITY, f32::min)
                })
                .sum();
            total_dist / result.offspring.len().max(1) as f32
        };

        // Both parents share dispersal 2.0.
        let narrow_spread = run(2.0, 2.0);
        // Both parents share dispersal 4.0.
        let wide_spread = run(4.0, 4.0);

        assert!(
            wide_spread > narrow_spread,
            "wider dispersal should produce wider scatter: narrow={}, wide={}",
            narrow_spread,
            wide_spread
        );
    }

    #[test]
    fn sexual_distant_parents_place_offspring_at_a_parent_not_the_midpoint() {
        // Two spatially distant but trait-compatible parents pair via gamete
        // broadcast (reproductive reach, #285). With zero dispersal, each
        // offspring should originate at one of the two parents' positions —
        // never in the empty space between them (the old midpoint behaviour).
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            sensing_range_coefficient: 2000.0, // mobility 0.3 -> reach 600 > gap 300
            reproductive_compatibility_distance: 10.0,
            world_extent: 1000.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 10.0,
            asexual_propensity: 0.0,
            dispersal: 0.0, // zero dispersal: offspring land exactly on the seed parent
            ..zero_traits()
        };
        let a_pos = (0.0, 0.0);
        let b_pos = (300.0, 0.0); // far apart; midpoint would be (150, 0)
        let mut agents = vec![
            Agent {
                id: 1,
                position: a_pos,
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 30.0,
                repro_nutrient: 30.0,
            },
            Agent {
                id: 2,
                position: b_pos,
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 30.0,
                repro_nutrient: 30.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(1000.0, 10.0);
        grid.insert(0, a_pos);
        grid.insert(1, b_pos);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");
        for child in &result.offspring {
            let at_a = (child.position.0 - a_pos.0).abs() < 1e-6
                && (child.position.1 - a_pos.1).abs() < 1e-6;
            let at_b = (child.position.0 - b_pos.0).abs() < 1e-6
                && (child.position.1 - b_pos.1).abs() < 1e-6;
            assert!(
                at_a || at_b,
                "offspring should originate at a parent's position, not the \
                 midpoint; got {:?} (parents at {:?} and {:?})",
                child.position,
                a_pos,
                b_pos
            );
        }
    }

    #[test]
    fn sexual_colocated_parents_place_offspring_near_both() {
        // When the two parents share (effectively) the same position, seed-parent
        // placement is indistinguishable from the old midpoint behaviour: with
        // zero dispersal every offspring lands on that shared spot — near both
        // parents (issue #283 acceptance: co-located parents unchanged).
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            sensing_range_coefficient: 100.0,
            reproductive_compatibility_distance: 10.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.3,
            kappa: 0.5,
            fecundity: 10.0,
            asexual_propensity: 0.0,
            dispersal: 0.0, // zero dispersal: offspring land on the shared spot
            ..zero_traits()
        };
        let shared = (20.0, 20.0);
        let mut agents = vec![
            Agent {
                id: 1,
                position: shared,
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 30.0,
                repro_nutrient: 30.0,
            },
            Agent {
                id: 2,
                position: shared,
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 30.0,
                repro_nutrient: 30.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, shared);
        grid.insert(1, shared);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");
        for child in &result.offspring {
            assert!(
                (child.position.0 - shared.0).abs() < 1e-6
                    && (child.position.1 - shared.1).abs() < 1e-6,
                "co-located parents: offspring should land on the shared spot {:?}, got {:?}",
                shared,
                child.position
            );
        }
    }

    #[test]
    fn dispersal_is_independent_of_mobility() {
        // A sessile agent (zero mobility) with high dispersal should still
        // disperse offspring widely. This tests independence.
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 10.0,
            asexual_propensity: 1.0,
            mobility: 0.0,  // sessile
            dispersal: 5.0, // high dispersal (like a dandelion)
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 50.0,
            repro_nutrient: 50.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty());
        // Offspring should be dispersed despite zero mobility
        let total_dist: f32 = result
            .offspring
            .iter()
            .map(|o| (o.position.0 * o.position.0 + o.position.1 * o.position.1).sqrt())
            .sum();
        let mean_dist = total_dist / result.offspring.len() as f32;
        assert!(
            mean_dist > 0.5,
            "sessile agent with high dispersal should still disperse offspring: mean_dist={}",
            mean_dist
        );
    }

    #[test]
    fn dispersal_is_heritable_asexual() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 1.0,
            mutation_rate: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 5.0,
            asexual_propensity: 1.0,
            dispersal: 3.7,
            ..zero_traits()
        };
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: 20.0,
            repro_nutrient: 20.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty());
        for child in &result.offspring {
            assert_eq!(
                child.traits.dispersal, 3.7,
                "offspring should inherit parent's dispersal trait"
            );
        }
    }

    #[test]
    fn sexual_reproduction_seed_parent_scatter_wraps_on_torus() {
        // A seed parent sitting right at the wrap edge, scattering offspring
        // with a dispersal kernel wide enough to push some across the seam,
        // must produce offspring wrapped to within the world bounds — not
        // placed off the edge of the torus (issue #283).
        let extent = 100.0_f32;
        let params = WorldParameters {
            reproduction_efficiency: 1.0,
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            sensing_range_coefficient: 2000.0, // wide reach so the distant pair forms
            reproductive_compatibility_distance: 10.0,
            world_extent: extent,
            ..test_params()
        };
        // Both parents sit at the +x edge so whichever is seeded, scatter
        // straddles the wrap seam.
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 1.0,
            kappa: 0.5,
            fecundity: 20.0, // many offspring so scatter explores the seam
            dispersal: 5.0,  // wide enough to cross the edge
            ..zero_traits()
        };
        let mut agents = vec![
            make_agent(1, (49.0, 0.0), 100.0, traits),
            make_agent(2, (48.0, 30.0), 100.0, traits),
        ];
        for a in agents.iter_mut() {
            a.nutrient = 10.0;
            a.repro_reserve = 15.0;
            a.repro_nutrient = 15.0;
        }

        let dead_ids: std::collections::HashSet<u64> = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, agents[0].position);
        grid.insert(1, agents[1].position);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");
        let half = extent / 2.0;
        for child in &result.offspring {
            // Positions on a [-50, 50] torus must stay within bounds.
            assert!(
                child.position.0 >= -half - 1e-3 && child.position.0 <= half + 1e-3,
                "offspring x={} must be wrapped within [-50, 50]",
                child.position.0
            );
            assert!(
                child.position.1 >= -half - 1e-3 && child.position.1 <= half + 1e-3,
                "offspring y={} must be wrapped within [-50, 50]",
                child.position.1
            );
        }
        // Sanity: at least one offspring actually landed on the far side of the
        // seam (negative x), proving wrapping occurred rather than clamping.
        assert!(
            result.offspring.iter().any(|c| c.position.0 < 0.0),
            "expected some offspring to wrap across the +x edge to negative x"
        );
    }

    #[test]
    fn trait_vector_has_seven_dimensions() {
        assert_eq!(TraitVector::NUM_DIMS, 7);
        // dispersal is at index 6
        let traits = TraitVector {
            dispersal: 2.5,
            ..zero_traits()
        };
        assert_eq!(traits.get(6), 2.5);
    }

    #[test]
    fn superlinear_maintenance_quadratic_pays_4x_for_double_trait() {
        // With exponent=2.0, trait=1.0 pays 1.0^2 * cost = cost
        // while trait=0.5 pays 0.5^2 * cost = 0.25 * cost → 4x ratio.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 0.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };

        let mut agents_full = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
        )];
        let mut agents_half = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut agents_full, &params);
        let (_, _) = metabolise(&mut agents_half, &params);

        let cost_full = 100.0 - agents_full[0].reserve;
        let cost_half = 100.0 - agents_half[0].reserve;

        assert!(
            (cost_full / cost_half - 4.0).abs() < 1e-6,
            "trait=1.0 should pay 4x trait=0.5 under quadratic costs, got ratio {}",
            cost_full / cost_half
        );
    }

    #[test]
    fn superlinear_maintenance_exponent_one_is_linear_regression() {
        // With exponent=1.0 (default), trait=1.0 pays exactly 2x what trait=0.5 pays.
        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 0.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 1.0,
            ..test_params()
        };

        let mut agents_full = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                ..zero_traits()
            },
        )];
        let mut agents_half = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut agents_full, &params);
        let (_, _) = metabolise(&mut agents_half, &params);

        let cost_full = 100.0 - agents_full[0].reserve;
        let cost_half = 100.0 - agents_half[0].reserve;

        assert!(
            (cost_full / cost_half - 2.0).abs() < 1e-6,
            "exponent=1.0 should give linear 2x ratio, got {}",
            cost_full / cost_half
        );
    }

    #[test]
    fn superlinear_maintenance_specialist_pays_more_per_trait_than_generalist() {
        // Under quadratic costs with equal total trait sum:
        //   generalist (0.5, 0.5): 0.5^2 + 0.5^2 = 0.50 total cost
        //   specialist (1.0, 0.0): 1.0^2 + 0.0^2 = 1.00 total cost
        // The specialist pays more in absolute maintenance. The anti-generalist
        // mechanism works because each trait's *output* is proportional to its
        // value (linear returns), but its *cost* grows superlinearly — so the
        // specialist's single high trait is more cost-effective per unit of
        // capability (1.0 capability for 1.0 cost) than the generalist's two
        // moderate traits would need to be to match the specialist's single
        // capability. The trade-off is economic: you pay dearly for a high
        // trait, but you get full capability; spreading investment is cheaper
        // in maintenance but yields less capability per dimension.

        let params = WorldParameters {
            base_metabolic_rate: 0.0,
            photo_maintenance_cost: 1.0,
            heterotrophy_maintenance_cost: 1.0,
            mobility_maintenance_cost: 0.0,
            structure_maintenance_coefficient: 0.0,
            maintenance_cost_exponent: 2.0,
            ..test_params()
        };

        // Generalist: 0.5 in both photo and heterotrophy
        let mut generalist = vec![make_agent(
            1,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 0.5,
                heterotrophy: 0.5,
                ..zero_traits()
            },
        )];
        // Specialist: 1.0 in photo, 0.0 in heterotrophy
        let mut specialist = vec![make_agent(
            2,
            (0.0, 0.0),
            100.0,
            TraitVector {
                photosynthetic_absorption: 1.0,
                heterotrophy: 0.0,
                ..zero_traits()
            },
        )];

        let (_, _) = metabolise(&mut generalist, &params);
        let (_, _) = metabolise(&mut specialist, &params);

        let cost_generalist = 100.0 - generalist[0].reserve;
        let cost_specialist = 100.0 - specialist[0].reserve;

        // Under quadratic costs:
        // generalist: 0.5^2 * 1.0 + 0.5^2 * 1.0 = 0.25 + 0.25 = 0.5
        // specialist: 1.0^2 * 1.0 + 0.0^2 * 1.0 = 1.0 + 0.0 = 1.0
        // Specialist actually pays MORE — superlinear costs penalise
        // concentration, making specialisation expensive per unit capability.
        // This is the correct mathematical result.
        assert!(
            cost_specialist > cost_generalist,
            "under quadratic costs, specialist (1.0,0.0) should pay more ({}) \
             than generalist (0.5,0.5) ({}) — superlinear costs penalise \
             concentrated investment",
            cost_specialist,
            cost_generalist
        );
        // Verify exact values
        assert!((cost_generalist - 0.5).abs() < 1e-6);
        assert!((cost_specialist - 1.0).abs() < 1e-6);
    }

    #[test]
    fn drain_demand_only_proportional_split() {
        // Two consumers with equal raw demand (heterotrophy=1.0) but different
        // trait distances to the target. The split is by demand only; trophic
        // efficiency applies after the split to determine energy gained.
        let mut params = test_params();
        params.base_trophic_efficiency = 1.0;
        params.trophic_distance_decay = 1.0;

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let consumer_a_traits = TraitVector {
            heterotrophy: 1.0,
            photosynthetic_absorption: 0.8,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.0,
            mobility: 2.0,
            ..zero_traits()
        };

        let eff_a = crate::trophic_transfer_efficiency(&consumer_a_traits, &target_traits, &params);
        let eff_b = crate::trophic_transfer_efficiency(&consumer_b_traits, &target_traits, &params);
        assert!(eff_a > eff_b, "sanity: A should be more efficient than B");

        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_a_traits),
            make_agent(2, (1.0, 0.0), 0.0, consumer_b_traits),
            make_agent(3, (0.5, 0.0), 0.0, target_traits),
        ];
        agents[2].structure = 1.0;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let _result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        // Equal demand → equal drain (0.5 each). Energy = drain * trophic_eff.
        // Ratio of energy gained = eff_a / eff_b.
        let a_gained = agents[0].reserve;
        let b_gained = agents[1].reserve;
        let actual_ratio = a_gained / b_gained;
        let expected_ratio = eff_a / eff_b;

        assert!(
            (actual_ratio - expected_ratio).abs() < 0.01,
            "energy ratio should be eff_a/eff_b = {expected_ratio:.4}, got {actual_ratio:.4}"
        );
    }

    #[test]
    fn drain_trophic_weighted_split_conserves_energy() {
        // With trophic-weighted split, total energy drained from target must
        // equal energy gained by consumers plus dissipated energy.
        let mut params = test_params();
        params.base_trophic_efficiency = 0.8;
        params.trophic_distance_decay = 2.0;

        let target_traits = TraitVector {
            photosynthetic_absorption: 1.0,
            ..zero_traits()
        };
        let consumer_a_traits = TraitVector {
            heterotrophy: 2.0,
            photosynthetic_absorption: 0.5,
            ..zero_traits()
        };
        let consumer_b_traits = TraitVector {
            heterotrophy: 1.5,
            mobility: 1.0,
            ..zero_traits()
        };

        let initial_structure = 2.0;
        let mut agents = vec![
            make_agent(1, (0.0, 0.0), 0.0, consumer_a_traits),
            make_agent(2, (1.0, 0.0), 0.0, consumer_b_traits),
            make_agent(3, (0.5, 0.0), 0.0, target_traits),
        ];
        agents[2].structure = initial_structure;

        let mut carcasses: Vec<Carcass> = Vec::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (1.0, 0.0));
        grid.insert(2, (0.5, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        let result = resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        let total_drained = initial_structure - agents[2].structure;
        let total_gained = agents[0].reserve + agents[1].reserve;
        let total_out = total_gained + result.dissipated;

        assert!(total_drained > 0.0, "something should have been drained");
        assert!(
            (total_drained - total_out).abs() < 1e-5,
            "energy conservation: drained {total_drained} != gained {total_gained} + dissipated {}",
            result.dissipated
        );
    }

    /// Conservation equation for asexual reproduction with structure provisioning:
    ///
    ///   parent_committed_investment
    ///     = sum(offspring.reserve)
    ///     + sum(offspring.structure)
    ///     + result.dissipated
    ///
    /// where `result.dissipated` accounts for both the reproduction-efficiency
    /// loss (parents → offspring transfer) and the growth-efficiency loss
    /// (offspring energy → offspring structure conversion).
    #[test]
    fn asexual_reproduction_conserves_energy_with_structure_provisioning() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            growth_efficiency: 0.8,
            offspring_structure_fraction: 0.25,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 3.0,
            asexual_propensity: 1.0,
            ..zero_traits()
        };
        // Funded above the #310 viability gate (fecundity 3 × death threshold).
        let initial_repro_reserve = 120.0_f32;
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: initial_repro_reserve,
            repro_nutrient: 10.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");

        let offspring_reserve_sum: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let offspring_structure_sum: f32 = result.offspring.iter().map(|c| c.structure).sum();

        // Every offspring must be born with strictly positive structure.
        for child in &result.offspring {
            assert!(
                child.structure > 0.0,
                "newborn structure must be > 0, got {}",
                child.structure
            );
            assert!(
                child.reserve > 0.0,
                "newborn reserve must be > 0, got {}",
                child.reserve
            );
        }

        let committed = initial_repro_reserve;
        let accounted = offspring_reserve_sum + offspring_structure_sum + result.dissipated;
        assert!(
            (committed - accounted).abs() < 1e-4,
            "energy conservation: committed {committed} != reserve {offspring_reserve_sum} + structure {offspring_structure_sum} + dissipated {} (sum {accounted})",
            result.dissipated
        );
    }

    /// Energy conservation must still hold when the dispersal propagule cost is
    /// active: the energy spent on propagule structures dissipates rather than
    /// vanishing, so committed == reserve + structure + dissipated.
    #[test]
    fn dispersal_propagule_cost_conserves_energy() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.7,
            growth_efficiency: 0.8,
            offspring_structure_fraction: 0.25,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            dispersal_propagule_cost_coefficient: 0.5,
            dispersal_propagule_cost_exponent: 2.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            kappa: 0.5,
            fecundity: 3.0,
            asexual_propensity: 1.0,
            dispersal: 1.0, // active propagule cost
            ..zero_traits()
        };
        // Funded above the #310 viability gate (fecundity 3, dispersal 1.0 with
        // an active propagule cost, so the gate is high).
        let initial_repro_reserve = 300.0_f32;
        let mut agents = vec![Agent {
            id: 1,
            position: (0.0, 0.0),
            reserve: 50.0,
            structure: 5.0,
            peak_structure: 5.0,
            nutrient: 100.0,
            traits,
            wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
            repro_reserve: initial_repro_reserve,
            repro_nutrient: 10.0,
        }];
        let dead_ids = std::collections::HashSet::new();
        let grid = SpatialGrid::new(100.0, 10.0);

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");
        let offspring_reserve_sum: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let offspring_structure_sum: f32 = result.offspring.iter().map(|c| c.structure).sum();
        let committed = initial_repro_reserve;
        let accounted = offspring_reserve_sum + offspring_structure_sum + result.dissipated;
        assert!(
            (committed - accounted).abs() < 1e-4,
            "energy conservation with propagule cost: committed {committed} != reserve {offspring_reserve_sum} + structure {offspring_structure_sum} + dissipated {} (sum {accounted})",
            result.dissipated
        );
        // Sanity: the propagule cost actually bit (dissipated exceeds the bare
        // reproduction-efficiency loss).
        let efficiency_loss = initial_repro_reserve * (1.0 - params.reproduction_efficiency);
        assert!(
            result.dissipated > efficiency_loss + 1e-3,
            "propagule cost should add to dissipation beyond the efficiency loss"
        );
    }

    /// Sexual reproduction must observe the same conservation equation:
    ///
    ///   invest_a + invest_b
    ///     = sum(offspring.reserve)
    ///     + sum(offspring.structure)
    ///     + result.dissipated
    #[test]
    fn sexual_reproduction_conserves_energy_with_structure_provisioning() {
        let params = WorldParameters {
            reproduction_energy_threshold: 10.0,
            reproduction_nutrient_threshold: 1.0,
            reproduction_efficiency: 0.6,
            growth_efficiency: 0.5,
            offspring_structure_fraction: 0.3,
            reproductive_compatibility_distance: 5.0,
            mutation_rate: 0.0,
            mutation_magnitude: 0.0,
            sensing_range_coefficient: 50.0,
            ..test_params()
        };
        let traits = TraitVector {
            photosynthetic_absorption: 0.5,
            mobility: 0.5,
            kappa: 0.5,
            fecundity: 4.0,
            asexual_propensity: 0.0, // force sexual
            ..zero_traits()
        };
        // Funded above the #310 viability gate (fecundity 4 × death threshold).
        let invest_a = 160.0_f32;
        let invest_b = 140.0_f32;
        let mut agents = vec![
            Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: invest_a,
                repro_nutrient: 10.0,
            },
            Agent {
                id: 2,
                position: (1.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 100.0,
                traits,
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: invest_b,
                repro_nutrient: 10.0,
            },
        ];
        let dead_ids = std::collections::HashSet::new();
        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(1, (0.0, 0.0));
        grid.insert(2, (1.0, 0.0));

        let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, 0, 0);

        assert!(!result.offspring.is_empty(), "should produce offspring");
        for child in &result.offspring {
            assert!(child.structure > 0.0, "newborn structure > 0");
            assert!(child.reserve > 0.0, "newborn reserve > 0");
        }

        let offspring_reserve_sum: f32 = result.offspring.iter().map(|c| c.reserve).sum();
        let offspring_structure_sum: f32 = result.offspring.iter().map(|c| c.structure).sum();
        let committed = invest_a + invest_b;
        let accounted = offspring_reserve_sum + offspring_structure_sum + result.dissipated;
        assert!(
            (committed - accounted).abs() < 1e-3,
            "sexual conservation: committed {committed} != reserve {offspring_reserve_sum} + structure {offspring_structure_sum} + dissipated {} (sum {accounted})",
            result.dissipated
        );
    }

    /// Per-offspring structure must scale inversely with fecundity: doubling
    /// the offspring count from the same parental investment must roughly
    /// halve per-offspring structure.
    #[test]
    fn per_offspring_structure_scales_inversely_with_offspring_count() {
        // Use mutation-free, deterministic setup. We fix the offspring count
        // by setting fecundity high enough that Poisson rounding is the same
        // across runs — easier: assert ratio across two configurations.
        fn run(fecundity: f32, seed: u64) -> (usize, f32) {
            let params = WorldParameters {
                reproduction_energy_threshold: 10.0,
                reproduction_nutrient_threshold: 1.0,
                reproduction_efficiency: 1.0,
                growth_efficiency: 1.0,
                offspring_structure_fraction: 0.5,
                mutation_rate: 0.0,
                mutation_magnitude: 0.0,
                ..test_params()
            };
            let traits = TraitVector {
                photosynthetic_absorption: 0.5,
                kappa: 0.5,
                fecundity,
                asexual_propensity: 1.0,
                ..zero_traits()
            };
            let mut agents = vec![Agent {
                id: 1,
                position: (0.0, 0.0),
                reserve: 50.0,
                structure: 5.0,
                peak_structure: 5.0,
                nutrient: 1000.0,
                traits,
                // Funded above the #310 viability gate for fecundity up to 20.
                wear: [0.0; FUNCTIONAL_TRAIT_COUNT],
                repro_reserve: 600.0,
                repro_nutrient: 100.0,
            }];
            let dead_ids = std::collections::HashSet::new();
            let grid = SpatialGrid::new(100.0, 10.0);
            let result = resolve_reproduction(&mut agents, &dead_ids, &grid, &params, seed, 0);
            let n = result.offspring.len();
            let per = if n > 0 {
                result.offspring[0].structure
            } else {
                0.0
            };
            (n, per)
        }

        let (n_low, per_low) = run(2.0, 1);
        let (n_high, per_high) = run(20.0, 1);

        assert!(n_low > 0 && n_high > 0, "both runs should yield offspring");
        assert!(
            n_high > n_low,
            "higher fecundity should yield more offspring (got {n_low} vs {n_high})"
        );

        // Total structure across offspring should be roughly equal between
        // the two configs (same investment, same efficiencies). Therefore
        // per-offspring structure ratio ~= offspring-count inverse ratio.
        let total_low = per_low * n_low as f32;
        let total_high = per_high * n_high as f32;
        // Identical investment and efficiencies → totals equal up to f32 noise.
        assert!(
            (total_low - total_high).abs() < 1e-3,
            "totals should match: {total_low} vs {total_high}"
        );
        assert!(
            per_high < per_low,
            "per-offspring structure should fall with fecundity (got {per_low} -> {per_high})"
        );
    }

    #[test]
    fn consumption_reach_grows_with_structure() {
        let mut params = test_params();
        params.contact_range_coefficient = 5.0;
        params.body_reach_coefficient = 2.0;
        let small = consumption_reach(1.0, 1.0, &params);
        let large = consumption_reach(1.0, 100.0, &params);
        assert!(
            large > small,
            "a larger body must reach farther ({small} -> {large})"
        );
    }

    #[test]
    fn consumption_reach_is_zero_for_non_heterotroph() {
        let mut params = test_params();
        params.contact_range_coefficient = 5.0;
        params.body_reach_coefficient = 2.0;
        // A non-heterotroph (eff_heterotrophy = 0) gains no foraging reach no
        // matter how large its body: the foraging body is the heterotrophic body.
        assert_eq!(consumption_reach(0.0, 100.0, &params), 0.0);
    }

    #[test]
    fn consumption_reach_reduces_to_contact_range_with_no_body_term() {
        let mut params = test_params();
        params.contact_range_coefficient = 5.0;
        params.body_reach_coefficient = 0.0;
        // With the body term disabled, reach is the historical
        // eff_heterotrophy * contact_range_coefficient — backward compatible.
        assert_eq!(consumption_reach(1.0, 100.0, &params), 5.0);
    }

    #[test]
    fn large_sessile_heterotroph_reaches_carcass_a_small_one_cannot() {
        // Two sessile (mobility 0) heterotrophs identical except body size, each
        // beside a carcass placed just beyond the contact-only reach. Only the
        // large-bodied one — its mycelium grown through the substrate — should
        // touch and drain its carcass.
        let mut params = test_params();
        params.contact_range_coefficient = 5.0;
        params.body_reach_coefficient = 2.0;

        let het = TraitVector {
            heterotrophy: 1.0,
            ..zero_traits()
        };
        // contact-only reach = 1.0 * 5.0 = 5.0; carcass sits at distance 8.0.
        // small body (structure 1) reach = 5 + 2*sqrt(1) = 7.0 < 8.0 (no touch)
        // large body (structure 9) reach = 5 + 2*sqrt(9) = 11.0 > 8.0 (touch)
        let mut small = make_agent(0, (0.0, 0.0), 10.0, het);
        small.structure = 1.0;
        let mut large = make_agent(1, (50.0, 0.0), 10.0, het);
        large.structure = 9.0;
        let mut agents = vec![small, large];

        let mut carcasses = vec![
            Carcass {
                id: 100,
                position: (8.0, 0.0),
                energy: 20.0,
                nutrient: 0.0,
                traits: zero_traits(),
            },
            Carcass {
                id: 101,
                position: (58.0, 0.0),
                energy: 20.0,
                nutrient: 0.0,
                traits: zero_traits(),
            },
        ];

        let mut grid = SpatialGrid::new(100.0, 10.0);
        grid.insert(0, (0.0, 0.0));
        grid.insert(1, (50.0, 0.0));

        let mut nutrient_grid = crate::spatial::NutrientGrid::new(100.0, 10.0, 0.0);
        resolve_drains(
            &mut agents,
            &mut carcasses,
            &grid,
            &params,
            &mut nutrient_grid,
        );

        assert!(
            (carcasses[0].energy - 20.0).abs() < 1e-3,
            "small-bodied heterotroph must NOT reach its carcass at distance 8 (energy {})",
            carcasses[0].energy
        );
        assert!(
            carcasses[1].energy < 20.0,
            "large-bodied heterotroph must reach its carcass at distance 8 by body extent (energy {})",
            carcasses[1].energy
        );
    }
}
