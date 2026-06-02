/// An endpoint in a nutrient flow: a pool that nutrient moves between.
///
/// Unlike energy, nutrient is a *closed* resource — it cycles between pools
/// rather than entering from a tap or leaving via a drain (per CONTEXT.md:
/// "nutrients are conserved — they cycle between pools rather than flowing
/// from source to sink"). There is therefore no analogue of SolarTap or
/// Dissipation here.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NutrientEndpoint {
    /// A grid cell holding available substrate nutrient, identified by cell index.
    Grid(usize),
    /// A living agent's internal nutrient store, identified by id.
    Agent(u64),
    /// A carcass's retained nutrient, identified by id.
    Carcass(u64),
    /// Pre-existing nutrient carried into the tick (initial endowment).
    Endowment,
    /// Nutrient retained across all pools at tick end. This is a pure sink
    /// used to reconcile the closed-system balance: every unit endowed at tick
    /// start must end up retained, with nothing created or destroyed.
    Retained,
}

/// Total nutrient held by each pool category at a moment in time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolTotals {
    /// Sum of nutrient across all grid cells (available substrate).
    pub grid: f32,
    /// Sum of nutrient across all living agents.
    pub agents: f32,
    /// Sum of nutrient across all carcasses.
    pub carcasses: f32,
}

/// Records every nutrient flow as a (source, destination, amount) triple.
///
/// Enforces the closed-system invariant: nutrient is neither created nor
/// destroyed, only moved between pools (grid cells, agents, carcasses).
pub struct NutrientLedger {
    flows: Vec<(NutrientEndpoint, NutrientEndpoint, f32)>,
}

impl NutrientLedger {
    pub fn new() -> Self {
        Self { flows: Vec::new() }
    }

    /// Record a nutrient flow from source to destination.
    pub fn record(&mut self, source: NutrientEndpoint, destination: NutrientEndpoint, amount: f32) {
        self.flows.push((source, destination, amount));
    }

    /// Returns all recorded flows as (source, destination, amount) triples.
    pub fn flows(&self) -> &[(NutrientEndpoint, NutrientEndpoint, f32)] {
        &self.flows
    }

    /// Total nutrient received by this endpoint (sum of all inflows).
    pub fn net_received(&self, endpoint: &NutrientEndpoint) -> f32 {
        self.flows
            .iter()
            .filter(|(_, dest, _)| dest == endpoint)
            .map(|&(_, _, amount)| amount)
            .sum()
    }

    /// Total nutrient sent by this endpoint (sum of all outflows).
    pub fn net_sent(&self, endpoint: &NutrientEndpoint) -> f32 {
        self.flows
            .iter()
            .filter(|(source, _, _)| source == endpoint)
            .map(|&(_, _, amount)| amount)
            .sum()
    }

    /// Clear all recorded flows (for reuse across ticks).
    pub fn clear(&mut self) {
        self.flows.clear();
    }

    /// Build the tick's flows from pre- and post-tick nutrient totals for the
    /// three pool categories (grid, agents, carcasses).
    ///
    /// Each category is endowed with its pre-tick nutrient (`Endowment` →
    /// category) and routes its post-tick nutrient into the `Retained` sink
    /// (category → `Retained`). For a closed system every category's net
    /// balance is `pre - post`, and the global balance reduces to
    /// `total_endowed == total_retained`. If nutrient was created or destroyed
    /// during the tick the totals differ and `assert_balanced` panics.
    ///
    /// Clears any previously recorded flows first.
    pub fn build_from_pool_totals(&mut self, pre: PoolTotals, post: PoolTotals) {
        self.clear();

        let categories = [
            (NutrientEndpoint::Grid(0), pre.grid, post.grid),
            (NutrientEndpoint::Agent(0), pre.agents, post.agents),
            (NutrientEndpoint::Carcass(0), pre.carcasses, post.carcasses),
        ];
        // Record every category's pre as endowment and post as retained, whatever
        // the sign. Gating on `> 0.0` (as an earlier version did) silently dropped
        // a pool whenever it crossed zero within the tick — counting its pre in
        // `endowed` but not its post in `retained` (or vice versa) — which
        // manufactured a phantom imbalance equal to the crossing pool's magnitude.
        // That artifact was invisible until the detrital pathway began emptying
        // carcass/grid pools to ~0 (issue #303). The conservation check must read
        // the true signed pool totals so it reflects physics, not the sign of an
        // f32-noisy near-zero pool.
        for (endpoint, pre_amount, post_amount) in categories {
            if pre_amount != 0.0 {
                self.record(NutrientEndpoint::Endowment, endpoint.clone(), pre_amount);
            }
            if post_amount != 0.0 {
                self.record(endpoint, NutrientEndpoint::Retained, post_amount);
            }
        }
    }

    /// Verify the closed-system nutrient invariant.
    ///
    /// Checks that:
    /// 1. No nutrient flows INTO the Endowment (it is a pure source).
    /// 2. No nutrient flows OUT of the Retained sink (it is a pure sink).
    /// 3. Global conservation: the nutrient endowed at tick start equals the
    ///    nutrient retained across all pools at tick end (nothing created or
    ///    destroyed).
    ///
    /// Panics if the ledger is imbalanced.
    pub fn assert_balanced(&self) {
        // Endowment is a source — no nutrient may flow into it.
        let endowment_inflow = self.net_received(&NutrientEndpoint::Endowment);
        assert!(
            endowment_inflow <= f32::EPSILON,
            "nutrient ledger imbalanced: Endowment received {endowment_inflow} \
             (nutrient flowing INTO the endowment)"
        );

        // Retained is a sink — no nutrient may flow out of it.
        let retained_outflow = self.net_sent(&NutrientEndpoint::Retained);
        assert!(
            retained_outflow <= f32::EPSILON,
            "nutrient ledger imbalanced: Retained sent {retained_outflow} \
             (nutrient flowing OUT of the retained sink)"
        );

        // Global conservation: total endowed at tick start == total retained at
        // tick end. Any net creation or destruction breaks this equality.
        let total_endowed = self.net_sent(&NutrientEndpoint::Endowment);
        let total_retained = self.net_received(&NutrientEndpoint::Retained);
        let diff = (total_endowed - total_retained).abs();
        // Scale tolerance with magnitude — f32 has ~7 digits of precision. Since
        // embodiment (ADR-0003) makes the bound portion of each agent's nutrient
        // a *recomputed* quantity (`structure × demand`), the per-tick totals now
        // carry the same f32 accumulation error as `structure` itself. We match
        // the energy ledger's 1e-3 relative tolerance, which sums structure the
        // same way; the absolute drift remains a vanishing fraction of the total.
        let scale = total_endowed.abs().max(1.0);
        let tolerance = scale * 1e-3;
        assert!(
            diff < tolerance,
            "nutrient ledger imbalanced: endowed={total_endowed}, \
             retained={total_retained}, diff={diff}, tolerance={tolerance}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_is_balanced() {
        let ledger = NutrientLedger::new();
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "nutrient flowing INTO the endowment")]
    fn nutrient_flowing_into_endowment_panics() {
        let mut ledger = NutrientLedger::new();
        ledger.record(NutrientEndpoint::Agent(1), NutrientEndpoint::Endowment, 5.0);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "nutrient flowing OUT of the retained sink")]
    fn nutrient_flowing_out_of_retained_panics() {
        let mut ledger = NutrientLedger::new();
        ledger.record(NutrientEndpoint::Retained, NutrientEndpoint::Agent(1), 5.0);
        ledger.assert_balanced();
    }

    #[test]
    fn build_from_conserved_pool_totals_is_balanced() {
        let mut ledger = NutrientLedger::new();
        // 10 nutrient moves from grid into agents; total unchanged (37 -> 37).
        let pre = PoolTotals {
            grid: 30.0,
            agents: 5.0,
            carcasses: 2.0,
        };
        let post = PoolTotals {
            grid: 20.0,
            agents: 15.0,
            carcasses: 2.0,
        };
        ledger.build_from_pool_totals(pre, post);
        ledger.assert_balanced();
    }

    #[test]
    fn build_from_full_cycle_uptake_death_decompose_is_balanced() {
        let mut ledger = NutrientLedger::new();
        // Grid -> agents (uptake), agents -> carcasses (death), and a carcass
        // partially decomposes back to grid. Total conserved (50 -> 50).
        let pre = PoolTotals {
            grid: 40.0,
            agents: 8.0,
            carcasses: 2.0,
        };
        let post = PoolTotals {
            grid: 35.0,
            agents: 6.0,
            carcasses: 9.0,
        };
        ledger.build_from_pool_totals(pre, post);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "nutrient ledger imbalanced")]
    fn build_from_created_nutrient_panics() {
        let mut ledger = NutrientLedger::new();
        // Post total (40) exceeds pre total (37): nutrient created from nowhere.
        // No pool loses, so a naive delta-shuffle would miss this — the
        // Endowment/Retained reconciliation catches it.
        let pre = PoolTotals {
            grid: 30.0,
            agents: 5.0,
            carcasses: 2.0,
        };
        let post = PoolTotals {
            grid: 30.0,
            agents: 8.0,
            carcasses: 2.0,
        };
        ledger.build_from_pool_totals(pre, post);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "nutrient ledger imbalanced")]
    fn build_from_destroyed_nutrient_panics() {
        let mut ledger = NutrientLedger::new();
        // Post total (32) is less than pre total (37): nutrient vanished.
        let pre = PoolTotals {
            grid: 30.0,
            agents: 5.0,
            carcasses: 2.0,
        };
        let post = PoolTotals {
            grid: 25.0,
            agents: 5.0,
            carcasses: 2.0,
        };
        ledger.build_from_pool_totals(pre, post);
        ledger.assert_balanced();
    }
}
