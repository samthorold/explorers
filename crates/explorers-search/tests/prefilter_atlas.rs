//! The *a priori* viability prefilter against the committed atlas (issue #461).
//!
//! `prefilter::fails_energy_death_gate` carries a named energy anchor,
//! `STRUCTURE_MIN` (`viability.md`, *Gate — energy death*; research note
//! `440-dimensionless-groups.md`, smell S3). These tests pin two things:
//!
//! * every live cell of the committed `atlas.json` clears the gate — a
//!   mis-drawn floor would route a mapped, living world to the dead frontier
//!   without a rollout;
//! * the implemented gate is exactly its π-form with the floor as its own
//!   dimensionless group, `π_N ≥ π_ρ · π_S + 1`, `π_S = STRUCTURE_MIN / ε`.

use explorers_genesis::WorldParameters;
use explorers_search::prefilter::{fails_energy_death_gate, prefilter_cliff};
use explorers_search::search::{decode, default_ranges};

/// Decode every live cell's unit vector from the committed `atlas.json` at the
/// repo root, through the same decoder the search uses.
fn atlas_live_cells() -> Vec<(Vec<usize>, WorldParameters)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../atlas.json");
    let text = std::fs::read_to_string(path).expect("committed atlas.json at the repo root");
    let atlas: serde_json::Value = serde_json::from_str(&text).expect("atlas.json parses");
    let ranges = default_ranges();
    let cells = atlas["cells"]
        .as_array()
        .expect("atlas has a `cells` array");
    assert!(!cells.is_empty(), "the committed atlas has live cells");
    cells
        .iter()
        .map(|cell| {
            let unit: Vec<f64> = cell["unit"]
                .as_array()
                .expect("cell has a `unit` vector")
                .iter()
                .map(|v| v.as_f64().expect("unit coordinate is a number"))
                .collect();
            assert_eq!(unit.len(), ranges.len(), "unit vector spans the search box");
            let idx: Vec<usize> = cell["cell"]
                .as_array()
                .expect("cell has an index")
                .iter()
                .map(|v| v.as_u64().expect("cell index is an integer") as usize)
                .collect();
            (idx, decode(&unit, &ranges).0)
        })
        .collect()
}

#[test]
fn every_committed_atlas_live_cell_clears_the_energy_death_gate() {
    for (idx, params) in atlas_live_cells() {
        assert!(
            !fails_energy_death_gate(&params),
            "atlas live cell {idx:?} would be gated energy-dead: pool {} vs base_nutrient_ratio {} and N_r {}",
            params.initial_nutrient_pool,
            params.base_nutrient_ratio,
            params.reproduction_nutrient_threshold
        );
        assert_eq!(
            prefilter_cliff(&params),
            None,
            "atlas live cell {idx:?} must not be prefiltered dead"
        );
    }
}

#[test]
fn energy_death_gate_is_its_pi_form_with_structure_min_as_its_own_group() {
    use explorers_search::prefilter::STRUCTURE_MIN;

    // The anchor is one unit of energy in today's units — the value the gate has
    // always used, now named (`viability.md`, Open tier; S3).
    assert_eq!(STRUCTURE_MIN, 1.0);

    // π-form: π_N ≥ π_ρ · π_S · (1 + π_ρs · σ_min) + 1, with σ_min = 0 so the
    // bracket is 1, π_N = N_total / N_r, π_ρ = ρ_b · ε / N_r, π_S = S_min / ε,
    // ε = B · τ (τ = 1 tick). Sweep the decoded search box, including the
    // energy-scale axis B, and check the raw gate agrees with the π-form at
    // every point — including the π_S = 1/ε dependence on B that makes the
    // implemented gate not scale-free.
    let ranges = default_ranges();
    let tau = 1.0_f32;
    for step in 0..=8 {
        let u = step as f64 / 8.0;
        let (mut params, _) = decode(&vec![u; ranges.len()], &ranges);
        for scale in [0.25_f32, 1.0, 4.0] {
            params.base_metabolic_rate *= scale;
            // Put the pool near the floor so the comparison is discriminating.
            let raw_floor =
                STRUCTURE_MIN * params.base_nutrient_ratio + params.reproduction_nutrient_threshold;
            for pool in [raw_floor * 0.5, raw_floor, raw_floor * 2.0] {
                params.initial_nutrient_pool = pool;

                let epsilon = params.base_metabolic_rate * tau;
                let n_r = params.reproduction_nutrient_threshold;
                let pi_n = params.initial_nutrient_pool / n_r;
                let pi_rho = params.base_nutrient_ratio * epsilon / n_r;
                let pi_s = STRUCTURE_MIN / epsilon;
                let pi_form_fails = pi_n < pi_rho * pi_s + 1.0;

                assert_eq!(
                    fails_energy_death_gate(&params),
                    pi_form_fails,
                    "raw gate and π-form disagree at u={u}, B scale {scale}, pool {pool}"
                );
            }
        }
    }
}
