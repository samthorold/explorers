# Register

The status of every hypothesis in the [end state](end-state.md), with its last reading. Status lives here and nowhere else: the design states each hypothesis in the present tense with no status, and its reading rule is declared there.

Kept by hand. Before any change, review the rows marked Due: read the hypothesis now, defer it with a reason written into its Due cell, or revise the design. A reading is recorded with the commit it ran on; it is true of that implementation only.

- **Status** is *supported*, *refuted*, *undecided* or *Due*. *Supported* means only that the hypothesis has survived its tests so far.
- **Highest rung** is the highest rung of its ladder that held at the last reading, for a hypothesis whose rule declares a ladder; "–" otherwise.
- **Last reading** names the reading, its issue and its research note. A reading made under another rule than the hypothesis's declared one, such as a mesocosm reading or an atlas audit from before the end state, is marked so and leaves the row Due: it says what is known, not whether the rule holds.
- **Due** says why the row is Due, or why it was deferred. "No test yet" means no test implements the declared rule yet, even where an older test checks something close to it; the test is named here once it is written against the rule.

## Part 1: the physics is sound

| Part | Hypothesis | Status | Highest rung | Last reading | Due |
|---|---|---|---|---|---|
| 1 | S1 Energy is conserved | Due | – | none | Due: never read under its rule. `prop_conservation` checks the ledgers. |
| 1 | S2 Nutrient is conserved, founding included | Due | – | none | Due: never read under its rule. `prop_conservation` and `founder_nutrient_binding` check it. |
| 1 | S3 Energy flows one way; the pyramid of flow cannot invert | Due | – | none | Due: no test yet. |
| 1 | S4 Carcass energy leaves only through drains; leachable nutrient leaches first order | Due | – | none | Due: no test yet. |
| 1 | S5 Wear relaxes to an equilibrium rising with activity, falling with kappa | Due | – | none | Due: no test yet. |
| 1 | S6 Senescent mortality rises with wear and plateaus; lifespan is a gradient in kappa | Due | – | none | Due: no test yet. |
| 1 | S7 Light is shared by size-weighted competition | Due | – | none | Due: no test yet. |
| 1 | S8 Uptake shares a cell by demand; kappa splits it | Due | – | none | Due: no test yet. |
| 1 | S9 One heterotrophy drains living and dead alike, within reach | Due | – | none | Due: no test yet. |
| 1 | S10 A consumer retains limiting nutrient and mineralises the excess | Due | – | none | Due: no test yet. |
| 1 | S11 Lethality follows drain size relative to the target | Due | – | none | Due: no test yet. |
| 1 | S12 Assimilation falls with trait distance, inside the domain's bounds | Due | – | Not under its rule. The committed atlas was searched before flow 7's bound (#773): 2 of its 78 cells lie inside it, and the committed recipe's decay, 2.38, lies outside. | Due: no test yet. |
| 1 | S13 Capability upkeep is superlinear in its own investment (unsourced) | Due | – | none | Due: no test yet. |
| 1 | S14 Growth is mobilised at a bounded rate, split by kappa, co-limited by nutrient | Due | – | none | Due: no test yet. |
| 1 | S15 Reproduction needs both earmarks and conserves the investment | Due | – | none | Due: no test yet. |
| 1 | S16 Structural death at a fraction of peak structure that rises with breadth | Due | – | none | Due: no test yet. |
| 1 | S17 A forager steps up the fractional gradient of what targets release | Due | – | none | Due: no test yet. |
| 1 | S18 A consumer spares living targets that resemble it | Due | – | none | Due: no test yet; and its mapping's reason cites the fungi bullet in [cannibalism and kin](../ecology/cannibalism-and-kin.md), *Beyond animals*, whose sentence changed in #875: identity at every *het* locus is sufficient for fusion, not required, and the claim covers filamentous ascomycetes only. |
| 1 | S19 A world is deterministic per seed and order-independent (unsourced) | Due | – | none | Due: no test yet. |
| 1 | S20 Growth dilutes wear | Due | – | Not under its rule: #840, in mode 1's mesocosm ([840-mode1-ageing.md](../research/840-mode1-ageing.md)). With dilution on, decomposer founders' wear at death is a median 0.92 times that without it, too little to move their deaths. | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S21 Tissue turnover: upkeep follows the living part | Due | – | none | Due: no test yet; not in the code. |
| 1 | S22 The base rate is proportional to structure | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S23 The starvation cascade: earmark, then structure | Due | – | Not under its rule: #822, in mode 1's mesocosm ([822-mode1-lean-time-reserve.md](../research/822-mode1-lean-time-reserve.md)). The earmark draw delays the median decomposer founder's death by about 120 ticks at reach up to 10 pieces, shrinking alone by 3. | Due: no test yet; the code runs it only behind two switches until its one-physics slice. |
| 1 | S24 Carcass decomposition saturates in its decomposers | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S25 Living tissue resists drains | Due | – | Not under its rule: #813's smoke run in mode 1's mesocosm (seed 1, 300 ticks; no research note). At `ρ = 1` the decomposer guild's living share of its drain falls from 0.047 to 0. | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S26 Reproductive isolation follows ancestry | Due | – | Not under its rule: #837, in mode 1's mesocosm ([837-mode1-crossing.md](../research/837-mode1-crossing.md)). With producers and decomposer founders as unrelated founding clusters, no birth in the founders' line has a parent outside it, in 10 runs. | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |

## Part 2: the world persists sensibly

| Part | Hypothesis | Status | Highest rung | Last reading | Due |
|---|---|---|---|---|---|
| 2 | P1 Producers, consumers and decomposers emerge as roles | Due | – | none | Due: no test yet. |
| 2 | G The decomposer guild: litter is returned by a heterotroph guild that stays heterotroph | Due | 0 | Rung 0 held, on #856's merge commit: `g_rung_0_lone_founder_income_covers_its_upkeep_at_the_founding_litter_ratio`. A lone founder at litter ratio 170, under the mesocosm's rules (#814's four, `S_p` 6.937, L 1, K 1), earns 0.19890 a tick against an upkeep of 0.19865 (1.0012 times), with no margin to spare: at ratio 168 it falls below. Rung 3 has no reading under its rule. The mesocosm readings from #796 to #840 (`docs/research/`), read under #814's combined rule, never had a guild of the founders' line hold in 3 of 5 seeds; the last, #840, held a guild in 1 of 5 at five founders ([840-mode1-ageing.md](../research/840-mode1-ageing.md)). | Due: rungs 1–3 not yet read; rung 0 can refute, never support. |
| 2 | P2 Populations fluctuate | Due | – | none | Due: no test yet. |
| 2 | P3 Trait-space clustering (unsourced) | Due | – | none | Due: no test yet. |
| 2 | P4 Coexistence | Due | – | Not under its rule: #700, on the atlas then committed ([700-leaching-calibration.md](../research/700-leaching-calibration.md)). Monoculture seeds rise from 6 of 495 without leaching to 10–17 at every leaching rate above 0. | Due: no test yet. |
| 2 | P5 Demographic turnover | Due | – | none | Due: no test yet. |
| 2 | P6 Age structure and survivorship variation | Due | – | none | Due: no test yet; refuted line not yet declared. |
| 2 | P7 Reproductive mode variation | Due | – | none | Due: no test yet; refuted line not yet declared. |
| 2 | P8 Energy throughput | Due | – | Not under its rule: #700, on the atlas then committed ([700-leaching-calibration.md](../research/700-leaching-calibration.md)). Nutrient-lockup seeds fall from 19 of 495 without leaching to 2 at the rate genesis's worlds run at. | Due: no test yet. |

## Part 3: the reference modes

| Part | Hypothesis | Status | Highest rung | Last reading | Due |
|---|---|---|---|---|---|
| 3 | M1 Colonisation overshoot | Due | none | none | Due: no test yet; blocked by G. Rule declared, its refuted line not yet declared. |
| 3 | M2 Gap dynamics | Due | – | none | Due: no test yet; blocked by G; rule not yet declared. |
| 3 | M3 Shifting patch mosaic | Due | – | none | Due: no test yet; rule not yet declared. |
| 3 | M4 Divergent recovery | Due | – | none | Due: no test yet; blocked by G; rule not yet declared. |
| 3 | M5 Mutualism | Due | – | Not under its rule (none is declared): #743, the network on across the committed atlas by #736's rule ([736-ecm-reading.md](../research/736-ecm-reading.md)). Where a producer–decomposer pair forms, the routed trade carries net nutrient to the producer, but few pairs form and robustness falls. | Due: no test yet; rule not yet declared. |
