#[derive(Clone)]
pub struct Voice {
    pub is_on: bool,
    pub sampler_phase: f32,
    pub current_note: u8,
    // these 2 are in midi notes, not in Hertz
    pub current_notepitch: f32,
    pub target_notepitch: f32,
    // how much current_notepitch should be increased with, if relevant
    pub increment: f32,
    // pub filter: ModellingFilter,
}
impl Voice {
    pub fn new() -> Voice {
        // let filter = ModellingFilter::new(params.filter_p.clone());
        Voice {
            is_on: false,
            current_note: 0,
            current_notepitch: 0.,
            target_notepitch: 0.,
            increment: 0.,
            sampler_phase: 0.,
            // phases: [f32x8::splat(0.); 4],
        }
    }
    pub fn update_pitch(&mut self) -> bool {
        if
        /*self.increment == 0. ||*/
        self.current_notepitch == self.target_notepitch {
            false
        } else {
            // TODO: When should the current_notepitch stop updating?
            if self.increment > 0. {
                self.current_notepitch = (self.current_notepitch + self.increment)
                    .clamp(self.current_notepitch, self.target_notepitch);
            } else {
                self.current_notepitch = (self.current_notepitch + self.increment)
                    .clamp(self.target_notepitch, self.current_notepitch);
            }
            true
        }
    }

    pub fn release(&mut self) {
        self.is_on = false;
    }
    // fn set_pitch(&self, new_pitch)
}
