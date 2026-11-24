# Register

The status of every hypothesis in the [end state](end-state.md), with its last reading. Status lives here and nowhere else: the design states each hypothesis in the present tense with no status, and its reading rule is declared there.

Kept by hand. Before any change, review the rows marked Due: read the hypothesis now, defer it with a reason written into its Due cell, or revise the design. A reading is recorded with the commit it ran on; it is true of that implementation only.

- **Status** is *supported*, *refuted*, *undecided* or *Due*. *Supported* means only that the hypothesis has survived its tests so far.
- **Highest rung** is the highest rung of its ladder that held at the last reading, for a hypothesis whose rule declares a ladder; "–" otherwise.
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
| 1 | S12 Assimilation falls with trait distance, inside the domain's bounds | Due | – | none | Due: no test yet. |
| 1 | S13 Capability upkeep is superlinear in its own investment (unsourced) | Due | – | none | Due: no test yet. |
| 1 | S14 Growth is mobilised at a bounded rate, split by kappa, co-limited by nutrient | Due | – | none | Due: no test yet. |
| 1 | S15 Reproduction needs both earmarks and conserves the investment | Due | – | none | Due: no test yet. |
| 1 | S16 Structural death at a fraction of peak structure that rises with breadth | Due | – | none | Due: no test yet. |
| 1 | S17 A forager steps up the fractional gradient of what targets release | Due | – | none | Due: no test yet. |
| 1 | S18 A consumer spares living targets that resemble it | Due | – | none | Due: no test yet; and its mapping's reason cites the fungi bullet in [cannibalism and kin](../ecology/cannibalism-and-kin.md), *Beyond animals*, whose sentence changed in #875: identity at every *het* locus is sufficient for fusion, not required, and the claim covers filamentous ascomycetes only. |
| 1 | S19 A world is deterministic per seed and order-independent (unsourced) | Due | – | none | Due: no test yet. |
| 1 | S20 Growth dilutes wear | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S21 Tissue turnover: upkeep follows the living part | Due | – | none | Due: no test yet; not in the code. |
| 1 | S22 The base rate is proportional to structure | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S23 The starvation cascade: earmark, then structure | Due | – | none | Due: no test yet; the code runs it only behind two switches until its one-physics slice. |
| 1 | S24 Carcass decomposition saturates in its decomposers | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S25 Living tissue resists drains | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |
| 1 | S26 Reproductive isolation follows ancestry | Due | – | none | Due: no test yet; the code runs it only behind a switch until its one-physics slice. |

## Part 2: the world persists sensibly

| Part | Hypothesis | Status | Highest rung | Last reading | Due |
|---|---|---|---|---|---|
| 2 | P1 Producers, consumers and decomposers emerge as roles | Due | – | none | Due: no test yet. |
| 2 | G The decomposer guild: litter is returned by a heterotroph guild that stays heterotroph | Due | 0 | Rung 0 held, on #856's merge commit: `g_rung_0_lone_founder_income_covers_its_upkeep_at_the_founding_litter_ratio`. A lone founder at litter ratio 170, under the mesocosm's rules (#814's four, `S_p` 6.937, L 1, K 1), earns 0.19890 a tick against an upkeep of 0.19865 (1.0012 times), with no margin to spare: at ratio 168 it falls below. | Due: rungs 1–3 not yet read; rung 0 can refute, never support. |
| 2 | Sp Specialists, not mixotrophs, hold living energy where nutrient is not scarce | Due | – | none | Due: never read under its rule. Its test is `reading_sp_on_its_setup`, ignored as a long run. |
| 2 | P2 Populations fluctuate | Due | – | none | Due: no test yet. |
| 2 | P3 Trait-space clustering (unsourced) | Due | – | none | Due: no test yet. |
| 2 | P4 Coexistence | Due | – | none | Due: no test yet. |
| 2 | P5 Demographic turnover | Due | – | none | Due: no test yet. |
| 2 | P6 Age structure and survivorship variation | Due | – | none | Due: no test yet; refuted line not yet declared. |
| 2 | P7 Reproductive mode variation | Due | – | none | Due: no test yet; refuted line not yet declared. |
| 2 | P8 Energy throughput | Due | – | none | Due: no test yet. |

## Part 3: the reference modes

| Part | Hypothesis | Status | Highest rung | Last reading | Due |
|---|---|---|---|---|---|
| 3 | M1 Colonisation overshoot | Due | none | none | Due: no test yet; blocked by G. Rule declared, its refuted line not yet declared. |
| 3 | M2 Gap dynamics | Due | – | none | Due: no test yet; blocked by G; rule not yet declared. |
| 3 | M3 Shifting patch mosaic | Due | – | none | Due: no test yet; rule not yet declared. |
| 3 | M4 Divergent recovery | Due | – | none | Due: no test yet; blocked by G; rule not yet declared. |
| 3 | M5 Mutualism | Due | – | none | Due: no test yet; rule not yet declared. |
