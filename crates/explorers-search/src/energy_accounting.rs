//! Realised energy accounting for an injected lineage (#591).
//!
//! [`LineageAccountant`] steps a world one tick at a time and books every
//! energy flow of a lineage's members — the cohort an invasion bin injects
//! and its descendants ([`Lineage`]) — off the event log, the roster before
//! and after the tick, and a replay of the grow phase (which emits no event
//! for the reproductive earmark or wear repair). The result is an
//! [`EnergyAccount`]: income by route (photosynthesis against its unshaded
//! ceiling, carcass intake against the closed form's `k · h · u_H · e` with
//! the gap split into wear, exhaustion and sharing, living prey), every
//! maintenance term, growth loss, repair, movement, being grazed,
//! reproduction and the earmark, deaths and births.
//!
//! The account closes: the lineage's stock change equals the booked terms,
//! and the booked flows equal the stepper's own energy ledger for the
//! members (pinned in the tests). A member that dies in a tick has what it
//! held at death booked as the death terms (the carcass it leaves, and the
//! rest dissipated), since the stepper does not expose the reserve it held
//! at the moment of death; every surviving member's flows are booked from
//! events and replay, so the residual measures what the account misses.
//!
//! Observer-side only: no stepper change. The network (flow 5) is disabled
//! across the search box and is not booked.

use std::collections::{HashMap, HashSet};

use explorers_sim::energy_ledger::EnergyEndpoint;
use explorers_sim::event::{Event, EventKind};
use explorers_sim::{Agent, FUNCTIONAL_TRAIT_COUNT, TraitVector, World, WorldParameters};

use crate::grazer_hunger::{GrazerHunger, PreStep, Satiation};
use crate::invasion::Lineage;

/// The same flows as the stepper's own energy ledger credits them, summed
/// over the lineage: the reconciliation side of the account.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LedgerFlows {
    /// `SolarTap → member`.
    pub photosynthesis: f64,
    /// `Carcass → member`.
    pub carcass_gain: f64,
    /// `Agent → member` (living prey).
    pub living_gain: f64,
    /// Everything a member sends: dissipation, grazers, its carcass.
    pub sent: f64,
    /// `Endowment → newborn member`.
    pub births_endowment: f64,
}

/// Carcass intake, summed over member-ticks. `ceiling` is the closed form's
/// `h · u_H · e` (nominal heterotrophy, each carcass's own `e`) over every
/// carcass a member reached; `gain` is what it kept. The gap closes exactly:
/// `ceiling − wear_gap − exhaustion_gap − sharing_gap = gain`.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CarcassIntake {
    /// (member, carcass) contacts: carcasses in consumption reach with
    /// anything left, the realised `k` summed over member-ticks.
    pub reached: u64,
    /// Of those, carcasses holding energy at the tick's start.
    pub reached_with_energy: u64,
    /// Carcasses holding energy in the member's nutrient-grid cell at the
    /// tick's start (presence, against `reached`, reach).
    pub in_cell: u64,
    /// Other consumers on the same carcass, summed over contacts.
    pub co_consumers: u64,
    /// Member-ticks with `k` = 0, 1, 2, 3, 4, ≥ 5 carcasses reached.
    pub k_histogram: [u64; 6],
    pub ceiling: f64,
    /// Heterotrophy lost to wear: `(h − h_eff) · u_H · e`.
    pub wear_gap: f64,
    /// The carcass held less than the eater's demand: `(d − min(d, E_c)) · e`.
    pub exhaustion_gap: f64,
    /// Co-consumers' proportional split of what the carcass would have given
    /// the eater alone: `(min(d, E_c) − drain) · e`.
    pub sharing_gap: f64,
    /// Energy drained off carcasses (before the transfer efficiency).
    pub drained: f64,
    pub gain: f64,
}

/// The metabolic charge (flow 8) by term, as paid. When a starving member
/// cannot fund the whole charge the stepper caps it at the reserve; the paid
/// terms are then scaled pro rata and the shortfall is `unpaid`.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaintenancePaid {
    pub base: f64,
    pub photosynthesis: f64,
    pub heterotrophy: f64,
    pub mobility: f64,
    pub asexual: f64,
    pub structure: f64,
    pub unpaid: f64,
}

impl MaintenancePaid {
    pub fn total(&self) -> f64 {
        self.base
            + self.photosynthesis
            + self.heterotrophy
            + self.mobility
            + self.asexual
            + self.structure
    }

    /// The two terms #587 and #589's margin count: base and heterotrophy.
    pub fn margin_terms(&self) -> f64 {
        self.base + self.heterotrophy
    }
}

/// A lineage's energy account over a window, summed over member-ticks (a
/// member alive at a tick's start contributes one member-tick). Energy is
/// `reserve + structure + reproductive earmark`, the stepper's own measure.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnergyAccount {
    pub member_ticks: u64,
    /// Income by route.
    pub photosynthesis: f64,
    /// The unshaded light income, `solar_flux` per photosynthesising
    /// member-tick: what a member alone in its light neighbourhood would get.
    pub photosynthesis_ceiling: f64,
    pub carcass: CarcassIntake,
    pub living_gain: f64,
    /// Upkeep.
    pub maintenance: MaintenancePaid,
    /// Growth's conversion loss (`to_structure / growth_efficiency −
    /// to_structure`); the structure built stays in the member.
    pub growth_loss: f64,
    pub repair: f64,
    pub movement: f64,
    /// Structure drained off living members by grazers, and the part of it
    /// other members took (the lineage grazing itself).
    pub grazed: f64,
    pub grazed_by_kin: f64,
    /// Reproduction: energy moved into the earmark (internal), and the whole
    /// earmark committed by a member that reproduced (it leaves the member:
    /// to offspring, or dissipated).
    pub earmark_fill: f64,
    pub reproduction_outlay: f64,
    /// Deaths: what a dying member held, to its carcass and dissipated; of
    /// the dissipated part, the reproductive earmark stranded.
    pub deaths: u64,
    /// Of those, members that could not fund the tick's metabolic charge
    /// (starved: the stepper capped the charge at the reserve).
    pub deaths_starved: u64,
    /// Members that died in a tick they were grazed alive, not starved (the
    /// graze took structure below the fragility threshold).
    pub deaths_grazed: u64,
    /// Deaths of members born inside the window within [`INFANT_TICKS`] of
    /// their birth, and of those, the ones grazed to death.
    pub infant_deaths: u64,
    pub infant_deaths_grazed: u64,
    /// The grazers of members grazed to death, each (grazer, member) pair
    /// read at drain time: kin when the grazer is itself a member, by trait
    /// distance and the grazer's satiation (#606).
    #[serde(default)]
    pub killers: GrazerHunger,
    pub death_to_carcass: f64,
    pub death_dissipated: f64,
    pub stranded_earmark: f64,
    /// Births into the lineage and the energy they arrive with.
    pub births: u64,
    pub births_endowment: f64,
    /// Reproduction gates at the tick's start: summed earmark levels and the
    /// member-ticks at or above each threshold.
    pub earmark_level: f64,
    pub nutrient_earmark_level: f64,
    pub energy_gate_met: u64,
    pub nutrient_gate_met: u64,
    /// Lineage energy at each tick's start and end, summed over ticks.
    pub stock_start: f64,
    pub stock_end: f64,
    /// Σ |stock after − (stock before + booked terms)| over surviving
    /// members: what the account fails to book.
    pub residual_abs: f64,
    pub ledger: LedgerFlows,
}

impl EnergyAccount {
    pub fn income(&self) -> f64 {
        self.photosynthesis + self.carcass.gain + self.living_gain
    }

    /// Every flow out of the lineage's members, death included.
    pub fn outgo(&self) -> f64 {
        self.maintenance.total()
            + self.growth_loss
            + self.repair
            + self.movement
            + self.grazed
            + self.reproduction_outlay
            + self.death_to_carcass
            + self.death_dissipated
    }

    /// Upkeep: what living costs, before reproduction and death.
    pub fn upkeep(&self) -> f64 {
        self.maintenance.total() + self.growth_loss + self.repair + self.movement
    }
}

/// A death this many ticks or fewer after birth is an infant death.
pub const INFANT_TICKS: u64 = 50;

/// A member's state at a tick's start.
#[derive(Clone, Copy, Debug)]
struct MemberState {
    reserve: f32,
    structure: f32,
    repro_reserve: f32,
    repro_nutrient: f32,
    wear: [f32; FUNCTIONAL_TRAIT_COUNT],
    traits: TraitVector,
    position: (f32, f32),
}

impl MemberState {
    fn of(a: &Agent) -> Self {
        MemberState {
            reserve: a.reserve,
            structure: a.structure,
            repro_reserve: a.repro_reserve,
            repro_nutrient: a.repro_nutrient,
            wear: a.wear,
            traits: a.traits,
            position: a.position,
        }
    }

    fn energy(&self) -> f64 {
        self.reserve as f64 + self.structure as f64 + self.repro_reserve as f64
    }
}

/// A carcass's state at a tick's start.
#[derive(Clone, Copy, Debug)]
struct CarcassState {
    energy: f32,
    traits: TraitVector,
}

/// The metabolic charge's terms (flow 8), in the stepper's order.
fn maintenance_terms(t: &TraitVector, structure: f32, p: &WorldParameters) -> [f32; 6] {
    let x = p.maintenance_cost_exponent;
    [
        p.base_metabolic_rate,
        t.photosynthetic_absorption.powf(x) * p.photo_maintenance_cost,
        t.heterotrophy.powf(x) * p.heterotrophy_maintenance_cost,
        t.mobility.powf(x) * p.mobility_maintenance_cost,
        t.asexual_propensity.powf(x) * p.asexual_propensity_maintenance_cost,
        structure * p.structure_maintenance_coefficient,
    ]
}

/// What the grow phase does to a member this tick, replayed from its
/// tick-start state, its photosynthesis and its metabolic charge (the grow
/// phase emits no event for the earmark or repair).
struct GrowReplay {
    /// Energy moved into the reproductive earmark.
    earmark_fill: f32,
    /// Energy spent on wear repair (1:1, dissipated).
    repair: f32,
    /// Wear after repair: what the drain pass reads effective traits from.
    wear: [f32; FUNCTIONAL_TRAIT_COUNT],
}

fn replay_grow(m: &MemberState, photo: f32, metab: f32, p: &WorldParameters) -> GrowReplay {
    let mut out = GrowReplay {
        earmark_fill: 0.0,
        repair: 0.0,
        wear: m.wear,
    };
    let mut reserve = m.reserve;
    reserve += photo;
    reserve -= metab;
    if reserve <= 0.0 {
        return out;
    }
    let [a, b, c, d, e, f] = maintenance_terms(&m.traits, m.structure, p);
    let retention = (a + b + c + d + e + f) * p.growth_retention_multiplier;
    let surplus = (reserve - retention).max(0.0) * p.reserve_mobilisation_rate;
    if surplus <= 0.0 {
        return out;
    }
    let kappa = m.traits.kappa.clamp(0.0, 1.0);
    let soma = surplus * kappa;
    out.earmark_fill = surplus - soma;
    let decay = p.repair_decay;
    if soma > 0.0 && decay > 0.0 {
        let base_repair = kappa * explorers_sim::units::KAPPA_REPAIR_PER_TICK;
        let mut spent = 0.0_f32;
        for ft in 0..FUNCTIONAL_TRAIT_COUNT {
            if out.wear[ft] <= 0.0 {
                continue;
            }
            let repair = (base_repair * (-decay * out.wear[ft]).exp()).min(out.wear[ft]);
            if spent + repair > soma {
                let capped = (soma - spent).min(out.wear[ft]);
                out.wear[ft] -= capped;
                spent = soma;
                break;
            }
            out.wear[ft] -= repair;
            spent += repair;
        }
        out.repair = spent;
    }
    out
}

/// One member's flows in one tick.
#[derive(Default)]
struct MemberTick {
    photo: f32,
    metab: f32,
    carcass_gain: f64,
    living_gain: f64,
    growth_loss: f64,
    movement: f64,
    grazed: f64,
    grazed_by_kin: f64,
    reproduced: bool,
}

/// Steps a world and books every energy flow of a lineage's members.
pub struct LineageAccountant {
    lineage: Lineage,
    cursor: usize,
    account: EnergyAccount,
    /// Tick of birth of each member born under the accountant.
    born_at: HashMap<u64, u64>,
}

/// The event kinds the account reads.
const KINDS: [EventKind; 8] = [
    EventKind::Photosynthesized,
    EventKind::Metabolized,
    EventKind::Grew,
    EventKind::Consumed,
    EventKind::Reproduced,
    EventKind::Died,
    EventKind::Moved,
    EventKind::Born,
];

impl LineageAccountant {
    /// Follow `lineage` on `world`. Widens the world's event retention to the
    /// kinds the account reads (an observer-side setting).
    pub fn new(world: &mut World, lineage: Lineage) -> Self {
        world.retain_event_kinds(&KINDS);
        LineageAccountant {
            lineage,
            cursor: world.event_log().len(),
            account: EnergyAccount::default(),
            born_at: HashMap::new(),
        }
    }

    /// Step the world one tick and book the lineage's flows. The log is
    /// compacted behind the read, so a long window holds one tick of events.
    /// Returns the tick's events (every retained kind), for a caller that
    /// follows more than the lineage.
    pub fn step(&mut self, world: &mut World) -> Vec<Event> {
        let p = world.params().clone();
        let pre: HashMap<u64, MemberState> = world
            .agents()
            .iter()
            .filter(|a| self.lineage.is_member(a.id))
            .map(|a| (a.id, MemberState::of(a)))
            .collect();
        let agent_traits: HashMap<u64, TraitVector> =
            world.agents().iter().map(|a| (a.id, a.traits)).collect();
        let carcasses: HashMap<u64, CarcassState> = world
            .carcasses()
            .iter()
            .map(|c| {
                let state = CarcassState {
                    energy: c.energy,
                    traits: c.traits,
                };
                (c.id, state)
            })
            .collect();
        let mut carcasses_by_cell: HashMap<usize, u64> = HashMap::new();
        for c in world.carcasses().iter().filter(|c| c.energy > 0.0) {
            *carcasses_by_cell
                .entry(world.nutrient_grid().cell_index_for(c.position))
                .or_default() += 1;
        }
        let cells: HashMap<u64, usize> = pre
            .iter()
            .map(|(id, m)| (*id, world.nutrient_grid().cell_index_for(m.position)))
            .collect();

        let pre_step = PreStep::capture(world);
        world.step();
        let events: Vec<Event> = world.event_log().since(self.cursor).to_vec();
        self.cursor = world.event_log().len();
        world.compact_event_log_before(self.cursor);
        self.lineage.absorb(&events);

        let a = &mut self.account;
        let mut tick: HashMap<u64, MemberTick> =
            pre.keys().map(|id| (*id, MemberTick::default())).collect();
        let mut eaters_per_carcass: HashMap<u64, u64> = HashMap::new();
        let mut grazers_of: HashMap<u64, Vec<u64>> = HashMap::new();
        for ev in &events {
            if ev.kind == EventKind::Consumed && ev.target_was_carcass {
                *eaters_per_carcass
                    .entry(ev.target.expect("target"))
                    .or_default() += 1;
            }
            if ev.kind == EventKind::Consumed
                && !ev.target_was_carcass
                && let Some(t) = ev.target.and_then(|t| tick.get_mut(&t))
            {
                t.grazed += ev.energy_delta as f64;
                let grazers = grazers_of.entry(ev.target.expect("target")).or_default();
                if !grazers.contains(&ev.source) {
                    grazers.push(ev.source);
                }
                if pre.contains_key(&ev.source) {
                    t.grazed_by_kin += ev.energy_delta as f64;
                }
            }
            if ev.kind == EventKind::Reproduced
                && let Some(t) = ev.target.and_then(|t| tick.get_mut(&t))
            {
                t.reproduced = true;
            }
            let Some(t) = tick.get_mut(&ev.source) else {
                continue;
            };
            match ev.kind {
                EventKind::Photosynthesized => t.photo += ev.energy_delta,
                EventKind::Metabolized => t.metab += ev.energy_delta,
                EventKind::Grew => {
                    let spent = ev.energy_delta / p.growth_efficiency;
                    t.growth_loss += (spent - ev.energy_delta) as f64;
                }
                EventKind::Moved => t.movement += ev.energy_delta as f64,
                EventKind::Reproduced => t.reproduced = true,
                EventKind::Consumed if !ev.target_was_carcass => {
                    let target = agent_traits[&ev.target.expect("target")];
                    let e = explorers_sim::trophic_transfer_efficiency(
                        &pre[&ev.source].traits,
                        &target,
                        &p,
                    );
                    t.living_gain += (ev.energy_delta * e) as f64;
                }
                _ => {}
            }
        }
        let grow: HashMap<u64, GrowReplay> = pre
            .iter()
            .map(|(id, m)| (*id, replay_grow(m, tick[id].photo, tick[id].metab, &p)))
            .collect();

        // Carcass intake against the closed form's ceiling.
        let mut k_of: HashMap<u64, u64> = pre.keys().map(|id| (*id, 0)).collect();
        let k_steep = p.wear_degradation_steepness;
        let u_h = explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK as f64;
        for ev in &events {
            if ev.kind != EventKind::Consumed || !ev.target_was_carcass {
                continue;
            }
            let Some(m) = pre.get(&ev.source) else {
                continue;
            };
            let cid = ev.target.expect("target");
            let c = carcasses[&cid];
            let e32 = explorers_sim::trophic_transfer_efficiency(&m.traits, &c.traits, &p);
            let e = e32 as f64;
            let h = m.traits.heterotrophy as f64;
            let h_eff =
                (m.traits.heterotrophy * (-k_steep * grow[&ev.source].wear[1]).exp()) as f64;
            let demand = h_eff * u_h;
            let alone = demand.min(c.energy.max(0.0) as f64);
            let drain = ev.energy_delta as f64;
            let gain = (ev.energy_delta * e32) as f64;
            let ci = &mut a.carcass;
            ci.reached += 1;
            if c.energy > 0.0 {
                ci.reached_with_energy += 1;
            }
            ci.co_consumers += eaters_per_carcass[&cid] - 1;
            ci.ceiling += h * u_h * e;
            ci.wear_gap += (h - h_eff) * u_h * e;
            ci.exhaustion_gap += (demand - alone) * e;
            ci.sharing_gap += (alone - drain) * e;
            ci.drained += drain;
            ci.gain += gain;
            tick.get_mut(&ev.source).expect("member").carcass_gain += gain;
            *k_of.get_mut(&ev.source).expect("member") += 1;
        }
        for (id, k) in &k_of {
            a.carcass.k_histogram[(*k as usize).min(5)] += 1;
            a.carcass.in_cell += carcasses_by_cell.get(&cells[id]).copied().unwrap_or(0);
        }

        // Each member's balance.
        let post: HashMap<u64, f64> = world
            .agents()
            .iter()
            .filter(|x| self.lineage.is_member(x.id))
            .map(|x| (x.id, x.energy() as f64))
            .collect();
        let new_carcass: HashMap<u64, f64> = world
            .carcasses()
            .iter()
            .filter(|c| !carcasses.contains_key(&c.id))
            .map(|c| (c.id, c.energy as f64))
            .collect();
        let mut drain_time: Option<HashMap<u64, Satiation>> = None;
        a.member_ticks += pre.len() as u64;
        for (id, m) in &pre {
            let t = &tick[id];
            let g = &grow[id];
            let terms = maintenance_terms(&m.traits, m.structure, &p);
            let uncapped: f32 = terms.iter().sum();
            let scale = if uncapped > 0.0 {
                (t.metab / uncapped) as f64
            } else {
                0.0
            };
            let mp = &mut a.maintenance;
            mp.base += terms[0] as f64 * scale;
            mp.photosynthesis += terms[1] as f64 * scale;
            mp.heterotrophy += terms[2] as f64 * scale;
            mp.mobility += terms[3] as f64 * scale;
            mp.asexual += terms[4] as f64 * scale;
            mp.structure += terms[5] as f64 * scale;
            mp.unpaid += (uncapped - t.metab).max(0.0) as f64;
            let earmark = m.repro_reserve as f64 + g.earmark_fill as f64;
            let outlay = if t.reproduced { earmark } else { 0.0 };
            a.photosynthesis += t.photo as f64;
            if m.traits.photosynthetic_absorption * (-k_steep * m.wear[0]).exp() > 0.0
                && m.structure > 0.0
            {
                a.photosynthesis_ceiling += p.solar_flux_magnitude as f64;
            }
            a.living_gain += t.living_gain;
            a.growth_loss += t.growth_loss;
            a.repair += g.repair as f64;
            a.movement += t.movement;
            a.grazed += t.grazed;
            a.grazed_by_kin += t.grazed_by_kin;
            a.earmark_fill += g.earmark_fill as f64;
            a.reproduction_outlay += outlay;
            a.earmark_level += m.repro_reserve as f64;
            a.nutrient_earmark_level += m.repro_nutrient as f64;
            if m.repro_reserve >= p.reproduction_energy_threshold {
                a.energy_gate_met += 1;
            }
            if m.repro_nutrient >= p.reproduction_nutrient_threshold {
                a.nutrient_gate_met += 1;
            }
            let income = t.photo as f64 + t.carcass_gain + t.living_gain;
            let outgo =
                (t.metab + g.repair) as f64 + t.growth_loss + t.movement + t.grazed + outlay;
            let expected = m.energy() + income - outgo;
            a.stock_start += m.energy();
            match post.get(id) {
                Some(&after) => a.residual_abs += (after - expected).abs(),
                None => {
                    let carcass = new_carcass.get(id).copied().unwrap_or(0.0);
                    a.deaths += 1;
                    let grazed_to_death = t.metab >= uncapped && t.grazed > 0.0;
                    if t.metab < uncapped {
                        a.deaths_starved += 1;
                    } else if grazed_to_death {
                        a.deaths_grazed += 1;
                        let satiation = drain_time.get_or_insert_with(|| {
                            pre_step
                                .drain_time_agents(&p)
                                .iter()
                                .map(|x| (x.id, Satiation::of(x, &p)))
                                .collect()
                        });
                        for g in grazers_of.get(id).into_iter().flatten() {
                            a.killers.record(
                                pre.contains_key(g),
                                m.traits.distance(&agent_traits[g]),
                                satiation[g],
                            );
                        }
                    }
                    if let Some(born) = self.born_at.get(id)
                        && world.tick() - born <= INFANT_TICKS
                    {
                        a.infant_deaths += 1;
                        if grazed_to_death {
                            a.infant_deaths_grazed += 1;
                        }
                    }
                    a.death_to_carcass += carcass;
                    a.death_dissipated += expected - carcass;
                    if !t.reproduced {
                        a.stranded_earmark += earmark;
                    }
                }
            }
        }
        let pre_ids: HashSet<u64> = pre.keys().copied().collect();
        for (id, e) in &post {
            a.stock_end += e;
            if !pre_ids.contains(id) {
                self.born_at.insert(*id, world.tick());
                a.births += 1;
                a.births_endowment += e;
            }
        }

        for (src, dst, amount) in world.energy_ledger().flows() {
            let amount = *amount as f64;
            if let EnergyEndpoint::Agent(id) = src
                && pre_ids.contains(id)
            {
                a.ledger.sent += amount;
            }
            let EnergyEndpoint::Agent(id) = dst else {
                continue;
            };
            if pre_ids.contains(id) {
                match src {
                    EnergyEndpoint::SolarTap => a.ledger.photosynthesis += amount,
                    EnergyEndpoint::Carcass(_) => a.ledger.carcass_gain += amount,
                    EnergyEndpoint::Agent(_) => a.ledger.living_gain += amount,
                    _ => {}
                }
            } else if post.contains_key(id) && *src == EnergyEndpoint::Endowment {
                a.ledger.births_endowment += amount;
            }
        }
        events
    }

    pub fn lineage(&self) -> &Lineage {
        &self.lineage
    }

    pub fn account(&self) -> &EnergyAccount {
        &self.account
    }

    /// Members alive on the world's roster.
    pub fn alive(&self, world: &World) -> usize {
        self.lineage.alive(world.agents())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::invasion::place_cohort;
    use crate::search::default_ranges;
    use explorers_sim::{AgentSpec, CarcassSpec, TraitVector, WorldRecipe};

    /// A small `sample:31` world: resident producers, a clutch of carcasses at
    /// the origin (one nearly spent), and a cohort of near-producers with
    /// raised heterotrophy injected around them — each in reach of the
    /// origin carcass but (reach `0.6 × 0.81 ≈ 0.48`) not of each other — so
    /// the cohort photosynthesises, shares carcasses, and runs one dry.
    fn pile_world() -> (World, Vec<u64>) {
        let sampled = sampled_units(default_ranges().len());
        let (params, dist) =
            resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let producer = TraitVector {
            photosynthetic_absorption: dist.mean_traits.photosynthetic_absorption
                + dist.mean_traits.heterotrophy,
            heterotrophy: 0.0,
            ..dist.mean_traits
        };
        let residents = [(-1.0, 0.5), (1.5, -0.5), (8.0, 8.0), (-9.0, 4.0)]
            .map(|position| AgentSpec {
                position,
                reserve: dist.initial_energy_per_agent,
                traits: producer,
                nutrient: 0.0,
            })
            .to_vec();
        let carcasses = [((0.0, 0.0), 0.05), ((0.3, 0.2), 3.0), ((-0.3, -0.2), 20.0)]
            .map(|(position, energy)| CarcassSpec {
                position,
                energy,
                traits: producer,
                nutrient: 2.0,
            })
            .to_vec();
        let recipe = WorldRecipe {
            parameters: params,
            initial_distribution: None,
            agents: Some(residents),
            carcasses: Some(carcasses),
            max_ticks: 100,
        };
        let mut world = World::from_recipe(&recipe, 7);
        let stepped = TraitVector {
            heterotrophy: 0.6,
            mobility: 0.0,
            ..producer
        };
        let ids = place_cohort(
            &mut world,
            stepped,
            &[(0.35, 0.0), (-0.35, 0.0), (0.0, 0.35), (0.0, -0.35)],
            dist.initial_energy_per_agent,
        );
        (world, ids)
    }

    /// The photosynthetic income booked to the lineage is the solar flow the
    /// stepper's own energy ledger credits to its members.
    #[test]
    fn photosynthesis_booked_matches_the_ledger_solar_flow() {
        let (mut world, ids) = pile_world();
        let mut acct = LineageAccountant::new(&mut world, Lineage::new(ids));
        for _ in 0..20 {
            acct.step(&mut world);
        }
        let a = acct.account();
        assert!(a.member_ticks > 0);
        assert!(a.photosynthesis > 0.0);
        assert!((a.photosynthesis - a.ledger.photosynthesis).abs() < 1e-3 * a.photosynthesis);
    }

    /// Carcass intake is booked against its ceiling `Σ h · u_H · e` over the
    /// carcasses in reach, and the gap splits exactly into wear, exhaustion
    /// (the carcass had less than the eater's demand) and sharing (co-consumers
    /// took part of what it would have had alone). The kept energy matches the
    /// ledger's carcass → member flow. The pile world is built so a near-empty
    /// carcass is shared by the cohort on the first tick.
    #[test]
    fn carcass_intake_gap_splits_into_wear_exhaustion_and_sharing() {
        let (mut world, ids) = pile_world();
        let mut acct = LineageAccountant::new(&mut world, Lineage::new(ids));
        for _ in 0..20 {
            acct.step(&mut world);
        }
        let c = acct.account().carcass;
        assert!(c.gain > 0.0 && c.reached > 0);
        assert!(c.exhaustion_gap > 0.0 && c.sharing_gap > 0.0, "{c:?}");
        let closed = c.ceiling - c.wear_gap - c.exhaustion_gap - c.sharing_gap;
        assert!((closed - c.gain).abs() < 1e-4 * c.ceiling, "{c:?}");
        let ledger = acct.account().ledger.carcass_gain;
        assert!(
            (c.gain - ledger).abs() < 1e-3 * c.gain,
            "{} vs {ledger}",
            c.gain
        );
    }

    /// Step `world` for `ticks` under the accountant and check the account
    /// closes: the lineage's stock change equals the booked terms (every
    /// member's income, each maintenance term, growth loss, repair, movement,
    /// reproduction outlay, being grazed, deaths and births), and the booked
    /// flows equal the stepper's energy ledger for the lineage's members.
    fn assert_account_reconciles(mut world: World, ids: Vec<u64>, ticks: u64) -> EnergyAccount {
        let mut acct = LineageAccountant::new(&mut world, Lineage::new(ids));
        for _ in 0..ticks {
            acct.step(&mut world);
        }
        let a = *acct.account();
        assert!(a.deaths_starved + a.deaths_grazed <= a.deaths);
        assert!(a.infant_deaths_grazed <= a.infant_deaths && a.infant_deaths <= a.deaths);
        assert!(a.grazed_by_kin <= a.grazed);
        let k = a.killers;
        assert!(
            k.total(true) + k.total(false) >= a.deaths_grazed,
            "every grazed-to-death member has a killing grazer: {k:?}"
        );
        let throughput = a.income() + a.outgo() + a.births_endowment;
        let tol = 1e-4 * throughput.max(1.0);
        assert!(
            a.residual_abs < tol,
            "residual {} (tol {tol}): {a:#?}",
            a.residual_abs
        );
        let booked = a.stock_end - a.stock_start;
        let terms = a.income() - a.outgo() + a.births_endowment;
        assert!(
            (booked - terms).abs() < tol,
            "stock {booked} vs terms {terms}"
        );
        let l = a.ledger;
        for (mine, ledger, what) in [
            (a.photosynthesis, l.photosynthesis, "photosynthesis"),
            (a.carcass.gain, l.carcass_gain, "carcass gain"),
            (a.living_gain, l.living_gain, "living gain"),
            (a.outgo(), l.sent, "outgo"),
            (a.births_endowment, l.births_endowment, "births"),
        ] {
            assert!(
                (mine - ledger).abs() < tol,
                "{what}: booked {mine} vs ledger {ledger}"
            );
        }
        a
    }

    /// The reconciliation pin on the constructed pile world: every member
    /// survives the horizon, so every term is booked from events and replay,
    /// none as a residual.
    #[test]
    fn account_reconciles_with_the_ledger_on_the_pile_world() {
        let (world, ids) = pile_world();
        let a = assert_account_reconciles(world, ids, 30);
        assert!(a.maintenance.total() > 0.0 && a.earmark_fill > 0.0);
    }

    /// The reconciliation pin on a real fork: `sample:31` stepped to tick
    /// 120, a cohort of the founder mean phenotype (a mixotroph) injected
    /// uniformly, 60 ticks under the accountant. Residents die, carcasses
    /// form, members may die or breed: whatever happens, the account closes.
    #[test]
    fn account_reconciles_with_the_ledger_on_a_sample_31_fork() {
        let sampled = sampled_units(default_ranges().len());
        let (params, dist) =
            resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        let mut world = World::new(params, dist.clone(), 1000);
        for _ in 0..120 {
            world.step();
        }
        let positions: Vec<(f32, f32)> = (0..8)
            .map(|i| (-20.0 + 5.0 * i as f32, 10.0 - 3.0 * i as f32))
            .collect();
        let ids = place_cohort(
            &mut world,
            dist.mean_traits,
            &positions,
            dist.initial_energy_per_agent,
        );
        assert_account_reconciles(world, ids, 60);
    }

    /// The killing grazers of members grazed to death are read at drain time
    /// (#606): a tight cohort of near-producers with raised heterotrophy
    /// breeds and grazes its own young, and each such death books its
    /// grazers by kinship (a lineage member), trait distance and satiation.
    /// Run ungated (`satiation_sensitivity = 0`): at the default surplus gate
    /// (#623) this cohort kills none of its young within the horizon, and the
    /// pin is of the read, not of the gate.
    #[test]
    fn account_reads_the_killing_grazers_of_members_grazed_to_death() {
        let sampled = sampled_units(default_ranges().len());
        let (mut params, dist) =
            resolve_config(ConfigSource::SAMPLE, 31, &Default::default(), &sampled);
        params.satiation_sensitivity = 0.0;
        let mut world = World::new(params, dist.clone(), 1000);
        for _ in 0..120 {
            world.step();
        }
        let traits = TraitVector {
            heterotrophy: dist.mean_traits.heterotrophy + 0.3,
            ..dist.mean_traits
        };
        let positions: Vec<(f32, f32)> = (0..8).map(|i| (0.2 * i as f32, 0.0)).collect();
        let ids = place_cohort(
            &mut world,
            traits,
            &positions,
            dist.initial_energy_per_agent,
        );
        let a = assert_account_reconciles(world, ids, 300);
        assert!(a.deaths_grazed > 0, "{a:#?}");
        assert!(a.killers.total(true) > 0, "{:?}", a.killers);
    }
}
