# Life History Theory

Life history theory addresses the most fundamental allocation problem in biology: how organisms partition a finite energy budget across growth, maintenance, reproduction, defense, and dispersal over the course of a lifetime. Every allocation decision involves trade-offs — energy committed to one function is unavailable for others — and natural selection shapes these allocation strategies in response to mortality schedules, resource availability, and density dependence. From a systems perspective, life history traits are the parameters that govern the rates of recruitment, mortality, and energy throughput in population and community models.

## The core trade-offs

Energy is a conserved quantity. An organism's lifetime energy budget flows through a fixed set of compartments — somatic growth, tissue maintenance, immune defense, reproduction — and every allocation to one compartment reduces the flow available to others. This is not metaphor; it is thermodynamic constraint. The core trade-offs that structure life history theory are all consequences of this budget limitation.

**Current reproduction vs. future reproduction.** Energy invested in producing offspring now reduces the energy available for somatic growth and maintenance, which reduces future body size, survival probability, and therefore future reproductive output. Williams (1966) formalized this as the "cost of reproduction" — organisms that reproduce heavily in one season show reduced survival or fecundity in subsequent seasons. Empirical demonstrations span taxa: fruit flies with artificially elevated early reproduction die sooner (Rose & Charlesworth 1981), red deer hinds that reproduce in consecutive years show reduced calf weight (Clutton-Brock et al. 1983), and perennial plants that fruit heavily in one year often skip reproduction the next (masting cycles).

**Number vs. quality of offspring.** A fixed reproductive energy budget can produce many small, poorly provisioned offspring or few large, well-provisioned offspring. Smith & Fretwell (1974) modeled this as an optimization problem: offspring fitness is a diminishing-returns function of per-offspring investment, and the optimal clutch size maximizes total fitness (number x per-offspring fitness). This trade-off is universal — it appears in plants (many small seeds vs. few large seeds), fish (millions of tiny eggs vs. dozens of large, yolked eggs), mammals (large litters of altricial young vs. single precocial offspring), and invertebrates.

**Growth vs. reproduction.** Somatic growth increases future reproductive capacity (larger organisms generally produce more offspring), but delays the onset of reproduction. The optimal age at first reproduction balances the fitness gained from growing larger against the risk of dying before reproducing at all. In high-mortality environments, early reproduction is favored even at small body size. In low-mortality environments, delayed reproduction with extended growth is favored — the organism "invests" in a larger soma that will yield higher lifetime reproductive output (Stearns 1992).

**Survival vs. reproduction.** Reproductive activity itself increases mortality risk. Mate-finding exposes mobile organisms to predation. Pregnancy and lactation impose metabolic costs. Bright plumage or loud courtship calls attract both mates and predators. This trade-off is particularly acute for organisms with high parental investment — the energy and risk committed to current offspring directly reduce the parent's survival probability.

These trade-offs are not independent. They interact through the common currency of energy, creating a multidimensional allocation surface. An organism's position on this surface — its life history strategy — is a phenotype subject to selection.

## r/K selection and density dependence

MacArthur & Wilson (1967) introduced the r/K framework in the context of island biogeography, and Pianka (1970) generalized it into a continuum of life history strategies shaped by the relative importance of density-independent vs. density-dependent mortality.

**r-selection** favors traits that maximize the intrinsic rate of natural increase (r): high fecundity, small offspring size, early maturation, short generation time, low parental investment, and short lifespan. These traits are favored when populations are frequently below carrying capacity — after disturbance events, during colonization of empty habitat, or in environments with high density-independent mortality (storms, drought, unpredictable catastrophe). The strategy is: reproduce fast, saturate the environment, accept high individual mortality.

**K-selection** favors traits that maximize competitive ability near carrying capacity (K): low fecundity, large offspring size, delayed maturation, long generation time, high parental investment, and long lifespan. These traits are favored when populations are persistently near carrying capacity and mortality is primarily density-dependent (competition for resources, predation, disease). The strategy is: produce few, high-quality offspring that can compete effectively in a crowded environment.

The r/K framework maps directly to population dynamics:

- **r-strategist populations** exhibit boom-bust dynamics. Rapid reproduction drives exponential growth; overshoot of carrying capacity causes resource depletion and population crash; recolonization from surviving individuals or immigration restarts the cycle. Population size is highly variable over time.
- **K-strategist populations** exhibit relatively stable dynamics near carrying capacity. Growth is slow, density-dependent regulation is strong, and populations rarely overshoot dramatically. However, K-strategist populations are fragile to perturbation — their slow reproduction means recovery from a population crash takes many generations.

In systems terms, r-selection produces high-gain, low-damping population dynamics (fast positive feedback through reproduction, weak negative feedback through density dependence). K-selection produces low-gain, high-damping dynamics (slow reproductive positive feedback, strong competitive negative feedback).

The r/K framework has been criticized for oversimplification (Stearns 1977, Reznick et al. 2002). Real organisms occupy a multidimensional trait space, and the single r-K axis fails to capture important variation — particularly the distinction between competitive ability and stress tolerance. Grime's (1977) CSR framework addresses this gap specifically for plants, proposing a three-axis model: Competitors (high resource environments, high density — analogous to K-strategists), Stress-tolerators (low resource environments, chronic limitation — a dimension absent from r/K), and Ruderals (high disturbance, low competition — analogous to r-strategists). The CSR triangle has been empirically validated across global plant trait datasets (Pierce et al. 2017) and provides a richer mapping from trait space to ecological strategy than the r/K continuum alone.

Despite its limitations, the r/K framework remains useful as a first-order description of how density dependence shapes life history evolution, and it maps well onto agent-based models where carrying capacity and disturbance frequency are explicit parameters.

## Reproductive strategies

Reproductive mode determines how organisms convert energy into offspring and how those offspring enter the population. The major axes of variation interact with life history allocation to produce a wide range of strategies.

**Iteroparity vs. semelparity.** Iteroparous organisms reproduce multiple times over their lifetime (most vertebrates, perennial plants, many invertebrates). Semelparous organisms reproduce once and die (Pacific salmon, annual plants, many insects, agave). Semelparity concentrates the entire lifetime reproductive budget into a single event, maximizing per-event fecundity at the cost of zero post-reproductive survival. Cole (1954) showed that, all else equal, an iteroparous organism needs only slightly higher per-event fecundity than a semelparous one to have equal fitness — the "Cole's paradox." Resolving this paradox requires accounting for juvenile mortality: when juvenile survival is low, semelparity is favored because concentrating investment in one massive reproductive event can overwhelm the juvenile mortality bottleneck (Charnov & Schaffer 1973).

**Sexual vs. asexual reproduction.** Sexual reproduction produces genetically variable offspring through recombination, at the cost of mate-finding, mating investment, and the "twofold cost of sex" (only half the population — females — directly produces offspring). Asexual reproduction (budding, fragmentation, parthenogenesis) avoids these costs but produces genetically uniform offspring. Many organisms switch between modes: aphids reproduce asexually during favorable conditions and sexually when conditions deteriorate, producing resistant overwintering eggs. Daphnia, rotifers, and many fungi show similar facultative switching. The pattern is general: asexual reproduction when conditions are stable and favorable (exploit a proven genotype), sexual reproduction when conditions are changing or stressful (generate variation for uncertain futures). This connects directly to bet-hedging theory.

**Viviparity vs. oviparity.** Viviparous organisms retain offspring internally during development, providing sustained resource transfer and protection. Oviparous organisms deposit eggs externally, often with a one-time yolk investment. Viviparity increases per-offspring investment and survival but reduces fecundity and imposes mobility and metabolic costs on the parent. The transition from oviparity to viviparity has evolved independently in many lineages (fish, reptiles, invertebrates) and is generally associated with environments where egg survival is low — cold climates, high-predation environments, unstable substrates.

**Sessile vs. mobile reproduction.** This axis interacts profoundly with dispersal. Sessile organisms (plants, fungi, corals, bryozoans) cannot move to find mates and must rely on gamete dispersal — wind, water, or animal vectors for pollen; spore release for fungi. Their reproductive energy budget includes investment in dispersal structures: flowers, fruits, spore-bearing bodies. Mobile organisms (most animals) can actively seek mates, but expend energy on locomotion, courtship, and intrasexual competition. The mate-finding problem is trivial for sessile organisms that broadcast gametes into a medium (wind-pollinated grasses, broadcast-spawning corals) but becomes a significant energy cost for mobile organisms at low density — Allee effects in mate-finding can create extinction thresholds in sparse populations.

## Dispersal and fecundity

Dispersal — movement of offspring away from the parent — is a life history trait in its own right, subject to trade-offs with fecundity, offspring size, and competitive ability.

**The dispersal-fecundity trade-off.** Producing dispersible offspring requires investment in dispersal structures (wings, plumes, flotation devices in plants; flight capability, fat reserves for migratory journeys in animals). This investment comes at the cost of either offspring number (fewer, better-dispersed offspring) or offspring size/provisioning (smaller offspring with more dispersal capacity). Levin et al. (2003) modeled how this trade-off interacts with spatial structure: in patchy environments, dispersal is favored because it enables colonization of empty patches; in homogeneous environments, dispersal is wasteful because offspring that disperse leave favorable habitat.

**Sessile organisms and dispersal investment.** For sessile organisms, dispersal of offspring is the only mechanism for spatial spread. Plants invest heavily in seed dispersal — wind dispersal (dandelion parachutes, maple samaras), animal dispersal (fleshy fruits, hooked burs), ballistic dispersal (explosive seed pods), and water dispersal. The investment in dispersal structures directly reduces the energy available for seed provisioning. Small-seeded species (orchids, many wind-dispersed species) produce thousands to millions of tiny, poorly provisioned seeds that disperse widely but have low individual establishment probability. Large-seeded species (oaks, coconut palms) produce few, well-provisioned seeds with high establishment probability but limited dispersal distance.

**Mobile organisms and mate-finding.** For mobile organisms, dispersal is less about offspring movement and more about the organism's own movement to find mates and suitable habitat. Natal dispersal distance trades off against energy invested in growth and survival at the natal site. Long-distance dispersers pay energetic costs and face predation risk during transit, but gain access to unoccupied habitat and avoid inbreeding. The optimal dispersal distance depends on habitat patchiness, population density, and kin competition (Hamilton & May 1977).

**Spatial structure and dispersal kernels.** The distribution of dispersal distances (the dispersal kernel) shapes population spatial structure. Leptokurtic kernels (most offspring disperse short distances, a fat tail of long-distance dispersers) are common across taxa and produce clustered populations with occasional colonization of distant patches. The shape of the dispersal kernel interacts with landscape structure to determine metapopulation connectivity, gene flow, and the spatial scale of population dynamics (Clobert et al. 2012).

Dispersal is a critical trait because it couples local dynamics to landscape-scale patterns. High-dispersal organisms effectively mix the population, homogenizing local dynamics. Low-dispersal organisms create spatial autocorrelation — local clusters with potentially distinct evolutionary trajectories.

## Parental investment

Trivers (1972) defined parental investment as any investment by the parent in an individual offspring that increases the offspring's survival at the cost of the parent's ability to invest in other offspring. This definition reframes reproduction as an allocation problem: the total reproductive budget is divided among offspring, and the per-offspring allocation determines offspring quality.

**Asymmetric investment.** In sexually reproducing species, the two parents often invest asymmetrically. The parent producing larger gametes (typically the female) has a higher minimum per-offspring investment. Trivers argued that the sex with greater investment becomes the limiting resource for the other sex's reproduction, driving intrasexual competition in the low-investing sex and mate selectivity in the high-investing sex. This asymmetry in investment drives sexual selection — the evolution of traits that enhance competitive ability (weapons, body size) or attractiveness (ornaments, displays) in the competing sex.

**Investment level and survivorship.** The level of parental investment per offspring directly shapes offspring survivorship curves. High-investment strategies (few offspring, extensive provisioning and protection) produce Type I survivorship — low juvenile mortality, most mortality concentrated in old age (large mammals, humans). Low-investment strategies (many offspring, minimal provisioning) produce Type III survivorship — extremely high juvenile mortality, with survival probability increasing for those that reach a certain size or stage (marine invertebrates, many fish, annual plants). Type II survivorship (constant mortality rate across age classes) is characteristic of organisms with moderate investment and exposure to age-independent mortality sources (many birds, some reptiles).

**The investment continuum.** Parental investment exists on a continuum from zero post-zygotic investment (broadcast spawning, spore release — offspring are on their own from the moment of release) through provisioning (yolk, endosperm, milk), protection (egg guarding, nest defense, brooding), to extended care (feeding, teaching, social integration). Each step up the investment continuum increases per-offspring survival but reduces the number of offspring produced. The optimal position on this continuum depends on the environment: when juvenile mortality is density-independent and largely random (harsh abiotic conditions, unpredictable predation), low investment and high fecundity are favored. When juvenile mortality is density-dependent and can be reduced by parental care, high investment is favored.

Reproductive investment maps directly onto this continuum. High-investment organisms produce fewer, better-provisioned offspring with higher initial survival probability. Low-investment organisms produce more offspring with lower individual survival. Mate selectivity interacts with this: high selectivity reduces mating frequency but potentially increases offspring quality through genetic compatibility.

## Bet-hedging

Environmental uncertainty creates a fundamental problem for allocation strategies: the optimal life history in one year may be catastrophic in the next. Bet-hedging theory (Seger & Brockmann 1987, Philippi & Seger 1989) addresses how organisms cope with unpredictable variation in selection pressures.

**Diversified bet-hedging.** Producing offspring with high phenotypic variance — a spread of trait values — so that some offspring are suited to whatever conditions arise. The parent sacrifices expected mean fitness for reduced variance in fitness across environments. The canonical example is variable seed dormancy in desert annuals: a fraction of seeds germinate immediately, a fraction remains dormant for one year, another fraction for two years, etc. In any given year, some fraction encounters favorable conditions. The population persists across unpredictable droughts because it never commits all offspring to a single germination event (Cohen 1966, Venable 2007).

**Conservative bet-hedging.** Producing offspring clustered around a safe, generalist phenotype that performs reasonably (though not optimally) across all environments. The parent sacrifices peak performance in any single environment for reliable performance across environments. Conservative bet-hedging is favored when environmental variation is moderate and no single environment is reliably catastrophic.

**The connection to mutation.** Mutation rate and magnitude function as bet-hedging parameters. High mutation variance produces diversified offspring (a wider spread of phenotypes), while low mutation variance produces conservative offspring (clustering near parental phenotypes). In stable environments, selection favors low mutation variance (conserve a proven genotype). In fluctuating environments, higher mutation variance is effectively diversified bet-hedging — the lineage that maintains phenotypic diversity across generations persists through environmental shifts.

**Bet-hedging and reproductive mode.** Asexual reproduction with mutation produces conservative bet-hedging (small phenotypic steps from the parent). Sexual reproduction with recombination produces diversified bet-hedging (offspring can combine parental traits in novel ways). This provides one resolution to the cost-of-sex question: sex is a bet-hedging strategy, favored when environmental uncertainty is high (Meyers & Bull 2002).

## Senescence and mortality

Why do organisms age and die? In a system with no intrinsic constraint on lifespan, selection should favor immortality — more lifespan means more reproduction. The observation that nearly all organisms senesce (decline in physiological function with age) requires evolutionary explanation.

**Mutation accumulation.** Medawar (1952) proposed that mutations with deleterious effects expressed only late in life accumulate in populations because selection against them is weak — most individuals have already died from extrinsic causes before the mutations manifest. Late-acting deleterious alleles drift to higher frequencies than early-acting ones.

**Antagonistic pleiotropy.** Williams (1957) proposed that some genes have beneficial effects early in life (enhancing growth, reproduction, competitive ability) but deleterious effects late in life. Selection favors these genes because the early benefits outweigh the late costs — most individuals never survive long enough to pay the cost. The prediction is specific: organisms in high-extrinsic-mortality environments should evolve faster senescence (invest heavily in early reproduction, tolerate late-life decline) because the probability of surviving to old age is low regardless. This prediction has been empirically supported in guppies (Reznick et al. 2004), opossums (Austad 1993), and comparative analyses across birds and mammals.

**Disposable soma theory.** Kirkwood (1977) framed senescence explicitly as an energy allocation problem. Somatic maintenance (DNA repair, protein turnover, antioxidant defense) costs energy. An organism could theoretically maintain its soma indefinitely, but the energy invested in maintenance is unavailable for reproduction. The optimal allocation sacrifices long-term somatic maintenance for current reproduction, producing a soma that degrades over time — it is "disposable." The rate of degradation (the rate of senescence) is calibrated to the extrinsic mortality rate: when extrinsic mortality is high, investing in somatic maintenance beyond the expected lifespan is wasteful.

**Survivorship curves and ecological implications.** Mortality schedules are described by three idealized survivorship curves:

- **Type I** (convex): Low mortality through most of the lifespan, then rapid mortality in old age. Characteristic of K-strategists with high parental investment (large mammals, humans). Populations with Type I survivorship are dominated by adults, have stable age structure, and are sensitive to changes in adult survival.
- **Type II** (linear on a log scale): Constant mortality rate across age classes. Characteristic of organisms with moderate investment and age-independent mortality (many birds, some reptiles, some perennial plants). Populations have a geometric age distribution.
- **Type III** (concave): Extremely high juvenile mortality, low adult mortality. Characteristic of r-strategists with low parental investment (most marine invertebrates, many fish, annual plants producing many seeds). Populations are dominated by juveniles, and population dynamics are driven by variation in juvenile survival — a good recruitment year can dominate population structure for years.

These survivorship types are not fixed categories but endpoints on a continuum, and they connect directly to parental investment: Type I curves emerge from high investment, Type III from low investment.

## Feedback loops

Life history strategies do not just respond to environmental conditions — they create population-level dynamics that feed back to reshape selection pressures. These feedbacks are the mechanism by which individual-level allocation decisions produce community-level structure.

**r-strategists and boom-bust dynamics.** Populations of r-strategists amplify environmental variability. A pulse of resources triggers rapid reproduction, population overshoot, resource depletion, and crash. This boom-bust dynamic creates temporal resource pulses for other species (predators, scavengers, decomposers) and periodic disturbance of the habitat (overgrazing, soil disruption). In communities, r-strategist booms can suppress competing species through resource preemption, but busts create windows for recolonization by other species — a mechanism for maintaining diversity through temporal niche partitioning.

**K-strategists and competitive stability.** Populations of K-strategists produce stable, slow dynamics. They monopolize resources through sustained competitive ability, resist invasion by r-strategists under stable conditions, and maintain predictable age and size structure. However, this stability is fragile: perturbations that reduce K-strategist populations below a threshold (Allee effects in mate-finding, loss of mutualist partners, habitat fragmentation reducing population connectivity) can cause slow, irreversible decline because reproductive rates are too low for rapid recovery.

**Density-dependent selection.** The density of the population itself is a selection pressure on life history. At low density (after a disturbance, during colonization), r-selected traits are favored — fast reproduction fills empty space. As population density increases toward carrying capacity, K-selected traits become favored — competitive ability matters more than reproductive speed. This density-dependent selection can produce cyclical shifts in life history traits within a population, particularly in species with sufficient standing genetic variation or phenotypic plasticity (Mueller 1997). In agent-based models with heritable life history traits and explicit population dynamics, this density-dependent selection shift can emerge endogenously — the trait distribution evolves in response to population dynamics that are themselves shaped by the trait distribution.

**Life history and community assembly.** The mix of life history strategies in a community determines its response to disturbance. Communities dominated by r-strategists recover quickly from disturbance but are inherently unstable. Communities dominated by K-strategists are stable but slow to recover. Intermediate disturbance regimes favor coexistence of both strategies — r-strategists exploit post-disturbance opportunities, K-strategists dominate during stable intervals (Connell 1978). This is one mechanism behind the intermediate disturbance hypothesis: peak diversity occurs at intermediate disturbance frequencies because both strategic types persist.

**Evolutionary feedback through population structure.** Life history strategies shape population age and size structure, which in turn determines the population's aggregate resource demand, competitive impact, and vulnerability to predation. A population dominated by large, old K-strategists has different total resource consumption, different predation risk profile, and different reproductive output than one dominated by small, young r-strategists — even at the same total population size. These structural differences feed back through resource competition and predator-prey dynamics to affect community composition and ecosystem function.

## ABM and computational literature

Life history theory has been explored extensively through individual-based and agent-based models, where explicit energy budgets and heritable traits allow life history strategies to evolve under selection rather than being imposed as fixed parameters.

### Dynamic Energy Budget models

Kooijman's (2010) Dynamic Energy Budget (DEB) theory provides a mechanistic framework for individual energy allocation that maps directly to life history trade-offs. DEB models track energy flow through an individual organism: assimilation from food, allocation to growth vs. reproduction (governed by the "kappa rule" — a fixed fraction to soma, the remainder to reproduction), somatic maintenance costs, and maturation. Life history variation emerges from differences in DEB parameter values. DEB-based IBMs (Martin et al. 2012) have been used to model population dynamics of fish, invertebrates, and other organisms, with life history trade-offs arising naturally from the energy budget rather than being imposed as correlations between traits.

### Evolving allocation strategies

Several ABM frameworks have demonstrated the emergence of life history strategies from evolving energy allocation rules. Agents are given a heritable allocation parameter (e.g., fraction of energy devoted to reproduction vs. growth) and compete in an environment with specific mortality and resource dynamics. The key finding across these models is that the evolved allocation strategy depends on the mortality regime and resource dynamics, consistent with life history theory predictions:

- High extrinsic mortality environments evolve r-like strategies: early reproduction, high fecundity, low somatic investment.
- Stable, resource-limited environments evolve K-like strategies: delayed reproduction, high competitive ability, high somatic investment.
- Fluctuating environments evolve bet-hedging: variable allocation strategies or polymorphic populations.

### Adaptive dynamics

Adaptive dynamics models (Metz et al. 1996, Geritz et al. 1998) analyze the evolution of life history traits by studying the fitness of rare mutants in a population at equilibrium. These models have been used to predict evolutionary branching — the spontaneous divergence of a single population into two coexisting life history strategies (e.g., an r-strategist and a K-strategist coexisting in a heterogeneous environment). Adaptive dynamics models of the growth-reproduction trade-off (Dieckmann & Doebeli 1999) have shown that disruptive selection on life history traits can drive sympatric speciation when trade-offs generate frequency-dependent fitness.

### Digital evolution

Digital evolution experiments provide direct evidence that life history strategies emerge from evolved energy allocation in agent-based systems. In Avida (Lenski et al. 2003), self-replicating digital organisms evolve in environments with varying resource availability and mortality rates. Organisms that evolve in resource-rich, low-competition environments tend toward rapid reproduction (r-like), while those in stable, competitive environments evolve slower replication with more complex metabolic capabilities (K-like). Tierra (Ray 1991) demonstrated similar evolutionary dynamics, with organisms evolving diverse replication strategies including parasitic and hyperparasitic life histories — an analogue to the full spectrum of exploitative strategies seen in biological communities.

### Life history evolution in NetLogo and similar platforms

The NetLogo models library includes several models exploring life history trade-offs. The Bug Hunt Coevolution models demonstrate how predator-prey coevolution can drive life history changes in both predator and prey populations. More specialized models have explored the evolution of senescence (agent lifespan evolving under selection with trade-offs between early reproduction and late survival), the evolution of dispersal (agents evolving natal dispersal distance in fragmented landscapes), and the evolution of reproductive timing.

### Relevance to energy-budget ABMs

The key insight from the computational literature is that explicit energy budgets are sufficient to generate life history evolution in ABMs. When agents have:

1. A finite energy income (from foraging, photosynthesis, or other acquisition),
2. Competing demands on that energy (growth, maintenance, reproduction, movement, defense),
3. Heritable parameters governing allocation among those demands,
4. A mortality regime that depends on both environment and allocation decisions,

then selection will drive the evolution of life history strategies that reflect the trade-offs inherent in the energy budget. The strategies that emerge are not pre-programmed — they are selected from the space of possible allocation rules by the interaction between individual energy dynamics and population-level competition.

---

## References

Austad, S.N. (1993). Retarded senescence in an insular population of Virginia opossums (*Didelphis virginiana*). *Journal of Zoology*, 229(4), 695-708.

Charnov, E.L. (1993). *Life History Invariants: Some Explorations of Symmetry in Evolutionary Ecology*. Oxford University Press.

Charnov, E.L. & Schaffer, W.M. (1973). Life-history consequences of natural selection: Cole's result revisited. *The American Naturalist*, 107(958), 791-793.

Clobert, J., Baguette, M., Benton, T.G. & Bullock, J.M. (eds.) (2012). *Dispersal Ecology and Evolution*. Oxford University Press.

Clutton-Brock, T.H., Guinness, F.E. & Albon, S.D. (1983). The costs of reproduction to red deer hinds. *Journal of Animal Ecology*, 52(2), 367-383.

Cohen, D. (1966). Optimizing reproduction in a randomly varying environment. *Journal of Theoretical Biology*, 12(1), 119-129.

Cole, L.C. (1954). The population consequences of life history phenomena. *The Quarterly Review of Biology*, 29(2), 103-137.

Connell, J.H. (1978). Diversity in tropical rain forests and coral reefs. *Science*, 199(4335), 1302-1310.

Dieckmann, U. & Doebeli, M. (1999). On the origin of species by sympatric speciation. *Nature*, 400(6742), 354-357.

Geritz, S.A.H., Kisdi, E., Meszena, G. & Metz, J.A.J. (1998). Evolutionarily singular strategies and the adaptive growth and branching of the evolutionary tree. *Evolutionary Ecology*, 12(1), 35-57.

Grime, J.P. (1977). Evidence for the existence of three primary strategies in plants and its relevance to ecological and evolutionary theory. *The American Naturalist*, 111(982), 1169-1194.

Hamilton, W.D. & May, R.M. (1977). Dispersal in stable habitats. *Nature*, 269(5629), 578-581.

Kirkwood, T.B.L. (1977). Evolution of ageing. *Nature*, 270(5635), 301-304.

Kooijman, S.A.L.M. (2010). *Dynamic Energy Budget Theory for Metabolic Organisation*. 3rd ed. Cambridge University Press.

Lenski, R.E., Ofria, C., Pennock, R.T. & Adami, C. (2003). The evolutionary origin of complex features. *Nature*, 423(6936), 139-144.

Levin, S.A., Muller-Landau, H.C., Nathan, R. & Chave, J. (2003). The ecology and evolution of seed dispersal: a theoretical perspective. *Annual Review of Ecology, Evolution, and Systematics*, 34, 575-604.

MacArthur, R.H. & Wilson, E.O. (1967). *The Theory of Island Biogeography*. Princeton University Press.

Martin, B.T., Zimmer, E.I., Grimm, V. & Jager, T. (2012). Dynamic Energy Budget theory meets individual-based modelling: a generic and accessible implementation. *Methods in Ecology and Evolution*, 3(2), 445-449.

Medawar, P.B. (1952). *An Unsolved Problem of Biology*. H.K. Lewis and Co.

Metz, J.A.J., Geritz, S.A.H., Meszena, G., Jacobs, F.J.A. & van Heerwaarden, J.S. (1996). Adaptive dynamics: a geometrical study of the consequences of nearly faithful reproduction. In *Stochastic and Spatial Structures of Dynamical Systems* (pp. 183-231). North-Holland.

Meyers, L.A. & Bull, J.J. (2002). Fighting change with change: adaptive variation in an uncertain world. *Trends in Ecology & Evolution*, 17(12), 551-557.

Mueller, L.D. (1997). Theoretical and empirical examination of density-dependent selection. *Annual Review of Ecology and Systematics*, 28, 269-288.

Philippi, T. & Seger, J. (1989). Hedging one's evolutionary bets, revisited. *Trends in Ecology & Evolution*, 4(2), 41-44.

Pianka, E.R. (1970). On r- and K-selection. *The American Naturalist*, 104(940), 592-597.

Pierce, S., Negreiros, D., Cerabolini, B.E.L., et al. (2017). A global method for calculating plant CSR ecological strategies applied across biomes world-wide. *Functional Ecology*, 31(2), 444-457.

Ray, T.S. (1991). An approach to the synthesis of life. In *Artificial Life II* (pp. 371-408). Addison-Wesley.

Reznick, D.N., Bryant, M.J., Roff, D., Ghalambor, C.K. & Ghalambor, D.E. (2004). Effect of extrinsic mortality on the evolution of senescence in guppies. *Nature*, 431(7012), 1095-1099.

Reznick, D., Bryant, M.J. & Bashey, F. (2002). r- and K-selection revisited: the role of population regulation in life-history evolution. *Ecology*, 83(6), 1509-1520.

Roff, D.A. (2002). *Life History Evolution*. Sinauer Associates.

Rose, M.R. & Charlesworth, B. (1981). Genetics of life history in *Drosophila melanogaster*. I. Sib analysis of adult females. *Genetics*, 97(1), 173-186.

Seger, J. & Brockmann, H.J. (1987). What is bet-hedging? In *Oxford Surveys in Evolutionary Biology* (Vol. 4, pp. 182-211). Oxford University Press.

Smith, C.C. & Fretwell, S.D. (1974). The optimal balance between size and number of offspring. *The American Naturalist*, 108(962), 499-506.

Stearns, S.C. (1977). The evolution of life history traits: a critique of the theory and a review of the data. *Annual Review of Ecology and Systematics*, 8, 145-171.

Stearns, S.C. (1992). *The Evolution of Life Histories*. Oxford University Press.

Trivers, R.L. (1972). Parental investment and sexual selection. In *Sexual Selection and the Descent of Man, 1871-1971* (pp. 136-179). Aldine.

Venable, D.L. (2007). Bet hedging in a guild of desert annuals. *Ecology*, 88(5), 1086-1090.

Williams, G.C. (1957). Pleiotropy, natural selection, and the evolution of senescence. *Evolution*, 11(4), 398-411.

Williams, G.C. (1966). *Adaptation and Natural Selection*. Princeton University Press.
