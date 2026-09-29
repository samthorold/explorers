//! The heterotroph margin (#589): can any heterotroph phenotype cover its
//! maintenance on carcass income? The closed-form read #587 made on
//! `sample:31`, generalised to a line in trait space.

use explorers_genesis::{InitialDistribution, WorldParameters};
use explorers_sim::TraitVector;
use explorers_sim::units::HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK;

/// The straight line in trait space from the founder producer centroid to
/// the pure heterotroph with the same trophic budget
/// (`photosynthetic_absorption + heterotrophy`) and every other trait held.
/// Every phenotype on it targets the producer centroid's carcass: a carcass
/// keeps its dead agent's trait vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeterotrophLine {
    pub producer: TraitVector,
    pub heterotroph: TraitVector,
}

/// One phenotype's carcass economics against the producer carcass, in E/tick.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MarginTerms {
    /// `trophic_transfer_efficiency(phenotype, producer carcass)`.
    pub efficiency: f32,
    /// `heterotrophy × u_H × e`: energy kept per tick from one carcass in
    /// reach (the binary-reach drain).
    pub intake_ceiling: f32,
    /// #587's terms: `base_metabolic_rate + heterotrophy^x · c_H`.
    pub maintenance: f32,
    /// `intake_ceiling − maintenance`.
    pub margin: f32,
    /// The trait maintenance #587 left out: `photosynthetic_absorption`,
    /// `mobility` and `asexual_propensity`, each `trait^x · c`.
    pub other_trait_maintenance: f32,
    /// `margin − other_trait_maintenance`.
    pub strict_margin: f32,
    /// `maintenance / intake_ceiling`: carcasses in reach each tick needed
    /// to break even on #587's terms (infinite with no intake, which a JSON
    /// row writes as `null`).
    pub carcasses_to_break_even: f32,
}

/// Grid resolution of the line read.
pub const LINE_STEPS: u32 = 1000;

/// A config's heterotroph margin: the best point on its line.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LineRead {
    /// Where on the line (0 = producer, 1 = pure heterotroph) the margin peaks.
    pub best_t: f32,
    pub best: MarginTerms,
    /// The largest `t` on the grid with a positive margin: how far toward
    /// heterotrophy one carcass in reach still pays. `None` if nowhere.
    pub positive_reach: Option<f32>,
    /// The best point among nominal heterotrophs (`heterotrophy >
    /// photosynthetic_absorption`, `topology::trophic_roles`' split).
    pub best_heterotroph_t: f32,
    pub best_heterotroph: MarginTerms,
    /// The best point on the strict margin (all trait terms charged).
    pub best_strict: MarginTerms,
    /// The fewest carcasses in reach per tick any phenotype on the line
    /// needs to break even on #587's terms.
    pub min_carcasses_to_break_even: f32,
    /// The pure heterotroph (`t = 1`), #587's `full` phenotype.
    pub full: MarginTerms,
    /// Some phenotype on the line has a strictly positive margin.
    pub positive: bool,
}

impl HeterotrophLine {
    /// The line from the founder producer centroid `World::new` seeds
    /// (the mean with all of its trophic budget on photosynthesis; the mean
    /// itself for a single-cluster founding) to the pure heterotroph.
    pub fn from_founders(dist: &InitialDistribution) -> Self {
        let mean = dist.mean_traits;
        let budget = mean.photosynthetic_absorption + mean.heterotrophy;
        let producer = if dist.initial_cluster_count <= 1 || budget <= 0.0 {
            mean
        } else {
            TraitVector {
                photosynthetic_absorption: budget,
                heterotrophy: 0.0,
                ..mean
            }
        };
        HeterotrophLine {
            producer,
            heterotroph: TraitVector {
                photosynthetic_absorption: 0.0,
                heterotrophy: budget,
                ..mean
            },
        }
    }

    /// The phenotype a fraction `t ∈ [0, 1]` of the way to the heterotroph.
    pub fn phenotype(&self, t: f32) -> TraitVector {
        let lerp = |a: f32, b: f32| a + t * (b - a);
        TraitVector {
            photosynthetic_absorption: lerp(
                self.producer.photosynthetic_absorption,
                self.heterotroph.photosynthetic_absorption,
            ),
            heterotrophy: lerp(self.producer.heterotrophy, self.heterotroph.heterotrophy),
            ..self.producer
        }
    }

    /// The line read on a fixed grid of `LINE_STEPS + 1` points
    /// (`t = i / LINE_STEPS`): the best margin, where it peaks (the smallest
    /// `t` on a tie), and the full heterotroph's terms.
    pub fn read(&self, params: &WorldParameters) -> LineRead {
        let mut best_t = 0.0;
        let mut best = self.terms_at(0.0, params);
        let mut best_strict = best;
        let mut min_carcasses_to_break_even = best.carcasses_to_break_even;
        let mut positive_reach = (best.margin > 0.0).then_some(0.0);
        let mut best_heterotroph: Option<(f32, MarginTerms)> = None;
        for i in 1..=LINE_STEPS {
            let t = i as f32 / LINE_STEPS as f32;
            let terms = self.terms_at(t, params);
            if terms.margin > 0.0 {
                positive_reach = Some(t);
            }
            let p = self.phenotype(t);
            if p.heterotrophy > p.photosynthetic_absorption
                && best_heterotroph.is_none_or(|(_, b)| terms.margin > b.margin)
            {
                best_heterotroph = Some((t, terms));
            }
            if terms.margin > best.margin {
                best_t = t;
                best = terms;
            }
            if terms.strict_margin > best_strict.strict_margin {
                best_strict = terms;
            }
            min_carcasses_to_break_even =
                min_carcasses_to_break_even.min(terms.carcasses_to_break_even);
        }
        // A zero trophic budget has no nominal heterotroph on its line; the
        // pure end (itself then a zero-trait phenotype) stands in.
        let (best_heterotroph_t, best_heterotroph) =
            best_heterotroph.unwrap_or((1.0, self.terms_at(1.0, params)));
        LineRead {
            best_t,
            best,
            positive_reach,
            best_heterotroph_t,
            best_heterotroph,
            best_strict,
            min_carcasses_to_break_even,
            full: self.terms_at(1.0, params),
            positive: best.margin > 0.0,
        }
    }

    /// The economics of the phenotype at `t` on the producer carcass.
    pub fn terms_at(&self, t: f32, params: &WorldParameters) -> MarginTerms {
        margin_terms(&self.phenotype(t), &self.producer, params)
    }
}

/// A phenotype's carcass economics on a carcass carrying `carcass` traits.
pub fn margin_terms(
    phenotype: &TraitVector,
    carcass: &TraitVector,
    params: &WorldParameters,
) -> MarginTerms {
    let efficiency = explorers_sim::trophic_transfer_efficiency(phenotype, carcass, params);
    let intake_ceiling =
        phenotype.heterotrophy * HETEROTROPHY_STRUCTURE_DRAIN_PER_TICK * efficiency;
    let x = params.maintenance_cost_exponent;
    let maintenance = params.base_metabolic_rate
        + phenotype.heterotrophy.powf(x) * params.heterotrophy_maintenance_cost;
    let other_trait_maintenance = phenotype.photosynthetic_absorption.powf(x)
        * params.photo_maintenance_cost
        + phenotype.mobility.powf(x) * params.mobility_maintenance_cost
        + phenotype.asexual_propensity.powf(x) * params.asexual_propensity_maintenance_cost;
    let margin = intake_ceiling - maintenance;
    MarginTerms {
        efficiency,
        intake_ceiling,
        maintenance,
        margin,
        other_trait_maintenance,
        strict_margin: margin - other_trait_maintenance,
        carcasses_to_break_even: if intake_ceiling > 0.0 {
            maintenance / intake_ceiling
        } else {
            f32::INFINITY
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_source::{ConfigSource, resolve_config, sampled_units};
    use crate::sweep::AtlasUnits;

    fn sample_31() -> (
        explorers_sim::WorldParameters,
        explorers_genesis::InitialDistribution,
    ) {
        resolve_config(
            ConfigSource::SAMPLE,
            31,
            &AtlasUnits::default(),
            &sampled_units(32),
        )
    }

    /// #587's economics read at the full heterotroph on `sample:31`:
    /// `e ≈ 0.02` of a producer carcass, and a maintenance of ≈ 0.034 E/tick
    /// (`0.0117 + 1.086^1.80 × 0.0194`, the base and heterotrophy terms).
    #[test]
    fn full_heterotroph_on_sample_31_reproduces_587s_efficiency_and_maintenance() {
        let (params, dist) = sample_31();
        let line = HeterotrophLine::from_founders(&dist);
        let full = line.terms_at(1.0, &params);
        assert_eq!((full.efficiency * 100.0).round() / 100.0, 0.02);
        assert_eq!((full.maintenance * 1000.0).round() / 1000.0, 0.034);
        assert!(full.margin < 0.0);
    }

    /// The same computation #587's bin made: on seed 1000 its artifact
    /// (`target/reinvasion-barrier.json`) recorded, for the full phenotype
    /// against the resident producer centroid at tick 1000, `e = 0.023537695`
    /// and a carcass-intake ceiling of `0.02555098` E/tick. The resident
    /// centroid is copied from that record.
    #[test]
    fn margin_terms_match_587s_recorded_ceiling_on_a_resident_producer_carcass() {
        let (params, dist) = sample_31();
        let resident = TraitVector {
            photosynthetic_absorption: 1.0771803,
            heterotrophy: 0.03596048,
            mobility: 0.21300776,
            kappa: 0.113023266,
            fecundity: 0.42842895,
            asexual_propensity: 0.41228205,
            dispersal: 0.29509285,
        };
        let full = HeterotrophLine::from_founders(&dist).heterotroph;
        let terms = margin_terms(&full, &resident, &params);
        assert_eq!(terms.efficiency, 0.023537695);
        assert_eq!(terms.intake_ceiling, 0.02555098);
    }

    /// The best point on the line is no worse than any other point on it and
    /// the full heterotroph is its `t = 1` end.
    #[test]
    fn best_on_line_dominates_every_point_on_the_line() {
        let (params, dist) = sample_31();
        let line = HeterotrophLine::from_founders(&dist);
        let read = line.read(&params);
        assert!((0.0..=1.0).contains(&read.best_t));
        assert_eq!(read.full, line.terms_at(1.0, &params));
        assert_eq!(read.best, line.terms_at(read.best_t, &params));
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            assert!(read.best.margin >= line.terms_at(t, &params).margin);
        }
    }

    /// Cheap heterotrophy on a flat kernel makes the carcass pay: the read
    /// then has a positive margin, and it says so.
    #[test]
    fn a_flat_kernel_and_cheap_heterotrophy_give_a_positive_margin() {
        let (mut params, dist) = sample_31();
        params.base_trophic_efficiency = 0.9;
        params.trophic_distance_decay = 0.1;
        params.base_metabolic_rate = 0.01;
        params.heterotrophy_maintenance_cost = 0.001;
        let read = HeterotrophLine::from_founders(&dist).read(&params);
        assert!(read.best.margin > 0.0);
        assert!(read.positive);
    }

    /// The read also names how far toward heterotrophy the carcass pays: the
    /// last grid point with a positive margin, and the best point among
    /// nominal heterotrophs (`heterotrophy > photosynthetic_absorption`, the
    /// role split `topology::trophic_roles` makes).
    #[test]
    fn read_names_the_positive_reach_and_the_best_nominal_heterotroph() {
        let (params, dist) = sample_31();
        let line = HeterotrophLine::from_founders(&dist);
        let read = line.read(&params);
        let reach = read
            .positive_reach
            .expect("sample:31's line pays somewhere");
        assert!(line.terms_at(reach, &params).margin > 0.0);
        let next = reach + 1.0 / LINE_STEPS as f32;
        assert!(next > 1.0 || line.terms_at(next, &params).margin <= 0.0);
        let het = read.best_heterotroph;
        let p = line.phenotype(read.best_heterotroph_t);
        assert!(p.heterotrophy > p.photosynthetic_absorption);
        assert_eq!(het, line.terms_at(read.best_heterotroph_t, &params));
        assert!(het.margin >= read.full.margin);
        assert!(het.margin <= read.best.margin);
    }

    /// The strict margin also charges the trait terms #587 left out (the
    /// phenotype's photosynthesis, mobility and asexual-propensity
    /// maintenance), and the break-even count is how many carcasses in reach
    /// each tick the maintenance needs.
    #[test]
    fn strict_margin_charges_the_other_trait_terms_and_break_even_counts_carcasses() {
        let (params, dist) = sample_31();
        let line = HeterotrophLine::from_founders(&dist);
        let x = params.maintenance_cost_exponent;
        let p = line.phenotype(0.5);
        let terms = line.terms_at(0.5, &params);
        let other = p.photosynthetic_absorption.powf(x) * params.photo_maintenance_cost
            + p.mobility.powf(x) * params.mobility_maintenance_cost
            + p.asexual_propensity.powf(x) * params.asexual_propensity_maintenance_cost;
        assert_eq!(terms.other_trait_maintenance, other);
        assert_eq!(terms.strict_margin, terms.margin - other);
        assert_eq!(
            terms.carcasses_to_break_even,
            terms.maintenance / terms.intake_ceiling
        );
        let read = line.read(&params);
        assert!(read.best_strict.strict_margin <= read.best.margin);
        assert!(read.min_carcasses_to_break_even <= terms.carcasses_to_break_even);
        assert!(
            line.terms_at(0.0, &params)
                .carcasses_to_break_even
                .is_infinite()
        );
    }
}
