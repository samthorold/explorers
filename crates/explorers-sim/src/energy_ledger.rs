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

    /// Verify the open-system energy invariant.
    ///
    /// Checks that:
    /// 1. Total solar input equals total dissipation plus energy retained
    ///    by agents and carcasses.
    /// 2. No agent or carcass endpoint has a negative net balance
    ///    (cannot spend energy it never received).
    ///
    /// Panics if the ledger is imbalanced.
    pub fn assert_balanced(&self) {
        // Compute net balance per endpoint
        let mut balances: HashMap<EnergyEndpoint, f32> = HashMap::new();
        for (source, destination, amount) in &self.flows {
            *balances.entry(source.clone()).or_default() -= amount;
            *balances.entry(destination.clone()).or_default() += amount;
        }

        // SolarTap is the sole source — no energy may flow into it
        let solar_inflow: f32 = self.flows.iter()
            .filter(|&(_, dest, _)| *dest == EnergyEndpoint::SolarTap)
            .map(|&(_, _, amount)| amount)
            .sum();
        assert!(
            solar_inflow <= f32::EPSILON,
            "energy ledger imbalanced: SolarTap received {solar_inflow} \
             (energy flowing INTO the tap)"
        );

        // Dissipation should have non-negative balance (it only absorbs energy)
        let dissipation_balance = *balances.get(&EnergyEndpoint::Dissipation).unwrap_or(&0.0);
        assert!(
            dissipation_balance >= -f32::EPSILON,
            "energy ledger imbalanced: Dissipation has negative balance {dissipation_balance} \
             (energy flowing OUT of dissipation)"
        );

        // No agent or carcass should have negative balance
        for (endpoint, balance) in &balances {
            match endpoint {
                EnergyEndpoint::Agent(id) => {
                    assert!(
                        *balance >= -f32::EPSILON,
                        "energy ledger imbalanced: Agent({id}) has negative balance {balance}"
                    );
                }
                EnergyEndpoint::Carcass(id) => {
                    assert!(
                        *balance >= -f32::EPSILON,
                        "energy ledger imbalanced: Carcass({id}) has negative balance {balance}"
                    );
                }
                _ => {}
            }
        }

        // Global conservation: solar_in == dissipated + retained
        let solar_balance = *balances.get(&EnergyEndpoint::SolarTap).unwrap_or(&0.0);
        let total_in = -solar_balance; // solar emits, so its balance is negative
        let total_dissipated = dissipation_balance;
        let retained: f32 = balances.iter()
            .filter(|(ep, _)| matches!(ep, EnergyEndpoint::Agent(_) | EnergyEndpoint::Carcass(_)))
            .map(|(_, b)| b)
            .sum();
        let diff = (total_in - total_dissipated - retained).abs();
        assert!(
            diff < 1e-4,
            "energy ledger imbalanced: solar_input={total_in}, dissipated={total_dissipated}, \
             retained={retained}, diff={diff}"
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
        ledger.record(EnergyEndpoint::Carcass(1), EnergyEndpoint::Dissipation, 15.0); // transfer loss
        // Consumer and decomposer pay metabolic costs
        ledger.record(EnergyEndpoint::Agent(2), EnergyEndpoint::Dissipation, 40.0);
        ledger.record(EnergyEndpoint::Agent(3), EnergyEndpoint::Dissipation, 15.0);
        // All energy accounted for: 100 solar = 20+10+15+40+15 dissipated = 100
        ledger.assert_balanced();
        assert_eq!(ledger.flows().len(), 9);
    }
}
