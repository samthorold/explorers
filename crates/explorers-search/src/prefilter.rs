//! The *a priori* viability prefilter — the two committed closed-form gates from
//! `docs/system-design/viability.md`, run ahead of each genesis rollout.
//!
//! A config that fails a gate is provably dead for *every* seed and functional
//! form, so it is routed to the atlas's dead frontier as an **a priori** death
//! without spending an ensemble — shrinking the search box exactly as
//! `viability.md`'s payoff promises. The gates are *negative*: clearing one is
//! necessary, never sufficient (a cleared config still rolls out normally).
//!
//! Only the two **existence** gates are implemented here — extinction's flux
//! floor and energy death's nutrient floor. Nutrient lockup is deliberately
//! *not* prefiltered: it has no cheap a priori gate (it is decomposer-mass
//! dependent and emergent per seed), so the atlas maps it by running (#367).

use explorers_genesis::WorldParameters;

use crate::qd::Cliff;

/// Extinction flux floor (`viability.md`, *Gate — extinction*).
///
/// An isolated producer receives at most the full flux magnitude `F`; metabolism
/// charges at least the base rate `B` every tick. So the per-tick net of any
/// producer is at most `F − B`. If `F ≤ B`, no producer is ever net-positive, no
/// energy ever enters the living system, and extinction is guaranteed for every
/// other parameter and every functional form.
///
/// `F` is `solar_flux_magnitude`; `B` is `base_metabolic_rate` (flow 8).
pub fn fails_extinction_gate(params: &WorldParameters) -> bool {
    params.solar_flux_magnitude <= params.base_metabolic_rate
}

/// Energy-death nutrient floor (`viability.md`, *Gate — energy death*).
///
/// For a lineage to persist, at least one agent must be simultaneously
/// **embodied** (`structure > 0`) and **reproduction-ready** (earmark
/// `≥ N_repro_threshold`). Both draw on the conserved total nutrient `N_total`.
/// Hence the necessary condition
///
/// ```text
/// N_total ≥ structure_min · (base_nutrient_ratio + spec_coeff · σ_min) + N_repro_threshold
/// ```
///
/// If `N_total` is below this floor no agent can be both embodied and able to
/// reproduce, so there is no persistence regardless of solar flux.
///
/// `N_total` is the conserved total nutrient, which at world creation equals
/// `initial_nutrient_pool` (the closed nutrient ledger — `crate::search::decode`
/// puts every unit of nutrient in the pool). `N_repro_threshold` is
/// `reproduction_nutrient_threshold`. The gate is deliberately the *weakest*
/// floor: `structure_min` and `σ_min` take their minimal viable values — one
/// unit of structure (an agent must be embodied, `structure > 0`) and zero
/// specification (the cheapest viable body invests nothing in
/// autotrophy/heterotrophy/mobility). Smaller minima make the floor smaller, so
/// failing it is *sufficient* for deadness, exactly as the doc requires.
pub fn fails_energy_death_gate(params: &WorldParameters) -> bool {
    const STRUCTURE_MIN: f32 = 1.0;
    const SIGMA_MIN: f32 = 0.0;

    let body_demand = STRUCTURE_MIN
        * (params.base_nutrient_ratio + params.specification_nutrient_coefficient * SIGMA_MIN);
    let floor = body_demand + params.reproduction_nutrient_threshold;
    params.initial_nutrient_pool < floor
}

/// The a priori viability prefilter: run both committed existence gates and
/// return the cliff a config provably dies on, or `None` if it clears both gates
/// (and must roll out to decide). Extinction is checked first (the weakest
/// necessary condition for any life at all); energy death second.
///
/// Nutrient lockup is intentionally absent — it has no cheap a priori gate.
pub fn prefilter_cliff(params: &WorldParameters) -> Option<Cliff> {
    if fails_extinction_gate(params) {
        Some(Cliff::Extinction)
    } else if fails_energy_death_gate(params) {
        Some(Cliff::EnergyDeath)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::{decode, default_ranges};

    fn baseline() -> WorldParameters {
        // The decoder midpoint is the #326 known-viable baseline — it must clear
        // both gates.
        let ranges = default_ranges();
        let unit = vec![0.5_f64; ranges.len()];
        decode(&unit, &ranges).0
    }

    #[test]
    fn extinction_gate_fires_when_flux_at_or_below_metabolism() {
        let mut p = baseline();
        // F ≤ B: no producer is ever net-positive.
        p.solar_flux_magnitude = 0.3;
        p.base_metabolic_rate = 0.3;
        assert!(fails_extinction_gate(&p), "F == B must fail the gate");

        p.solar_flux_magnitude = 0.2;
        assert!(fails_extinction_gate(&p), "F < B must fail the gate");
    }

    #[test]
    fn extinction_gate_clears_when_flux_exceeds_metabolism() {
        let p = baseline();
        // The known-viable midpoint has F well above B.
        assert!(p.solar_flux_magnitude > p.base_metabolic_rate);
        assert!(!fails_extinction_gate(&p));
    }

    #[test]
    fn energy_death_gate_fires_when_pool_below_the_floor() {
        let mut p = baseline();
        // Floor = 1.0·(base_nutrient_ratio + spec_coeff·0) + N_repro_threshold
        //       = base_nutrient_ratio + reproduction_nutrient_threshold.
        let floor = p.base_nutrient_ratio + p.reproduction_nutrient_threshold;
        // A pool a hair below the floor cannot embody one agent AND clear the
        // reproduction earmark → guaranteed energy death.
        p.initial_nutrient_pool = floor - 0.01;
        assert!(
            fails_energy_death_gate(&p),
            "pool below the floor must fail; floor {floor}, pool {}",
            p.initial_nutrient_pool
        );
    }

    #[test]
    fn energy_death_gate_clears_when_pool_meets_the_floor() {
        let mut p = baseline();
        let floor = p.base_nutrient_ratio + p.reproduction_nutrient_threshold;
        // Exactly at the floor clears (the condition is N_total ≥ floor).
        p.initial_nutrient_pool = floor;
        assert!(!fails_energy_death_gate(&p), "pool == floor must clear");
        // The known-viable baseline (huge pool) clears with room to spare.
        let viable = baseline();
        assert!(!fails_energy_death_gate(&viable));
    }

    #[test]
    fn prefilter_reports_extinction_before_energy_death() {
        let mut p = baseline();
        // Make both gates fire; extinction is the weaker necessary condition and
        // is reported first.
        p.solar_flux_magnitude = 0.1;
        p.base_metabolic_rate = 0.2;
        p.initial_nutrient_pool = 0.0;
        assert_eq!(prefilter_cliff(&p), Some(Cliff::Extinction));
    }

    #[test]
    fn prefilter_reports_energy_death_when_only_nutrient_floor_fails() {
        let mut p = baseline();
        // Flux clears extinction; pool fails the nutrient floor.
        assert!(p.solar_flux_magnitude > p.base_metabolic_rate);
        p.initial_nutrient_pool = 0.0;
        assert_eq!(prefilter_cliff(&p), Some(Cliff::EnergyDeath));
    }

    #[test]
    fn prefilter_clears_the_known_viable_baseline() {
        // The known-viable decoder midpoint must NOT be prefiltered dead — that
        // would be a mis-drawn gate killing the viable manifold.
        assert_eq!(prefilter_cliff(&baseline()), None);
    }

    #[test]
    fn prefilter_never_returns_nutrient_lockup() {
        // Nutrient lockup has no cheap a priori gate (#367). Sweeping a coarse
        // grid of the searched cube, the prefilter only ever emits the two
        // existence cliffs, never lockup.
        let ranges = default_ranges();
        for step in 0..=4 {
            let u = step as f64 / 4.0;
            let unit = vec![u; ranges.len()];
            let (params, _) = decode(&unit, &ranges);
            if let Some(cliff) = prefilter_cliff(&params) {
                assert_ne!(
                    cliff,
                    Cliff::NutrientLockup,
                    "the prefilter must never gate nutrient lockup"
                );
            }
        }
    }
}
