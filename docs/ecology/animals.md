# Animals

Animals are the mobile consumer compartment of ecosystems. In systems terms, they are the principal agents that move energy laterally and vertically through food webs, regulate producer biomass through consumption, and redistribute nutrients across space through movement. Every animal, from a soil nematode to an apex predator, exists as a node in a network of stocks and flows — drawing down one stock (prey, forage, carrion) and feeding another (predator biomass, fecal nutrients, carcass pools).

## Systemic role

Animals occupy consumer trophic levels. Primary consumers (herbivores) draw energy from the producer stock. Secondary and tertiary consumers (predators) draw from consumer stocks. Detritivores and scavengers draw from the dead organic matter pool, operating as a parallel decomposition pathway alongside microbial decomposition.

The key stocks and flows:

- **Producer biomass** (stock) -> **herbivore consumption** (flow) -> **herbivore biomass** (stock)
- **Herbivore biomass** (stock) -> **predation** (flow) -> **predator biomass** (stock)
- **All animal biomass** (stock) -> **mortality/egestion** (flow) -> **detrital pool** (stock)
- **All trophic levels** -> **respiration** (flow) -> **heat loss** (exits the system)

At each trophic transfer, roughly 10% of energy passes upward (Lindeman 1942). This thermodynamic constraint means animal biomass pyramids are steep: producer biomass >> herbivore biomass >> predator biomass. The constraint also limits food chain length to typically 4-5 levels (Pimm 1982).

Animals also function as nutrient vectors. Herbivores accelerate nutrient cycling by converting recalcitrant plant tissue into labile fecal matter — the "grazing acceleration hypothesis" (McNaughton et al. 1997). Predators concentrate nutrients through carcass deposition. Migratory animals move nutrients across ecosystem boundaries entirely: Pacific salmon transport marine-derived nitrogen into freshwater and terrestrial systems (Helfield & Naiman 2001), and seabirds deposit guano that fertilizes island ecosystems at rates orders of magnitude above background deposition (Anderson & Polis 1999).

## Energy and nutrient flows

### Consumption as a regulated flow

Animal consumption is not a fixed rate — it is a behaviorally modulated flow. Functional response curves (Holling 1959) describe how per-capita consumption rate changes with prey density:

- **Type I**: Linear increase (filter feeders, passive consumers).
- **Type II**: Decelerating — consumption saturates as handling time dominates. Most common for individual predators.
- **Type III**: Sigmoidal — low consumption at low prey density (prey switching, search image formation), accelerating at intermediate density, then saturating. Creates a low-density refuge for prey.

These functional responses are critical because they determine whether consumer-resource interactions stabilize or oscillate. Type II responses are inherently destabilizing (consumers overexploit rare prey), while Type III responses can stabilize dynamics by releasing prey from predation pressure at low density (Murdoch & Oaten 1975).

### Assimilation efficiency

Not all consumed energy enters animal biomass. Assimilation efficiency varies by diet: carnivores assimilate roughly 80% of ingested energy, herbivores only 20-50% depending on plant tissue quality (Begon et al. 2006). The remainder passes through as feces, entering the detrital pool. This asymmetry means herbivores contribute disproportionately to the detrital pathway relative to their trophic position.

### Stoichiometric constraints

Animals maintain relatively homeostatic body nutrient ratios (particularly C:N:P) compared to their food sources. Herbivores consuming nitrogen-poor plant tissue must process large volumes to meet nitrogen demands, excreting excess carbon. This stoichiometric mismatch (Sterner & Elser 2002) shapes consumption rates, growth efficiency, and the nutrient composition of waste products — effectively filtering nutrient ratios as energy moves up trophic levels.

## Feedback loops

### Predator-prey oscillations

The canonical feedback loop in animal ecology. Prey increase -> predators increase (with a lag) -> prey decrease -> predators decrease (with a lag) -> prey increase. Lotka (1925) and Volterra (1926) formalized this as coupled differential equations, and the Hudson's Bay Company lynx-hare fur records (Elton & Nicholson 1942) provided the empirical archetype.

At the individual/agent level, this emerges from: each predator searches, encounters prey stochastically, kills, satiates, reproduces if energy sufficient. Each prey forages, grows, reproduces, and dies if caught. No individual "knows" about the population cycle — it is a purely emergent phenomenon, making it a natural target for agent-based modeling.

The stability of these oscillations depends on functional response type, predator interference, prey refugia, and environmental heterogeneity. In homogeneous, well-mixed systems, Lotka-Volterra dynamics tend toward neutrally stable orbits or extinction. Spatial structure (patchy landscapes, movement costs) generally stabilizes persistence through asynchronous local dynamics — the "statistical stabilization" effect (de Roos et al. 1991).

### Trophic cascades

When a top predator regulates herbivore abundance, it indirectly controls producer biomass. This is a three-level feedback chain:

- **Top-down cascade**: Predator increase -> herbivore decrease -> producer increase. The classic example is the reintroduction of wolves to Yellowstone reducing elk browsing pressure on riparian willows and aspens, with downstream effects on stream morphology and songbird habitat (Ripple & Beschta 2004).
- **Mesopredator release**: Removing apex predators releases mid-level predators from suppression, often increasing total predation pressure on small prey (Soule et al. 1988).

Trophic cascades are stronger in aquatic systems (where producers are small and fast-growing) than terrestrial systems (where producers are large and structurally defended) (Shurin et al. 2002). This has implications for modeling: cascade strength depends on producer traits, not just food web topology.

### Grazing-regrowth dynamics

Herbivory and plant regrowth form a negative feedback loop, but the dynamics depend on consumption mode:

- **Partial consumption** (grazing, browsing): Removes tissue but leaves the plant alive. Can stimulate compensatory regrowth in grasses (McNaughton 1979). At moderate intensity, grazing can increase net primary productivity — the "grazing optimization hypothesis."
- **Lethal consumption** (seed predation, seedling herbivory, bark stripping): Removes entire individuals from the producer stock. No compensatory response possible.

This distinction matters for modeling. Partial consumption creates a fast negative feedback (plant regrows, herbivore returns). Lethal consumption creates slower dynamics mediated by recruitment from seed banks or dispersal.

### Fear and the landscape of risk

Predators affect prey not only through direct killing but through behavioral modification. Prey reduce foraging time and shift habitat use in response to predation risk — the "landscape of fear" (Laundre et al. 2001). This non-consumptive effect can be as large as or larger than direct mortality in its impact on prey fitness and ecosystem structure (Preisser et al. 2005). In systems terms, predation risk acts as an information flow that modulates the consumption flow between herbivores and producers.

## Key strategies

### Herbivory as a flow

Herbivores are the primary conduit between producer and consumer compartments. Their systemic function depends on what they consume:

- **Grazers** (ungulates on grasslands, zooplankton on phytoplankton): Consume fast-turnover tissue. High throughput, relatively low per-unit impact. Maintain producers in a high-productivity, low-biomass state.
- **Browsers** (deer on shrubs, caterpillars on leaves): Consume slower-turnover tissue. Can shift plant community composition by selectively removing palatable species, favoring defended or unpalatable plants.
- **Granivores and frugivores**: Consume reproductive tissue. Act as both consumers and dispersal agents — a dual role that creates mutualistic feedbacks (seed dispersal loops).

### Predation as population regulation

Predation serves a regulatory function in ecosystems. Whether predators actually regulate prey populations (hold them below carrying capacity) or merely track them depends on the predator's numerical and functional responses (Sinclair et al. 2003). Key distinctions:

- **Specialist predators**: Track single prey populations. Tend to create oscillatory dynamics (boom-bust).
- **Generalist predators**: Switch between prey species. Can stabilize individual prey populations through apparent competition — when prey A is scarce, predator switches to prey B, releasing A from predation pressure (Holt 1977).
- **Keystone predation**: Some predators maintain diversity by preferentially consuming competitive dominants, preventing monopolization. Paine's (1966) sea star removal experiments demonstrated that removing a single predator collapsed intertidal community diversity.

### Scavenging as parallel decomposition

Scavengers (vultures, hyenas, crabs, blowflies) compete with microbial decomposers for the carrion stock. Scavenging is often faster than microbial decomposition and produces different nutrient redistribution patterns — mobile scavengers transport carcass nutrients to roost sites, dens, and latrines rather than releasing them in situ (Wilson & Wolkovich 2011). The loss of obligate scavengers (e.g., vulture declines in South Asia) shifts decomposition toward slower microbial pathways, with consequences for disease ecology as carcass persistence increases (Markandya et al. 2008).

### Social behavior as emergent coordination

Social behaviors — herding, pack hunting, mobbing, colony formation — are emergent properties of individual decision rules operating under selection pressure. From a systems perspective:

- **Herding/schooling**: Reduces individual predation risk through dilution and confusion effects. Creates spatially concentrated grazing pressure, producing patchy disturbance patterns on the landscape. The selfish herd hypothesis (Hamilton 1971) explains aggregation as individual risk-minimization without group-level selection.
- **Pack hunting**: Enables predation on prey larger than individual predators could take. Alters the size spectrum of available prey, connecting energy pools that would otherwise be inaccessible. Wolf packs taking elk, army ants overwhelming large arthropods.
- **Eusociality** (ants, termites, naked mole-rats): Division of labor creates superorganism-level efficiency. Leaf-cutter ant colonies function as agricultural systems, cultivating fungal gardens — a consumer that has internalized its own producer.

## The mobility advantage

Mobility is the defining systemic property that distinguishes animals from producers and most decomposers. It has several interacting consequences:

**Resource tracking**: Mobile consumers can move toward resource-rich patches and away from depleted ones. This enables exploitation of spatially and temporally variable resources — seasonal flushes, patchy prey aggregations, ephemeral resource pulses. The cost is metabolic: movement consumes energy (roughly 10-15x basal metabolic rate during active locomotion in terrestrial vertebrates; Schmidt-Nielsen 1972).

**Predator avoidance**: Mobility enables flight responses, habitat shifting, and migration away from high-predation areas. This creates an evolutionary arms race between predator pursuit and prey evasion capabilities, driving morphological and behavioral elaboration.

**Mate finding**: Sexual reproduction requires encounter between gamete-producing individuals. Mobility solves the mate-finding problem but introduces energy costs and predation risk during mate search. In sessile organisms (plants, corals), gametes themselves are the mobile agents; in animals, the whole organism moves.

**Spatial coupling of ecosystems**: Mobile animals connect spatially separated ecosystem compartments. Diel vertical migration of zooplankton transports carbon from ocean surface waters to depth (the biological pump; Steinberg et al. 2000). Hippos transport terrestrial organic matter into rivers. Migrating wildebeest move nutrients across the Serengeti along a precipitation gradient (Holdo et al. 2009). These cross-boundary flows would not exist without animal mobility.

**The cost-benefit tradeoff**: Mobility imposes a metabolic tax. Endothermic animals (birds, mammals) spend 10-30x more energy per unit body mass than ectotherms (reptiles, insects) at rest. The energy budget for movement constrains body size, reproductive output, and population density. Each organism's energy budget creates a direct coupling between movement decisions and survival.

## Invertebrates as system infrastructure

Vertebrate animals are conspicuous but invertebrates dominate most ecosystem processes by biomass and abundance.

### Soil fauna and nutrient cycling

Soil invertebrates — nematodes, earthworms, mites, springtails, termites — process the majority of terrestrial detrital input. Earthworms alone can turn over the top 15 cm of soil every few decades (Darwin 1881). They fragment litter, mix soil horizons, create macropores for water infiltration, and stimulate microbial activity through gut passage of organic matter. Termites are the dominant decomposers in tropical systems, processing up to 50% of annual leaf litter in some forests (Bignell & Eggleton 2000).

In systems terms, soil fauna regulate the rate of the detritus -> inorganic nutrient flow. Their removal slows nutrient cycling, reduces soil structure, and impairs plant productivity.

### Pollinators as a flow pathway

Approximately 87% of flowering plant species depend on animal pollination (Ollerton et al. 2011). Pollinators — primarily insects (bees, butterflies, beetles, flies) but also birds and bats — mediate the flow from adult plant to seed. This is not an energy flow in the trophic sense but a reproductive flow: pollinators enable the recruitment term in the producer stock equation.

Pollinator loss does not remove energy from the system directly but collapses the recruitment pathway for dependent producers, eventually reducing producer diversity and biomass. The systemic vulnerability is high because pollination networks are often nested — many plant species depend on a few generalist pollinators (Bascompte et al. 2003).

### Aquatic invertebrates

In freshwater and marine systems, invertebrates dominate the consumer trophic levels by biomass. Zooplankton are the primary consumers of phytoplankton and the primary prey of larval and planktivorous fish. Filter-feeding bivalves (mussels, oysters) can filter entire water column volumes in days, controlling phytoplankton biomass and water clarity — an ecosystem engineering function (Newell 2004).

## ABM and computational literature

Agent-based models of animal dynamics have a rich history, and animals are arguably the most-modeled kingdom in computational ecology.

### Classic predator-prey ABMs

The **Wolf Sheep Predation** model in NetLogo (Wilensky 1997) is the canonical introductory ABM for trophic dynamics. Wolves and sheep move randomly on a grid; sheep eat grass, wolves eat sheep, both reproduce if energy exceeds a threshold and die if energy depletes. Despite its simplicity, it reproduces Lotka-Volterra-like oscillations, demonstrates the stabilizing effect of grass regrowth rates, and illustrates how spatial structure (movement rules, grid size) affects persistence. Variants add spatial heterogeneity, predator learning, and prey refugia.

Grimm & Railsback (2005, *Individual-based Modeling and Ecology*) use predator-prey systems as a recurring example throughout their textbook, demonstrating how individual variation in body size, energy reserves, and movement behavior produces population-level dynamics that differ qualitatively from equation-based models.

### Foraging models

**Optimal foraging theory** (MacArthur & Pianka 1966; Charnov 1976) has been implemented extensively in ABMs. Agents decide which prey types to pursue and when to leave a depleted patch (the marginal value theorem). ABM implementations allow relaxation of the theory's assumptions: imperfect information, learning, memory of patch locations, social information transfer.

Railsback & Harvey (2002) developed a foraging ABM for stream-dwelling salmonids where individual fish select feeding positions based on expected energy intake minus predation risk. This "fitness-based habitat selection" approach has been widely adopted in individual-based ecology.

### Flocking and herding models

**Reynolds' Boids** (1987) demonstrated that coherent flocking emerges from three local rules: separation (avoid crowding neighbors), alignment (steer toward average heading of neighbors), and cohesion (steer toward average position of neighbors). This model, while originally developed for computer graphics, became foundational for understanding collective animal movement.

**Couzin et al. (2002)** extended this framework to show how varying the relative weights of alignment and attraction zones produces distinct collective states: swarm, torus, dynamic parallel group, and highly parallel group. The model predicts phase transitions between collective states and demonstrates how a small proportion of informed individuals can guide naive groups.

### Movement ecology ABMs

Movement ecology has become a major ABM application area. Models simulate individual animals making movement decisions based on internal state (hunger, reproductive status), sensory information (resource gradients, predator cues), and navigation capacity (memory, path integration).

**McLane et al. (2011)** reviewed ABM applications in movement ecology, identifying models of migration (agents following environmental cues with energetic constraints), dispersal (natal dispersal rules producing population-level connectivity patterns), and home range formation (movement rules producing emergent territory structures).

**Tang & Bennett (2010)** reviewed agent-based models in landscape ecology, noting that animal movement ABMs have been particularly useful for understanding habitat fragmentation effects — how individual movement rules interact with spatial landscape structure to produce population-level connectivity or isolation.

### ODD protocol and standardization

The ODD (Overview, Design concepts, Details) protocol (Grimm et al. 2006, 2010) emerged from ecology ABM practice and is now the standard for documenting agent-based models. Many ODD-documented models in the ecological literature are animal models: population viability analyses, disease transmission through animal contact networks, fisheries models, and wildlife management simulations.

### Multi-species and food web ABMs

More complex ABMs simulate entire animal communities. **Railsback & Grimm (2019, *Agent-Based and Individual-Based Modeling*, 2nd ed.)** present progressively complex models from single-species to multi-species interactions. The challenge in multi-species ABMs is parameterization: each species adds behavioral rules and parameters, and calibration becomes combinatorially difficult.

**DeAngelis & Grimm (2014)** reviewed individual-based models in ecology, noting that animal IBMs have been most successful when focused on specific mechanisms (e.g., foraging behavior driving population dynamics) rather than attempting comprehensive simulation of all ecological interactions simultaneously.

## System without animals

Removing animals from an ecosystem does not simply reduce biodiversity — it fundamentally alters system dynamics at every level.

### Remove herbivores

Without herbivores, producer biomass accumulates unchecked by consumption. Competitive exclusion among plants accelerates — fast-growing species dominate, shade-tolerant species may eventually replace them, but the successional trajectory changes because grazing-maintained openings disappear. Grasslands convert to shrubland or forest in the absence of grazing (as observed in exclosure experiments worldwide). Litter accumulates, decomposition shifts entirely to the microbial pathway, and nutrient cycling slows (particularly where herbivores were accelerating cycling through labile fecal matter). Fire risk increases as standing dead biomass accumulates.

### Remove predators

Without predators, herbivore populations grow toward resource-limited carrying capacity — often overshooting due to reproductive time lags before plant resources decline. This produces a pulse of overgrazing followed by herbivore population crash and a degraded producer base. The Kaibab Plateau deer irruption after predator removal (Leopold 1943) is the textbook example, though its interpretation has been debated (Binkley et al. 2006). More robust examples include island ecosystems where introduced herbivores (goats, rabbits) lacking predators have denuded vegetation.

The loss of predator-mediated behavioral effects is equally important. Without the landscape of fear, herbivores shift to optimal foraging locations (often riparian areas, regeneration zones) and remain there, concentrating damage on the most productive and ecologically sensitive areas.

### Remove detritivores and scavengers

Decomposition slows. Carcasses persist longer, potentially increasing disease transmission. Soil structure degrades without bioturbation. Nutrient recycling rates decline. In extreme cases (loss of dung beetles from pastoral systems), fecal matter accumulates on soil surfaces, physically blocking plant growth and creating anaerobic zones.

### Remove pollinators

Recruitment failure in animal-pollinated plant species. Over a generation timescale, these species decline and are replaced by wind-pollinated or self-pollinating species, reducing plant diversity and potentially altering vegetation structure. Cascading effects on all consumers that depend on fruits and seeds of animal-pollinated species.

### The compounding effect

These removals interact. Losing predators AND pollinators simultaneously produces different dynamics than losing either alone. The system's response is nonlinear and path-dependent — the interactions between trophic levels, functional groups, and spatial processes generate emergent dynamics that cannot be predicted from single-factor analysis.

---

## References

Anderson, W.B. & Polis, G.A. (1999). Nutrient fluxes from water to land: seabirds affect plant nutrient status on Gulf of California islands. *Oecologia*, 118(3), 324-332.

Bascompte, J., Jordano, P., Melian, C.J. & Olesen, J.M. (2003). The nested assembly of plant-animal mutualistic networks. *Proceedings of the National Academy of Sciences*, 100(16), 9383-9387.

Begon, M., Townsend, C.R. & Harper, J.L. (2006). *Ecology: From Individuals to Ecosystems*. 4th ed. Blackwell Publishing.

Bignell, D.E. & Eggleton, P. (2000). Termites in ecosystems. In *Termites: Evolution, Sociality, Symbioses, Ecology* (pp. 363-387). Springer.

Binkley, D., Moore, M.M., Romme, W.H. & Brown, P.M. (2006). Was Aldo Leopold right about the Kaibab deer herd? *Ecosystems*, 9(2), 227-241.

Charnov, E.L. (1976). Optimal foraging, the marginal value theorem. *Theoretical Population Biology*, 9(2), 129-136.

Couzin, I.D., Krause, J., James, R., Ruxton, G.D. & Franks, N.R. (2002). Collective memory and spatial sorting in animal groups. *Journal of Theoretical Biology*, 218(1), 1-11.

Darwin, C. (1881). *The Formation of Vegetable Mould Through the Action of Worms*. John Murray.

DeAngelis, D.L. & Grimm, V. (2014). Individual-based models in ecology after four decades. *F1000Prime Reports*, 6, 39.

de Roos, A.M., McCauley, E. & Wilson, W.G. (1991). Mobility versus density-limited predator-prey dynamics on different spatial scales. *Proceedings of the Royal Society B*, 246(1316), 117-122.

Elton, C. & Nicholson, M. (1942). The ten-year cycle in numbers of the lynx in Canada. *Journal of Animal Ecology*, 11(2), 215-244.

Grimm, V. & Railsback, S.F. (2005). *Individual-based Modeling and Ecology*. Princeton University Press.

Grimm, V., Berger, U., Bastiansen, F., et al. (2006). A standard protocol for describing individual-based and agent-based models. *Ecological Modelling*, 198(1-2), 115-126.

Grimm, V., Berger, U., DeAngelis, D.L., et al. (2010). The ODD protocol: a review and first update. *Ecological Modelling*, 221(23), 2760-2768.

Hamilton, W.D. (1971). Geometry for the selfish herd. *Journal of Theoretical Biology*, 31(2), 295-311.

Helfield, J.M. & Naiman, R.J. (2001). Effects of salmon-derived nitrogen on riparian forest growth and implications for stream productivity. *Ecology*, 82(9), 2403-2409.

Holdo, R.M., Holt, R.D., Sinclair, A.R.E., Godley, B.J. & Thirgood, S. (2009). Migratory wildebeest and nutrient cycling in the Serengeti. In *Serengeti III* (pp. 207-234). University of Chicago Press.

Holling, C.S. (1959). The components of predation as revealed by a study of small-mammal predation of the European pine sawfly. *The Canadian Entomologist*, 91(5), 293-320.

Holt, R.D. (1977). Predation, apparent competition, and the structure of prey communities. *Theoretical Population Biology*, 12(2), 197-229.

Laundre, J.W., Hernandez, L. & Altendorf, K.B. (2001). Wolves, elk, and bison: reestablishing the "landscape of fear" in Yellowstone National Park. *Canadian Journal of Zoology*, 79(8), 1401-1409.

Leopold, A. (1943). Deer irruptions. *Wisconsin Conservation Bulletin*, 8, 3-11.

Lindeman, R.L. (1942). The trophic-dynamic aspect of ecology. *Ecology*, 23(4), 399-417.

Lotka, A.J. (1925). *Elements of Physical Biology*. Williams & Wilkins.

MacArthur, R.H. & Pianka, E.R. (1966). On optimal use of a patchy environment. *The American Naturalist*, 100(916), 603-609.

Markandya, A., Taylor, T., Longo, A., et al. (2008). Counting the cost of vulture decline — an appraisal of the human health and other benefits of vultures in India. *Ecological Economics*, 67(2), 194-204.

McLane, A.J., Semeniuk, C., McDermid, G.J. & Marceau, D.J. (2011). The role of agent-based models in wildlife ecology and management. *Ecological Modelling*, 222(8), 1544-1556.

McNaughton, S.J. (1979). Grazing as an optimization process: grass-ungulate relationships in the Serengeti. *The American Naturalist*, 113(5), 691-703.

McNaughton, S.J., Banyikwa, F.F. & McNaughton, M.M. (1997). Promotion of the cycling of diet-enhancing nutrients by African grazers. *Science*, 278(5344), 1798-1800.

Murdoch, W.W. & Oaten, A. (1975). Predation and population stability. *Advances in Ecological Research*, 9, 1-131.

Newell, R.I.E. (2004). Ecosystem influences of natural and cultivated populations of suspension-feeding bivalve molluscs: a review. *Journal of Shellfish Research*, 23(1), 51-62.

Ollerton, J., Winfree, R. & Tarrant, S. (2011). How many flowering plants are pollinated by animals? *Oikos*, 120(3), 321-326.

Paine, R.T. (1966). Food web complexity and species diversity. *The American Naturalist*, 100(910), 65-75.

Pimm, S.L. (1982). *Food Webs*. Chapman and Hall.

Preisser, E.L., Bolnick, D.I. & Benard, M.F. (2005). Scared to death? The effects of intimidation and consumption in predator-prey interactions. *Ecology*, 86(2), 501-509.

Railsback, S.F. & Grimm, V. (2019). *Agent-Based and Individual-Based Modeling: A Practical Introduction*. 2nd ed. Princeton University Press.

Railsback, S.F. & Harvey, B.C. (2002). Analysis of habitat-selection rules using an individual-based model. *Ecology*, 83(7), 1817-1830.

Reynolds, C.W. (1987). Flocks, herds and schools: a distributed behavioral model. *ACM SIGGRAPH Computer Graphics*, 21(4), 25-34.

Ripple, W.J. & Beschta, R.L. (2004). Wolves and the ecology of fear: can predation risk structure ecosystems? *BioScience*, 54(8), 755-766.

Schmidt-Nielsen, K. (1972). Locomotion: energetic cost of swimming, flying, and running. *Science*, 177(4045), 222-228.

Shurin, J.B., Borer, E.T., Seabloom, E.W., et al. (2002). A cross-ecosystem comparison of the strength of trophic cascades. *Ecology Letters*, 5(6), 785-791.

Sinclair, A.R.E., Mduma, S. & Brashares, J.S. (2003). Patterns of predation in a diverse predator-prey system. *Nature*, 425(6955), 288-290.

Soule, M.E., Bolger, D.T., Alberts, A.C., et al. (1988). Reconstructed dynamics of rapid extinctions of chaparral-requiring birds in urban habitat islands. *Conservation Biology*, 2(1), 75-92.

Steinberg, D.K., Carlson, C.A., Bates, N.R., Goldthwait, S.A., Madin, L.P. & Michaels, A.F. (2000). Zooplankton vertical migration and the active transport of dissolved organic and inorganic carbon in the Sargasso Sea. *Deep Sea Research Part I*, 47(1), 137-158.

Sterner, R.W. & Elser, J.J. (2002). *Ecological Stoichiometry: The Biology of Elements from Molecules to the Biosphere*. Princeton University Press.

Tang, W. & Bennett, D.A. (2010). Agent-based modeling of animal movement: a review. *Geography Compass*, 4(7), 682-700.

Volterra, V. (1926). Fluctuations in the abundance of a species considered mathematically. *Nature*, 118, 558-560.

Wilensky, U. (1997). NetLogo Wolf Sheep Predation model. Center for Connected Learning and Computer-Based Modeling, Northwestern University.

Wilson, E.E. & Wolkovich, E.M. (2011). Scavenging: how carnivores and carrion structure communities. *Trends in Ecology & Evolution*, 26(3), 129-135.
