# Spatial Ecology

Space is not a backdrop — it is a driver. When organisms are distributed across a landscape rather than mixed into a homogeneous pool, qualitatively different dynamics emerge: species coexist that cannot coexist in well-mixed models, patterns form without templates, and local interactions propagate into landscape-scale structure. This document covers the systems ecology of space — the stocks, flows, and feedback loops that exist only because organisms occupy positions, move between them, and interact locally.

## Why space matters

The default assumption in much of theoretical ecology is the well-mixed (mean-field) approximation: every individual interacts with every other individual with equal probability, and resources are uniformly available. This is analytically convenient and sometimes adequate, but it is wrong in a specific and consequential way. Real organisms experience the world locally — they compete with neighbors, not with the population mean. They consume resources in their immediate vicinity, creating local depletion zones. They disperse offspring to nearby locations, not to random points in the landscape.

The consequences of this locality are not merely quantitative (slightly different parameter values) but qualitative (different outcomes entirely):

**Coexistence.** In a well-mixed Lotka-Volterra competition model, two species competing for the same resource cannot stably coexist — the competitive exclusion principle (Hardin 1960). But in spatially explicit versions of the same model, coexistence emerges readily. The mechanism is spatial segregation: inferior competitors persist in patches where the superior competitor has not yet arrived, or where local disturbance has reset the competitive clock (Tilman 1994). Spatial structure effectively fragments a single competitive arena into many semi-independent arenas operating asynchronously.

**Stability.** Predator-prey systems that are unstable in well-mixed models (oscillations amplify until extinction) can be stabilized by space. When predators and prey are distributed across a landscape, local extinctions of prey are followed by recolonization from adjacent patches, while predator populations crash locally after depleting prey and must rediscover new prey concentrations. This spatial asynchrony — different patches in different phases of the cycle — stabilizes the global population even as local populations fluctuate wildly (de Roos, McCauley & Wilson 1991, Durrett & Levin 1994). The statistical mechanics term is self-organized spatial heterogeneity: the system generates its own stabilizing structure.

**Pattern formation.** Uniform initial conditions can spontaneously generate spatial patterns — regular vegetation spots, stripes, labyrinths — through the interaction of local positive feedbacks and longer-range negative feedbacks (Turing 1952, Rietkerk et al. 2002). These patterns are not imposed by environmental heterogeneity; they emerge from the organisms' own interactions with their local environment. A well-mixed model cannot produce them because it has no concept of "local" versus "distant."

**Heterogeneity begets heterogeneity.** Spatial variation in abiotic conditions (soil depth, moisture, topography) creates variation in organism density and community composition, which in turn modifies abiotic conditions (shading, nutrient depletion, litter deposition), reinforcing and elaborating the initial heterogeneity. This is the landscape as a dynamical system — not a static template but a co-evolving surface of biotic and abiotic state variables.

The practical implication: any model of ecological dynamics that aspires to capture coexistence mechanisms, pattern formation, or spatial population persistence must be spatially explicit. Mean-field models are useful null models — they tell you what space adds by showing what happens without it — but they are not the default expectation for real ecosystems.

## Patch dynamics and metapopulations

The simplest spatial framework treats the landscape not as a continuous surface but as a collection of discrete patches, each of which can be occupied or empty. This is the metapopulation concept, formalized by Levins (1969) and developed extensively by Hanski (1998, 1999).

**The Levins model.** Let *p* be the fraction of patches occupied by a species. Occupied patches go extinct at rate *e* and colonize empty patches at rate *cp(1-p)* (colonization requires both a source patch and an empty target). The equilibrium is *p\* = 1 - e/c*. The species persists regionally (*p\* > 0*) as long as *c > e* — the colonization rate exceeds the extinction rate. This is a balance between two flows: colonization (inflow to the occupied stock) and extinction (outflow). The critical insight is that the species can persist regionally even though every local population eventually goes extinct, provided recolonization is fast enough. No single patch is permanent habitat; the metapopulation is a dynamic equilibrium of turnover.

**Hanski's incidence function model.** Levins' model assumes all patches are identical. Hanski (1994, 1998) extended it to realistic landscapes where patches vary in area (larger patches have lower extinction rates due to larger populations) and isolation (more isolated patches have lower colonization rates due to fewer incoming dispersers). The incidence of occupancy — the probability that a given patch is occupied at equilibrium — becomes a function of patch area and connectivity. This framework has been applied successfully to real metapopulations, most famously the Glanville fritillary butterfly in the Aland Islands (Hanski 1999).

**Colonization-extinction balance.** The metapopulation is a stock maintained by the balance of two opposing flows:

- **Inflow (colonization)**: proportional to the number of occupied patches (sources of dispersers) and the number of empty patches (available targets), modulated by inter-patch distance and dispersal ability.
- **Outflow (extinction)**: proportional to the number of occupied patches, with per-patch extinction rate depending on patch quality and population size (demographic and environmental stochasticity).

Disturb this balance — fragment the landscape (reducing colonization) or degrade patch quality (increasing extinction) — and the metapopulation can collapse. The collapse is often nonlinear: as patches are removed, the remaining patches become more isolated, reducing colonization further, triggering more extinctions, in a reinforcing feedback that can drive the system past an extinction threshold.

**Rescue effects.** Immigration from nearby occupied patches can reduce local extinction rates by supplementing declining populations before they reach zero (Brown & Kodric-Brown 1977). This rescue effect blurs the sharp distinction between "occupied" and "empty" patches — some patches persist not because their local dynamics are viable but because they are continually subsidized by dispersers from source populations. This is the source-sink dynamic (Pulliam 1988): source patches have positive local growth rates and export individuals; sink patches have negative local growth rates but persist through immigration.

**Patch dynamics in communities.** Extending the metapopulation concept beyond single species, patch dynamics theory describes how communities in discrete habitat patches are shaped by the interplay of local succession, disturbance, and inter-patch dispersal (Pickett & White 1985). A landscape of patches in different successional stages — recently disturbed, mid-succession, late-succession — maintains higher regional diversity than any single patch could support, because different species specialize on different stages.

## Dispersal

Dispersal is the spatial flow of individuals (or propagules) between locations. It is the connective tissue of spatial ecology — without it, every location would be an isolated system, and spatial structure would be irrelevant.

**Dispersal kernels.** The probability distribution of dispersal distances is called the dispersal kernel. Its shape matters enormously:

- **Exponential (thin-tailed) kernels**: Most dispersal events are short, with a rapid decline in probability at longer distances. This produces slow, diffusion-like spread and strong spatial autocorrelation in population density. Many wind-dispersed seeds and passively dispersing invertebrates approximate this pattern (Nathan & Muller-Landau 2000).
- **Fat-tailed kernels (leptokurtic)**: Most dispersal is short-range, but rare long-distance events occur with much higher probability than an exponential distribution would predict. These rare events have disproportionate ecological consequences — they drive rapid range expansion, colonize distant patches, and maintain gene flow across fragmented landscapes. Fat-tailed kernels arise from animal-mediated dispersal, extreme weather events, and human transport (Clark et al. 1999, Nathan et al. 2008).

The distinction is not academic. Reid's paradox — the observation that trees recolonized post-glacial landscapes far faster than their typical seed dispersal distances would allow — is resolved by fat-tailed dispersal: rare long-distance events establish distant outposts that then serve as new sources (Clark et al. 1999).

**Dispersal-fecundity trade-offs.** Producing dispersible propagules is costly. Seeds with wings, plumes, or fleshy fruits require energy investment; mobile offspring require provisioning. The fundamental trade-off is between the number of offspring (fecundity) and how far they go (dispersal distance), or equivalently between local competitive ability (investing in growth) and colonization ability (investing in dispersal). This trade-off underpins the competition-colonization coexistence mechanism: good competitors that disperse poorly coexist with poor competitors that disperse well, because the latter can always reach empty patches first (Tilman 1994, Levins & Culver 1971).

**Natal versus breeding dispersal.** Natal dispersal is the movement from birthplace to the site of first reproduction. Breeding dispersal is movement between successive breeding sites. The distinction matters because natal dispersal is typically longer and more common — it is the primary mechanism by which populations spread and gene flow occurs. Breeding dispersal is often shorter and driven by local conditions (territory quality, mate availability, predation risk).

**Dispersal and gene flow.** Dispersal is the spatial expression of gene flow. Limited dispersal produces spatial genetic structure — nearby individuals are more related than distant ones (isolation by distance, Wright 1943). This spatial genetic structure enables local adaptation, kin selection, and ultimately, if barriers to dispersal are strong enough, speciation. The tension between gene flow (homogenizing) and local selection (differentiating) is mediated by dispersal distance relative to the spatial scale of environmental variation.

**Dispersal in fragmented landscapes.** When habitat is fragmented, the matrix between patches becomes a critical factor. Dispersal is not just a distance function but a landscape-resistance function: some matrix types are hostile (cleared land for forest species), others are permeable (hedgerows, stepping-stone habitats). Connectivity — the degree to which the landscape facilitates movement between patches — is the emergent property that determines whether a fragmented metapopulation functions as a connected system or a set of isolated remnants.

## Movement ecology

Dispersal is one component of a broader phenomenon: animal movement. Nathan et al. (2008) proposed a unifying framework for movement ecology organized around four components: the internal state of the organism (why move?), the motion capacity (how to move?), the navigation capacity (where to move?), and the external environment (constraints and cues). This framework makes movement a first-class ecological process rather than a parameter to be estimated.

**Foraging movement.** How organisms move while searching for food shapes their encounter rates with resources and with each other. Two idealized models bracket the range of foraging strategies:

- **Brownian motion (random walk)**: The organism moves in random directions with fixed step lengths. This is the null model. In homogeneous environments with uniformly distributed resources, Brownian motion is adequate. But in patchy environments — where resources are clustered — it is inefficient because the organism wastes time re-searching depleted areas.
- **Levy flights**: The organism takes many short steps interspersed with rare long steps, with step lengths drawn from a power-law distribution. Viswanathan et al. (1999) argued that Levy flights optimize search efficiency in environments where resources are sparse and patchily distributed, because the long steps allow the organism to escape depleted areas and discover new resource patches. Subsequent work has refined this: the "Levy flight foraging hypothesis" holds that Levy-like movement emerges when organisms balance exploitation (short steps within a patch) with exploration (long steps between patches), and that the optimal strategy depends on the spatial distribution of resources (Humphries et al. 2010, Sims et al. 2008).

Organisms foraging via gradient-following in a landscape with patchy resource distributions produce movement patterns that fall somewhere on this continuum. The emergent movement statistics depend on the interaction between the organism's sensory range, the resource distribution, and the movement strategy.

**Optimal foraging in patchy landscapes.** The marginal value theorem (Charnov 1976) predicts when a forager should leave a depleting patch: when the instantaneous intake rate in the current patch drops below the average rate achievable by traveling to a new patch. This creates a coupling between movement and resource dynamics — the forager's decision to stay or leave depends on both local resource state and landscape-level patch distribution (travel time). In spatially explicit models, this produces characteristic patterns: foragers aggregate in high-quality patches, deplete them, then disperse to find new patches — a spatial analogue of the predator-prey oscillation.

**Home range formation.** Many animals restrict their movement to a defined home range rather than wandering freely. Home ranges emerge from the interaction of site fidelity (returning to familiar areas where resource locations and refugia are known) and avoidance of areas occupied by conspecifics (territorial exclusion) or depleted of resources. Mechanistic home range models (Moorcroft & Lewis 2006) derive home range patterns from movement rules — advection toward a central place, diffusion during foraging, and conspecific avoidance — producing realistic territory mosaics without imposing home ranges as a boundary condition.

**Migration as spatial coupling.** Migratory species physically transport energy and nutrients between ecosystems that may be separated by thousands of kilometers. Salmon carry marine-derived nitrogen into freshwater systems. Arctic terns link polar ecosystems. Wildebeest migrations redistribute grazing pressure across the Serengeti. Migration is a spatial flow of biomass that couples the dynamics of distant ecosystems — a long-range interaction in what would otherwise be a system dominated by local processes.

## Spatial pattern formation

One of the most striking phenomena in spatial ecology is the emergence of regular spatial patterns from uniform initial conditions. No template is imposed — the pattern self-organizes from the interaction of local processes.

**Turing instability in ecology.** Alan Turing (1952) showed that a system of two interacting substances with different diffusion rates can spontaneously form spatial patterns. The mechanism requires a local activator (positive feedback at short range) and a long-range inhibitor (negative feedback at long range). If the inhibitor diffuses faster than the activator, small perturbations from uniformity are amplified: the activator builds up locally faster than the inhibitor can suppress it, but the inhibitor spreads outward and suppresses growth at a distance. The result is a characteristic wavelength of pattern — spots, stripes, or labyrinths depending on parameters.

**Vegetation patterns.** The most extensively documented ecological Turing patterns occur in dryland vegetation. In arid and semi-arid regions, vegetation self-organizes into regular spatial patterns visible at landscape scale: gaps in dense vegetation, spots of vegetation in bare ground, labyrinthine stripes on slopes (Rietkerk et al. 2002, 2004). The mechanism is a vegetation-water feedback:

- **Local facilitation (activator)**: Vegetation captures water through infiltration enhancement — plant root systems and organic matter increase soil infiltration rates, and surface runoff flows from bare ground to vegetated patches. More vegetation means more water capture, means more growth. This is a short-range positive feedback.
- **Long-range competition (inhibitor)**: Root systems extract water from a zone larger than the plant canopy. Lateral water uptake by roots depletes soil moisture in surrounding areas, inhibiting growth of nearby plants. Water that infiltrates in vegetated patches is drawn from the surrounding bare ground, creating a zone of suppression around each vegetated patch.

The interplay of short-range facilitation and long-range inhibition produces the Turing instability. The characteristic pattern depends on aridity: as conditions become drier, vegetation transitions from gaps in a continuous cover, to labyrinthine stripes, to spots in bare ground, and finally to bare desert — a sequence predicted by the models and observed in the field (Rietkerk et al. 2004).

**Scale-dependent feedbacks.** The vegetation-water example illustrates the general principle of scale-dependent feedback: the sign of the feedback (positive or negative) depends on the spatial scale. At short range, the interaction between vegetation patches is facilitative (shared water capture, microclimate amelioration). At longer range, it is competitive (water depletion). This scale dependence is the spatial analog of Turing's activator-inhibitor mechanism and is the generic recipe for self-organized pattern formation in ecology (Rietkerk & van de Koppel 2008).

**Patterns as early warning signals.** Spatial pattern transitions — from gaps to labyrinths to spots — occur in a predictable sequence as environmental stress increases. This means that the spatial pattern itself carries information about the system's proximity to a critical transition (collapse to bare desert). Monitoring pattern geometry can serve as an early warning indicator, analogous to critical slowing down in temporal dynamics (Rietkerk et al. 2004, Kefi et al. 2007).

## Spatial competition

Competition in spatial settings differs fundamentally from competition in well-mixed models because the spatial arrangement of competitors determines who interacts with whom.

**Asymmetric versus symmetric competition.** Competition can be symmetric (each competitor reduces the other's resource access proportionally) or asymmetric (one competitor disproportionately suppresses the other). In terrestrial plant communities, the key distinction maps onto resource type:

- **Light competition is asymmetric.** Light arrives from above. A taller plant shades a shorter neighbor, preempting the resource entirely — the shorter plant cannot "shade back" (Weiner 1990). This asymmetry means that spatial arrangement (who is next to whom, and who got there first) matters more than average trait values. A slightly taller plant in the right position can suppress all its immediate neighbors, while the same plant surrounded by taller individuals gains nothing.
- **Soil nutrient competition is symmetric.** Nutrients diffuse through soil in all directions. Each plant's root system creates a depletion zone, and overlapping depletion zones reduce resource availability for both competitors proportionally to their uptake rates. Spatial arrangement still matters (closer neighbors compete more intensely), but the competition is size-symmetric — large and small plants deplete each other's nutrient supply in proportion to their root biomass (Schwinning & Weiner 1998).

This distinction has a spatial consequence: in systems where light is the primary limiting resource, competitive outcomes depend heavily on the spatial configuration of individuals (canopy position, gap proximity). In nutrient-limited systems, competitive outcomes depend more on trait values (root allocation, nutrient uptake efficiency) and less on precise spatial position.

**Zone of influence models.** A common spatial competition framework represents each individual as a zone of influence — a circular area centered on the individual, within which it captures resources and suppresses neighbors. Resource capture is proportional to the overlap between the individual's zone and available resource patches, minus the overlap with competitors' zones. This framework makes competition inherently spatial: the competitive effect depends on the distance between individuals, the size of their zones, and the geometry of overlap (Berger et al. 2008).

**Spatial lottery competition.** In many communities, competitive outcomes depend on who arrives first rather than who is the better competitor. A site occupied by one species is unavailable to others, regardless of competitive ability — this is preemption. In well-mixed models, the best competitor always wins. In spatial models with local recruitment, a species can hold territory through priority effects even if it would lose in direct competition, because it controls space and excludes competitors from recruitment sites. The spatial lottery model (Chesson & Warner 1981) formalizes this: species coexist because each can hold sites once occupied, and environmental fluctuations ensure that no single species monopolizes all recruitment opportunities.

**Neighborhood effects.** In spatially explicit models, the relevant competitive environment is the local neighborhood — the set of individuals within interaction distance. This means that population-level competitive outcomes are emergent properties of many local competitive interactions, not direct consequences of species-level competitive coefficients. Two populations with identical mean traits can have different competitive outcomes depending on their spatial distributions — clustered, regular, or random (Pacala & Levin 1997).

## Feedback loops

Spatial feedbacks are feedback loops in which the spatial distribution of organisms is both a cause and a consequence of ecological dynamics.

### Reinforcing (positive) feedbacks

**Vegetation-water infiltration feedback.** In drylands, vegetated patches enhance local water infiltration, supporting further growth, while bare interspaces develop soil crusts that shed water toward vegetated patches. More vegetation concentrates more water, enabling more vegetation — a reinforcing feedback that maintains the patchy pattern and can create alternative stable states (vegetated patches vs. bare ground) at the local scale (Rietkerk et al. 2002).

**Aggregation-Allee feedback.** Many species have Allee effects — per-capita fitness increases with local population density (up to a point), due to cooperative predator defense, mate-finding efficiency, or collective environmental modification. In spatial settings, this creates a reinforcing feedback: organisms aggregate, aggregation increases fitness, higher fitness increases local population density, denser populations attract or retain more individuals. The spatial expression is clumping — populations do not spread uniformly but concentrate in high-density patches separated by low-density or empty matrix.

**Movement-resource depletion feedback.** Mobile consumers create their own resource landscape through consumption. As they deplete resources in one area, they move to richer areas, concentrating consumption there. This creates a wave of depletion that propagates across the landscape — a traveling front of consumers followed by a zone of depleted resources followed by a zone of recovery. The spatial pattern of resource availability is a direct consequence of consumer movement, and consumer movement is a direct response to resource availability — a tightly coupled spatial feedback.

**Soil-vegetation state switching.** Plant species modify soil chemistry, microbial communities, and physical structure. In spatially explicit settings, this creates sharp boundaries between vegetation types: species A modifies soil to favor species A, creating a patch of A-soil; at the boundary, species B modifies soil to favor species B. The boundary persists because each species maintains its own favorable soil conditions locally — a spatially explicit reinforcing feedback that maintains landscape heterogeneity even on uniform substrates (Wilson & Agnew 1992).

### Balancing (negative) feedbacks

**Depletion-dispersal feedback.** Consumers deplete local resources, reducing local intake rates, which triggers movement to new areas. The departure allows local resources to recover. This is a negative feedback loop that prevents permanent resource exhaustion and produces rotational use of the landscape — organisms cycle through areas, allowing each to recover before being revisited. The spatial manifestation is a shifting mosaic of use and recovery.

**Density-dependent dispersal.** As local population density increases, per-capita resource availability and space decrease, triggering increased emigration. Emigrants colonize less-crowded areas, reducing density at the source and increasing it at the destination. This is a spatial negative feedback that tends to equalize population density across the landscape — a spatial analogue of density-dependent population regulation.

**Predator-prey spatial pursuit.** Predators aggregate where prey are dense, increasing local predation pressure, which drives prey to flee or suffer mortality, reducing local prey density. Predators then redistribute to track the new prey distribution. This spatial cat-and-mouse dynamic is a negative feedback that prevents prey from permanently aggregating (predators would follow) and prevents predators from permanently depleting prey (prey flee and recover elsewhere).

### Cross-scale feedbacks

**Local interactions producing landscape patterns.** The Turing patterns described above are a cross-scale feedback: local facilitation and competition interact at the individual scale, producing patterns at the landscape scale, and those landscape-scale patterns (the spacing between vegetation patches, the connectivity of the vegetated network) in turn constrain the local dynamics by determining dispersal distances, resource redistribution, and microclimate. The landscape pattern is both a product of and a constraint on the local processes that generate it.

**Edge-mediated feedbacks.** The boundaries between habitat types (ecotones) are not passive lines but active zones with their own dynamics. Edge effects — altered microclimate, increased predation, wind exposure, light penetration — modify organism fitness at boundaries. In spatially explicit systems, the position and shape of edges feed back to the dynamics of the adjacent habitats: a forest edge recedes due to wind damage, increasing edge exposure, causing further recession — a reinforcing feedback. Conversely, edge vegetation can stabilize boundaries through windbreak effects — a negative feedback maintaining edge position.

## ABM and computational literature

Spatial ecology is where agent-based models are not merely useful but arguably necessary. The core phenomena — local interactions, individual movement, neighborhood-dependent competition, spatial pattern formation — are inherently individual-based. Differential equation models can approximate some spatial dynamics (reaction-diffusion equations, partial differential equations on continuous space), but they assume continuous population densities and struggle with individual-level stochasticity, discrete organisms, and complex movement rules. ABMs represent individuals explicitly in space and time, making them the natural modeling framework for spatial ecology.

**Cellular automata and lattice models.** The simplest spatial ABMs are cellular automata: space is discretized into a grid, each cell has a state (occupied/empty, species identity, resource level), and update rules depend on the states of neighboring cells. Despite their simplicity, cellular automata have produced fundamental insights:

- Durrett & Levin (1994) showed that spatial predator-prey models on lattices produce qualitatively different dynamics from their well-mixed equivalents — coexistence is easier, oscillations are damped, and spatial patterns emerge spontaneously. They established a classification of lattice model behaviors and their correspondence to stochastic spatial processes.
- Cellular automata models of vegetation dynamics have demonstrated how local facilitation and competition produce landscape-scale patterns without environmental heterogeneity (Scanlon et al. 2007).
- The contact process — a simple lattice model where occupied sites can colonize empty neighbors and go extinct stochastically — is the spatial equivalent of the logistic model and exhibits a phase transition between survival and extinction at a critical colonization rate.

**Spatially explicit forest models.** Forest dynamics have been a major application domain for spatial ABMs:

- **SORTIE** (Pacala et al. 1996) is a spatially explicit, individual-based forest model where each tree is located in continuous space, competes for light based on its position relative to neighbors, disperses seeds according to empirically measured kernels, and grows or dies based on local conditions. SORTIE demonstrated that spatial processes — gap dynamics, seed dispersal limitation, neighborhood competition — are necessary to explain observed forest composition and diversity.
- **FORMIND** (Kohler & Huth 1998, Fischer et al. 2016) applies similar principles to tropical forests, tracking individual trees in spatial patches and simulating light competition, growth, mortality, and recruitment. It has been used to assess logging impacts, climate change effects, and successional dynamics.
- **iLand** (Seidl et al. 2012) extends spatially explicit forest modeling to landscape scales, coupling individual-based tree dynamics with spatial processes including seed dispersal, disturbance spread (fire, wind), and management.

**Predator-prey persistence in space.** de Roos, McCauley & Wilson (1991) used individual-based spatial models to demonstrate that predator-prey systems that are unstable in well-mixed (nonspatial) models can persist stably when individuals occupy positions in space. The mechanism is spatial self-structuring: predators create local prey-free zones, prey populations recover in predator-free areas, and the spatial mosaic of predator and prey patches maintains the global population. This result was foundational for establishing that spatial structure is not a detail but a qualitative determinant of population dynamics.

**Movement ecology ABMs.** Computational models of animal movement have become increasingly sophisticated:

- Mechanistic home range models represent individuals as agents moving according to rules (directed movement toward resources, away from conspecifics, toward familiar areas) and derive emergent home ranges and territory patterns (Moorcroft & Lewis 2006).
- Foraging models implement optimal foraging rules in spatially explicit resource landscapes, producing realistic movement statistics (Levy-like vs. Brownian) as emergent properties of the interaction between agent behavior and resource distribution, rather than imposing them as movement rules (Humphries et al. 2010).
- Social foraging models simulate group formation and collective search, showing how information sharing about resource locations (direct observation, following, signal production) can improve group-level foraging efficiency at the cost of increased local competition — a spatial social dilemma.

**Grimm and Railsback on spatial IBMs.** Grimm & Railsback (2005) provide the standard methodological reference for individual-based models in ecology. Their treatment of spatial IBMs emphasizes: (1) the importance of representing space explicitly when local interactions drive system dynamics, (2) the use of the ODD (Overview, Design concepts, Details) protocol (Grimm et al. 2006, 2010) for standardized model description, and (3) pattern-oriented modeling — using observed spatial patterns as targets for model calibration and validation. Spatial patterns are particularly valuable as validation targets because they integrate many underlying processes and are difficult to reproduce with incorrect mechanisms.

**NetLogo spatial models.** NetLogo (Wilensky 1999) has been the dominant platform for educational and exploratory spatial ABMs. Its built-in support for grid-based and continuous-space models, combined with an extensive model library, has made spatial ecological ABMs accessible. Relevant library models include predator-prey dynamics on grids, fire spread, disease transmission in spatial networks, vegetation pattern formation, and flocking/schooling as spatial self-organization.

**Continuous-space ABMs.** While lattice models are analytically tractable, many ecological systems operate in continuous space — organisms do not snap to grid cells. Continuous-space ABMs place individuals at real-valued coordinates, compute interactions based on Euclidean distances, and allow continuous movement. This is more realistic but computationally more demanding (neighbor-finding requires spatial indexing rather than simple grid lookups). Efficient spatial data structures (k-d trees, spatial hashing) are required to maintain performance as agent counts grow.

## References

- Bar-On, Y.M., Phillips, R. & Milo, R. (2018) The biomass distribution on Earth. *Proceedings of the National Academy of Sciences*, 115, 6506-6511.
- Berger, U., Piou, C., Schiffers, K. & Grimm, V. (2008) Competition among plants: concepts, individual-based modelling approaches, and a proposal for a future benchmark model. *Ecological Modelling*, 204, 270-297.
- Brown, J.H. & Kodric-Brown, A. (1977) Turnover rates in insular biogeography: effect of immigration on extinction. *Ecology*, 58, 445-449.
- Charnov, E.L. (1976) Optimal foraging, the marginal value theorem. *Theoretical Population Biology*, 9, 129-136.
- Chesson, P.L. & Warner, R.R. (1981) Environmental variability promotes coexistence in lottery competitive systems. *American Naturalist*, 117, 923-943.
- Clark, J.S., Silman, M., Kern, R., Macklin, E. & HilleRisLambers, J. (1999) Seed dispersal near and far. *Ecology*, 80, 1475-1494.
- de Roos, A.M., McCauley, E. & Wilson, W.G. (1991) Mobility versus density-limited predator-prey dynamics on different spatial scales. *Proceedings of the Royal Society B*, 246, 117-122.
- Durrett, R. & Levin, S.A. (1994) The importance of being discrete (and spatial). *Theoretical Population Biology*, 46, 363-394.
- Fischer, R., Bohn, F., Dantas de Paula, M. et al. (2016) Lessons learned from applying a forest gap model to understand ecosystem and carbon dynamics of complex tropical forests. *Ecological Modelling*, 326, 124-133.
- Grimm, V. & Railsback, S.F. (2005) *Individual-based Modeling and Ecology*. Princeton University Press.
- Grimm, V., Berger, U., Bastiansen, F. et al. (2006) A standard protocol for describing individual-based and agent-based models. *Ecological Modelling*, 198, 115-126.
- Grimm, V., Berger, U., DeAngelis, D.L., Polhill, J.G., Giske, J. & Railsback, S.F. (2010) The ODD protocol: A review and first update. *Ecological Modelling*, 221, 2760-2768.
- Hanski, I. (1994) A practical model of metapopulation dynamics. *Journal of Animal Ecology*, 63, 151-162.
- Hanski, I. (1998) Metapopulation dynamics. *Nature*, 396, 41-49.
- Hanski, I. (1999) *Metapopulation Ecology*. Oxford University Press.
- Hardin, G. (1960) The competitive exclusion principle. *Science*, 131, 1292-1297.
- Humphries, N.E., Queiroz, N., Dyer, J.R.M. et al. (2010) Environmental context explains Levy and Brownian movement patterns of marine predators. *Nature*, 465, 1066-1069.
- Kefi, S., Rietkerk, M., Alados, C.L. et al. (2007) Spatial vegetation patterns and imminent desertification in Mediterranean arid ecosystems. *Nature*, 449, 213-217.
- Kohler, P. & Huth, A. (1998) The effects of tree species grouping in tropical rainforest modelling. *Ecological Modelling*, 109, 301-321.
- Levins, R. (1969) Some demographic and genetic consequences of environmental heterogeneity for biological control. *Bulletin of the Entomological Society of America*, 15, 237-240.
- Levins, R. & Culver, D. (1971) Regional coexistence of species and competition between rare species. *Proceedings of the National Academy of Sciences*, 68, 1246-1248.
- Moorcroft, P.R. & Lewis, M.A. (2006) *Mechanistic Home Range Analysis*. Princeton University Press.
- Nathan, R. & Muller-Landau, H.C. (2000) Spatial patterns of seed dispersal, their determinants and consequences for recruitment. *Trends in Ecology & Evolution*, 15, 278-285.
- Nathan, R., Getz, W.M., Revilla, E. et al. (2008) A movement ecology paradigm for unifying organismal movement research. *Proceedings of the National Academy of Sciences*, 105, 19052-19059.
- Pacala, S.W. & Levin, S.A. (1997) Biologically generated spatial pattern and the coexistence of competing species. *Spatial Ecology: The Role of Space in Population Dynamics and Interspecific Interactions* (eds D. Tilman & P. Kareiva), pp. 204-232. Princeton University Press.
- Pacala, S.W., Canham, C.D., Saponara, J., Silander, J.A., Kobe, R.K. & Ribbens, E. (1996) Forest models defined by field measurements: estimation, error analysis and dynamics. *Ecological Monographs*, 66, 1-43.
- Pickett, S.T.A. & White, P.S. (1985) *The Ecology of Natural Disturbance and Patch Dynamics*. Academic Press.
- Pulliam, H.R. (1988) Sources, sinks, and population regulation. *American Naturalist*, 132, 652-661.
- Rietkerk, M. & van de Koppel, J. (2008) Regular pattern formation in real ecosystems. *Trends in Ecology & Evolution*, 23, 169-175.
- Rietkerk, M., Boerlijst, M.C., van Langevelde, F. et al. (2002) Self-organization of vegetation in arid ecosystems. *American Naturalist*, 160, 524-530.
- Rietkerk, M., Dekker, S.C., de Ruiter, P.C. & van de Koppel, J. (2004) Self-organized patchiness and catastrophic shifts in ecosystems. *Science*, 305, 1926-1929.
- Scanlon, T.M., Caylor, K.K., Levin, S.A. & Rodriguez-Iturbe, I. (2007) Positive feedbacks promote power-law clustering of Kalahari vegetation. *Nature*, 449, 209-212.
- Schwinning, S. & Weiner, J. (1998) Mechanisms determining the degree of size asymmetry in competition among plants. *Oecologia*, 113, 447-455.
- Seidl, R., Rammer, W., Scheller, R.M. & Spies, T.A. (2012) An individual-based process model to simulate landscape-scale forest ecosystem dynamics. *Ecological Modelling*, 231, 87-100.
- Sims, D.W., Southall, E.J., Humphries, N.E. et al. (2008) Scaling laws of marine predator search behaviour. *Nature*, 451, 1098-1102.
- Tilman, D. (1994) Competition and biodiversity in spatially structured habitats. *Ecology*, 75, 2-16.
- Tilman, D. & Kareiva, P. (eds) (1997) *Spatial Ecology: The Role of Space in Population Dynamics and Interspecific Interactions*. Princeton University Press.
- Turing, A.M. (1952) The chemical basis of morphogenesis. *Philosophical Transactions of the Royal Society B*, 237, 37-72.
- Viswanathan, G.M., Buldyrev, S.V., Havlin, S. et al. (1999) Optimizing the success of random searches. *Nature*, 401, 911-914.
- Weiner, J. (1990) Asymmetric competition in plant populations. *Trends in Ecology & Evolution*, 5, 360-364.
- Wilensky, U. (1999) NetLogo. Center for Connected Learning and Computer-Based Modeling, Northwestern University.
- Wilson, J.B. & Agnew, A.D.Q. (1992) Positive-feedback switches in plant communities. *Advances in Ecological Research*, 23, 263-336.
- Wright, S. (1943) Isolation by distance. *Genetics*, 28, 114-138.
