#![feature(portable_simd)]
use nih_plug::{nih_export_vst3, prelude::*};
use std::sync::Arc;
use voice::Voice;

mod editor;
mod parameters;
mod sampler;
pub mod utils;
use parameters::{OnOff, SamplerParams};
mod ui;

mod resources;

mod voice;

pub struct Plug<'a> {
    params: Arc<SamplerParams>,
    pub sampler: sampler::Sampler<'a>,

    sample_rate: f32,
    pressed_notes: Vec<u8>,
    voices: Vec<Voice>,
}

impl<'a> Plug<'a> {
    pub fn note_off(&mut self, note: u8) {
        // remove all copies of the note from pressed_notes
        self.pressed_notes.retain(|x| x != &note);
        for i in 0..self.voices.len() {
            if self.voices[i].current_note == note && self.voices[i].is_on {
                // if there are more notes left, toss one of them into the voice
                // TODO: If we switch to last_note dropping, we could probably extend this to working in poly mode
                if self.params.mono.value() == OnOff::On && !self.pressed_notes.is_empty() {
                    // TODO: It might be of interest to keep velocity with pressed_notes? Instead of using old velocity
                    // let old_velocity = (self.voices[i].mod_matrix.velocity * 127.) as u8;
                    self.trigger_voice(
                        self.pressed_notes[0],
                        // self.params.legato.value() == OnOff::On,
                        false,
                        0,
                    )
                }
                // if not, just release it
                else {
                    self.voices[i].release();
                }
                break;
            }
        }
    }
    pub fn note_on(&mut self, note: u8) {
        // for safety, remove note from pressed_notes if note is already there
        if let Some(pos) = self.pressed_notes.iter().position(|x| x == &note) {
            self.pressed_notes.swap_remove(pos);
        }
        // TODO: Does pressed_notes need to be a deque so we can push in front and pop from back?
        // would help with knowing which note to switch back to in case more than 8 are playing
        self.pressed_notes.push(note);
        // let legato = self.params.legato.value() == OnOff::On;
        let legato = false;
        // if the synth is in mono mode, just trigger the first voice
        if self.params.mono.value() == OnOff::On {
            self.trigger_voice(note, legato, 0);
        } else {
            for i in 0..self.voices.len() {
                // if self.voices[i].vol_env.output.is_none() {
                if !self.voices[i].is_on {
                    self.trigger_voice(note, legato, i);

                    // if this happens, we've found a free voice and can safely return
                    return;
                }
            }
            // Finding voice with the lowest current_note
            // self.voices.iter().enumerate().map(|x| x_current_note).min_by(|x, y| x.cmp(y)).unwrap()
            let mut lowest_note = 128;
            let mut voice_number = 50;

            for i in 0..self.voices.len() {
                if self.voices[i].current_note < lowest_note {
                    lowest_note = self.voices[i].current_note;
                    voice_number = i;
                }
            }
            self.trigger_voice(note, legato, voice_number);
        }
    }
    fn trigger_voice(&mut self, note: u8, legato: bool, voice_n: usize) {
        let voice = &mut self.voices[voice_n];
        if voice.is_on {
            // if voice is already playing and legato is on, don't restart envs
            if !legato {
                // voice.vol_env.trigger_env();
                // voice.mod_matrix.trigger(velocity);
            }
            // if voice is already playing, it should glide
            // since glide_time is gotten here instead of per sample,
            // changes only take effect to notes happeining after any change to glide_time
            voice.current_note = note;
            voice.target_notepitch = note as f32;
            // let glide_time = self.params.glide_time.value;
            let glide_time = 0.;
            // this formula finds an increment value fitting the glide time
            voice.increment = (voice.target_notepitch - voice.current_notepitch)
                / (glide_time * self.sample_rate * 2.) as f32;
        } else {
            // if voice was not on, just set pitch, trigger envelopes and reset phase
            voice.is_on = true;
            voice.current_note = note;
            voice.current_notepitch = note as f32;
            voice.target_notepitch = note as f32;
            voice.increment = 0.;
            // voice.vol_env.trigger_env();
            // voice.mod_matrix.trigger(velocity);
            self.voices[voice_n].sampler_phase = self.params.pos.value;
        }
        self.sampler
            .voice_pitch_changed(&self.params, &self.voices, voice_n);
    }
}

impl<'a> Default for Plug<'a> {
    fn default() -> Self {
        let params = Arc::new(SamplerParams::default());
        let mut sampler = sampler::Sampler::new();

        sampler.source_changed(&params);
        Self {
            sampler,
            params,
            sample_rate: 48000.,
            voices: vec![Voice::new(); 8],
            pressed_notes: vec![],
        }
    }
}

impl Plugin for Plug<'static> {
    const NAME: &'static str = "sampler-rs";
    const VENDOR: &'static str = "???";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = "0.0.1";

    const DEFAULT_NUM_INPUTS: u32 = 0;
    const DEFAULT_NUM_OUTPUTS: u32 = 2;

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    // const ACCEPTS_MIDI: bool = true;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&self) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();

        editor::create_vizia_editor(move |cx, context| {
            ui::plugin_gui(cx, params.clone(), context.clone());
        })
    }

    fn initialize(
        &mut self,
        _bus_config: &BusConfig,
        _buffer_config: &BufferConfig,
        _context: &mut impl ProcessContext,
    ) -> bool {
        let rate = _buffer_config.sample_rate;
        self.sampler.set_standard_pitch(rate);
        self.sample_rate = rate;
        true
    }

    fn process(&mut self, buffer: &mut Buffer, context: &mut impl ProcessContext) -> ProcessStatus {
        // use the param updates that has happened since last buffer
        if self.params.pitch_changed.check_reset() {
            self.sampler
                .pitch_params_changed(&self.params, &self.voices)
        }
        if self.params.source_changed.check_reset() {
            // load the data
            self.params.load_sample();
            // perform pre-processing
            self.sampler.source_changed(&self.params);
        }

        let mut next_event = context.next_event();

        for (sample_id, mut channel_samples) in buffer.iter_samples().enumerate() {
            'midi_events: loop {
                match next_event {
                    Some(event) if event.timing() == sample_id as u32 => match event {
                        NoteEvent::NoteOn { note, .. } => {
                            self.note_on(note);
                        }
                        NoteEvent::NoteOff { note, .. } => {
                            self.note_off(note);
                        }
                        _ => (),
                    },
                    _ => break 'midi_events,
                }

                next_event = context.next_event();
            }
            // if sample_id == midi_queue[0

            // self.engine.params.update_smootheds();
            let mut summed_output = [0.; 2];
            for i in 0..self.voices.len() {
                if self.voices[i].is_on {
                    let output =
                        self.sampler
                            .process(i, &mut self.voices[i].sampler_phase, &self.params);
                    summed_output[0] += output[0];
                    summed_output[1] += output[1];
                }
            }
            // let frame_out = *output.as_array();

            *channel_samples.get_mut(0).unwrap() = summed_output[0];
            *channel_samples.get_mut(1).unwrap() = summed_output[1];
        }

        ProcessStatus::Normal
    }

    fn initialize_block_smoothers(&mut self, max_block_size: usize) {
        for (_, mut param, _) in self.params().param_map() {
            unsafe { param.initialize_block_smoother(max_block_size) };
        }
    }
}
impl ClapPlugin for Plug<'static> {
    const CLAP_ID: &'static str = "com.rocket-physician.sampler-rs";
    const CLAP_DESCRIPTION: &'static str = "A basic resampling sampler";
    const CLAP_FEATURES: &'static [&'static str] = &["instrument", "mono", "stereo", "utility"];
    const CLAP_MANUAL_URL: &'static str = Self::URL;
    const CLAP_SUPPORT_URL: &'static str = Self::URL;
}
nih_export_clap!(Plug<'static>);
// Comment this in if you want a vst3
// impl Vst3Plugin for Plug<'static> {
//     const VST3_CLASS_ID: [u8; 16] = *b"Sampler-rs      ";
//     const VST3_CATEGORIES: &'static str = "Instrument|Synth";
// }

// nih_export_vst3!(Plug);
