use std::collections::HashMap;

/// An endpoint in an energy flow: where energy comes from or goes to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnergyEndpoint {
    /// Solar flux — the sole source of energy entering the system.
    SolarTap,
    /// A living agent, identified by id.
    Agent(u64),
    /// A dead agent retaining energy, identified by id.
    Carcass(u64),
    /// Energy leaving the system (metabolic cost, transfer loss).
    Dissipation,
    /// Pre-existing energy carried into the tick (initial endowment).
    Endowment,
}

/// Records every energy flow as a (source, destination, amount) triple.
///
/// Enforces the open-system invariant: solar input is the sole tap,
/// metabolic cost and trophic transfer loss are the drains.
pub struct EnergyLedger {
    flows: Vec<(EnergyEndpoint, EnergyEndpoint, f32)>,
}

impl EnergyLedger {
    pub fn new() -> Self {
        Self { flows: Vec::new() }
    }

    /// Record an energy flow from source to destination.
    pub fn record(&mut self, source: EnergyEndpoint, destination: EnergyEndpoint, amount: f32) {
        self.flows.push((source, destination, amount));
    }

    /// Returns all recorded flows as (source, destination, amount) triples.
    pub fn flows(&self) -> &[(EnergyEndpoint, EnergyEndpoint, f32)] {
        &self.flows
    }

    /// Total energy received by this endpoint (sum of all inflows).
    pub fn net_received(&self, endpoint: &EnergyEndpoint) -> f32 {
        self.flows
            .iter()
            .filter(|(_, dest, _)| dest == endpoint)
            .map(|&(_, _, amount)| amount)
            .sum()
    }

    /// Total energy sent by this endpoint (sum of all outflows).
    pub fn net_sent(&self, endpoint: &EnergyEndpoint) -> f32 {
        self.flows
            .iter()
            .filter(|(source, _, _)| source == endpoint)
            .map(|&(_, _, amount)| amount)
            .sum()
    }

    /// Total energy entering the system from SolarTap.
    pub fn total_solar_input(&self) -> f32 {
        self.net_sent(&EnergyEndpoint::SolarTap)
    }

    /// Total energy leaving the system via Dissipation.
    pub fn total_dissipated(&self) -> f32 {
        self.net_received(&EnergyEndpoint::Dissipation)
    }

    /// Clear all recorded flows (for reuse across ticks).
    pub fn clear(&mut self) {
        self.flows.clear();
    }

    /// Verify the open-system energy invariant.
    ///
    /// Checks that:
    /// 1. No energy flows INTO SolarTap or Endowment.
    /// 2. No energy flows OUT of Dissipation.
    /// 3. No agent or carcass endpoint has a negative net balance
    ///    (cannot spend energy it never received).
    /// 4. Total input (solar + endowment) equals total dissipation plus
    ///    energy retained by agents and carcasses.
    ///
    /// Panics if the ledger is imbalanced.
    pub fn assert_balanced(&self) {
        // Compute net balance per endpoint
        let mut balances: HashMap<EnergyEndpoint, f32> = HashMap::new();
        for (source, destination, amount) in &self.flows {
            *balances.entry(source.clone()).or_default() -= amount;
            *balances.entry(destination.clone()).or_default() += amount;
        }

        // SolarTap is a source — no energy may flow into it
        let solar_inflow: f32 = self
            .flows
            .iter()
            .filter(|&(_, dest, _)| *dest == EnergyEndpoint::SolarTap)
            .map(|&(_, _, amount)| amount)
            .sum();
        assert!(
            solar_inflow <= f32::EPSILON,
            "energy ledger imbalanced: SolarTap received {solar_inflow} \
             (energy flowing INTO the tap)"
        );

        // Endowment is a source — no energy may flow into it
        let endowment_inflow: f32 = self
            .flows
            .iter()
            .filter(|&(_, dest, _)| *dest == EnergyEndpoint::Endowment)
            .map(|&(_, _, amount)| amount)
            .sum();
        assert!(
            endowment_inflow <= f32::EPSILON,
            "energy ledger imbalanced: Endowment received {endowment_inflow} \
             (energy flowing INTO the endowment)"
        );

        // Dissipation should have non-negative balance (it only absorbs energy)
        let dissipation_balance = *balances.get(&EnergyEndpoint::Dissipation).unwrap_or(&0.0);
        assert!(
            dissipation_balance >= -f32::EPSILON,
            "energy ledger imbalanced: Dissipation has negative balance {dissipation_balance} \
             (energy flowing OUT of dissipation)"
        );

        // No agent or carcass should have negative balance (within floating-point tolerance)
        for (endpoint, balance) in &balances {
            let tolerance = match endpoint {
                EnergyEndpoint::Agent(_) | EnergyEndpoint::Carcass(_) => {
                    // Scale tolerance by total throughput for this endpoint
                    let throughput: f32 = self
                        .flows
                        .iter()
                        .filter(|(_, d, _)| d == endpoint)
                        .map(|&(_, _, a)| a.abs())
                        .sum::<f32>()
                        .max(1.0);
                    throughput * 1e-4
                }
                _ => continue,
            };
            match endpoint {
                EnergyEndpoint::Agent(id) => {
                    assert!(
                        *balance >= -tolerance,
                        "energy ledger imbalanced: Agent({id}) has negative balance {balance}"
                    );
                }
                EnergyEndpoint::Carcass(id) => {
                    assert!(
                        *balance >= -tolerance,
                        "energy ledger imbalanced: Carcass({id}) has negative balance {balance}"
                    );
                }
                _ => {}
            }
        }

        // Global conservation: input == dissipated + retained
        let solar_balance = *balances.get(&EnergyEndpoint::SolarTap).unwrap_or(&0.0);
        let endowment_balance = *balances.get(&EnergyEndpoint::Endowment).unwrap_or(&0.0);
        let total_in = -solar_balance - endowment_balance;
        let total_dissipated = dissipation_balance;
        let retained: f32 = balances
            .iter()
            .filter(|(ep, _)| matches!(ep, EnergyEndpoint::Agent(_) | EnergyEndpoint::Carcass(_)))
            .map(|(_, b)| b)
            .sum();
        let diff = (total_in - total_dissipated - retained).abs();
        // Scale tolerance with magnitude — f32 has ~7 digits of precision
        let scale = total_in.abs().max(1.0);
        let tolerance = scale * 1e-4;
        assert!(
            diff < tolerance,
            "energy ledger imbalanced: input={total_in}, dissipated={total_dissipated}, \
             retained={retained}, diff={diff}, tolerance={tolerance}"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger_is_balanced() {
        let ledger = EnergyLedger::new();
        ledger.assert_balanced();
    }

    #[test]
    fn balanced_solar_to_agent_to_dissipation() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 10.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 10.0);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "energy ledger imbalanced")]
    fn agent_spending_more_than_received_panics() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 5.0);
        // Agent dissipates more than it received
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 10.0);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "energy flowing INTO the tap")]
    fn energy_flowing_into_solar_tap_panics() {
        let mut ledger = EnergyLedger::new();
        // Energy cannot flow back into the solar tap
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::SolarTap, 5.0);
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 5.0);
        ledger.assert_balanced();
    }

    #[test]
    #[should_panic(expected = "energy flowing OUT of dissipation")]
    fn energy_flowing_out_of_dissipation_panics() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::Dissipation, EnergyEndpoint::Agent(1), 5.0);
        ledger.assert_balanced();
    }

    #[test]
    fn net_received_sums_inflows_for_agent() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 10.0);
        ledger.record(EnergyEndpoint::Agent(2), EnergyEndpoint::Agent(1), 5.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 3.0);
        assert!((ledger.net_received(&EnergyEndpoint::Agent(1)) - 15.0).abs() < 1e-5);
    }

    #[test]
    fn net_sent_sums_outflows_for_agent() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 10.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 3.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Agent(2), 2.0);
        assert!((ledger.net_sent(&EnergyEndpoint::Agent(1)) - 5.0).abs() < 1e-5);
    }

    #[test]
    fn total_solar_input_sums_all_solar_outflows() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 10.0);
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(2), 7.0);
        assert!((ledger.total_solar_input() - 17.0).abs() < 1e-5);
    }

    #[test]
    fn total_dissipated_sums_all_dissipation_inflows() {
        let mut ledger = EnergyLedger::new();
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 10.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 4.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 6.0);
        assert!((ledger.total_dissipated() - 10.0).abs() < 1e-5);
    }

    #[test]
    fn full_trophic_chain_with_lossy_transfers() {
        let mut ledger = EnergyLedger::new();
        // Solar flux enters producer (agent 1)
        ledger.record(EnergyEndpoint::SolarTap, EnergyEndpoint::Agent(1), 100.0);
        // Producer pays metabolic cost
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 20.0);
        // Consumer (agent 2) consumes producer — lossy transfer
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Agent(2), 40.0);
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Dissipation, 10.0); // transfer loss
        // Producer dies, becomes carcass with remaining energy
        ledger.record(EnergyEndpoint::Agent(1), EnergyEndpoint::Carcass(1), 30.0);
        // Decomposer (agent 3) scavenges carcass — lossy transfer
        ledger.record(EnergyEndpoint::Carcass(1), EnergyEndpoint::Agent(3), 15.0);
        ledger.record(
            EnergyEndpoint::Carcass(1),
            EnergyEndpoint::Dissipation,
            15.0,
        ); // transfer loss
        // Consumer and decomposer pay metabolic costs
        ledger.record(EnergyEndpoint::Agent(2), EnergyEndpoint::Dissipation, 40.0);
        ledger.record(EnergyEndpoint::Agent(3), EnergyEndpoint::Dissipation, 15.0);
        // All energy accounted for: 100 solar = 20+10+15+40+15 dissipated = 100
        ledger.assert_balanced();
        assert_eq!(ledger.flows().len(), 9);
    }
}
