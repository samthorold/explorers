# End State

What the world is meant to become, stated as **hypotheses**: claims each part of the end state rests on, none of them known to hold yet. Each hypothesis names the design sections it maps, its domain source, and its **reading rule**: what its test must show for it to be *supported*, *refuted* or *undecided*, declared before any reading it judges. The terms are the Commons' (*End states and hypotheses*, `~/.claude/commons/CONTEXT.md`).

This document carries no status. Whether a hypothesis is supported, refuted, undecided or Due lives only in the [register](register.md). A refuted hypothesis changes this design, and that change is how the end state is revised. A declared value (a rule, a panel, a review interval) changes only together with such a change.

## The purpose

**The world's ecological dynamics emerge from per-agent rules, with nothing hardcoded.** Every behaviour the world shows over time, from a stand's biomass curve to a seed ensemble's persistence, comes out of the [world rules](world-rules.md) acting on individual agents. No mechanism produces a phenomenon it is meant to show.

The game's arc ([project vision](../agents/project-vision.md): confusion, wonder, symbiosis) is not in the end state. It is one way a player moves through the world, not a property the world must have, and the player is outside the boundary ([system design](README.md), *Purpose before structure*). The world has to be worth moving through; how a player moves through it is designed on top of it.

## The tree

Under the purpose sit three kinds of part. The first gates the other two.

1. **[The physics is sound](#part-1-the-physics-is-sound).** Every world rule is a hypothesis, mapped from [world rules](world-rules.md) and the [execution model](execution-model.md) as they stand. It gates every other part: an emergent finding inherits every error in what it emerges from, and no reading of the finding can tell.
2. **[The world persists sensibly](#part-2-the-world-persists-sensibly).** The stage condition, at world scale: the [expected properties](expected-properties.md), refuted by the failure modes, read over a seed ensemble on found worlds. Under trophic structure sits **[the decomposer guild](#the-decomposer-guild)**.
3. **[The reference modes](#part-3-the-reference-modes)**, one part each: [colonisation overshoot](#mode-1-colonisation-overshoot), [gap dynamics](#mode-2-gap-dynamics), [shifting patch mosaic](#mode-3-shifting-patch-mosaic), [divergent recovery](#mode-4-divergent-recovery), and [mutualism](#mutualism) with no test yet.

**Instruments are not parts.** [Genesis search](genesis-search.md) finds the worlds part 2 reads; it is how part 2 gets its panel, not something the world must be. [Viability](viability.md) is closed-form code derived from the rules, tested as ordinary code: a gate that disagrees with a rollout is a bug in the gate or the rules, found by the ordinary suite. The [`reference_mode`](reference-modes.md) mesocosms are setups the part 3 hypotheses name, and the setups are authored on purpose: what they author is the starting state and the trigger, never the outcome.

*Why the physics is called the physics here.* The Emergent systems Guide calls the layer behaviour emerges from the *substrate*. In this project's language the substrate is the physical medium agents live on (`CONTEXT.md`, *Substrate*), one input to the physics, so part 1 is named for the whole: the world rules.

## How hypotheses are written and read

### Hypotheses and rules come from the domain

- **A hypothesis is a domain phenomenon** the world should reproduce, cited to its section in the [ecology layer](../ecology/). One that no ecology section supports is marked **unsourced**. Unsourced hypotheses are the ones that drift, so each is a candidate either to source or to drop.
- **Where the domain gives an empirical range, the rule uses it:** supported inside the range, refuted wholly outside it, undecided if the reading's interval straddles an end. The rule cites where the number is, and does not restate it, so the number has one home.
- **Where the domain gives only a direction,** the rule is the Emergent systems Guide's directional rule: supported when most of the panel agrees on the sign and the mean's interval excludes zero; refuted when the interval lies below the panel's measured sensitivity (a null run with identical arms, and a planted effect of known size). Where that sensitivity has not been measured, the rule says so, and until it is measured a reading can support or leave undecided but not refute.
- **Where no refuted line follows** from the domain or the design, the rule says "not yet declared". A line invented to fill the gap would be a threshold of our own with nothing behind it.
- **Exact hypotheses** (a conservation law, a closed-form relation on a few agents) are refuted by any violation and run in the fast lane, `cargo test -- --skip slow_`, on every change.
- **A phenomenon hypothesis names its setup:** the reference setup, or a controlled variant that changes one thing with all else the same. Requiring every phenomenon to appear in one reference setup is the wrong test.

### Reading rules fail fast: the refutation ladder

A reading rule declares **rungs ordered by cost**. Each rung below the top is a necessary condition that can only refute. A rung runs only after every cheaper rung holds, and only the top rung can support.

- **Why.** A reading that runs for hours to learn something a closed-form budget or a three-agent test would have shown wastes the hours, and worse, its failure cannot say where it failed. A ladder places every failure on the cheapest rung that shows it.
- **A rung that fails gets a fix designed against that rung,** not against the top rung's reading. A fix chosen to make the full reading pass, while a cheaper necessary condition is broken, tunes the world to the instrument. The fix answers the rung, the ladder is climbed again from the bottom, and the top rung reads the hypothesis as declared.
- **The top rung declares its size or its stopping rule before it runs.** A reading that leans one way without crossing its line is undecided. Adding runs to the same reading until it crosses is not a reading. A sequential rule that stops when a declared bound is crossed either way is.

The [decomposer guild](#the-decomposer-guild) declares its rungs as the worked case.

### One physics

The world has one physics. **A mechanism is in the design and runs in every world, or it is not in the code.**

- **A switch exists only to set up a controlled variant** a hypothesis reads in: a mesocosm arm without decomposers, a world without leaching to measure what leaching does. It is never how a mechanism waits to be adopted.
- **"Implemented, off by default, until a reading recommends it" is not a state the design has.** Whether a mechanism is in is decided by the ecology: the domain says the rule holds, and the design carries it. A reading that passes once a mechanism is switched on is evidence about the world with the mechanism, not a reason for the mechanism. A mechanism adopted because it made a reading pass is the phenomenon built in, whatever it is called (Emergent systems Guide, *the model never contains its phenomena*).
- **Why.** A world with optional physics is many worlds. A hypothesis read on one combination says nothing about the others, a combination chosen because its reading passed is a fitted world, and the design stops saying which world it describes.

Where the code still runs a mechanism the design states only behind a switch, the hypothesis is stated here as the design states it, and the register defers it until the code is brought into line, one mechanism a slice. [Mechanisms awaiting the one-physics decision](#mechanisms-awaiting-the-one-physics-decision) lists them.

### Due and review

A hypothesis is **Due** when it has no reading under its current rule, when its review interval has passed, or when what it was read on has changed. Due hypotheses are reviewed before any other change: read now, deferred with a reason, or the design revised.

- **A change touching a mechanism in a hypothesis's mapping makes that hypothesis Due.** The mapping is the design sections each hypothesis links to, and the code that implements them.
- **A change to the physics makes every hypothesis Due**, because every part rests on part 1. The physics is the stepper, `crates/explorers-sim/src`. Tests beside it, recorded fixtures and readouts that change no state do not count.
- **A change to a mesocosm's setup in `crates/explorers-reference-modes`** makes the hypotheses that name that setup Due. **A change to the committed atlas or recipe, or to the genesis evaluator,** makes part 2 Due.
- **The review interval is 90 days** for every hypothesis unless its rule declares another.

The register is kept by hand. Each row's Due cell says why it is Due, or the reason it was deferred.

## Part 1: the physics is sound

Each row is one world rule, mapped as it stands. *Exact* rows run in the fast lane. *Directional* rows take the directional rule above, with sensitivity not yet measured. The panel for a directional row is a set of few-agent worlds built for the rule, not genesis worlds, because the rule is about the mechanism, not about any one world.

| Id | Hypothesis | Maps to | Source | Kind and rule |
|---|---|---|---|---|
| S1 | Energy is conserved: total energy changes each tick by exactly photosynthetic input less dissipation. | [Conservation laws](world-rules.md#conservation-laws); [energy conservation verification](execution-model.md#energy-conservation-verification) | [energy flow](../ecology/energy-flow.md), *The two laws in ecology* | Exact. Refuted by any tick whose ledger misses by more than float rounding. |
| S2 | Nutrient is conserved across its four pools, founding included: total nutrient at creation is the initial pool. | [Conservation laws](world-rules.md#conservation-laws) | [trophic roles](../ecology/trophic-roles.md), *Matter: cyclic flow* | Exact, as S1. |
| S3 | Energy flows one way: heterotrophic income never exceeds primary production, so the pyramid of flow cannot invert. | [Energy stocks](world-rules.md#energy-stocks); flow 7, [Dissipation](world-rules.md#dissipation) | [energy flow](../ecology/energy-flow.md), *Pyramids of flow and pyramids of stock* | Exact. Refuted by any run whose summed heterotrophic income exceeds its summed photosynthetic input. |
| S4 | A carcass's energy leaves only through a drain; its leachable nutrient leaches first order to the pool, down to its structure's own ratio. | [Carcass energy decays only through agents; carcass nutrient leaches](world-rules.md#carcass-energy-decays-only-through-agents-carcass-nutrient-leaches) | [stability and resilience](../ecology/stability-and-resilience.md), *Real litter always has an unconditional exit*; [trophic roles](../ecology/trophic-roles.md), *Why decomposition falls to specialists* | Exact, on carcasses with no drainer. |
| S5 | Wear relaxes to an equilibrium that rises with activity and falls with kappa. | [Somatic wear](world-rules.md#somatic-wear), *Repair is first order in damage* | [life history theory](../ecology/life-history-theory.md), *The form of senescence* | Exact, on one agent held at fixed activity. |
| S6 | Senescent mortality rises with wear and then plateaus, and lifespan is a continuous gradient in kappa. | [Somatic wear](world-rules.md#somatic-wear), *Senescent death is a hazard linear in wear* | [life history theory](../ecology/life-history-theory.md), *The form of senescence*; *Senescence and mortality* | Directional: lifespan rises with kappa across the panel. Sensitivity not yet measured. |
| S7 | Light is shared by size-weighted competition: a bigger producer takes a larger share and shades smaller ones. | Flow 1, [Input flows](world-rules.md#input-flows) | [plants](../ecology/plants.md), *Light competition* | Exact: shares sum to the flux and rise with structure at fixed autotrophy. |
| S8 | Nutrient uptake shares a depleted cell in proportion to demand, and kappa splits every unit acquired between body and offspring. | Flow 2, [Input flows](world-rules.md#input-flows) | [plants](../ecology/plants.md), *Root allocation and nutrient uptake*; [trophic roles](../ecology/trophic-roles.md), *Autotrophs: nutrients from abiotic substrate* | Exact. |
| S9 | One heterotrophy drains living and dead targets alike, within feeding reach. | Flow 3, [Flows between living agents](world-rules.md#flows-between-living-agents); [Consumption is incremental](world-rules.md#consumption-is-incremental-and-its-character-emerges-from-target-and-size) | [trophic roles](../ecology/trophic-roles.md), *Consumer and decomposer: same machinery, different targets* | Exact. |
| S10 | A consumer retains nutrient while it limits growth and mineralises the excess at the feeding site. | Flow 3, [Flows between living agents](world-rules.md#flows-between-living-agents) | [animals](../ecology/animals.md), *Stoichiometric constraints*; [nutrient cycling](../ecology/nutrient-cycling.md), *Stoichiometry and nutrient recycling* | Exact. |
| S11 | Whether a drain kills follows its size relative to the target: predation, grazing and scavenging are one capability. | [Consumption is incremental](world-rules.md#consumption-is-incremental-and-its-character-emerges-from-target-and-size) | [trophic roles](../ecology/trophic-roles.md), *Body size and the geometry of heterotrophic interactions* | Directional: the share of drains that kill rises with drain size relative to the target. Sensitivity not yet measured. |
| S12 | Assimilation falls with trait distance between consumer and target, inside the domain's bounds. | Flow 7, [Dissipation](world-rules.md#dissipation), *What the two parameters stand for, and their domain bounds* | [energy flow](../ecology/energy-flow.md), *Components of transfer efficiency*; [trophic roles](../ecology/trophic-roles.md), *How efficiently decomposers use dead tissue* | Range: supported when every world's base efficiency and decay sit inside the bounds the design derives from those sections, refuted when any sits wholly outside. |
| S13 | Every capability costs upkeep, superlinear in its own investment. | Flow 8, [Dissipation](world-rules.md#dissipation); [Cost structure](world-rules.md#cost-structure-trade-offs) | Unsourced | Exact on the cost form. The domain layer gives no source for superlinear trait upkeep. |
| S14 | Growth is mobilised from reserve at a bounded rate, split by kappa at the flow, and co-limited by free nutrient. | Flow 9, [Dissipation](world-rules.md#dissipation) | [energy flow](../ecology/energy-flow.md), *Dynamic Energy Budget theory* | Exact. |
| S15 | Reproduction needs both earmarks, and a parent's investment is conserved into its offspring and heat, divided among them by fecundity. | Flow 4, [Flows between living agents](world-rules.md#flows-between-living-agents) | [life history theory](../ecology/life-history-theory.md), *Parental investment*; *Dispersal and fecundity* | Exact. |
| S16 | An agent dies when its structure falls below a fraction of its peak, and that fraction rises with the breadth of its investment. | Flow 6, [Flows from living to dead](world-rules.md#flows-from-living-to-dead); trade-off 9, [Cost structure](world-rules.md#cost-structure-trade-offs) | [energy flow](../ecology/energy-flow.md), *Dynamic Energy Budget theory* (death below a critical structure); the breadth rule is unsourced | Exact on the threshold. |
| S17 | A forager steps up the fractional gradient of what its targets release, weighted by its heterotrophy. | [Foraging movement follows what a resource releases](world-rules.md#foraging-movement-follows-what-a-resource-releases) | [spatial ecology](../ecology/spatial-ecology.md), *How decomposers find food* | Exact, on a few agents: a forager between a rich and a poor target steps toward the rich one. |
| S18 | A consumer spares living targets that resemble it, never carcasses. | [Recognition](world-rules.md#recognition-living-targets-that-resemble-the-consumer-are-spared) | [cannibalism and kin](../ecology/cannibalism-and-kin.md), *How organisms avoid eating kin* | Exact, on a few agents. |
| S19 | A world is deterministic per seed and does not depend on the order agents are stored in. | [Stochasticity and determinism](execution-model.md#stochasticity-and-determinism) | Unsourced: a property of the instrument, not of the domain | Exact. |
| S20 | Growth dilutes wear, so a body that keeps growing barely ages. | [Somatic wear](world-rules.md#somatic-wear), *Growth dilutes wear* | [fungi](../ecology/fungi.md), *The saprotroph's life history* | Exact, on one agent. The code runs it only behind a switch. |
| S21 | A living body sheds and regrows its tissue, so upkeep follows the living part. | [Somatic wear](world-rules.md#somatic-wear), *Tissue turnover* | [fungi](../ecology/fungi.md), *The saprotroph's life history* (*Upkeep as it grows*); [plants](../ecology/plants.md), *Resorption before shedding* | Exact, on one agent. Not in the code yet. |
| S22 | The base rate of upkeep is proportional to structure. | Flow 8, [Dissipation](world-rules.md#dissipation), *The base rate scales with structure* | [energy flow](../ecology/energy-flow.md), *Criticisms and limits* | Exact. The code runs it only behind a switch. |
| S23 | When reserve cannot pay, the reproductive earmark pays upkeep, then structure does. | Flow 8, [Dissipation](world-rules.md#dissipation), *The starvation cascade* | [energy flow](../ecology/energy-flow.md), *Lean times* | Exact, on one starving agent. The code runs it only behind two switches. |
| S24 | Carcass decomposition saturates in its decomposers, first order in what the carcass holds. | [Carcass decomposition saturates in its decomposers](world-rules.md#carcass-decomposition-saturates-in-its-decomposers) | [nutrient cycling](../ecology/nutrient-cycling.md), *Decomposition kinetics* | Exact, on one carcass. The code runs it only behind a switch. |
| S25 | Living tissue resists every drain, and death removes the resistance. | [Living tissue resists drains](world-rules.md#living-tissue-resists-drains) | [plants](../ecology/plants.md), *Defense and tolerance* (*Defence against microbes*) | Exact, on a few agents. The code runs it only behind a switch. |
| S26 | Reproductive isolation follows ancestry: unrelated lineages never cross, and isolation among relatives grows smoothly with divergence. | Flow 4, [Flows between living agents](world-rules.md#flows-between-living-agents), *Reproductive isolation follows ancestry* | [trophic roles](../ecology/trophic-roles.md), *A decomposer lineage stays a decomposer* | Exact for unrelated founding clusters; directional for relatives (compatibility falls with marker distance), sensitivity not yet measured. The code runs it only behind a switch. |

Rows S20 to S26 are rules the design states as the world's, which the code does not yet run in every world. They are hypotheses of the end state because the end state is the design. The register defers each until its one-physics slice lands.

### Mechanisms awaiting the one-physics decision

These are in the code behind a switch or a zero default, and the design holds them latent or in reserve rather than stating them as the world's rule. Each is decided by the ecology, one mechanism a slice: it becomes part of the physics, with a part 1 row, or it leaves the code.

| Mechanism | Design section | Domain the decision reads |
|---|---|---|
| Size-scaled nutrient uptake | Flow 2, [Input flows](world-rules.md#input-flows) | [plants](../ecology/plants.md), *Root allocation and nutrient uptake* |
| Hyphal uptake | Flow 2, [Input flows](world-rules.md#input-flows) | [fungi](../ecology/fungi.md); [trophic roles](../ecology/trophic-roles.md) |
| Network redistribution, with excretion routing | Flow 5, [Flows between living agents](world-rules.md#flows-between-living-agents) | [fungi](../ecology/fungi.md), *Mycorrhizal strategy* |
| The autotrophy × heterotrophy cross-trait cost | Trade-off 5, [Cost structure](world-rules.md#cost-structure-trade-offs) | [trophic roles](../ecology/trophic-roles.md), *Mixotrophy: how common, what it eats, what it costs* |
| Body reach (the body-extent term of feeding reach, default 0) | Flow 3, [Flows between living agents](world-rules.md#flows-between-living-agents) | [fungi](../ecology/fungi.md), *How much litter one saprotroph commands* |
| Leaching in the stepper (default 0, while genesis's worlds run at a fixed rate) | [Carcass energy decays only through agents](world-rules.md#carcass-energy-decays-only-through-agents-carcass-nutrient-leaches) | [stability and resilience](../ecology/stability-and-resilience.md), *Real litter always has an unconditional exit* |

A parameter genesis searches across a range that includes its inert value, such as the reserve mobilisation rate, is not a switch: the mechanism is in, and the search sets how strongly it acts.

## Part 2: the world persists sensibly

The stage condition, at world scale ([system design](README.md), *Purpose before structure*). Each expected property is a hypothesis, refuted by the failure modes that mark its absence ([expected properties](expected-properties.md#relationship-between-properties-and-failure-modes)).

**The panel.** Found worlds: every cell of the committed atlas, each over the audit seeds genesis already runs it on ([genesis search](genesis-search.md#genesis-selects-for-worlds-that-are-sensible-across-initial-conditions)). The panel's size is the atlas's, declared by the atlas before any reading.

**The rule, for every row unless the row says otherwise.**
- *Supported* when, in most of the panel's runs, the property's indicator is present and none of its failure modes fires.
- *Refuted* when one of its failure modes is the modal outcome over the panel's runs.
- *Undecided* otherwise.

The domain gives these properties as directions, not ranges, so the rule is the directional one, with the failure modes as the declared refuted line. The panel's sensitivity (a null panel with identical arms, and a planted effect) is not yet measured.

| Id | Hypothesis | Maps to | Refuted by | Source | Indicator |
|---|---|---|---|---|---|
| P1 | Agents differentiate into producers, consumers and decomposers, as roles that emerge. | [Trophic structure](expected-properties.md#trophic-structure) | Monoculture, generalist dominance | [trophic roles](../ecology/trophic-roles.md), *Why three roles are necessary* | The heterotroph guild fractions by role. |
| G | Litter is returned by a heterotroph guild that lives on it and stays heterotroph. | [The decomposer guild](#the-decomposer-guild), under P1 | Its ladder, below | [trophic roles](../ecology/trophic-roles.md), *Producers do not mineralise detritus themselves*, *A decomposer lineage stays a decomposer*; [fungi](../ecology/fungi.md), *The saprotroph's life history* | Read in the decomposer mesocosm, not on the panel. |
| P2 | Populations fluctuate rather than reaching fixed points. | [Population oscillations](expected-properties.md#population-oscillations) | Frozen dynamics | [animals](../ecology/animals.md), *Predator-prey oscillations* | Oscillation strength. |
| P3 | Agents cluster into distinct strategy groups with gaps between them. | [Trait-space clustering](expected-properties.md#trait-space-clustering) | Generalist dominance, monoculture | Unsourced: the domain layer has no section on speciation | Clustering strength. |
| P4 | Several clusters persist together; no strategy wins everywhere. | [Coexistence](expected-properties.md#coexistence) | Monoculture | [stability and resilience](../ecology/stability-and-resilience.md), *Constant forcing removes the storage effect, but not relative nonlinearity* | Coexistence duration. |
| P5 | Agents are born and die at non-trivial rates. | [Demographic turnover](expected-properties.md#demographic-turnover) | Frozen dynamics, extinction | [life history theory](../ecology/life-history-theory.md), *Senescence and mortality* | Demographic turnover. |
| P6 | Strategies differ in survivorship, and populations hold mixed ages. | [Age structure and survivorship variation](expected-properties.md#age-structure-and-survivorship-variation) | Uniform lifespan (no failure-mode detector exists) | [life history theory](../ecology/life-history-theory.md), *Parental investment* | No test yet. Refuted line not yet declared. |
| P7 | Sexual and asexual reproduction both occur, their balance shifting with density. | [Reproductive mode variation](expected-properties.md#reproductive-mode-variation) | Obligate sexuality or asexuality (no detector exists) | [life history theory](../ecology/life-history-theory.md), *Reproductive strategies* | No test yet. Refuted line not yet declared. |
| P8 | Energy and nutrient flow through the living system at meaningful rates; the dead do not hold them. | [Energy throughput](expected-properties.md#energy-throughput) | Energy death, nutrient lockup | [trophic roles](../ecology/trophic-roles.md), *Why three roles are necessary* | The carcass-locked fraction and the energy-death read. |

Energy conservation, listed among the expected properties, is S1: a correctness criterion for the simulation, not a property that emerges. The pyramid of flow is S3. Population explosion is a runtime guard, not an ecological threshold ([expected properties](expected-properties.md#population-explosion)), so it refutes nothing here.

### The decomposer guild

**Hypothesis G.** Litter is returned by a heterotroph guild that lives on it and stays heterotroph. In the domain plants do not depolymerise dead matter; litter is returned by fungi, bacteria and detritivores that live on it ([trophic roles](../ecology/trophic-roles.md), *Producers do not mineralise detritus themselves*). A decomposer lineage does not cross with plants and has no route to autotrophy at a stand's timescale (*A decomposer lineage stays a decomposer*). A saprotroph's genet has no age-tied hazard, and its upkeep follows its living part ([fungi](../ecology/fungi.md), *The saprotroph's life history*).

**Its setup is a controlled variant of a stand, not a found world:** the decomposer mesocosm, mode 1's mesocosm in its arm with decomposers ([reference modes](reference-modes.md#1-colonisation-overshoot), *Mesocosm*). It holds the case the hypothesis needs, a producer stand with litter and decomposer founders at the heterotroph vertex, and it is cheap enough to climb the ladder on every change to the physics. Whether the guild also forms in worlds nobody designed is P1's reading on the panel.

**What counts as the guild.** The *heterotroph guild* of the decomposer role (`CONTEXT.md`), whose members invest more in heterotrophy than autotrophy, with the producers persisting beside it. Any agent with the decomposer role is too weak a reading: the role is read from income, so producer-line mixotrophs living on carcasses carry it, and a loop closed by producers eating their own litter is not the detrital loop the domain describes ([reference modes](reference-modes.md#1-colonisation-overshoot), *Community*).

**Its ladder.**

0. **Budget arithmetic** (exact, fast lane). A lone decomposer founder's income at the domain's founding litter ratio ([fungi](../ecology/fungi.md), *Decomposer biomass against its supply*; the mesocosm derives it in *Decomposer founders*) against its upkeep, in closed form from the mesocosm's declared parameters. Refuted if income falls below upkeep: no reading above could then pass for the domain's reason.
1. **The sourced saprotroph rules** (exact, fast lane), each on a few agents:
   - **No age-tied hazard.** A decomposer that keeps growing on ample litter does not see its senescence hazard rise with age ([fungi](../ecology/fungi.md), *The saprotroph's life history*).
   - **Upkeep follows living tissue.** A decomposer whose income falls sheds and its upkeep falls with its living part (*Upkeep as it grows*).
   - **No crossing with producers.** A decomposer and a producer from unrelated founding clusters never produce an offspring together ([trophic roles](../ecology/trophic-roles.md), *A decomposer lineage stays a decomposer*).
   Each is refuted by any violation.
2. **An early window.** The mesocosm to about tick 100 on seed 1. Refuted unless at least one decomposer founder is alive at tick 50 and the founders' line has more than zero births.
3. **The full reading.** The mesocosm's arm with decomposers, run to tick 3,000, seeds 1 to 5 run in sequence. A seed passes when the guild holds over the run's second half (at least five members on every sampled tick, and a birth there with a member as parent) and the paired control's producers persist. Supported at 3 passes, refuted at 3 failures, and the sequence stops as soon as either is reached. Each run is aborted, as a failure, when the decomposer founders' line (the founders and every descendant) has no living member: the domain's guild is the founders' line grown into its litter, and a guild that arises from the producers' line instead would be producers living on litter.

The rungs are ordered by cost: arithmetic in microseconds, few-agent tests in milliseconds, one short run in about a minute, and the full reading in CPU-hours ([instrument runtimes](../agents/instrument-runtimes.md)).

## Part 3: the reference modes

The purpose of the world, stated as behaviour over time at the patch scale ([reference modes](reference-modes.md)). Each mode is one part with one hypothesis, read by a perturbation on a known state against its paired control. Each names its setup.

### Mode 1: colonisation overshoot

**Hypothesis M1. At stand scale, biomass after colonisation rises, peaks and declines, then fluctuates about a mean** ([disturbance and succession](../ecology/disturbance-and-succession.md), *Biomass after colonisation: smooth accumulation or peak and decline*). That is the domain phenomenon, and the whole of what mode 1 reads.

**Its setup is a stand like the domain's:** producers with a working decomposer guild, the mode 1 mesocosm in its arm with decomposers, with the clearance perturbation and its paired control ([reference modes](reference-modes.md#1-colonisation-overshoot)). The domain observes the overshoot in stands whose litter is returned by decomposers. It never observes it without them, so a reading on producers alone would have no source to be judged against. **So M1 is blocked by G:** until the decomposer guild is supported, the stand M1 reads is not the domain's stand, and a reading of it says nothing about the mode.

**The three loops are explanation, not hypotheses.** The dynamic hypothesis names three loops that could produce the overshoot: cohort synchrony, internal storage and nutrient held in the dead ([reference modes](reference-modes.md#1-colonisation-overshoot), *Dynamic hypothesis*). The ecology names all three mechanisms and says which dominates depends on the system; it does not rank them. A rule that read one loop as the cause would declare a ranking no source gives. They stay in the design as our explanation of how the world produces the mode, and a reading that contradicts them is a reason to revisit the explanation, not a refutation of the end state.

**The no-decomposer arm is not mode 1's.** A stand that has lost its decomposers is divergent recovery's case (M4), which reads exactly that absence. Mode 1 reads only the arm with decomposers.

**Its rule.** The domain gives a direction and a shape, not a range: directional, with sensitivity not yet measured. The ladder and the refuted line are not yet declared.

### Mode 2: gap dynamics

**Hypothesis M2.** A large producer's death opens a light gap; decomposers return its carcass's nutrient, fast-growing producers colonise the gap, and larger ones close it ([disturbance and succession](../ecology/disturbance-and-succession.md), *Gap dynamics*). **Setup:** a mesocosm with a closed canopy and resident decomposers; the perturbation kills the largest producer in a patch ([reference modes](reference-modes.md#2-gap-dynamics)). Its setup needs a working decomposer guild, so it is blocked by G as M1 is. **Rule:** not yet declared. No test yet.

### Mode 3: shifting patch mosaic

**Hypothesis M3.** Consumers deplete a patch and move on; the patch recovers, neighbouring patches cycle out of phase, and the region persists ([spatial ecology](../ecology/spatial-ecology.md), *Patch dynamics and metapopulations*; [stability and resilience](../ecology/stability-and-resilience.md), *Space stabilises through asynchrony*). **Setup:** unforced, read across neighbouring patches of a known state with consumers ([reference modes](reference-modes.md#3-shifting-patch-mosaic)). **Rule:** not yet declared. No test yet.

### Mode 4: divergent recovery

**Hypothesis M4.** Two patches lose their producers alike; where decomposers remain, nutrient returns and producers recover, and where they are lost, the pool stays depleted for longer than the leaching half-life ([stability and resilience](../ecology/stability-and-resilience.md), *Donor control holds only while decomposers track the pile*; [disturbance and succession](../ecology/disturbance-and-succession.md), *Alternative stable states*). **Setup:** two comparable patches of a known state with a working decomposer guild; one also loses its decomposers ([reference modes](reference-modes.md#4-divergent-recovery)). It reads the absence of decomposers, so it carries the contrast mode 1 does not. It needs a guild to remove, so it is blocked by G. **Rule:** not yet declared. No test yet.

### Mutualism

**Hypothesis M5.** Coupled producer–decomposer pairs outperform uncoupled neighbours in nutrient-poor patches ([fungi](../ecology/fungi.md), *Mycorrhizal strategy*; [reference modes](reference-modes.md#next-mutualism)). It follows modes 2 and 4, whose detrital loop it is layered on, and it needs network redistribution, which awaits its one-physics decision. **Rule:** not yet declared. No test yet.
