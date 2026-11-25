//! Reproductive isolation follows ancestry, through a neutral marker (#837;
//! world-rules.md flow 4), physics in every world since #861.
//!
//! Each agent carries an eight-dimensional neutral marker, inherited by
//! crossover (or copied, asexually) and stepped by a unit Gaussian per
//! dimension at every birth. A candidate pair within reach mates only if it
//! passes one keyed draw against `c = exp(−d² / (2s²))`; a seeker takes the
//! passing candidate nearest in the marker. Founding clusters start at
//! origins `10 s` apart. The rung test of S26 (end-state.md, part 1) is
//! `s26_rung_…`.

use crate::support;

use explorers_sim::phase::resolve_reproduction;
use explorers_sim::spatial::SpatialGrid;
use explorers_sim::{
    Agent, AgentSpec, MARKER_DIMS, Marker, TraitVector, World, WorldParameters, WorldRecipe,
    marker_compatibility, marker_distance_squared, marker_origin,
};

const EXTENT: f32 = 100.0;

/// The world's own physics, with a few agents in reach: no marker field is
/// set, so founding and mating run at the default compatibility scale.
fn default_physics() -> WorldParameters {
    WorldParameters {
        world_extent: EXTENT,
        initial_population_size: 0,
        reproduction_energy_threshold: 10.0,
        reproduction_nutrient_threshold: 1.0,
        mutation_rate: 0.0,
        mutation_magnitude: 0.0,
        ..support::viable_baseline()
    }
}

/// The same physics with an instrument's compatibility scale.
fn params(scale: f32) -> WorldParameters {
    WorldParameters {
        compatibility_scale: scale,
        ..default_physics()
    }
}

fn traits(photo: f32, hetero: f32, asexual: f32) -> TraitVector {
    TraitVector {
        photosynthetic_absorption: photo,
        heterotrophy: hetero,
        mobility: 1.0,
        kappa: 0.5,
        fecundity: 3.0,
        asexual_propensity: asexual,
        dispersal: 0.0,
    }
}

/// An agent ready to reproduce, at `position`.
fn breeder(id: u64, position: (f32, f32), traits: TraitVector, marker: Marker) -> Agent {
    let mut a = Agent::new(id, position, 100.0, 5.0, 10.0, traits);
    a.repro_reserve = 30.0;
    a.repro_nutrient = 30.0;
    a.marker = marker;
    a
}

/// One reproduction pass over `agents` at `tick`.
fn reproduce(
    agents: &mut [Agent],
    params: &WorldParameters,
    tick: u64,
) -> explorers_sim::phase::ReproductionResult {
    let mut grid = SpatialGrid::new(EXTENT, 10.0);
    for (i, a) in agents.iter().enumerate() {
        grid.insert(i as u64, a.position);
    }
    let dead = rustc_hash::FxHashSet::default();
    resolve_reproduction(agents, &dead, &grid, params, 7, tick)
}

fn spec(position: (f32, f32), traits: TraitVector, founding_cluster: u32) -> AgentSpec {
    AgentSpec {
        position,
        reserve: 50.0,
        traits,
        nutrient: 0.0,
        founding_cluster,
    }
}

fn recipe(params: WorldParameters, agents: Vec<AgentSpec>) -> WorldRecipe {
    WorldRecipe {
        parameters: params,
        initial_distribution: None,
        agents: Some(agents),
        carcasses: None,
        max_ticks: 100,
    }
}

/// Two co-located, trait-identical breeders with markers `a` and `b`.
fn pair(a: Marker, b: Marker) -> Vec<Agent> {
    let t = traits(0.5, 0.5, 0.0);
    vec![
        breeder(1, (10.0, 10.0), t, a),
        breeder(2, (10.5, 10.0), t, b),
    ]
}

/// The sexual pairs a reproduction pass formed, each as (low id, high id).
fn pairs(result: &explorers_sim::phase::ReproductionResult) -> Vec<(u64, u64)> {
    let mut v: Vec<(u64, u64)> = (result.events.iter())
        .filter(|e| e.kind == explorers_sim::event::EventKind::Reproduced)
        .filter_map(|e| e.target.map(|t| (e.source.min(t), e.source.max(t))))
        .collect();
    v.sort();
    v
}

/// S26's rung (end-state.md, part 1): reproductive isolation follows
/// ancestry. Exact for unrelated founding clusters: two founders of
/// different clusters, co-located, trait-identical and ready, never cross,
/// and the cross they cannot make draws nothing from either earmark (S15's
/// conservation is untouched). Close relatives stay interfertile: two
/// founders of one cluster mate. Directional for relatives: the share of
/// ticks a pair mates falls with its marker distance.
#[test]
fn s26_rung_unrelated_founding_clusters_never_cross_and_relatives_stay_interfertile_until_their_markers_diverge()
 {
    let p = default_physics();
    let s = p.compatibility_scale;
    let t = traits(0.5, 0.5, 0.0);
    let founders = |a: u32, b: u32| -> Vec<Agent> {
        let roster = vec![spec((10.0, 10.0), t, a), spec((10.5, 10.0), t, b)];
        let world = World::from_recipe(&recipe(p.clone(), roster), 1);
        (world.agents().iter().cloned())
            .map(|mut x| {
                x.repro_reserve = 30.0;
                x.repro_nutrient = 30.0;
                x
            })
            .collect()
    };
    for tick in 0..2000 {
        let mut unrelated = founders(0, 1);
        let result = reproduce(&mut unrelated, &p, tick);
        assert!(
            result.offspring.is_empty(),
            "tick {tick}: unrelated crossed"
        );
        assert!(result.events.is_empty(), "tick {tick}: a cross was made");
        for x in &unrelated {
            assert_eq!((x.repro_reserve, x.repro_nutrient), (30.0, 30.0));
        }
    }
    let mut mated = 0;
    for tick in 0..50 {
        let mut kin = founders(1, 1);
        mated += usize::from(!pairs(&reproduce(&mut kin, &p, tick)).is_empty());
    }
    assert_eq!(mated, 50, "founders of one cluster are fully interfertile");

    // Relatives at marker distance d = k s / 2: the share of ticks they mate
    // falls with d.
    let shares: Vec<f64> = (0..5)
        .map(|k| {
            let mut far = [0.0; MARKER_DIMS];
            far[3] = k as f32 * s / 2.0;
            let n = 400;
            let mated = (0..n)
                .filter(|&tick| {
                    !pairs(&reproduce(&mut pair([0.0; MARKER_DIMS], far), &p, tick)).is_empty()
                })
                .count();
            mated as f64 / n as f64
        })
        .collect();
    for w in shares.windows(2) {
        assert!(
            w[1] < w[0],
            "mating does not fall with marker distance: {shares:?}"
        );
    }
}

#[test]
fn each_founding_cluster_starts_at_its_origin_ten_scales_from_every_other() {
    let s = 3.0;
    let agents = vec![
        spec((0.0, 0.0), traits(1.0, 0.0, 0.0), 0),
        spec((1.0, 0.0), traits(1.0, 0.0, 0.0), 0),
        spec((2.0, 0.0), traits(0.0, 1.0, 0.0), 1),
        spec((3.0, 0.0), traits(0.0, 1.0, 0.0), 2),
    ];
    let world = World::from_recipe(&recipe(params(s), agents), 1);
    let m: Vec<Marker> = world.agents().iter().map(|a| a.marker).collect();
    assert_eq!(m[0], marker_origin(0, s));
    assert_eq!(m[1], marker_origin(0, s));
    assert_eq!(m[2], marker_origin(1, s));
    assert_eq!(m[3], marker_origin(2, s));
    for k in 0..=MARKER_DIMS as u32 {
        for l in 0..k {
            let d = marker_distance_squared(&marker_origin(k, s), &marker_origin(l, s)).sqrt();
            assert!(
                (d - 10.0 * s).abs() < 1e-4 * s,
                "clusters {k}, {l}: d = {d}"
            );
        }
    }
}

#[test]
fn compatibility_falls_as_a_gaussian_in_marker_distance() {
    let s = 4.0_f32;
    let zero = [0.0; MARKER_DIMS];
    assert_eq!(marker_compatibility(&zero, &zero, s), 1.0);
    for d in [0.5_f32, 1.0, 4.0, 10.0, 40.0] {
        // Spread the distance over two axes, so it is the Euclidean distance
        // over the whole marker that counts.
        let mut m = zero;
        m[2] = d * 0.6;
        m[5] = d * 0.8;
        let c = marker_compatibility(&zero, &m, s);
        let expected = (-(d as f64).powi(2) / (2.0 * (s as f64).powi(2))).exp();
        assert!(
            (c - expected).abs() <= 1e-6 * expected,
            "d = {d}: c = {c}, expected {expected}"
        );
    }
    // Unrelated founding clusters: c = e⁻⁵⁰, not lost to f32 underflow.
    let c = marker_compatibility(&marker_origin(0, s), &marker_origin(1, s), s);
    assert!((c / (-50.0_f64).exp() - 1.0).abs() < 1e-4, "c = {c}");
}

#[test]
fn mates_are_chosen_by_marker_distance_not_trait_distance() {
    let s = 1000.0;
    let near = [0.0; MARKER_DIMS];
    let mut far = near;
    far[0] = 30.0; // c = exp(−900 / 2e6) ≈ 0.9998: compatible, but farther.
    let a = traits(0.5, 0.5, 0.0);
    // C is far from A in traits, B identical to it.
    let c = TraitVector {
        mobility: 9.0,
        ..traits(3.0, 0.0, 0.0)
    };
    assert!(a.distance(&c) > 5.0);
    for tick in 0..20 {
        let mut world = vec![
            breeder(1, (10.0, 10.0), a, near),
            breeder(2, (10.5, 10.0), a, far),
            breeder(3, (10.0, 10.5), c, near),
        ];
        let formed = pairs(&reproduce(&mut world, &params(s), tick));
        assert_eq!(formed, vec![(1, 3)], "tick {tick}: the marker pairs A, C");
    }
}

/// The step each offspring marker dimension took from the parent value it
/// inherited: from whichever parent marker is nearer in that dimension.
fn steps(offspring: &[Agent], parents: &[Marker]) -> Vec<f32> {
    let mut v = Vec::new();
    for child in offspring {
        for dim in 0..MARKER_DIMS {
            let step = (parents.iter())
                .map(|p| child.marker[dim] - p[dim])
                .min_by(|x, y| x.abs().total_cmp(&y.abs()))
                .unwrap();
            v.push(step);
        }
    }
    v
}

/// Mean and mean square of `steps`.
fn moments(steps: &[f32]) -> (f64, f64) {
    let n = steps.len() as f64;
    let mean = steps.iter().map(|&x| x as f64).sum::<f64>() / n;
    let square = steps.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / n;
    (mean, square)
}

#[test]
fn a_sexual_offspring_takes_each_marker_dimension_from_one_parent_plus_a_unit_step() {
    let p: Marker = std::array::from_fn(|d| d as f32);
    let q: Marker = std::array::from_fn(|d| 1000.0 + d as f32);
    let on = params(1.0e5);
    let (mut offspring, mut from_p, mut from_q) = (Vec::new(), 0, 0);
    for tick in 0..300 {
        let result = reproduce(&mut pair(p, q), &on, tick);
        offspring.extend(result.offspring);
    }
    for child in &offspring {
        for dim in 0..MARKER_DIMS {
            let (dp, dq) = (
                (child.marker[dim] - p[dim]).abs(),
                (child.marker[dim] - q[dim]).abs(),
            );
            assert!(dp.min(dq) < 6.0, "dim {dim} came from neither parent");
            if dp < dq { from_p += 1 } else { from_q += 1 }
        }
    }
    let n = (from_p + from_q) as f64;
    assert!(n > 2000.0, "{n} dimensions inherited");
    assert!(
        (from_p as f64 / n - 0.5).abs() < 0.05,
        "{from_p} of {n} from p"
    );
    let (mean, square) = moments(&steps(&offspring, &[p, q]));
    assert!(mean.abs() < 0.1, "mean step {mean}");
    assert!((square - 1.0).abs() < 0.1, "mean square step {square}");
}

#[test]
fn an_asexual_offspring_copies_its_parent_marker_plus_a_unit_step() {
    let m: Marker = std::array::from_fn(|d| 5.0 * d as f32 - 7.0);
    let on = params(10.0);
    let mut offspring = Vec::new();
    for tick in 0..300 {
        let mut alone = vec![breeder(1, (10.0, 10.0), traits(1.0, 0.0, 1.0), m)];
        offspring.extend(reproduce(&mut alone, &on, tick).offspring);
    }
    let steps = steps(&offspring, &[m]);
    assert!(steps.len() > 2000, "{} dimensions inherited", steps.len());
    assert!(steps.iter().all(|s| s.abs() < 6.0));
    let (mean, square) = moments(&steps);
    assert!(mean.abs() < 0.1, "mean step {mean}");
    assert!((square - 1.0).abs() < 0.1, "mean square step {square}");
}

/// A world founded from an initial distribution puts its clusters, laid on
/// round robin by id, at their origins too.
#[test]
fn a_distribution_world_founds_each_cluster_at_its_origin() {
    let mut p = params(2.0);
    p.initial_population_size = 9;
    let dist = explorers_sim::InitialDistribution {
        mean_traits: traits(0.5, 0.5, 0.0),
        trait_covariance: 0.1,
        initial_cluster_count: 3,
        initial_energy_per_agent: 50.0,
        founder_aggregation: explorers_sim::DEFAULT_FOUNDER_AGGREGATION,
    };
    let world = World::new(p, dist, 3);
    for a in world.agents() {
        assert_eq!(
            a.marker,
            marker_origin((a.id % 3) as u32, 2.0),
            "agent {}",
            a.id
        );
    }
}
