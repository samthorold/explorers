# ~~Spore dispersal: sessile agents reproduce at a distance via sensing range~~

**Superseded** — the mobility sigmoid has been replaced by contact-time gating. The effective reproduction radius now scales with contact time (consecutive ticks at a location) rather than a mobility sigmoid. Agents with high contact time can disperse spores over their sensing range; agents with low contact time require physical contact. See the world rules and CONTEXT.md for the current mechanism.

The original ADR chose to interpolate reproduction radius using the mobility sigmoid. That sigmoid has been removed from the system in favour of two structural mechanisms: the trait budget constraint (L1, traits sum to 1.0) and contact-time gating (sustained substrate contact enables nutrient uptake and spore dispersal). The consequences below from the original ADR that remain valid are marked.

## Original decision (superseded)

The effective reproduction radius was interpolated by the mobility gate: `gate * sensing_range + (1 - gate) * contact_radius`. This coupled photosynthetic gain, reproduction mechanism, and effective reproduction radius to a single sigmoid curve.

## What replaced it

The effective reproduction radius scales with contact time: `f(contact_time) * sensing_range + (1 - f(contact_time)) * contact_radius`. The function f maps contact time to a [0, 1] range. Agents that stay put develop the substrate-interfacing structures needed for spore dispersal (analogous to fruiting bodies or sporangia). Agents that move frequently cannot.

## Consequences that remain valid

- Sensing range is dual-purpose for sessile agents: detection radius and dispersal radius.
- A sessile agent that evolves high sensing range pays trait budget and metabolic cost but gains both better chemotactic awareness and wider mate access.
- The reproduction phase must compute per-agent effective radii. The pair-eligibility check is `spatial_dist <= max(effective_radius_i, effective_radius_j)`.
- Cross-type reproduction (producer × consumer) is naturally supported.
