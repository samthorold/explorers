# Stability and Resilience

What keeps food webs alive: which structures make real ecosystems stable, resilient or, in a narrow sense, anti-fragile, how strong the evidence is for each, and the conditions under which each mechanism holds or fails.

Real food webs persist less because any one structure is stable than because three feedbacks stay intact. Dead matter returns its nutrient by routes that do not hinge on a single consumer guild. Pressure on each prey slackens as that prey becomes rare. Space breaks populations into patches that fluctuate out of phase and re-seed one another.

Variability and disturbance add to persistence only when specific ingredients are present: species that respond differently to a driver, a coupling between good conditions and later crowding, a trade-off between competing and colonising, and populations large enough to ride out chance. Without those ingredients, variability mostly does harm.

"Anti-fragility" has an exact meaning, a convex response, so that by Jensen's inequality variance raises the mean. It holds for named quantities over limited ranges. No study yet shows a whole ecosystem gaining from variance in that formal sense.

Much of the reading behind this document worked from abstracts, publisher landing pages and secondary summaries, because many full texts are paywalled. Each section closes with an *Evidence quality* note, and a consolidated list appears near the end. Disturbance regimes, alternative stable states, the resilience definitions and the adaptive cycle are covered in [Disturbance and Succession](disturbance-and-succession.md); this document treats them briefly and adds what bears on stability.

## Stability is several properties, and they do not move together

"Stability" names at least four distinct properties: engineering resilience (return speed), ecological resilience (the size of disturbance absorbed before a shift to another domain of attraction), resistance (how far a variable moves under a perturbation) and persistence (how long the system lasts). The first three are defined in [Disturbance and Succession](disturbance-and-succession.md), *Resilience and resistance* ([Pimm 1984, *Nature*](https://doi.org/10.1038/307321a0); [Holling 1973, *Annu. Rev. Ecol. Syst.*](https://doi.org/10.1146/annurev.es.04.110173.000245)). Holling's central point is that a system can return quickly within a small basin and still be easily tipped.

These dimensions are often only weakly correlated in real and model communities ([Donohue et al. 2016, *Ecol. Lett.*](https://doi.org/10.1111/ele.12648); [Ives & Carpenter 2007, *Science*](https://doi.org/10.1126/science.1133258)). A system can score well on one and badly on another. Grimm and Wissel catalogued the terminological sprawl ([Grimm & Wissel 1997, *Oecologia*](https://doi.org/10.1007/s004420050090)). Any claim that a mechanism "stabilises" has to name which property it improves.

**Catastrophic shifts** are sudden switches to a contrasting state, documented in lakes, coral reefs, drylands and forests; the examples and the role of hysteresis are in [Disturbance and Succession](disturbance-and-succession.md), *Alternative stable states*. A loss of ecological resilience usually precedes them ([Scheffer et al. 2001, *Nature*](https://research.wur.nl/en/publications/catastrophic-shifts-in-ecosystems/)). Near such a fold, recovery from small perturbations slows. That slowing shows up as rising lag-1 autocorrelation and variance, "critical slowing down" ([Scheffer et al. 2009, *Nature*](https://www.nature.com/articles/nature08227); [Dakos et al. 2012, *PLoS ONE*](https://harvardforest1.fas.harvard.edu/publications/pdfs/Dakos_PLoS1_2012.pdf)).

There are two important counterweights. First, the same warning signals precede non-catastrophic transitions, so they are not specific ([Kéfi et al. 2013, *PLoS ONE*](https://sciences.ucf.edu/biology/d4lab/wp-content/uploads/sites/23/2024/08/Kefi-etal-2013.pdf)). Second, a meta-analysis finds that thresholds in ecological responses to global change mostly do not emerge from empirical data ([Hillebrand et al. 2020, *Nat. Ecol. Evol.*](https://www.nature.com/articles/s41559-020-1256-9)). Apparent regime shifts can also be long transients or "ghost" attractors rather than true alternative states ([Hastings et al. 2018, *Science*](https://doi.org/10.1126/science.aat6412)).

So a system whose persistence is sensitive to small changes in its conditions fits any of three pictures: narrow basins, conditions near a fold, or long transients. They call for different explanations, and telling them apart takes more than the observation of sensitivity.

**Panarchy's adaptive cycle** frames the same ideas as a sequence of phases, exploitation, conservation, release and reorganisation ([Gunderson & Holling 2002, *Panarchy*, Island Press](https://doi.org/10.2307/jj.41003538); see [Disturbance and Succession](disturbance-and-succession.md), *Resilience and resistance*). Through the conservation phase, capital accumulates and connectedness rises while resilience falls. Release frees the locked capital for renewal.

*Evidence quality.* The Holling and Pimm definitions are taken from secondary sources; the primary texts were not fetched. The Grimm and Wissel count of definitions is recalled from the abstract and is left out here. The panarchy phase descriptions are standard summaries; the book itself was not read.

## Detritus stabilises only while something eats it

### Donor control holds only while decomposers track the pile

Detritus stabilises food webs in theory because its supply is **donor-controlled**. Its input does not depend on its own stock, it does not reproduce, and it pays no maintenance, so it acts as an energy reservoir ([Moore et al. 2004, *Ecol. Lett.*](https://rosemondlab.ecology.uga.edu/wp-content/uploads/2014/11/Moore-et-al.-2004-EcolLet.pdf)).

In Moore and colleagues' detritus–microbe model, the self-damping term on the detritus compartment is the microbes' uptake, which scales with detrital stock. In their words, stabilisation arises "not through self-limitation of the resource… but rather as a result of the constant inputs of detritus." In Monte Carlo comparisons, detritus-based chains were more often energetically feasible and returned to equilibrium faster than chains built on producers ([Moore et al. 2004](https://rosemondlab.ecology.uga.edu/wp-content/uploads/2014/11/Moore-et-al.-2004-EcolLet.pdf)).

This stabiliser has a precise failure condition. The damping term exists only while decomposer biomass is present and its uptake tracks the pile. **When decomposers are scarce or out of reach, the term vanishes, and the dead pool simply integrates mortality with no damping at all.** In a closed nutrient system, that integration draws down every other compartment.

The models that make detritus a strong stabiliser also assume external (allochthonous) input. In a closed system the "donor" is the living web's own mortality, so the stabilising effect is plausibly weaker. This is an inference: the source says only that fixed-input models "mimic input from outside the local system." No source was found that analyses donor control in a fully closed nutrient system.

Recycling efficiency cuts both ways. In the DeAngelis and Loreau line of theory, more efficient recycling lowers asymptotic resilience (return speed) but raises resistance, meaning a smaller deviation after a perturbation ([DeAngelis, Bartell & Brenkert 1989, *Am. Nat.*](https://www.researchgate.net/publication/249140433_Effects_of_Nutrient_Recycling_and_Food-Chain_Length_on_Resilience); Loreau 1994, *Am. Nat.*, as summarised in [Theis et al. 2022, *Funct. Ecol.*](https://besjournals.onlinelibrary.wiley.com/doi/10.1111/1365-2435.13961)). The same work reports that recycling stabilises species at odd distances from the top of a chain and destabilises those at even distances. So recycling is not simply good. What matters is that it exists and tracks the stock.

### Real litter always has an unconditional exit

Real dead matter loses mass by abiotic or weakly biotic routes as well as through decomposers. Leaching in the first four hours can account for up to a third of the total mass loss measured for green tea in a standard litter-bag assay (the Tea Bag Index) ([Lind et al. 2022, *Ecol. Evol.*](https://onlinelibrary.wiley.com/doi/full/10.1002/ece3.9118)). Sunlight, through photodegradation, is the dominant control on grass-litter decomposition in a semi-arid Patagonian grassland (Austin & Vivanco 2006, *Nature*, as summarised in [King et al. 2012, *Biogeochemistry*](https://link.springer.com/article/10.1007/s10533-012-9737-9)). Fire is the mineraliser of last resort in peat-forming boreal systems.

These routes are slow but **unconditional**: they need no particular heterotroph to be present. They are a donor-controlled release term in its purest form. Biological decomposers (bacteria, fungi and their grazers) still carry out most degradation; no robust global partition of litter mass loss into microbial and abiotic shares was found.

The real systems that most resemble a nutrient lockup in dead matter are those where both abiotic loss and microbial activity are suppressed. In **paludifying boreal forest**, a thick Sphagnum layer accumulates, makes soils colder and wetter, and cuts nutrient availability. Black spruce productivity falls by **50–80 %** as paludification proceeds ([Simard et al. 2007, *Ecol. Appl.*](https://www.researchgate.net/publication/5932126_Forest_productivity_decline_caused_by_successional_paludification_of_boreal_soils)). Buried deadwood persists "until consumed by high intensity fire" ([Jacobs et al. 2015, *Ecosphere*](https://esajournals.onlinelibrary.wiley.com/doi/full/10.1890/ES14-00063.1)).

Over millennia, **retrogression** in six long-term chronosequences brings reduced productivity, slower decomposition and slower nutrient cycling together, as phosphorus and nitrogen become unavailable ([Wardle, Walker & Bardgett 2004, *Science*](https://www.science.org/doi/10.1126/science.1098778)). The same feedback appears as the oligotrophic trap in [Nutrient Cycling](nutrient-cycling.md), *Feedback loops*.

Paludification is a positive feedback: accumulation makes conditions worse for decomposition, which drives further accumulation. Real lockup is nonetheless usually partial and slow. Abiotic routes and a redundant decomposer guild keep some throughput going, and fire resets the store.

Small-scale exclusions point the same way. Removing microarthropods with naphthalene "drastically reduced" oak litter breakdown and raised immobilisation ([OSTI record](https://www.osti.gov/etdeweb/biblio/6102545)). Fungicide cut fungal nitrogen translocation by 59–78 % ([*Soil Biol. Biochem.*](https://www.sciencedirect.com/science/article/abs/pii/S0038071799002059)). No clean whole-ecosystem "remove all decomposers" experiment was found.

### Stoichiometry decides whether a decomposer is a recycler or a hoarder

Two conditions must hold for producers and decomposers to coexist ([Daufresne & Loreau 2001, *Ecology*](https://esajournals.onlinelibrary.wiley.com/doi/abs/10.1890/0012-9658(2001)082%5B3069:ESPPDI%5D2.0.CO;2)). First, decomposers must be **limited by the carbon (energy) in detritus**, not by mineral nutrient. Second, the **gap between producer and decomposer carbon-to-nutrient ratios must be small enough**. The first condition holds when decomposers out-compete producers for mineral nutrient. They then stay energy-limited, keep processing detritus, and excrete their surplus nutrient.

A global synthesis of about 2800 observations makes the threshold explicit ([Manzoni et al. 2008, *Science*](https://jacksonlab.stanford.edu/sites/g/files/sbiybj20871/files/media/file/science08.pdf)). Net nutrient release begins when litter nutrient-to-carbon exceeds a critical ratio, **r_CR = decomposer carbon-use efficiency × decomposer nutrient-to-carbon**. Below that ratio, decomposers immobilise nutrient rather than release it. On poor litter, decomposers lower their carbon-use efficiency, respiring more carbon so that they can release nutrient earlier. Manzoni and colleagues describe this as "a universal response of decomposers in nutrient-poor conditions."

So a decomposer that processes dead matter can still starve the mineral pool if it retains most of what it takes. Whether decomposition feeds the pool or the decomposer turns on that ratio. The consumer-side version of the same mismatch is in [Nutrient Cycling](nutrient-cycling.md), *Stoichiometry and nutrient recycling*.

A further result concerns decomposers that can use both organic and mineral sources of the same element. Stoichiometric constraints create trade-offs between such decomposers and draw them toward co-limitation over evolutionary time ([Cherif & Loreau 2007, *Am. Nat.*](https://doi.org/10.1086/516844)). In Daufresne and Loreau's terms, an organism that both takes up mineral nutrient and processes detritus sits on the competitive pathway; a decomposer that takes no mineral nutrient sits on the facilitative one. This is an inference: Daufresne and Loreau define the two pathways by whether decomposers compete with producers for mineral nutrient, and neither source places an organism that does both on either one.

### Two recycling channels buffer better than one

Real webs, and soil webs especially, split into energy channels that turn over at different rates: a fast bacterial channel and a slow fungal one. Top predators couple them, and that asymmetric coupling "convey[s] both local and non-local stability" ([Rooney et al. 2006, *Nature*](https://www.nature.com/articles/nature04887)).

Moore and colleagues report that linked pathways are most stable when top predators take **20–60 %** of their energy from either channel, rather than all of it from one ([Moore et al. 2004](https://rosemondlab.ecology.uga.edu/wp-content/uploads/2014/11/Moore-et-al.-2004-EcolLet.pdf)). Most community webs (72.5 %) contain distinct energy channels. About a fifth of those channels start from detritus ([Moore & Hunt 1988, *Nature*, as reported in Moore et al. 2004](https://rosemondlab.ecology.uga.edu/wp-content/uploads/2014/11/Moore-et-al.-2004-EcolLet.pdf)).

The implication is that recycling carried by a single class of organism is a single channel, whose reliability depends on that class. Recycling that is episodic and tied to the state of the producers is a different channel from steady recycling by obligate decomposers that follows the pile, and a web with only the first lacks the slow reservoir channel. This reading is an inference from the channel literature; no source treats it directly.

*Evidence quality.* Moore et al. 2004 and Manzoni et al. 2008 were read in full. DeAngelis et al. 1989, Daufresne and Loreau 2001, Cherif and Loreau 2007, and Rooney et al. 2006 rest on abstracts or landing pages. The DeAngelis claim that resilience scales inversely with nutrient residence time is recalled, not verified. Austin and Vivanco 2006 is known through a review, not read. The leaching range of 5–40 % of carbon loss comes from a search snippet and is not used here. Paludification and retrogression are natural analogues, not controlled experiments.

## Prey need a refuge at low density

### The shape of the functional response sets the low-density refuge

A saturating (type II) functional response destabilises consumer–resource dynamics. It drives the **paradox of enrichment**: raising prey supply turns a stable equilibrium into large cycles that approach extinction ([Rosenzweig 1971, *Science*](https://doi.org/10.1126/science.171.3969.385)). In multi-species webs it produces chaos and extinctions.

A sigmoid (type III) response gives prey a refuge at low density. Williams and Martinez found that relaxing type II feeding "slightly" on rare prey "reduces or eliminates extinctions and non-persistent chaos" in ten-species webs ([Williams & Martinez 2004, *Eur. Phys. J. B*](https://link.springer.com/article/10.1140/epjb/e2004-00122-1)). The commonly cited dose is a Hill exponent of about 1.2, but that figure is recalled rather than read from the paper.

Predator interference works in a similar way. In model food webs, type III and interference (Beddington–DeAngelis) responses made systems "highly robust against accelerated oscillations due to enrichment," and they could turn connectance from destabilising into stabilising ([Rall, Guill & Brose 2008, *Oikos*](https://www.researchgate.net/publication/227712869_Food-web_connectance_and_predator_interference_dampen_the_paradox_of_enrichment)). Sigmoid responses that come from **switching** between prey stabilise only if the accelerating part of the curve falls at the prey densities actually experienced ([Oaten & Murdoch 1975, *Am. Nat.*](https://doi.org/10.1086/282998)).

A linear, unsaturated (type I) response is not destabilising in the classical equation sense, because it has no low-density positive feedback. But it also has no refuge. **A type I consumer's per-prey risk stays constant all the way down to zero prey.** Persistence then depends entirely on the consumers starving faster than they strip their prey. Where prey are discrete and local, a cluster of such consumers can strip a patch to zero, which is local extinction rather than a damped equilibrium.

Empirically, type III is uncommon. Reviews find it in under about 15 % of fitted responses. That figure may reflect poor sampling at low prey density and unrealistic arenas ([Kalinkat et al. 2023, *Front. Ecol. Evol.*](https://www.frontiersin.org/articles/10.3389/fevo.2023.1033818)). A sceptical view asks whether type III responses are "just statistical apparitions" ([DeLong 2025, *Ecosphere*](https://esajournals.onlinelibrary.wiley.com/doi/full/10.1002/ecs2.70247)). A preprint defends type I as commoner than assumed in consumers that feed on many prey at once ([bioRxiv 2024](https://www.biorxiv.org/content/10.1101/2024.05.14.594210v3.full)).

The safe reading is that **what stabilises is a low-density refuge, whatever produces it**: switching, spatial refuges, size structure, or interference. The literal per-predator curve matters less.

### Adaptive foraging can reverse May's result, but not robustly

In random communities, complexity destabilises: a random community matrix is almost surely unstable once interaction strength × √(species × connectance) exceeds 1 ([May 1972, *Nature*](https://doi.org/10.1038/238413a0)). Allesina and Tang refined this ([Allesina & Tang 2012, *Nature*](https://www.nature.com/articles/nature10832)). Predator–prey interactions are the most stabilising sign structure, while competition and mutualism destabilise. Imposing realistic food-web structure lowers predator–prey stability.

Kondoh showed that consumers who shift a fixed foraging effort towards more profitable prey make persistence *rise* with connectance ([Kondoh 2003, *Science*](https://www.science.org/doi/10.1126/science.1079154)). **The critique matters here.** Brose, Williams and Martinez found that the effect disappears with type II dynamics on niche-model webs ([Brose, Williams & Martinez 2003, *Science*](https://pubmed.ncbi.nlm.nih.gov/12920282/)). A review still concludes that adaptive behaviour broadly promotes stability and can reverse May's relationship ([Valdovinos et al. 2010, *Ecol. Lett.*](https://pubmed.ncbi.nlm.nih.gov/20937057/)). But the outcome depends on the functional response and the web structure.

The ingredient that matters is a **fixed effort budget reallocated among prey**. Pressure on a prey then falls as it becomes rare, which is a community-level version of the switching refuge. Consumers with no budget, for which each added prey in reach is another full-strength link, are May's destabilising case rather than Kondoh's.

### Weak links help only where they sit

Real webs have many weak links and few strong ones. Weak and intermediate links damp the oscillations of strong consumer–resource pairs ([McCann, Hastings & Huxel 1998, *Nature*](https://www.nature.com/articles/27427)). In empirically parameterised soil webs, the observed pattern of interaction strengths was more stable than the same webs with strengths randomised ([de Ruiter, Neutel & Moore 1995, *Science*](https://doi.org/10.1126/science.269.5228.1257)). Long trophic loops carry relatively many weak links, which resolves May's paradox for those webs ([Neutel, Heesterbeek & de Ruiter 2002, *Science*](https://doi.org/10.1126/science.1068326)). Weak omnivory damps chaos, while strong omnivory can destabilise ([McCann & Hastings 1997, *Proc. R. Soc. B*](https://doi.org/10.1098/rspb.1997.0172)).

**The critique matters again.** Allesina and Tang find that a preponderance of weak interactions *as such* lowers the probability of stability in random predator–prey matrices ([Allesina & Tang 2012](https://www.nature.com/articles/nature10832)). The benefit comes from **where** weak links sit (in long loops, or diverting pressure into a second channel), not from how many there are.

### Size structure separates time scales

In bioenergetic models, rates per unit biomass scale as body mass to the −¼ ([Yodzis & Innes 1992, *Am. Nat.*](https://doi.org/10.1086/285380)). Large consumers therefore interact weakly and slowly with small prey. Persistence rises with the predator-to-prey mass ratio and levels off at about **10× for invertebrates and 100× for ectotherm vertebrates**. At those ratios, the negative effect of species richness on persistence becomes neutral or positive ([Brose, Williams & Martinez 2006, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/10.1111/j.1461-0248.2006.00978.x)). **97 %** of tri-trophic chains in five natural webs fall inside the predicted persistence domain ([Otto, Rall & Brose 2007, *Nature*](https://www.nature.com/articles/nature06359)). Larger size ratios also shift functional responses towards type III ([Kalinkat et al. 2013, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/abs/10.1111/ele.12147)).

The ingredient that matters is **consumer rates per unit biomass that are slower than their resources' rates**. Body size is just the usual carrier. A mass ratio near 1, where consumers and prey run on similar time scales, is the destabilising end of Brose's range.

These benefits come from models that already include a mild type III response. Whether size structure alone, with no low-density relief, is enough was not verified. Heckmann and colleagues report that size structure and adaptive foraging partly substitute for each other ([Heckmann et al. 2012, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/10.1111/j.1461-0248.2011.01733.x)), but that result is recalled, not read.

*Evidence quality.* This whole section is almost entirely theory. Fetches from Wiley, Nature and PubMed were blocked, so Williams and Martinez 2004, Brose 2006, Otto 2007 and Allesina and Tang 2012 rest on abstracts. Several older papers (Holling, Rosenzweig, May, Murdoch and Oaten, de Ruiter, McCann and Hastings, Polis and Strong, Yodzis and Innes) are cited from bibliographic knowledge without re-reading. Kondoh's reply to Brose et al. was not retrieved. No study was found of type I versus type III responses in spatial individual-based webs with discrete prey.

## Space stabilises through asynchrony

The background on metapopulations, rescue effects and dispersal is in [Spatial Ecology](spatial-ecology.md). This section covers what space does for the persistence of interacting species.

### Subdivision buys time when the prey out-disperse the predator

Huffaker's mites on oranges are the classic demonstration ([Huffaker 1958, *Hilgardia*](https://en.wikipedia.org/wiki/Huffaker's_mite_experiment)). In simple arrangements the predator drove its prey extinct and then died out itself. Only one universe persisted for **three predator–prey oscillations**; the eleven others each gave a single peak and then extinction. That universe had barriers that slowed the predator and aids that helped the prey disperse.

The design worked through a **dispersal asymmetry favouring the prey**. It was unreplicated, and its result is best read as "space delays extinction."

Holyoak and Lawler replicated the effect with protists ([Holyoak & Lawler 1996, *Ecology*](https://esajournals.onlinelibrary.wiley.com/doi/10.2307/2265790)). Arrays of linked bottles persisted **130 days, about 600 prey and 440 predator generations**. Undivided microcosms of the same volume lost the predator within a mean of about **70 days**. The arrays showed asynchrony among patches, low dispersal and rescue.

Theory groups spatial stabilisation into three effects ([Briggs & Hoopes 2004, *Theor. Popul. Biol.*, cited via a 2015 *Bull. Math. Biol.* paper](https://link.springer.com/article/10.1007/s11538-015-0108-2)):

1. statistical averaging over patches whose dynamics are out of phase;
2. dispersal that itself acts like density dependence;
3. a shift in mean population sizes.

All three need dispersal that is neither too low (no recolonisation) nor too high (synchrony, which rebuilds a single well-mixed arena). High dispersal does synchronise subpopulations and lower metapopulation stability ([Vogwill, Fenton & Brockhurst 2009, *Oikos*, via Wang et al. 2015](https://peerj.com/articles/1295/)). In homogeneous metapopulations, the stabilising and synchronising effects of dispersal cancel. **Net stabilisation needs heterogeneity among patches** ([Wang et al. 2015, *PeerJ*](https://peerj.com/articles/1295/)).

### Insurance and portfolio effects need responses that differ

The **spatial insurance hypothesis** says that, in landscapes that are heterogeneous and fluctuate out of phase, intermediate dispersal maximises local diversity and productivity and minimises their variability ([Loreau, Mouquet & Gonzalez 2003, *PNAS*](https://www.pnas.org/doi/10.1073/pnas.2235465100)). Too little dispersal fails to bring in the locally best species, and too much homogenises the landscape. In multi-trophic metacommunities, consumer dispersal can undermine that insurance for producers ([Limberger et al. 2019, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/10.1111/ele.13365)).

Biodiversity steadies **aggregate** properties mainly through asynchrony among species, with a statistical-averaging component ([Yachi & Loreau 1999, *PNAS*](https://www.pnas.org/doi/full/10.1073/pnas.96.4.1463); [Doak et al. 1998, *Am. Nat.*](https://pubmed.ncbi.nlm.nih.gov/18811357/); [Loreau & de Mazancourt 2013, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/10.1111/ele.12073)). At Cedar Creek, the stability of total production rose with plant diversity **even as individual species became less stable** ([Tilman, Reich & Knops 2006, *Nature*](https://www.nature.com/articles/nature04742)).

That last result marks a limit. Portfolio effects stabilise the total, not the persistence of the parts. No source was found that links aggregate stability directly to avoiding extinction.

Insurance and portfolio mechanisms that need asynchrony imposed from outside are expected to be weak or absent where the environment is spatially uniform and constant in time. What remains in such a setting is asynchrony the community generates itself: predator–prey metapopulation dynamics of the Huffaker and Holyoak kind, and spatial self-structuring.

### Viscosity can select for restraint, but kin competition can cancel it

Local interaction can select for **prudent exploitation**: overly aggressive strategies exhaust their local host supply and fail to invade ([Rand, Keeling & Wilson 1995, *Proc. R. Soc. B*](https://www.jstor.org/stable/50234); [Lion & van Baalen 2008, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/abs/10.1111/j.1461-0248.2007.01132.x)). The phage-and-bacteria experiments found that restricted migration favoured prudent phage ([Kerr et al. 2006, *Nature*](https://doi.org/10.1038/nature04864)).

**The caveat is exact.** In a population held at fixed density, the benefit of higher relatedness from limited dispersal is exactly cancelled by increased competition among kin ([Taylor 1992, *Evol. Ecol.*](https://link.springer.com/article/10.1007/BF02270971); [Wilson, Pollock & Dugatkin 1992, *Evol. Ecol.*](https://link.springer.com/article/10.1007/BF02270969)). The cancellation breaks when there is demographic elasticity, such as empty sites or density-dependent dispersal ([PMC 2022](https://pmc.ncbi.nlm.nih.gov/articles/PMC8905158/)). Some lattice studies find self-structuring weaker than claimed ([*Sci. Rep.* 2018](https://www.nature.com/articles/s41598-018-30945-1)). Kin-directed interactions more broadly are in [Cannibalism and Kin](cannibalism-and-kin.md).

*Evidence quality.* Huffaker and Holyoak and Lawler are well established. The Briggs and Hoopes three-mechanism summary is recalled; the full text was not fetched. The quantitative claims of Loreau et al. 2003 (the dispersal hump) and the thrust of Limberger et al. 2019 are recalled; the PNAS full text was unavailable. The Vogwill result comes through a secondary summary. Kerr et al. 2006 is cited from bibliographic knowledge. No source treats metacommunity persistence on a continuous space with evolving dispersal.

## Variability helps only when its ingredients are present

### Constant forcing removes the storage effect, but not relative nonlinearity

Modern coexistence theory splits mechanisms into **equalising** ones, which shrink average fitness differences, and **stabilising** ones, which make each species limit itself more than it limits others ([Chesson 2000, *Annu. Rev. Ecol. Syst.*](https://doi.org/10.1146/annurev.ecolsys.31.1.343)).

The **temporal storage effect** has three ingredients ([Johnson, Godoy & Hastings, arXiv 2201.06687](https://arxiv.org/pdf/2201.06687), restating Chesson):

1. species respond differently to the environment;
2. environment and competition interact in per-capita growth;
3. environment and competition covary over time.

A long-lived stage such as a seed bank is neither necessary nor sufficient. The interaction can arise from nothing more than uptake rate × resource concentration. The storage effect is also not bet hedging: it reduces growth-rate variance mainly for residents, and invaders gain through a higher mean.

The empirical record supports it in some systems and not others. *Daphnia* in Oneida Lake show it ([Cáceres 1997, *PNAS*](https://www.pnas.org/doi/10.1073/pnas.94.17.9171)). In Kansas prairie grasses, "temporal variability increases low-density growth rates" ([Adler et al. 2006, *PNAS*](https://doi.org/10.1073/pnas.0600599103)). Sonoran desert annuals show it through a trade-off between growth capacity and tolerance of low resources ([Angert et al. 2009, *PNAS*](https://www.pnas.org/doi/10.1073/pnas.0904512106)). In an Idaho sagebrush steppe, however, the climate's contribution to coexistence was weak, because the third ingredient, the covariance, was very small ([Adler et al. 2009, *Ecology*](https://esajournals.onlinelibrary.wiley.com/doi/10.1890/08-2241.1)).

**The central critique** comes from Stump and Vasseur ([Stump & Vasseur 2023, *Ecol. Monogr.*](https://doi.org/10.1002/ecm.1585)). They argue that the storage effect depends on four restrictive assumptions:

1. density dependence acts without delay;
2. intra- and interspecific competition are nearly identical;
3. fitness differences are zero;
4. stochastic extinction can be ignored.

Each relaxation weakens the effect. Specialising on rare *temporal* niches is "far less effective" than specialising on other rare niches, and storage effects "tend to reduce mean persistence times, even if invader growth rates are positive." They conclude that the storage effect is "probably not an important explanation for species diversity in most systems." For any single population, variance also lowers long-run growth, because that growth follows the geometric mean ([Lewontin & Cohen 1969, *PNAS*](https://doi.org/10.1073/pnas.62.4.1056)).

**Relative nonlinearity** needs no external forcing. Species whose growth responds with different curvature to a shared limiting factor are helped or harmed by that factor's variance, by Jensen's inequality. One consumer's saturating response can generate endogenous cycles that a second consumer exploits ([Armstrong & McGehee 1980, *Am. Nat.*](https://doi.org/10.1086/283553)).

Resource-competition models under constant supply produce oscillations with three species and chaos with five, so more species persist than there are resources ([Huisman & Weissing 1999, *Nature*](https://doi.org/10.1038/46540)). Chaos appeared in a plankton mesocosm under roughly constant conditions ([Benincà et al. 2008, *Nature*](https://doi.org/10.1038/nature06512)).

In nectar yeasts, models built from single-species assays predicted mixed-culture outcomes with **83 % accuracy**. Relative nonlinearity was **equal to or larger than** the storage effect, which falsifies the assumption that it is negligible ([Letten et al. 2018, *PNAS*](https://doi.org/10.1073/pnas.1801846115)).

So constant external forcing removes the temporal storage effect by definition, and the spatial version needs heterogeneity imposed from outside. **But a system under constant forcing keeps relative nonlinearity and coexistence driven by its own oscillations.**

### Seasonality and pulses create niches, not stability

Seasonal forcing of predator–prey models, "even with small amplitudes, can increase the number of attractors… and lead [to]… chaos" ([Rinaldi, Muratori & Kuznetsov 1993, *Bull. Math. Biol.*](https://link.springer.com/article/10.1007/BF02461847)). A shared external driver synchronises patches. In protist microcosms, that synchrony depends on cycles generated by the predator ([Vasseur & Fox 2009, *Nature*](https://www.nature.com/articles/nature08208)). Synchrony raises the risk of global extinction. Environmental noise in nature is usually reddened, meaning positively autocorrelated, and its colour changes outcomes ([Vasseur & Yodzis 2004, *Ecology*](https://doi.org/10.1890/02-3122)).

**Resource pulses** are rare, large, brief episodes of plenty that arise in part from "spatiotemporal accumulation and release." They favour consumers that can store, switch or wait out lean periods ([Yang et al. 2008, *Ecology*](https://doi.org/10.1890/07-0175.1)). A store of dead matter that accumulates and is released late or never reads, in these terms, as a pulse whose release step has failed.

### Disturbance helps diversity only through a rare-species advantage

The intermediate disturbance hypothesis and the Fox–Sheil debate over it are summarised in [Disturbance and Succession](disturbance-and-succession.md), *Disturbance as a system concept* ([Connell 1978, *Science*](https://doi.org/10.1126/science.199.4335.1302); [Fox 2013, *TREE*](https://doi.org/10.1016/j.tree.2012.08.014); [Sheil & Burslem 2013, *TREE*](http://campus.lakeforest.edu/menke/PDFs/Bio373/Sheil&Burslem_2013_TREE.pdf)). What matters for stability is the result behind Fox's critique: harshness that lowers everyone's density or growth does not in itself promote coexistence; coexistence needs a rare-species advantage ([Chesson & Huntly 1997, *Am. Nat.*](https://doi.org/10.1086/286080)).

The robust core is that disturbance maintains diversity **only** through a rare-species advantage, such as a competition–colonisation trade-off or a mosaic of patch ages. Merely interrupting exclusion slows it without stopping it.

Disturbance also **renews resources and space** ([White & Pickett 1985](https://doi.org/10.1016/b978-0-12-554520-4.50006-x)). In panarchy's terms, a store of dead matter that accumulates with no release is a conservation phase that never reaches release: capital builds in a rigid stock, and nothing performs the "creative destruction" that would turn it into reorganisation. That is the same diagnosis as a missing donor-controlled release term, reached from a different literature.

### Anti-fragility is a local convexity, not a property of ecosystems

Taleb and Douady define fragility as concavity of a payoff in a stressor, and anti-fragility as convexity ([Taleb & Douady 2013, *Quant. Finance*](https://doi.org/10.2139/ssrn.2124595)). By Jensen's inequality, convex responses gain from variance ([Ruel & Ayres 1999, *TREE*](https://doi.org/10.1016/s0169-5347(99)01664-x)).

Equihua and colleagues carry this into ecology ([Equihua et al. 2020, *PeerJ*](https://peerj.com/articles/8533/)). They define an anti-fragile ecosystem as one that "benefits from environmental variability" and place resilience between fragility and anti-fragility. Their empirical content, however, is a single wildfire case study that uses Fisher information of a vegetation index. They concede this is "necessary but not sufficient." Their proposed indicators are not themselves convexity measures. Their mapping of Taleb's "barbell" (a mix of very safe and very risky positions) onto strong and weak interactions is speculative.

The only ecological quantities that rigorously gain from variance are three:

1. the mean growth of convex responders, through relative nonlinearity;
2. the invasion growth of rare species when a storage effect operates;
3. the relative advantage of a bet-hedging genotype.

**Bet hedging buffers variance rather than exploiting it.** It trades a lower arithmetic mean for a higher geometric mean ([Cohen 1966, *J. Theor. Biol.*](https://doi.org/10.1016/0022-5193(66)90188-3); [Philippi & Seger 1989, *TREE*, via secondary summary](https://link.springer.com/article/10.1007/s10539-024-09970-0); [Venable 2007, *Ecology*](https://doi.org/10.1890/06-1495)). Under constant conditions there is no variance to hedge, and theory predicts selection against it. Dormant propagules still matter, though, as a refuge stock that can re-found a population after a crash. That is persistence through ecological memory, not anti-fragility. Bet hedging as a life-history strategy is in [Life History Theory](life-history-theory.md).

Anti-fragility claims therefore need three things stated: the response variable, the perturbation variable, and the range. Any convex function turns concave when extended far enough. Anti-fragility is also distinct from resilience. A system can be resilient but fragile, or gain from variance while shifting state. No peer-reviewed study was found that demonstrates whole-ecosystem anti-fragility in the formal sense, and that absence is informative.

*Evidence quality.* The storage-effect theory and its critique are well sourced. The paraphrase of Fox's three mechanisms is from secondary knowledge, not the full text. A tally of supporting against non-supporting studies for the intermediate disturbance hypothesis is unverified and is left out here. Cohen 1966 and Venable 2007 are summarised from knowledge; only titles and venues were checked. The Philippi and Seger definition is quoted second-hand. Descamps-Julien and Gonzalez 2005 and Huisman and Weissing 1999 rest on search snippets. The Taleb book and Axenie et al. 2024 were not read.

## Evolution both rescues populations and drives them over the edge

### Evolutionary rescue needs large populations and slow change

Adaptive evolution can reverse a decline after environmental change, giving a U-shaped trajectory. Even populations with the genetic capacity to persist often fail ([Gomulkiewicz & Holt 1995, *Evolution*](https://academic.oup.com/evolut/article-abstract/49/1/201/6870448)).

Yeast exposed to lethal salt were rescued within about **25 generations**, but only above a clear population-size threshold ([Bell & Gonzalez 2009, *Ecol. Lett.*](https://onlinelibrary.wiley.com/doi/abs/10.1111/j.1461-0248.2009.01350.x)). Dispersal of resistant genotypes enabled rescue in metapopulations, and the mode of dispersal mattered ([Bell & Gonzalez 2011, *Science*](https://www.science.org/doi/10.1126/science.1203105)). Rescue depends on a slow enough rate of change ([Lindsey et al. 2013, *Nature*](https://www.nature.com/articles/nature11879)). It is hard to diagnose in the wild, because recovery has to be shown to be genetic rather than plastic or driven by immigration ([Carlson, Cunningham & Westley 2014, *TREE*](https://pubmed.ncbi.nlm.nih.gov/25038023/)).

Mutation supply scales with births, and births collapse during a decline, which is one reason the size threshold exists.

### Selection can drive a population to evolutionary suicide

Rapid evolution reshapes consumer–resource cycles. In rotifer–alga chemostats, it lengthened the predator lag well beyond a quarter-period ([Yoshida et al. 2003, *Nature*](https://www.nature.com/articles/nature01767)). Whether evolution yields coexistence or cycles depends on the shape of the defence–growth trade-off ([*PNAS* 2014](https://www.pnas.org/doi/full/10.1073/pnas.1406357111)).

Adaptive-dynamics theory proves that selection can drive a viable population to **evolutionary suicide**: an individually advantageous mutant invades, and the population then crosses a viability boundary ([Gyllenberg & Parvinen 2001, *Bull. Math. Biol.*](https://link.springer.com/article/10.1006/bulm.2001.0253)). This requires that a mutant's fitness depend on the resident strategy itself, as it does when individuals share a common resource. The typical route is a fold bifurcation, often tied to an Allee effect ([Parvinen 2005, *Acta Biotheor.*](https://link.springer.com/article/10.1007/s10441-005-2531-5)). A continuous, non-catastrophic route also exists ([*J. Math. Biol.* 2015](https://link.springer.com/article/10.1007/s00285-015-0945-5)).

The evolution of exploitative traits can also extinguish a partner species, sometimes called "evolutionary murder" ([Webb 2003, *Am. Nat.*](https://doi.org/10.1086/345858)). A consumer that individually gains from stronger exploitation, depletes the shared prey and then crashes is the tragedy-of-the-commons form of the same feedback. Rescue and suicide are two outcomes of one eco-evolutionary feedback ([Ferrière & Legendre 2013, *Phil. Trans. R. Soc. B*](https://royalsocietypublishing.org/doi/10.1098/rstb.2012.0081)). Clear empirical cases of evolutionary suicide in natural communities remain scarce.

*Evidence quality.* The rescue experiments are well sourced. Ferrière and Legendre 2013 rests on a search summary because the fetch was blocked. Hairston et al. 2005 and Webb 2003 are cited from knowledge. No source was found on rescue under continuous mutation with trade-offs across a trait vector. Whether rapid evolution stabilises or destabilises on average has no net verdict in the literature.

## Four findings recur across the literature

Four findings recur across otherwise separate literatures.

The first is that **per-prey pressure must relax at low prey density**. The functional-response literature asks for a low-density refuge. The adaptive-foraging literature asks for a fixed effort budget that moves away from rare prey. Interference and size structure weaken per-capita links. All of these describe the same requirement, and a linear, budget-free consumer meets none of them.

The second is that **detritus stabilises through a release that does not hinge on a scarce consumer**. Moore's stabilising term, Daufresne and Loreau's coexistence conditions, the abiotic leaching, photodegradation and fire routes, and the natural analogues of paludification and retrogression all locate stability there. Donor control is lost when decomposers are scarce or out of reach.

The third is that **accumulation without release is a stuck release phase**. The adaptive cycle, the theory of resource pulses, and disturbance as renewal reach the same diagnosis from three directions: capital accumulates in a rigid stock and the release step is broken. The remedy every literature describes is a release that is either unconditional or comes from a guild that tracks pulses. Disturbance treats the symptom, while a working decomposer niche treats the cause.

The fourth is that **space stabilises through asynchrony, which in a uniform, constant environment must be self-generated**. Spatial insurance, portfolio effects and the storage effect all need heterogeneity or forcing imposed from outside. Without them, what remains is out-of-phase local predator–prey dynamics under limited, asymmetric dispersal, plus the selection for restraint that viscosity can bring. Both are endogenous.

## ABM and computational literature

As elsewhere in this layer, these are findings about published computational systems, included because they illuminate the ecology ([README](README.md), *On ABM and computational literature*).

### Artificial-life worlds survived by scaffolding

The long-lived artificial-life ecosystems combined an imposed bound on population with external renewal of the limiting resource. Tierra used a "reaper" that culls when memory reaches 80 %, plus cosmic-ray mutation that prevents immortality ([Ray 1991](https://faculty.cc.gatech.edu/~turk/bio_sim/articles/tierra_thomas_ray.pdf)). Polyworld regrew food in patches and topped its population up when it fell below a minimum, according to designer descriptions ([Polyworld README](https://github.com/joyhughes/polyworld); [Yaeger & Sporns 2008](http://users.sussex.ac.uk/~inmanh/adsys10/Readings/Yaeger_Sporns_-_2008_-_Evolution_of_neural_structure_and_complexity_in_a_computational_ecology_-_ALProc.pdf)). EcoSim regrew its grass and imposed age caps and a complexity tax on energy ([Gras et al. 2009, *Artif. Life*](https://strathprints.strath.ac.uk/90261/1/Gras-etal-AL-2009-An-individual-based-evolving-predator-prey-ecosystem-simulation.pdf)).

**This scaffolding rules out collapse by construction, so the persistence these systems report says little about how robust their dynamics really are.** Their characteristic failure was stasis rather than death ([Taylor et al. 2016, *Artif. Life*](https://publications.aston.ac.uk/id/eprint/44173/1/artl_a_00210.pdf)).

Nutrient recycling through decomposers is almost absent from classic artificial life, which renews resources from outside. No artificial-life paper was found that reports what fraction of its parameter settings is viable.

Diversity in Avida peaks at intermediate resource availability ([Chow et al. 2004, *Science*](https://www.science.org/doi/10.1126/science.1096307)). Mass conservation in Flow-Lenia *helps* persistent, localised structure to form ([Plantec et al. 2023](https://www.researchgate.net/publication/374078600_Flow-Lenia_Towards_open-ended_evolution_in_cellular_automata_through_mass_conservation_and_parameter_localization)), so conservation of matter does not by itself prevent persistence.

*Evidence quality.* These results are mostly qualitative and come from single systems, often single runs. The Polyworld top-up is anecdotal, documented through a re-creation's README. No controlled ablations of scaffolding were found.

### How robustness is measured

**Basin stability** is the fraction of random initial conditions that end in a given attractor. It complements linear stability, which says nothing about how large a basin is ([Menck et al. 2013, *Nat. Phys.*](https://abdn.elsevierpure.com/en/publications/how-basin-stability-complements-the-linear-stability-paradigm/); [Mitra et al. 2017](https://arxiv.org/pdf/1612.06015)).

The **feasibility domain** is the volume of parameter space in which all species have positive equilibrium abundances; its size is a structural measure of how much perturbation a community tolerates ([Rohr, Saavedra & Bascompte 2014, *Science*](https://www.unifr.ch/bio/en/assets/public/Research/Rudolf-Rohr/2014_Science_Rohr_et_al.pdf)). No published guidance was found on how to size the parameter neighbourhood for individual-based models.

**Permanence** — the extinction boundary as a repeller — is tested empirically by invasion: remove a species, let the community settle, reintroduce it rare and read its early per-capita growth ([Law & Morton 1996, *Ecology*](https://pure.york.ac.uk/portal/en/publications/permanence-and-the-assembly-of-ecological-communities); [Hofbauer & Schreiber 2022, *J. Math. Biol.*](https://link.springer.com/article/10.1007/s00285-022-01815-2)).

For individual-based models, one-factor-at-a-time sweeps exposed collapse thresholds in 11 of 15 parameters of one model ([ten Broeke et al. 2016, *JASSS*](https://jasss.soc.surrey.ac.uk/19/1/5.html)). Structural robustness analysis re-runs a model under variant rules to see which results survive a change of submodel ([Grimm & Berger 2016, *Ecol. Model.*](https://econpapers.repec.org/RePEc:eee:ecomod:v:326:y:2016:i:c:p:162-167)).

## Consolidated evidence caveats

The source reading for this document hit widespread paywalls, so many claims rest on less than a full text.

The following rest on **abstracts, landing pages or search summaries** rather than full texts: Williams and Martinez 2004, Rall et al. 2008, Allesina and Tang 2012, Brose et al. 2003 and 2006, Otto et al. 2007, DeAngelis et al. 1989, Daufresne and Loreau 2001, Cherif and Loreau 2007, Rooney et al. 2006, Loreau et al. 2003, Ferrière and Legendre 2013, Briggs and Hoopes 2004, Austin and Vivanco 2006, Huisman and Weissing 1999, and Descamps-Julien and Gonzalez 2005.

The following are cited from **bibliographic knowledge without re-reading**: Holling 1959 and 1973, Pimm 1984, May 1972, Rosenzweig 1971, Murdoch and Oaten 1975, de Ruiter et al. 1995, McCann and Hastings 1997, Polis and Strong 1996, Yodzis and Innes 1992, Cohen 1966, Venable 2007, Kerr et al. 2006, Webb 2003, Hairston et al. 2005, and the panarchy phase model.

Several **specific numbers are recalled and unconfirmed**: the Hill exponent of 1.2 as the "small dose" of type III, the specific result of Heckmann et al. 2012, the inverse scaling of resilience with residence time in DeAngelis, and Fox's list of three mechanisms. A critical-ratio figure (C:N ≈ 25–35) attributed to Manzoni et al. 2008 by a summary tool could not be found in the text and is not used.

**Gaps in the literature.** No source was found that treats donor control in a fully closed nutrient system, type III responses in spatial individual-based models with discrete prey, metacommunity persistence on a continuous space with evolving dispersal, or basin stability in evolving individual-based ecosystems. Statements in this document that reach into those settings are inferences, and are marked as such.
