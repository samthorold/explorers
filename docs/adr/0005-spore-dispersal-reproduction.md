# Spore dispersal: sessile agents reproduce at a distance via sensing range

Sessile producers cannot move to encounter mates — the mobility gate that enables photosynthesis also prevents physical contact. Without a distance reproduction mechanism, producers never reproduce and the genesis search finds no viable ecologies.

## Considered options

- **Use sensing range as dispersal radius (chosen).** The effective reproduction radius is interpolated by the mobility gate: `gate * sensing_range + (1 - gate) * contact_radius`. Sessile agents (gate ≈ 1) reproduce over their sensing range; mobile agents (gate ≈ 0) require physical contact. A pair can reproduce when their spatial distance is within the maximum of both agents' effective radii (spores travel one way). No new parameters — reuses the existing mobility sigmoid, sensing range trait, and contact radius. Sensing range already has metabolic cost, so wide dispersal is expensive.
- **New world parameter (spore_dispersal_radius).** A fixed radius for sessile reproduction, searched by genesis. Adds a 29th search dimension. Decouples dispersal from sensing, but introduces a parameter that only matters for one trophic role.
- **Use light competition radius.** Reproduce with anyone you compete for light with. Conceptually neat but couples reproduction range to resource competition range — evolution can't tune them independently.
- **Hard cutoff instead of interpolation.** Below mobility 0.3 use sensing range, above use contact radius. Would be the only discontinuous mechanic in the simulation. The mobility gate is already continuous for photosynthesis; reproduction should match.

## Consequences

- The mobility sigmoid now governs three aspects of the sessile/mobile divide: photosynthetic gain, reproduction mechanism, and effective reproduction radius. One trait, one curve, three effects.
- Sensing range becomes dual-purpose for sessile agents: detection radius and dispersal radius. The glossary entry for sensing range should reflect this.
- A sessile agent that evolves high sensing range pays metabolic cost but gains both better chemotactic awareness and wider mate access. This is a meaningful trade-off, not a free lunch.
- The reproduction phase in `World::step` must compute per-agent effective radii instead of using the uniform `contact_radius`. The pair-eligibility check becomes `spatial_dist <= max(effective_radius_i, effective_radius_j)`.
- Cross-type reproduction (producer × consumer) is naturally supported: the producer's spore reaches the nearby consumer. The consumer doesn't need to be sessile.
