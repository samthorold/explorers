pub struct TraitVector {
    pub values: [f32; 2],
}

pub struct Agent {
    pub position: (f32, f32),
    pub energy: f32,
    pub traits: TraitVector,
}

pub struct World {
    agents: Vec<Agent>,
}

impl World {
    pub fn new(agents: Vec<Agent>) -> Self {
        Self { agents }
    }

    pub fn step(&mut self) {}

    pub fn agents(&self) -> &[Agent] {
        &self.agents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_created_with_agents_exposes_them() {
        let agents = vec![
            Agent {
                position: (1.0, 2.0),
                energy: 100.0,
                traits: TraitVector { values: [0.5, 0.8] },
            },
            Agent {
                position: (3.0, 4.0),
                energy: 80.0,
                traits: TraitVector { values: [0.2, 0.9] },
            },
        ];
        let world = World::new(agents);
        assert_eq!(world.agents().len(), 2);
    }

    #[test]
    fn agents_retain_state_after_world_creation() {
        let world = World::new(vec![Agent {
            position: (5.0, 10.0),
            energy: 42.0,
            traits: TraitVector { values: [0.1, 0.7] },
        }]);
        let agent = &world.agents()[0];
        assert_eq!(agent.position, (5.0, 10.0));
        assert_eq!(agent.energy, 42.0);
        assert_eq!(agent.traits.values, [0.1, 0.7]);
    }

    #[test]
    fn step_does_not_panic() {
        let mut world = World::new(vec![Agent {
            position: (0.0, 0.0),
            energy: 50.0,
            traits: TraitVector { values: [0.5, 0.5] },
        }]);
        world.step();
    }

    #[test]
    fn genesis_step_n_times_agents_survive() {
        let mut world = World::new(vec![
            Agent {
                position: (0.0, 0.0),
                energy: 100.0,
                traits: TraitVector { values: [0.3, 0.6] },
            },
            Agent {
                position: (5.0, 5.0),
                energy: 100.0,
                traits: TraitVector { values: [0.7, 0.2] },
            },
        ]);
        for _ in 0..100 {
            world.step();
        }
        assert_eq!(world.agents().len(), 2);
        assert!(world.agents().iter().all(|a| a.energy > 0.0));
    }
}
