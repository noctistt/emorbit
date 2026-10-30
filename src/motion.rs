//! Small finite transitions: no scheduler or timer is needed once settled.
#[derive(Clone, Copy, Debug, Default)]
pub struct Transition {
    pub value: f32,
    from: f32,
    target: f32,
    elapsed: f32,
    duration: f32,
}

impl Transition {
    pub fn target(&mut self, target: f32, duration: f32) {
        if self.target == target {
            return;
        }
        self.from = self.value;
        self.target = target;
        self.elapsed = 0.0;
        self.duration = duration.max(0.001);
    }

    pub fn reset(&mut self, value: f32) {
        *self = Self {
            value,
            from: value,
            target: value,
            ..Self::default()
        };
    }

    pub fn snap(&mut self) {
        self.value = self.target;
        self.elapsed = self.duration;
    }

    pub fn active(&self) -> bool {
        self.value != self.target
    }

    pub fn advance(&mut self, seconds: f32) {
        if !self.active() {
            return;
        }
        self.elapsed = (self.elapsed + seconds.max(0.0)).min(self.duration);
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let ease = t * t * (3.0 - 2.0 * t);
        self.value = self.from + (self.target - self.from) * ease;
        if self.elapsed >= self.duration {
            self.snap();
        }
    }
}

#[derive(Default)]
pub struct Motion {
    pub items: [Transition; 8],
    pub variants: [Transition; 6],
    pub search: Transition,
    pub expansion: Transition,
}

impl Motion {
    fn transitions(&mut self) -> impl Iterator<Item = &mut Transition> {
        self.items
            .iter_mut()
            .chain(self.variants.iter_mut())
            .chain([&mut self.search, &mut self.expansion])
    }

    pub fn active(&self) -> bool {
        self.items
            .iter()
            .chain(self.variants.iter())
            .chain([&self.search, &self.expansion])
            .any(Transition::active)
    }

    pub fn advance(&mut self, seconds: f32) {
        for transition in self.transitions() {
            transition.advance(seconds);
        }
    }

    pub fn snap(&mut self) {
        for transition in self.transitions() {
            transition.snap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn highlight_retargeting_is_continuous_and_settles_without_overshoot() {
        let mut t = Transition::default();
        t.target(1.0, 0.14);
        t.advance(0.05);
        let before = t.value;
        assert!(before > 0.0 && before < 1.0);
        t.target(0.0, 0.14);
        assert_eq!(t.value, before);
        for _ in 0..20 {
            t.advance(0.016);
            assert!((0.0..=before).contains(&t.value));
        }
        assert_eq!(t.value, 0.0);
        assert!(!t.active());
    }

    #[test]
    fn reduced_motion_and_delayed_frames_finish_exactly() {
        let mut motion = Motion::default();
        motion.items[3].target(1.0, 0.14);
        motion.expansion.target(1.0, 0.18);
        assert!(motion.active());
        motion.snap();
        assert!(!motion.active());
        assert_eq!(motion.items[3].value, 1.0);
        motion.items[3].target(0.0, 0.14);
        motion.advance(5.0);
        assert!(!motion.active());
    }
}
