# Reference Modes

The purpose of the world, stated as behaviour over time ([system design](README.md), *Purpose before structure*). Each reference mode is a characteristic dynamic of real ecosystems, drawn from the [ecology layer](../ecology/), that the world must be able to produce from its own rules. Each one is observed at the patch scale, one to three nutrient cells, over a horizon stated in the world's own time constants.

The ecology is encoded well when the world reproduces these behaviours. Encoding each mechanism faithfully is necessary but not sufficient: a world can be faithful mechanism by mechanism and still produce none of them.

## How a reference mode is read

A reference mode is read by a **perturbation experiment on a known state**, the way ecology establishes these dynamics: by gap creation, removal and exclusion experiments, often in mesocosms built to isolate one case.

1. Take a **known state**: a parameter set and an initial distribution, run under the rules until it settles, then snapshotted.
2. Apply the mode's canonical perturbation at the patch scale. Run the same state unperturbed beside it as the control.
3. Read the response over the mode's horizon. The mode is produced when the perturbed patch shows the stated behaviour and the control does not show it in its place.

A known state is either designed or found:
- **Designed mesocosms come first.** These are small worlds of a few patches, set up to hold the case a mode needs. For gap dynamics, that is a closed canopy with resident decomposers. They isolate the mode, run in minutes, and need no search.
- **Found states come second.** These are settled snapshots of worlds that [genesis](genesis-search.md) finds sensible. They test generality: whether the mode also arises in worlds nobody designed.

The one qualification for a known state, designed or found, is that **its control persists at the patch scale over the mode's horizon**. If the unperturbed patch is itself collapsing or blooming, the response cannot be told apart from the drift.

Why this form:
- **The response is emergent even when the state is designed.** The by-construction concern is about the outcome: a scenario arranged so that the outcome must happen tests wiring, not emergence ([expected properties](expected-properties.md), *By-construction examples test wiring, not emergence*). A designed state authors the starting point and the trigger. What follows the perturbation comes from the rules alone.
- **The perturbation supplies the rare event.** Waiting for a mode to occur on its own, say for a large producer to die in the patch being watched, turns the reading into a statistical search over many runs. The perturbation makes the event happen where and when the reading looks.
- **The paired control isolates the response.** The same state is run with and without the perturbation, so the difference is the response to the perturbation, not to anything else in the run.
- **It reads at the mode's own scale.** A mesocosm over a few generations is cheap, so a mode can be read every time the physics changes.

Reaching sensible worlds and perturbing a known state are separate concerns. Genesis answers where in parameter space sensible worlds live, at the world scale. Reference modes ask how a state responds, at the patch scale. They meet only where found states test generality.

## The first set

These four are in the first set because between them they exercise every flow and both currencies. Light competition, nutrient uptake and sharing, consumption of living targets, decomposition, leaching, growth, reproduction and dispersal each carry at least one of them. They also do not depend on any coupling mechanism beyond the committed physics.

The modes are worked in the order 1, 2, 4, 3:
- **Colonisation overshoot** needs almost no heterotroph community, so its mesocosm is the easiest to build. It is the first case for the whole method: designed state, persisting control, perturbation and paired reading.
- **Gap dynamics** adds the detrital loop and light competition.
- **Divergent recovery** reuses gap dynamics' detrital loop with that loop cut.
- **Shifting patch mosaic** is unforced, needs several patches cycling, and depends on consumers moving, so its mesocosm is the hardest.

### 1. Colonisation overshoot

**Behaviour.** An empty patch is colonised. Producer biomass rises, peaks, and declines to a lower level, around which it then fluctuates as the patch's producers come to have mixed ages.

**Perturbation.** Clear every agent and carcass from a patch of a known state. The patch's pool keeps its nutrient, and its neighbours are untouched, so colonisation comes from dispersal. The control is the same patch left alone, with producers of mixed ages.

**Horizon.** About one lifespan of the colonising cohort.

**Mesocosm.**
- **Parameters.** The committed recipe's world parameters, with somatic wear on. The recipe's other flows are already calibrated, and results carry over to found states.
  - **Exception: trophic transfer.** The recipe's decay of 2.38 lies outside flow 7's domain bound, so the mesocosm overrides it. It keeps the recipe's `base_trophic_efficiency` of 0.78, which is in bound, and sets `trophic_distance_decay` to 0.60, the bound's midpoint factor of about 0.4 at the vertex distance (world rules, flow 7, *What the two parameters stand for, and their domain bounds*).
- **Wear rate.** Producer lifespans of a few hundred ticks, shorter than the leaching half-life. This keeps the cohort's turnover apart from the slow return of nutrient through carcasses.
- **Size.** A toroidal world of 9 × 9 nutrient cells. The perturbed patch is the 3 × 3 block of cells at the centre, three cells across, the largest a patch can be. A single cell holds too few producers for its control to persist. The block has a full ring of cells around it as a source of colonists, and the wrap is far enough away that colonists do not arrive back from the far side at once. The clearance and every reading of the patch take the whole block.
- **Standing carcasses.** A colonised stand has litter, so every cell starts with standing carcasses, which carry the decomposer founders until producer deaths take over. Each cell gets a settled recipe world's carcass stock per cell area, as producer litter: 17 carcasses holding 54 energy and 58 nutrient in all. Their nutrient comes out of the cell's pool after the founders have bound theirs, as the founders' does (world rules, *Founders bind nutrient at world creation*), so total nutrient at creation is the pool. *Measured (#772, [`772-mesocosm-trophic-bound-reading.md`](../research/772-mesocosm-trophic-bound-reading.md)): row 2 of #764's reading. Decomposers persist under the override, but mixotrophs (autotrophy ≥ 0.1) take almost all the carcass-drain income, and the pure-vertex founders' line still dies out within 50 ticks.*
- **Community.** Producers and decomposers, with no consumers. Producers carry the cohort and storage loops. Decomposers carry the return from carcasses, so the mesocosm runs in two arms, with and without decomposers, to read the settled-level prediction.
- **If the control does not persist.** The recipe was calibrated without wear, so its control may not persist once wear is on. Then the wear rate is adjusted. Wear is not turned off.

**Why it is here.** At the scale of a stand, biomass after colonisation usually peaks and declines rather than levelling off smoothly. That is the aggradation and transition phases of forest development, before the shifting-mosaic steady state ([disturbance and succession](../ecology/disturbance-and-succession.md), *Biomass after colonisation: smooth accumulation or peak and decline*). The mode is the first step of a sequence the next modes continue. The declining cohort breaks up into gaps (gap dynamics), and patches cycling out of phase make the mosaic (shifting patch mosaic).

**Dynamic hypothesis.** Three loops, each with its own job. The ecology names all three, and which dominates depends on the system. In this world the first is primary.

- **Peak and decline come from cohort synchrony.** Colonists that establish within a short window form an even-aged cohort. An amplifying loop of producer biomass, reproduction and dispersal fills the patch. The shading loop then closes it to later recruits: bigger producers take a larger share of light, grow, and shade newcomers further (world rules, flow 1). So the patch holds one cohort, which wears out together through somatic wear and dies over a short span. The delay in this damping loop is the cohort's lifespan. That is why the patch overshoots rather than levelling off.
  *Predictions:* the decline begins about one cohort lifespan after colonisation, not when the pool runs dry. Colonisation spread over a longer window, so that the cohort is uneven in age, gives a lower peak.
- **The timing of the peak comes from internal storage.** An agent's free nutrient store and its reproductive nutrient earmark act like a cell quota. Growth and reproduction draw on what the agent already holds, so they continue for a while after the patch's pool is exhausted. This is a short delay, and a minor one.
  *Prediction:* births continue for a while after the patch's available pool reaches zero.
- **The settled level comes from nutrient held in the dead.** Colonising producers bind the pool into living bodies, so near the peak almost all of the patch's nutrient is alive. Once the cohort turns over, a standing share of it is always in transit through carcasses until it is drained or leaches, and the pool does not diffuse between cells to make it up. The settled living biomass therefore sits below the peak by that share.
  *Prediction:* the slower carcasses return nutrient, the lower the settled level relative to the peak. The patch's settled level is lowest with no decomposers and a low leaching rate.

### 2. Gap dynamics

**Behaviour.** A large producer dies, leaving a light gap and a carcass. Decomposers converge on the carcass and return its nutrient to the pool. Fast-growing producers colonise the gap, and slower, larger producers overtop them and close it.

**Perturbation.** Kill the largest producer in a patch of a known state.

**Horizon.** A few consumer lifespans, and shorter than the leaching half-life. The pulse back to the pool comes through decomposers, not through leaching.

**Why it is here.** Gap dynamics are the spatial unit of succession: a local reset inside an otherwise stable matrix ([disturbance and succession](../ecology/disturbance-and-succession.md), *Gap dynamics*, *Priority effects*). In this world one death touches light competition (the shading that held the gap closed), the carcass pool, decomposition, the nutrient pulse and the race between producer strategies. The mode is a causal chain through almost every flow, so it fails visibly at the first broken link.

### 3. Shifting patch mosaic

**Behaviour.** Consumers deplete a patch of producers and move on, and the patch recovers. Neighbouring patches go through the same cycle out of phase. Every patch rises and falls while the region as a whole persists.

**Perturbation.** None. The mode is an unforced behaviour, read by comparing neighbouring patches over the same span of a run from a known state. The control is the reading itself: the patches' cycles must be out of phase, not merely each cycling.

**Horizon.** Several consumer lifespans per cycle, with several cycles observed.

**Why it is here.** Regional persistence through local turnover is how space stabilises interacting species. A local population can go extinct while recolonisation from neighbours keeps the region occupied ([spatial ecology](../ecology/spatial-ecology.md), *Patch dynamics and metapopulations*; [stability and resilience](../ecology/stability-and-resilience.md), *Space stabilises through asynchrony*). It tests consumption of living targets, consumer movement and dispersal, and whether asynchrony arises without any external forcing, which this world has none of.

### 4. Divergent recovery

**Behaviour.** Two patches lose their producers alike. In the patch where decomposers remain, nutrient returns to the pool and producers recover. In the patch where decomposers are lost, the pool stays depleted and recovery takes many producer generations, driven only by leaching.

**Perturbation.** Remove the producers from two comparable patches of a known state, and remove the decomposers from one of them as well. Its carcasses stay where they are.

**Horizon.** The recovering patch recovers within a few consumer lifespans. The depleted patch stays depleted for longer than the leaching half-life.

**Why it is here.** Detritus stabilises a food web only while something eats it. When decomposers are scarce or out of reach, the dead pool integrates mortality with no damping ([stability and resilience](../ecology/stability-and-resilience.md), *Donor control holds only while decomposers track the pile*). The mode shows the detrital loop by its absence as well as its presence. It is the patch-scale version of what [expected properties](expected-properties.md) calls nutrient lockup. Whether the depleted state is a true alternative stable state, held by a reinforcing loop with hysteresis, or only slow recovery, is for the dynamic hypothesis to say. The domain has both ([disturbance and succession](../ecology/disturbance-and-succession.md), *Alternative stable states*). With leaching, this world's lockup is partial and slow, not absorbing ([world rules](world-rules.md), *Carcass energy decays only through agents; carcass nutrient leaches*).

## Next: mutualism

**Behaviour.** Coupled producer–decomposer pairs outperform uncoupled neighbours in nutrient-poor patches ([fungi](../ecology/fungi.md), *Mycorrhizal strategy*).

It follows the first set, not alongside it, because it is a coupling layered on the detrital loop that modes 2 and 4 exercise. A partnership needs decomposers that track carcasses and partners at patch scale. Until the world produces gap dynamics and divergent recovery, a failure of mutualism cannot be told apart from a failure of the loop beneath it.
