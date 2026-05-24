use crate::event::{EventKind, EventLog};
use crate::toroidal_distance;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActivityLayer {
    Feeding,
    Death,
    Carcass,
}

pub struct SpatialProjection {
    cursor: usize,
    last_tick: u64,
    world_extent: f32,
    cell_size: f32,
    cols: usize,
    decay_rate: f32,
    feeding: Vec<f32>,
    death: Vec<f32>,
    carcass: Vec<f32>,
}

impl SpatialProjection {
    pub fn new(world_extent: f32, cell_size: f32, decay_rate: f32) -> Self {
        let cols = (world_extent / cell_size).ceil() as usize;
        let n = cols * cols;
        Self {
            cursor: 0,
            last_tick: 0,
            world_extent,
            cell_size,
            cols,
            decay_rate,
            feeding: vec![0.0; n],
            death: vec![0.0; n],
            carcass: vec![0.0; n],
        }
    }

    pub fn update(&mut self, log: &EventLog, current_tick: u64) {
        let ticks_elapsed = current_tick.saturating_sub(self.last_tick);
        if ticks_elapsed > 0 {
            let factor = (-self.decay_rate * ticks_elapsed as f32).exp();
            for v in self.feeding.iter_mut().chain(self.death.iter_mut()).chain(self.carcass.iter_mut()) {
                *v *= factor;
            }
        }

        for event in log.since(self.cursor) {
            if let Some(pos) = event.position {
                let idx = self.cell_index(pos);
                match event.kind {
                    EventKind::Consumed => self.feeding[idx] += event.energy_delta,
                    EventKind::Decomposed => self.feeding[idx] += event.energy_delta,
                    EventKind::Died => self.death[idx] += 1.0,
                    EventKind::CarcassCreated => self.carcass[idx] += event.energy_delta,
                    EventKind::CarcassDepleted => self.carcass[idx] -= event.energy_delta,
                    _ => {}
                }
            }
            self.cursor += 1;
        }

        self.last_tick = current_tick;
    }

    pub fn density(&self, position: (f32, f32), radius: f32, layer: ActivityLayer) -> f32 {
        let grid = self.layer(layer);
        let cells_to_check = (radius / self.cell_size).ceil() as isize + 1;
        let (center_col, center_row) = self.cell_coords(position);
        let mut total = 0.0;

        for dr in -cells_to_check..=cells_to_check {
            for dc in -cells_to_check..=cells_to_check {
                let row = (center_row as isize + dr).rem_euclid(self.cols as isize) as usize;
                let col = (center_col as isize + dc).rem_euclid(self.cols as isize) as usize;
                let cell_center = self.cell_center(col, row);
                if toroidal_distance(position, cell_center, self.world_extent) <= radius + self.cell_size {
                    total += grid[row * self.cols + col];
                }
            }
        }
        total
    }

    pub fn gradient(&self, position: (f32, f32), radius: f32, layer: ActivityLayer) -> (f32, f32) {
        let grid = self.layer(layer);
        let cells_to_check = (radius / self.cell_size).ceil() as isize + 1;
        let (center_col, center_row) = self.cell_coords(position);
        let mut gx = 0.0_f32;
        let mut gy = 0.0_f32;

        for dr in -cells_to_check..=cells_to_check {
            for dc in -cells_to_check..=cells_to_check {
                let row = (center_row as isize + dr).rem_euclid(self.cols as isize) as usize;
                let col = (center_col as isize + dc).rem_euclid(self.cols as isize) as usize;
                let cell_center = self.cell_center(col, row);
                let dist = toroidal_distance(position, cell_center, self.world_extent);
                if dist <= radius + self.cell_size && dist > 0.0 {
                    let weight = grid[row * self.cols + col];
                    let (dx, dy) = crate::toroidal_displacement(position, cell_center, self.world_extent);
                    gx += weight * dx / dist;
                    gy += weight * dy / dist;
                }
            }
        }
        (gx, gy)
    }

    fn layer(&self, layer: ActivityLayer) -> &[f32] {
        match layer {
            ActivityLayer::Feeding => &self.feeding,
            ActivityLayer::Death => &self.death,
            ActivityLayer::Carcass => &self.carcass,
        }
    }

    fn cell_coords(&self, position: (f32, f32)) -> (usize, usize) {
        let half = self.world_extent / 2.0;
        let x = position.0 + half;
        let y = position.1 + half;
        let col = ((x / self.cell_size) as usize).min(self.cols - 1);
        let row = ((y / self.cell_size) as usize).min(self.cols - 1);
        (col, row)
    }

    fn cell_center(&self, col: usize, row: usize) -> (f32, f32) {
        let half = self.world_extent / 2.0;
        let x = (col as f32 + 0.5) * self.cell_size - half;
        let y = (row as f32 + 0.5) * self.cell_size - half;
        (x, y)
    }

    fn cell_index(&self, position: (f32, f32)) -> usize {
        let (col, row) = self.cell_coords(position);
        row * self.cols + col
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Event;

    fn make_log(events: Vec<Event>) -> EventLog {
        let mut log = EventLog::new();
        for e in events {
            log.append(e).unwrap();
        }
        log
    }

    #[test]
    fn consumed_event_deposits_energy_on_feeding_layer() {
        let log = make_log(vec![Event {
            tick: 1,
            seq: 0,
            kind: EventKind::Consumed,
            source: 10,
            target: Some(20),
            energy_delta: 5.0,
            position: Some((0.0, 0.0)),
        }]);

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.5);
        proj.update(&log, 1);

        let d = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        assert!(d > 0.0, "feeding density should be positive after Consumed event");
        assert!((d - 5.0).abs() < 0.01, "feeding density should be ~5.0, got {d}");
    }

    #[test]
    fn density_decays_exponentially_over_ticks() {
        let log = make_log(vec![Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 10.0,
            position: Some((0.0, 0.0)),
        }]);

        let decay_rate = 0.5;
        let mut proj = SpatialProjection::new(100.0, 10.0, decay_rate);
        proj.update(&log, 1);

        let d1 = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        assert!((d1 - 10.0).abs() < 0.01);

        proj.update(&log, 2);
        let d2 = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        let expected = 10.0 * (-decay_rate * 1.0_f32).exp();
        assert!((d2 - expected).abs() < 0.01, "after 1 tick decay: expected {expected}, got {d2}");

        proj.update(&log, 4);
        let d4 = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        let expected = d2 * (-decay_rate * 2.0_f32).exp();
        assert!((d4 - expected).abs() < 0.01, "after 2 more ticks: expected {expected}, got {d4}");
    }

    #[test]
    fn separate_layers_are_independent() {
        let log = make_log(vec![Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 5.0,
            position: Some((0.0, 0.0)),
        }]);

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.5);
        proj.update(&log, 1);

        assert!(proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding) > 0.0);
        assert_eq!(proj.density((0.0, 0.0), 5.0, ActivityLayer::Death), 0.0);
        assert_eq!(proj.density((0.0, 0.0), 5.0, ActivityLayer::Carcass), 0.0);
    }

    #[test]
    fn died_deposits_on_death_layer_carcass_on_carcass_layer() {
        let log = make_log(vec![
            Event {
                tick: 1, seq: 0, kind: EventKind::Died,
                source: 10, target: None, energy_delta: 0.0,
                position: Some((5.0, 5.0)),
            },
            Event {
                tick: 1, seq: 1, kind: EventKind::CarcassCreated,
                source: 10, target: None, energy_delta: 20.0,
                position: Some((5.0, 5.0)),
            },
        ]);

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.5);
        proj.update(&log, 1);

        assert!(proj.density((5.0, 5.0), 5.0, ActivityLayer::Death) > 0.0);
        assert!(proj.density((5.0, 5.0), 5.0, ActivityLayer::Carcass) > 0.0);
        assert_eq!(proj.density((5.0, 5.0), 5.0, ActivityLayer::Feeding), 0.0);
    }

    #[test]
    fn gradient_points_toward_activity_concentration() {
        let log = make_log(vec![Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 100.0,
            position: Some((20.0, 0.0)),
        }]);

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.5);
        proj.update(&log, 1);

        let (gx, gy) = proj.gradient((0.0, 0.0), 30.0, ActivityLayer::Feeding);
        assert!(gx > 0.0, "gradient x should point toward activity at (20,0), got {gx}");
        assert!(gy.abs() < gx.abs(), "gradient y should be small relative to x");
    }

    #[test]
    fn incremental_update_processes_only_new_events() {
        let mut log = EventLog::new();
        log.append(Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 5.0,
            position: Some((0.0, 0.0)),
        }).unwrap();

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.0);
        proj.update(&log, 1);

        log.append(Event {
            tick: 2, seq: 1, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 3.0,
            position: Some((0.0, 0.0)),
        }).unwrap();

        proj.update(&log, 2);
        let d2 = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        assert!((d2 - 8.0).abs() < 0.01, "should accumulate both events: got {d2}");
    }

    #[test]
    fn density_approaches_zero_after_many_ticks_without_events() {
        let log = make_log(vec![Event {
            tick: 1, seq: 0, kind: EventKind::Consumed,
            source: 10, target: Some(20), energy_delta: 100.0,
            position: Some((0.0, 0.0)),
        }]);

        let mut proj = SpatialProjection::new(100.0, 10.0, 0.5);
        proj.update(&log, 1);
        proj.update(&log, 100);

        let d = proj.density((0.0, 0.0), 5.0, ActivityLayer::Feeding);
        assert!(d < 1e-10, "density should approach zero, got {d}");
    }
}
