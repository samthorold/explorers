# Nutrient is embodied matter, with a reproductive-nutrient earmark mirroring energy

_Extended by [ADR-0004](0004-reproductive-nutrient-from-all-income.md)._ The embodiment and the reproductive-nutrient earmark established here stand. ADR-0004 clarifies that "each tick's uptake" means **all** nutrient income — autotrophic pool uptake *and* the nutrient ingested by consumption — so the earmark is route-agnostic and heterotrophs can fund reproduction from prey. The wiring here fed the earmark only from pool uptake.

Nutrient is treated as **matter embodied in biomass**: growth consumes free nutrient and binds it into structure (`bound nutrient = structure × demand`), released only when that structure is grazed or returned to a carcass at death. Within a living agent, nutrient takes the same shape as energy — a free store mirrors reserve, and a **reproductive-nutrient earmark** (fed the `(1 − kappa)` share of each tick's uptake, off-limits to growth) mirrors the reproductive allocation. Reproduction is gated on that earmark via a `reproduction_nutrient_threshold` and donates it to offspring; the former body-support gate (`nutrient ≥ structure × demand`) is dropped. This supersedes the build-permit mechanism of [ADR-0002](0002-growth-co-limited-by-nutrient.md).

## Problem

The build-permit (ADR-0002) fixed an energy-abundance lockout but produced the mirror failure (issue #269): the per-tick order absorb → grow → reproduce let growth greedily convert each tick's nutrient up to the permit ceiling `structure = nutrient / ratio`, leaving the agent pinned *exactly* on the reproduction gate `nutrient ≥ structure × demand` — never above it, never with free nutrient to donate. The root cause: **nutrient had no allocation split between growth and reproduction.** Energy has one (kappa → soma vs. reproductive allocation); nutrient did not, so whichever phase touched the single store first (growth) consumed all of it. No parameter setting changes this — it is structural.

A second, deeper incoherence: the build-permit said a living body binds *no* matter, yet grazing (`phase.rs:469`) and death already transferred nutrient *as if* it were embodied in structure. The model contradicted itself about what nutrient is, and contradicted `docs/ecology/nutrient-cycling.md`, where nutrient is matter held under homeostatic stoichiometry and the entire detrital chain exists to liberate nutrient locked in dead biomass.

## Considered options

- **κ-style earmark on a build-permit (rejected).** Add a reproductive-nutrient earmark but keep the somatic side a non-consumed permit. Fixes the pinning, but leaves the build-permit's self-contradiction (living bodies bind no matter, yet release it when grazed/dead) and keeps a redundant somatic-nutrient account that only ever grows.
- **Embodiment + reproductive earmark (chosen).** Growth consumes free nutrient into structure (matter is bound); the only earmark needed is the reproductive one, mirroring `repro_reserve` one-to-one. This makes nutrient structurally identical to energy, makes growth consistent with the grazing/death code that *already* assumed embodiment, and matches the documented domain. The earmark is off-limits to growth, so the pinning cannot recur. The lockout risk ADR-0002 cited for consumption ("the reproduction gate would read a free store that growth depletes") is neutralised: the gate now reads the earmark, which growth provably cannot touch.
- **Retune parameters only (rejected, again).** The lockout is structural, not parametric — confirmed in issue #269.

## Consequences

- **One allocation gene, both currencies.** The same kappa splits energy surplus and nutrient uptake. No new trait dimension — trait space stays 7-dimensional.
- **Grow-vs-breed tension on both axes.** Growth literally spends nutrient that could otherwise have been earmarked; high-fecundity events under-provision each offspring in both energy and nutrient (Smith & Fretwell), and a nutrient-starved offspring cannot grow and is selected against. Over-fecundity under scarcity self-corrects.
- **Grazing releases bound nutrient only.** A non-lethal graze transfers the nutrient bound in the structure eaten; the victim's free store and reproductive earmark stay with it until death. (This changes the prior behaviour of grazing the whole store proportionally.)
- **Decomposition matters more.** Nutrient sequestered in standing biomass returns to circulation only via grazing or death-then-decomposition — sharpening the decomposition-production coupling the ecology docs describe.
- **Conservation still holds trivially.** Growth binds nutrient internally (agent total unchanged); reproduction moves nutrient parent → offspring (living-pool total unchanged); death moves everything to the carcass. No nutrient is created or destroyed.
- **Implementation touches grow, reproduce, the reproduction gate, and the drain (grazing) phase**, plus a new `reproduction_nutrient_threshold` world parameter. Bound nutrient is derivable as `structure × demand` from state the carcass already carries, so no new persistent field is strictly required — an implementation detail for the follow-up.
