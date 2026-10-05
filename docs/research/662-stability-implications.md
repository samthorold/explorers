# Issue #662: what the stability literature implies for the world

**Status: literature review, docs only (2026-10-05); part (a) of the stepping-back research paused on
#662; input to the grill that follows; nothing decided here.**

The domain findings are in [Stability and Resilience](../ecology/stability-and-resilience.md). This
note holds what they imply for the world: where the world sits against each mechanism, how each
could be tested, which matter most, and how robustness could be measured. It also relates the
review to the fragility audit ([#693](693-fragility-audit.md)), part (b) of the same research, and
names one tension with the committed design. Every statement here about how the world behaves is
inference by analogy, not a cited finding.

## TL;DR

**A world with constant flux, a linear drain and carcasses that release nutrient only when drained
has lost two of the three feedbacks that keep real food webs alive, by construction. It can recover
the third, spatial asynchrony, only by generating it itself.**

- Its dominant failure, nutrient lockup, is the textbook case of a donor-controlled stock with no
  consumer, and an adaptive-cycle conservation phase with no release.
- The mechanisms most likely to matter are an unconditional release route, relief of per-target
  pressure at low density, and self-generated spatial asynchrony. Evolutionary suicide is the main
  suspect for the remaining collapses. Added variability ranks last.
- The robustness measures the literature supports are cause-coded death, seed-ensemble basin
  stability, a parameter-neighbourhood viability radius, pulse resistance and return time,
  role-invasion trials, and a Jensen-gap test for any claim of anti-fragility.
- #693's measurements agree: fragility tracks the carcass axis more than anything else, and it is
  a property of a world's dynamics, not of any parameter.
- The review's finding that real dead matter has abiotic exits (leaching, photodegradation, fire)
  bears on the domain claim behind *No passive decay* in
  [world rules](../system-design/world-rules.md). That is an open question for the grill (§6).

## 1. Where the world sits against each mechanism

These are the review's statements about the world, moved out of the ecology layer. Each is
reasoning from the mechanism to the world's rules.

**Stability is several properties.** The sensitivity of a world's viability to small changes in
physics fits any of three pictures: narrow basins, parameter settings near a fold, or long
transients. Each has a different remedy, so the cause has to be diagnosed before a fix is chosen.

**Donor control.** The world sits exactly at the failure condition of Moore's stabiliser. Its
carcasses release nutrient only through a heterotroph's drain, so its donor control is conditional
on drainers being present and in reach. The censuses record two things about who drains:
mixotrophs by investment drain much of the carcass pile, and decomposers by trophic role are a small
minority, under 1 %. The world is also closed, so its "donor" is its own mortality.

**Unconditional exits.** The world has none of the abiotic routes. It shares paludification's loop:
a growing pile depletes the pool, fewer agents are born, fewer agents drain, and the pile grows
larger relative to its drainers. That is a self-reinforcing absorbing state, not a slow drift. Real
lockup is partial and slow because abiotic routes and a redundant decomposer guild keep throughput
going and fire resets the store. The world lacks both the redundancy floor and the reset.

**Stoichiometry.** The world's drainer keeps the nutrient its demand calls for and excretes the rest
to the pool. Whether a drained carcass feeds the pool or the drainer therefore turns on the same
ratio as Manzoni's threshold, and a world whose drainers retain most of the carcass nutrient can be
draining its carcasses and still starve the pool. The world has no analogue of the flexible
carbon-use efficiency that real decomposers use to avoid immobilisation. Mixotrophs both take up
pool nutrient and drain carcasses, which is Daufresne and Loreau's competitive pathway; decomposers
by role, which take no pool nutrient, sit on the facilitative side.

**Recycling channels.** A world in which one class of drainer carries nearly all carcass return has a
single channel. Mixotroph draining is episodic and tied to the state of the producers; draining by
obligate decomposers would be steadier and follow the pile. A world served only by the first lacks
the slow reservoir channel.

**Low-density refuge.** The world's drain is type I. Targets are discrete and local, every consumer
drains every target in reach, and nothing reduces the per-target drain as targets thin out, so a
local cluster of consumers can strip a patch to zero. Carcasses are drained by the same unsaturated
heterotrophy, so strong heterotrophs clear detritus quickly and weak ones leave it locked.

**Adaptive foraging.** The world's consumers have no effort budget and no choice. Adding targets
within reach adds strong links without diluting any: May's destabilising case, not Kondoh's. The
only choice a consumer makes is recognition, which spares near-identical targets, and it is fixed,
not adaptive.

**Weak links.** A consumer's drain speed is the same on every target in reach and is set by its
heterotrophy alone. Trait-space distance changes how efficiently the drain transfers, and recognition
spares near-identical targets, but neither creates a few strong, well-placed links among many weak
ones. Variation in link strength comes mostly from variation in heterotrophy between consumers.

**Size structure.** The drain has no size scaling, so consumers and targets run on similar time
scales: the destabilising end of Brose's range, a mass ratio near 1.

**Space.** The world has no exogenous spatial or temporal heterogeneity: flux is constant and the
nutrient grid is a resource, not an environment. Insurance mechanisms that need imposed asynchrony
are therefore expected to be weak or absent. The Huffaker recipe needs producers to out-disperse
consumers, measured against the extent of the world; if evolution pushes heterotroph mobility above
producer dispersal, the refuge disappears. Founder aggregation is a one-off initial condition, so it
should mostly affect early transients, not the long-run outcome.

**Viscosity.** Agents move on an open torus rather than filling fixed slots, which plausibly provides
the demographic elasticity that breaks Taylor's cancellation. Asexual propensity raises local
relatedness.

**Variability.** Constant flux removes the temporal storage effect by construction, and the spatial
version needs heterogeneity the world lacks. Relative nonlinearity and coexistence driven by the
world's own oscillations remain open at no extra cost. In a world whose dynamics already flip
easily, seasonality is as likely to add attractors as to stabilise, and the design rules out imposed
environmental cycles. Nutrient lockup reads as a pulse whose release step is broken. A constant-flux
world should select against bet hedging.

**Evolution.** The world mutates traits at every birth, but mutation supply scales with births, and
births collapse during a decline, so the rescue size threshold applies directly. Both collapse modes
fit the evolutionary-suicide templates. In the first, consumers that individually gain from stronger
draining deplete the shared targets and crash; the unrelieved type I drain sharpens this tragedy of
the commons. In the second, agents that individually gain from retaining nutrient dry up the pool
for everyone; the shared nutrient grid supplies the resident-dependent fitness that suicide needs.

**Artificial life.** A world that conserves nutrient and energy and never spawns agents shows an
honest base rate of death, one that older systems engineered away with reapers, top-ups and external
renewal. Avida's diversity hump predicts a humped response to nutrient supply and recycling rate,
with lockup at one end and monoculture of the fastest grower at the other. Flow-Lenia argues against
blaming conservation itself.

## 2. The four convergences, applied to the world

Each of the review's four recurring findings points at a property the world lacks.

1. **Per-target pressure does not relax at low density.** The world's linear, budget-free drain meets
   none of the forms the literature asks for (refuge, effort budget, interference, size structure),
   on living targets or on carcasses.
2. **Donor control is lost when decomposers are scarce.** The world's release is conditional on
   drainers being present and in reach, and decomposers by role are under 1 % of agents.
3. **Lockup is a stuck release phase.** Capital accumulates in carcasses and the release step is
   broken. Every literature offers a release that is either unconditional or comes from a guild that
   tracks pulses. Disturbance treats the symptom; a working decomposer niche treats the cause.
4. **Space stabilises only through self-generated asynchrony.** The world has neither imposed
   heterogeneity nor forcing. What remains is out-of-phase local consumer–producer dynamics under
   limited, asymmetric dispersal, and the selection for restraint that viscosity can bring.

## 3. Relation to #693's fragility audit

The audit ([#693](693-fragility-audit.md)) measured, on the committed atlas, what this review
predicts from the literature. Where they meet:

- **Two populations of worlds.** 57 of 99 worlds have seeds that all agree and barely move under
  jitter (3–10 % flips); 42 have seed disagreement and flip a fifth to a third of the time. That is
  the picture of narrow basins or long transients in part of the atlas, not of parameters near a
  fold: two worlds only (18, 88) look like the fold picture.
- **Seed noise is parameter fragility** (ρ +0.80). The review's distinction between engineering
  resilience and ecological resilience predicts that these could diverge; on this atlas they do not.
  Fragility is a stochastic knife-edge in the dynamics, which is what seed-ensemble basin stability
  (§5) measures directly.
- **The search cannot see it.** Seed-to-seed fitness sd is 0.149 against a median cell fitness of
  0.191. The review's binomial argument for 20–50 seeds and #693's standard-error argument (about
  0.07 on a 5-seed mean) reach the same place by different routes.
- **Elites lean fragile** (fitness ρ +0.27). Fitness rewards oscillation, turnover and coexistence,
  and the review notes that the stabilising ingredients (a low-density refuge, slower consumer rates)
  are ones that damp oscillation. A fitness that pays for amplitude will tend to select worlds without
  them.
- **Fragility tracks the carcass axis** (ρ +0.37, stronger than any other axis or fitness). This is
  the review's first-ranked mechanism showing up in measurement: the knife-edge sits where donor
  control is lost.
- **No parameter carries it** (strongest attribution ρ 0.08). This fits the review's view that the
  missing feedbacks are structural, not settings. No value of an existing parameter supplies an
  unconditional release route or a low-density refuge.

## 4. Mechanisms, translations and readouts

Each mechanism maps onto a concrete test in the world. The intervention arms are research
instruments: they identify which feedback is load-bearing. They are not proposals to change the
committed rules. The design rules out imposed environmental cycles and treats carcass return
through drain as deliberate physics (but see §6).

| Mechanism | Translation into the world | Prediction and readout |
|---|---|---|
| Donor-controlled detritus (Moore 2004) | Measure how strongly total carcass drain responds to the size of the carcass-locked pool over a run | Survivors show a clearly positive response; lockup worlds show a response near zero, because drain is reach-limited rather than mass-action |
| Unconditional release (leaching, fire) | Add a counterfactual leak from carcass to pool and sweep its rate over decades on the dead frontier's lockup cells; add a one-shot "fire" that releases a share of carcass nutrient once the locked fraction crosses a threshold | A tiny leak that rescues most lockup worlds means the failure is structural (no unconditional path); needing a large leak means it is a rate deficit. Re-locking after the fire means drain throughput is too low |
| Lockup as positive feedback (paludification) | Along failing runs, regress the growth rate of the carcass-locked fraction on the fraction itself | A positive slope above a threshold marks a tipping point and gives a critical value for the existing behaviour axis |
| Stoichiometric recycler versus hoarder (Daufresne & Loreau; Manzoni) | Log, per carcass drain, nutrient excreted to the pool against nutrient retained; plot lockup frequency against the stoichiometric mismatch between producers and drainers | Lockup worlds retain most carcass nutrient; lockup rises with the mismatch, perhaps at a threshold |
| Two recycling channels (Rooney; Moore) | Share of carcass drain carried by the largest drainer class (mixotrophs, decomposers by role, scavenging consumers); exclusion runs that remove each class in surviving worlds | Worlds where one class carries more than ~80 % are lockup-prone; removing the load-bearing class shortens the time to pool exhaustion most |
| Low-density refuge (Williams & Martinez; Rall) | Factorial counterfactual arms: drain that relaxes on sparse local targets, a cap on intake, consumer interference | The refuge arm cuts extinction deaths; the cap alone raises oscillation amplitude; track the rate of local target extinctions and how deep the troughs go |
| Adaptive foraging (Kondoh; Brose) | Counterfactual effort budget split among targets by recent gain | Persistence rises with effective connectance under allocation and falls without it; stays non-positive if the budget saturates intake |
| Positioned weak links (McCann; Allesina & Tang) | Drain weighted by trait distance to a preferred offset, with a control that shuffles the weights | Positioned skew lowers amplitude and the size of extinction cascades; shuffled skew does not |
| Time-scale separation (Brose 2006; Otto 2007) | Drain and maintenance per unit structure scaling with structure to the −¼ | Persistence rises, then plateaus, with the realised consumer-to-target structure ratio |
| Self-generated spatial asynchrony (Huffaker; Holyoak & Lawler) | A shuffle control that relocates agents at random each tick; a sweep of producer dispersal against heterotroph mobility; cross-block synchrony | Shuffling sharply shortens time to collapse if space is load-bearing; persistence rises when producers out-disperse consumers; collapse is preceded by rising synchrony |
| Restraint through viscosity (Rand; Taylor) | Fixed low versus high dispersal with heterotrophy free to evolve; a variant that admits births only into vacated space | Lower evolved exploitation at low dispersal, lost when demographic elasticity is removed |
| Relative nonlinearity under constant flux (Armstrong & McGehee) | Two producer forms that differ in the curvature of their uptake, with one herbivore | Coexistence only while the herbivore sustains cycles |
| Storage effect (Chesson; Stump & Vasseur) | Decompose invasion growth rates (rerun with environment held at its mean, then competition at its mean); add varying flux only with a trait that sets each producer's flux response | Under constant flux the storage term is about zero; added variance raises minority invasion growth only with all three ingredients, and may *shorten* minority persistence time |
| Intermediate disturbance (Connell; Fox) | Patchy kills against uniform mortality at equal mean; separate arms for recycling only, killing only, and both | A hump appears only with an emergent competition–colonisation trade-off; if recycling alone rescues viability, the fragility is a cycling defect, not a coexistence defect |
| Evolutionary suicide (Gyllenberg & Parvinen) | Replay collapsed runs from checkpoints before the collapse with mutation off; inject the post-drift mutant into a fresh resident world | Frozen replays that persist mean collapse was evolution-driven; a mutant that invades and then collapses the world is suicide in the formal sense |
| Evolutionary rescue (Bell & Gonzalez) | Fork at a shock into mutation-on and mutation-off arms; compare a step change with a ramp | Rescue shows as on-persists, off-collapses, with a population-size threshold and a higher rescue probability under the ramp |
| Anti-fragility (Taleb & Douady; Ruel & Ayres) | Jensen gap: the response at a given variance minus the response at zero variance, for a named response and perturbation, holding the mean fixed | Positive and rising means anti-fragile over that range; expect total biomass to be fragile and minority invasion growth possibly convex at small variance |

## 5. Ranked shortlist

The ranking weighs three things: how directly a mechanism addresses the dominant failure, how strong
the evidence is, and whether the mechanism can work without external forcing.

| Rank | Mechanism | Why it ranks here |
|---|---|---|
| 1 | Unconditional, donor-controlled release of carcass nutrient | Lockup accounts for most deaths. Every relevant literature (detritus theory, litter decomposition, paludification, panarchy, pulses) locates stability in a release that does not hinge on a scarce consumer. The world's mean-field lockup repeller already depends on the fraction of the pile within reach, and reach is where the individual-based world departs from the reduction. #693's carcass-axis correlation points the same way |
| 2 | Recycler stoichiometry and channel redundancy | These decide whether draining actually refills the pool, and whether a second channel takes over when mixotroph draining falters. Strong theory, and the only global empirical synthesis in the review (Manzoni) |
| 3 | Low-density relief of per-target pressure | It addresses extinction and local stripping, and also the asymmetric carcass clearance that the linear drain causes. Strong theory, but the critiques (Brose 2003; the empirical rarity of type III) mean the effect should be measured, not assumed |
| 4 | Self-generated spatial asynchrony with producers out-dispersing consumers | The only spatial mechanism available without imposed heterogeneity. Strong experimental support for delaying extinction; weak support for indefinite coexistence |
| 5 | Evolutionary suicide and restraint through viscosity | Fits both collapse templates and can be diagnosed exactly with mutation-off replays in a deterministic stepper. Mostly theory |
| 6 | Relative nonlinearity and coexistence driven by the world's own oscillations | Available at no extra cost under constant flux; empirically comparable to the storage effect in yeasts |
| 7 | Imposed variability (storage effect, seasonality, disturbance) | Absent by design. Helps only with all its ingredients present, risks extra attractors and synchrony, and the strongest critique (Stump & Vasseur 2023) predicts shorter persistence in small populations |

## 6. An open question: *No passive decay*

[World rules](../system-design/world-rules.md), *No passive decay*, justifies the rule with a domain
claim: "In real ecosystems, what appears to be passive decay is always decomposition by organisms at
a finer resolution — bacteria, fungi, invertebrates." The review does not support that claim as
stated.

- **Abiotic routes exist.** Leaching removes up to a third of early mass loss in a standard
  litter-bag assay (Lind et al. 2022). Photodegradation is the dominant control on grass-litter
  decomposition in a semi-arid Patagonian grassland (Austin & Vivanco 2006). Fire mineralises dead
  matter in peat-forming boreal systems. These need no organism and act whether or not one is
  present: they are unconditional release terms.
- **Organisms still do most of the work.** Biological decomposers carry out most degradation, and
  no robust global abiotic share was found. The claim is wrong as "always", not as "mostly".
- **The closest natural analogue to lockup is paludification**, where both abiotic loss and microbial
  activity are suppressed, productivity falls 50–80 %, and fire is what eventually resets the store.
  Real lockup is partial and slow; the world's is an absorbing state.

The rule's other reason, that it makes decomposer strategies structurally necessary, is a design
choice and does not depend on the domain claim. The grill should decide whether the rule stands on
that reason alone, whether the domain sentence is corrected, and whether an unconditional release
term (the review's first-ranked mechanism) is admissible physics or a "safety valve" the rule
forbids. Nothing here changes world-rules.md.

## 7. Robustness measurements to adopt

These turn the observed knife-edge into measured properties. Most cost little beyond the runs the
search already makes. #693 has already measured the second and third on the committed atlas.

**Cause-coded death.** Every dead seed is classified as lockup, trophic extinction or monoculture, so
the atlas records why worlds fail and not only that they do.

**Seed-ensemble basin stability.** Raise the ensemble from 5 seeds to 20–50 and randomise the initial
composition, then report the fraction alive. With 5 seeds, a world that survives 4 of 5 has a 95 %
interval of roughly 0.28–0.99 (a binomial inference), so the current ensemble cannot separate robust
worlds from lucky ones ([Menck et al. 2013, *Nat. Phys.*](https://abdn.elsevierpure.com/en/publications/how-basin-stability-complements-the-linear-stability-paradigm/)).

**Parameter-neighbourhood viability radius.** The fraction alive in small balls around an elite in
normalised parameter space, and the radius at which it falls below one half: the analogue of
feasibility-domain size ([Rohr, Saavedra & Bascompte 2014, *Science*](https://www.unifr.ch/bio/en/assets/public/Research/Rudolf-Rohr/2014_Science_Rohr_et_al.pdf)).
It has to be calibrated against seed-to-seed variance; #693's basin width is this measurement.

**Pulse resistance and return time.** Cull producers, push free nutrient into carcasses, or inject an
extreme invader. Score the deviation against the world's own oscillation envelope rather than an
equilibrium. Record Holling-style resilience, the largest pulse a world absorbs, separately, because
the two are expected to correlate only weakly.

**Role-invasion trials.** Remove a trophic role, let the world settle, reintroduce a small propagule,
and measure its early per-capita growth: an empirical proxy for permanence ([Law & Morton 1996,
*Ecology*](https://pure.york.ac.uk/portal/en/publications/permanence-and-the-assembly-of-ecological-communities)).
It also catches coexistence that is really slow exclusion within the run. #443's invasion-growth
work is the existing instrument nearest to this.

**Free early-warning statistics.** Rolling lag-1 autocorrelation and variance of the free pool and role
biomass, with the trajectory of the carcass-locked fraction, which cycles in healthy worlds and rises
monotonically in dying ones. Flags, not proofs: the same signals precede non-catastrophic
transitions.

**Global screening of the physics.** One-factor-at-a-time sweeps first ([ten Broeke et al. 2016,
*JASSS*](https://jasss.soc.surrey.ac.uk/19/1/5.html)), then a Morris screen across all 34 parameters,
and variance-based (Sobol) analysis only on the few Morris flags. #693's per-parameter attribution
(no parameter above ρ 0.08) suggests the screen will find interactions, not single knobs.

**Structural robustness analysis.** Re-evaluate elites under variant rules, such as a leaching rule or
a different drain form ([Grimm & Berger 2016, *Ecol. Model.*](https://econpapers.repec.org/RePEc:eee:ecomod:v:326:y:2016:i:c:p:162-167)).
Report how far atlases from different search seeds overlap.

**Jensen-gap test.** Use this for any claim that a world is anti-fragile, and keep it separate from
resilience.

## 8. What this does not show

- **Inference by analogy.** No source treats donor control in a fully closed nutrient system, type III
  responses in spatial individual-based models with discrete prey, metacommunity persistence on a
  continuous space with evolving dispersal, or basin stability in evolving individual-based
  ecosystems. Every mapping above crosses one of those gaps.
- **Evidence quality.** Much of the reading rests on abstracts; the per-source caveats are in the
  ecology doc's *Consolidated evidence caveats*.
- **The census figures are inherited.** "Decomposers by role under 1 %" and "mixotrophs drain much of
  the pile" come from earlier censuses on earlier physics; they were not re-measured for this note.
- **The ranking is judgement.** It weighs evidence strength and fit to lockup; it is an input to the
  grill, not a plan.
