use std::sync::{Arc, RwLock};

use crate::{resources::SampleName, utils::*};
use nih_plug::prelude::*;

use std::path::PathBuf;
const SMOOTHING_TIME: f32 = 20.;
#[derive(Params)]
pub struct SamplerParams {
    // #[persist = "sample_name"]
    pub sample_name: RwLock<SampleName>,

    #[id = "mono"]
    pub mono: EnumParam<OnOff>,

    pub pitch_changed: Arc<AtomicBool>,
    // TODO: Octave should be some kind of signed integer
    // pub octave: ParameterUsize,
    #[id = "sampler root"]
    pub root: IntParam,
    #[id = "sampler fine tune"]
    pub fine_tune: FloatParam,
    #[id = "sampler coarse tune"]
    pub coarse_tune: FloatParam,
    #[id = "sampler volume"]
    pub volume: FloatParam,
    #[id = "sampler pan"]
    pub pan: FloatParam,

    #[id = "sampler start position"]
    pub pos: FloatParam,
    #[id = "sampler end position"]
    pub end: FloatParam,
    #[id = "sampler looping"]
    pub is_looping: EnumParam<OnOff>,
    // these 2 are used for interpolation
    // TODO: Find a way not to have this? Possibly unnecessary copy
    pub source: RwLock<Vec<f32>>,
    pub source_changed: AtomicBool,
    #[id = "sampler on/off"]
    pub is_on: EnumParam<OnOff>,
    /// sample rate of the current sample
    pub sample_sample_rate: AtomicF32,
    pub sample_mono: AtomicBool,

    #[id = "sampler keytrack"]
    pub keytrack: EnumParam<OnOff>,
}
impl SamplerParams {
    pub(crate) fn get_sample(&self) -> Vec<f32> {
        let wave = self.source.read().unwrap().to_vec();
        wave
    }
    pub fn get_sample_name(&self) -> String {
        let tables = self.sample_name.read().unwrap();
        format!("{}", tables)
    }
    pub fn load_sample(&self) {
        let sample_name = &*self.sample_name.read().unwrap();
        let result = sample_name.clone().load_sample();
        let source_y;
        let sample_rate;
        let n_channels;
        // let mut error = self.error.lock().unwrap();
        match result {
            Ok(sample) => {
                // dbg!(&x[0]);
                // dbg!(&x[x.len() - 1]);
                sample_rate = sample.sample_rate;
                n_channels = sample.channels;
                if sample_rate == 0. {
                    println!("Error in sample loading: Sample rate is 0");
                    // *error = Some(x.to_string());
                    return;
                }
                // The sampler needs to know the original sample rate of the sample, so it can perform the needed correction
                self.sample_sample_rate.set(sample_rate);
                // self.sampler_p.sample_mono.set(sample.channels == 1);
                source_y = sample.data;
            }
            Err(_x) => {
                println!("Error in sample loading: {sample_name}. Couldn't find file or unsupported format: {_x}");
                // *error = Some(x.to_string());
                return;
            }
        }
        *self.source.write().unwrap() = source_y.clone();
        // A mono sample
        if n_channels == 1 {
            self.sample_mono.set(true);
        } else if n_channels == 2 {
            self.sample_mono.set(false);
        } else {
            println!("Too many channels in sample: {sample_name}");
            // *error = Some(x.to_string());
            return;
        }
    }
}
impl Default for SamplerParams {
    fn default() -> Self {
        let pitch_changed = Arc::new(AtomicBool::new(false));
        let a = Self {
            sample_name: RwLock::new(SampleName(PathBuf::from("Hard kick 1.wav"))),
            mono: EnumParam::new("Mono", OnOff::Off),
            pitch_changed: pitch_changed.clone(),
            sample_sample_rate: AtomicF32::new(0.),

            root: IntParam::new("Sampler Root", 60, IntRange::Linear { min: 0, max: 127 })
                .with_value_to_string(formatters::v2s_i32_note_formatter())
                .with_callback(Arc::new({
                    let pitch_changed = pitch_changed.clone();
                    move |_| pitch_changed.set_release(true)
                })),
            fine_tune: FloatParam::new("Fine tune", 0.0, FloatRange::Linear { min: -1., max: 1. })
                .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
                .with_value_to_string(formatters::v2s_f32_percentage(1))
                .with_callback(Arc::new({
                    let pitch_changed = pitch_changed.clone();
                    move |_| pitch_changed.set_release(true)
                })),
            coarse_tune: FloatParam::new(
                "Coarse tune",
                0.0,
                FloatRange::Linear {
                    min: -24.,
                    max: 24.,
                },
            )
            .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
            .with_value_to_string(formatters::v2s_f32_rounded(1))
            .with_callback(Arc::new({
                let pitch_changed = pitch_changed.clone();
                move |_| pitch_changed.set_release(true)
            })),

            volume: FloatParam::new("Volume", 0.5, FloatRange::Linear { min: 0., max: 1. })
                .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
                .with_unit(" dB")
                .with_value_to_string(formatters::v2s_f32_gain_to_db(2)),

            pan: FloatParam::new("Pan", 0.0, FloatRange::Linear { min: -1., max: 1. })
                .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
                .with_value_to_string(formatters::v2s_f32_panning()),

            pos: FloatParam::new(
                "Sampler Pos",
                0.0,
                FloatRange::Linear { min: 0., max: 0.99 },
            )
            .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
            .with_value_to_string(formatters::v2s_f32_rounded(2)),
            end: FloatParam::new("Sampler End", 0.0, FloatRange::Linear { min: 0., max: 1. })
                .with_smoother(SmoothingStyle::Linear(SMOOTHING_TIME))
                .with_value_to_string(formatters::v2s_f32_rounded(2)),

            keytrack: EnumParam::new("Sampler Keytrack", OnOff::Off).with_callback(Arc::new({
                let pitch_changed = pitch_changed.clone();
                move |_| pitch_changed.set_release(true)
            })),

            source: RwLock::new(vec![0.; 2048]),
            source_changed: AtomicBool::new(false),
            is_on: EnumParam::new("Sampler On/off", OnOff::On),
            sample_mono: AtomicBool::new(true),
            is_looping: EnumParam::new("Sampler Loop", OnOff::Off),
        };
        a.load_sample();
        a
    }
}

#[derive(Enum, Debug, PartialEq)]
pub enum OnOff {
    Off,
    On,
}
