// For now the sampler will just be a modified wavetable lol
// TODO: More mips
// FIXME: Crash when playing note and changing sample. Dropping mutex too early or running the check for new sample too late or removing sample data too early?
// FIXME: finding mip should take base_pitch into account, lest we get aliasing

const N_VOICES: usize = 8;
use crate::parameters::OnOff;
use crate::parameters::SamplerParams;
use crate::utils::AtomicOps;
use crate::Voice;

mod fir;

#[inline]
pub fn calc_relative_pitch(note: f32, root: f32) -> f32 {
    2f32.powf((note - root) / 12.)
}
pub struct Sampler<'a> {
    data: SampleInterp<'a>,
    // phase: f32,
    base_pitch: f32,
    standard_pitch: f32,
    pub sample_rate: f32,
    ratios: [f32; N_VOICES],
    // phase_incs: [f32; N_VOICES],
    current_mip: [usize; N_VOICES],
    // for one-shotting the sample
    pub is_done: [bool; N_VOICES],
}

impl<'a> Sampler<'a> {
    pub fn new() -> Sampler<'a> {
        Sampler {
            data: SampleInterp::default(),
            standard_pitch: 21.533203125 * 2.,
            base_pitch: 1.,
            // phase: 0.,
            sample_rate: 44100.,
            // phase_incs: [0.; N_VOICES],
            ratios: [0.; N_VOICES],
            current_mip: [0; N_VOICES],
            is_done: [true; N_VOICES],
        }
    }
    // TODO: Divide total_pitch by 2.powi(current_mip) most likely, if get_sample doesn't use pitch anywhere else
    pub fn pitch_params_changed(&mut self, params: &SamplerParams, voices: &Vec<Voice>) {
        for i in 0..N_VOICES {
            let pitch = if params.keytrack.value() == OnOff::On {
                calc_relative_pitch(
                    params.coarse_tune.value + params.fine_tune.value + voices[i].current_notepitch, // + voice.current_note as f64,
                    // TODO: sample should be able to pitchbend
                    // + self.pitchbend * self.params.pitchbend_amt.get() as f32,
                    params.root.value as f32,
                )
            } else {
                calc_relative_pitch(
                    params.coarse_tune.value + params.fine_tune.value + params.root.value as f32, // + voice.current_note as f64,
                    params.root.value as f32,
                )
            };
            let mut pitch_mut = pitch * self.base_pitch;
            let mut current_mip = 0;
            // let phase_wrapped = (self.phase_wraparound(*phase + phase_mod));,
            // TODO: Hmm, base_pitch should be used to select mip map maybe?
            // let ratio = (pitch) / self.standard_pitch;
            while pitch_mut > 2. {
                pitch_mut /= 2.;
                current_mip += 1;
            }
            self.ratios[i] = pitch_mut;
            self.current_mip[i] = current_mip;
        }
    }
    pub fn voice_pitch_changed(&mut self, params: &SamplerParams, voices: &Vec<Voice>, idx: usize) {
        let pitch = if params.keytrack.value() == OnOff::On {
            calc_relative_pitch(
                params.coarse_tune.value + params.fine_tune.value + voices[idx].current_notepitch, // + voice.current_note as f64,
                // TODO: sample should be able to pitchbend
                // + self.pitchbend * self.params.pitchbend_amt.get() as f32,
                params.root.value as f32,
            )
        } else {
            calc_relative_pitch(
                params.coarse_tune.value + params.fine_tune.value + params.root.value as f32, // + voice.current_note as f64,
                params.root.value as f32,
            )
        };
        let mut pitch_mut = pitch * self.base_pitch;
        // println!("pitch: {}", pitch_mut);
        let mut current_mip = 0;
        // let phase_wrapped = (self.phase_wraparound(*phase + phase_mod));,
        // TODO: Hmm, base_pitch should be used to select mip map maybe?
        // let ratio = (pitch) / self.standard_pitch;
        while pitch_mut > 2. {
            pitch_mut /= 2.;
            current_mip += 1;
        }
        self.ratios[idx] = pitch_mut;
        self.current_mip[idx] = current_mip;
    }
    pub fn process(&mut self, voice_n: usize, phase: &mut f32, params: &SamplerParams) -> [f32; 2] {
        if params.is_on.value() == OnOff::Off || self.is_done[voice_n] {
            [0.; 2]
        } else {
            self.get_sample(voice_n, phase, params)
        }
    }

    pub fn get_sample(
        &mut self,
        voice_n: usize,
        phase: &mut f32,
        params: &SamplerParams,
    ) -> [f32; 2] {
        let current_mip = self.current_mip[voice_n];
        let ratio = self.ratios[voice_n];
        // let pitch = self.total_pitches[voice_n];
        let osc = &self.data;
        // let params = &self.params;
        // used to do the phase modulation
        let max_phase = (osc.len) as f32;
        // mip_offset moves us to the right mip-map
        let mip_offset = mip_offset(current_mip, osc.len);
        // ratio also needs to take into account that higher mips have half as many samples:
        // let downsample_ratio = 2usize.pow(current_mip as u32);
        // somewhat faster than the line above
        let downsample_ratio = 1 << current_mip;

        // ratio, or 1/sample_pitch give us how many samples we need to iterate forward
        // should never be more than 2 because of the mip mapping
        // downsampling phase to the right mip
        let phase_mipped = ((*phase + params.pos.value) * max_phase) / downsample_ratio as f32;
        // index gets us to the right polynomium, z lets us find the right sample value at the polynomium.
        let index = phase_mipped.floor() as usize + mip_offset;

        // x is kind of like time in between the samples and in the range [0,1], and therefore the fraction part of the phase
        // z is x - 0.5, which is basically a "coefficient offset" on the polynomial interpolation matrix.
        // this offset saves a few multiplications
        let z = phase_mipped - phase_mipped.floor() - 0.5;
        let vol = params.volume.value;
        let output = if params.sample_mono.get() {
            let output = ((osc.c3_l[index] * z + osc.c2_l[index]) * z + osc.c1_l[index]) * z
                + osc.c0_l[index];
            [output * vol, output * vol]
        } else {
            [
                (((osc.c3_l[index] * z + osc.c2_l[index]) * z + osc.c1_l[index]) * z
                    + osc.c0_l[index])
                    * vol,
                (((osc.c3_r[index] * z + osc.c2_r[index]) * z + osc.c1_r[index]) * z
                    + osc.c0_r[index])
                    * vol,
            ]
        };
        // TODO: The downsample_ratio stuff could prolly be precalc'd
        *phase += ratio * downsample_ratio as f32 / max_phase;
        // if the voice's phase moves outside of the correct wave, loop back around to the start
        while *phase + params.pos.value > 1. {
            *phase -= 1. - params.pos.value;
            if params.is_looping.value() == OnOff::Off {
                self.is_done[voice_n] = true;
            }
        }
        output
    }
    // For offline processing like BillyDM wanted, since it's probably a waste of time to precompute coefficients in that case
    pub fn _get_sample_no_precompute(
        &mut self,
        voice_n: usize,
        phase: &mut f32,
        params: &SamplerParams,
    ) -> [f32; 2] {
        let current_mip = self.current_mip[voice_n];
        let ratio = self.ratios[voice_n];
        // let pitch = self.total_pitches[voice_n];
        let osc = &self.data;
        // let params = &self.params;
        // used to do the phase modulation
        let max_phase = (osc.len) as f32;
        // mip_offset moves us to the right mip-map
        // ratio also needs to take into account that higher mips have half as many samples:
        // let downsample_ratio = 2usize.pow(current_mip as u32);
        // somewhat faster than the line above
        let downsample_ratio = 1 << current_mip;

        // ratio, or 1/sample_pitch give us how many samples we need to iterate forward
        // should never be more than 2 because of the mip mapping
        // downsampling phase to the right mip
        let phase_mipped = (*phase * max_phase) / downsample_ratio as f32;
        // index gets us to the right polynomium, z lets us find the right sample value at the polynomium.
        let index = phase_mipped.floor() as usize
            + (params.pos.value * osc.len as f32) as usize / downsample_ratio;

        // x is kind of like time in between the samples and in the range [0,1], and therefore the fraction part of the phase
        // z is x - 0.5, which is basically a "coefficient offset" on the polynomial interpolation matrix.
        // this offset saves a few multiplications
        let z = phase_mipped - phase_mipped.floor() - 0.5;
        let vol = params.volume.value;
        let output = if params.sample_mono.get() {
            // TODO: should use mips
            let y = &self.data.mips_l[current_mip];
            // TODO: "Bounds" checking. Gets a bit complicated with mips
            let even1;
            let odd1;
            let even2;
            let odd2;
            // make sure the end of mips are handled properly
            let current_mip_len = y.len();
            if index + 2 > current_mip_len {
                even1 = y[index + current_mip_len - 1] + y[index];
                odd1 = y[index + 1] - y[index];
                even2 = y[index + 2] + y[index - 1];
                odd2 = y[index + 2] + y[index - 1];
            } else {
                even1 = y[(index + 1).rem_euclid(current_mip_len)] + y[index];
                odd1 = y[(index + 1).rem_euclid(current_mip_len)] - y[index];
                even2 = y[(index + 2).rem_euclid(current_mip_len)]
                    + y[(index - 1).rem_euclid(current_mip_len)];
                odd2 = y[(index + 2).rem_euclid(current_mip_len)]
                    + y[(index - 1).rem_euclid(current_mip_len)];
            }
            let c0 = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            let c1 = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            let c2 = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            let c3 = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;

            let output = ((c3 * z + c2) * z + c1) * z + c0;
            [output * vol, output * vol]
        } else {
            [
                (((osc.c3_l[index] * z + osc.c2_l[index]) * z + osc.c1_l[index]) * z
                    + osc.c0_l[index])
                    * vol,
                (((osc.c3_r[index] * z + osc.c2_r[index]) * z + osc.c1_r[index]) * z
                    + osc.c0_r[index])
                    * vol,
            ]
        };
        // TODO: The downsample_ratio stuff could prolly be precalc'd
        *phase += ratio * downsample_ratio as f32 / max_phase;
        // if the voice's phase moves outside of the correct wave, loop back around to the start
        while *phase + params.pos.value > 1. {
            *phase -= 1. - params.pos.value;
        }
        output
    }

    // sets the sampler up with the new sample
    pub fn source_changed(&mut self, params: &SamplerParams) {
        // println!("source changed!");
        // new table data
        let source = params.source.read().unwrap();
        // println!("source len {}", source.len());
        // move to oscillator
        // self.oscs[osc_n].source_y = source.to_vec();
        if params.sample_mono.get() {
            self.data.source_l = source.to_vec();
            if self.data.source_l.len() % 2 != 0 {
                self.data.source_l.push(0.);
            }
            // drop mutex as soon as possible
            drop(source);
            self.set_base_pitch(params);
            self.standard_pitch = self.sample_rate * 2. / self.data.len as f32;
            // run the WaveTable's setup function to complete preprocessing
            self.data.setup_mono(true);
        }
        // TODO: Test if this is right
        else {
            // for (i, val) in source.iter().enumerate().map(|x|, (x.0,x)).step_by(2) {
            self.data.source_l.clear();
            self.data.source_r.clear();
            for i in (0..source.len()).step_by(2) {
                self.data.source_l.push(source[i]);
                self.data.source_r.push(source[i + 1]);
            }
            // attempt at fixing things by padding
            if self.data.source_l.len() % 2 != 0 {
                self.data.source_l.push(0.);
                self.data.source_r.push(0.);
            }
            // drop mutex as soon as possible
            drop(source);
            self.set_base_pitch(params);
            // run the WaveTable's setup function to complete preprocessing
            self.standard_pitch = self.sample_rate * 2. / self.data.len as f32;
            self.data.setup_stereo(true);
            // assert!(self.data.source_l.len() == self.data.source_r.len(), "somehow the 2 channels are different lengths");
        }
        // println!("sample len: {}", source.len());
    }
    pub fn set_base_pitch(&mut self, params: &SamplerParams) {
        let rate_of_sample = params.sample_sample_rate.get();
        // high sample rate has too many samples - iterate through it faster
        // times 2 because of internal oversampling
        self.base_pitch = rate_of_sample / (self.sample_rate * 2.);
    }
    pub fn set_standard_pitch(&mut self, rate: f32) {
        self.sample_rate = rate;
        self.standard_pitch = self.sample_rate * 2. / self.data.len as f32;
    }
}

pub struct SampleInterp<'a> {
    pub source_l: Vec<f32>,
    pub source_r: Vec<f32>,
    /// the number of waveforms in the current wavetable
    pub(crate) len: usize,

    mips_l: Vec<Vec<f32>>,
    mips_r: Vec<Vec<f32>>,

    pub c0_l: Vec<f32>,
    pub c1_l: Vec<f32>,
    pub c2_l: Vec<f32>,
    pub c3_l: Vec<f32>,

    pub c0_r: Vec<f32>,
    pub c1_r: Vec<f32>,
    pub c2_r: Vec<f32>,
    pub c3_r: Vec<f32>,
    downsample_fir: &'a [f32],
    mip_levels: usize,
}

impl<'a> SampleInterp<'a> {
    // prepares coeffs after loading new table
    pub fn setup_mono(&mut self, precompute: bool) {
        self.len = self.source_l.len();
        // self.oversample(2);
        let mips = self.mip_map(self.source_l.clone(), true, !precompute);
        if precompute {
            self.optimal_coeffs(&mips, true);
        }
    }
    pub fn setup_stereo(&mut self, precompute: bool) {
        self.len = self.source_l.len();
        self.len = self.source_l.len();
        // self.oversample(2);
        let mips_l = self.mip_map(self.source_l.clone(), true, !precompute);
        let mips_r = self.mip_map(self.source_r.clone(), false, !precompute);
        if precompute {
            self.optimal_coeffs(&mips_l, true);
            self.optimal_coeffs(&mips_r, false);
        }
    }
    fn downsample_2x(&self, signal: &[f32]) -> Vec<f32> {
        let mut temp = vec![0.];
        temp.resize(signal.len() / 2, 0.);
        // then we remove every second sample
        for j in 0..(signal.len() / 2) {
            temp[j] = signal[j * 2];
        }
        // let output = self.convolve_single_cycle(self.downsample_fir, &temp);
        self.convolve_circular(self.downsample_fir, &temp)
    }
    fn mip_map(&mut self, source: Vec<f32>, left: bool, save_mips: bool) -> Vec<f32> {
        if save_mips {
            if left {
                self.mips_l = vec![vec![]; self.mip_levels];
            } else {
                self.mips_r = vec![vec![]; self.mip_levels];
            }
        }
        // fill first layer with self.source_y
        let len = source.len();
        let mut temp: Vec<f32> = source;
        // fills the mip_levels with continually more downsampled vectors
        for j in 0..self.mip_levels {
            let mut temp2 = self.downsample_2x(&(&temp[0 + mip_offset(j, len)..temp.len()]));
            if save_mips {
                if left {
                    self.mips_l[j] = temp2.clone();
                } else {
                    self.mips_r[j] = temp2.clone();
                }
            }
            temp.append(&mut temp2);
        }
        temp
    }
    // TODO: Simd-optimize
    // make a slice 4 long, maybe?
    fn optimal_coeffs(&mut self, mips: &[f32], left: bool) {
        let len = self.len;
        // let new_wave_len = self.wave_len;
        let mut even1: f32;
        let mut even2: f32;
        let mut odd1: f32;
        let mut odd2: f32;
        let c0;
        let c1;
        let c2;
        let c3;
        if left {
            c0 = &mut self.c0_l;
            c1 = &mut self.c1_l;
            c2 = &mut self.c2_l;
            c3 = &mut self.c3_l;
        } else {
            c0 = &mut self.c0_r;
            c1 = &mut self.c1_r;
            c2 = &mut self.c2_r;
            c3 = &mut self.c3_r;
        }

        c0.resize(self.source_l.len() * 2, 0.);
        c1.resize(self.source_l.len() * 2, 0.);
        c2.resize(self.source_l.len() * 2, 0.);
        c3.resize(self.source_l.len() * 2, 0.);
        for _n in 0..self.mip_levels {
            //n represents mip-map levels
            for j in 1..self.len / 2usize.pow(_n as u32) - 2 {
                // let i = _i * self.wave_len / 2usize.pow(_n as u32);
                let n = mip_offset(_n, len);

                even1 = mips[n + j + 1] + mips[n + j];
                odd1 = mips[n + j + 1] - mips[n + j];
                even2 = mips[n + j + 2] + mips[n + j - 1];
                odd2 = mips[n + j + 2] - mips[n + j - 1];
                c0[n + j] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
                c1[n + j] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
                c2[n + j] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
                c3[n + j] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
            }
        }
        // makes sure the start of waveforms are handled properly
        for _n in 0..self.mip_levels {
            let j = self.len / 2usize.pow(_n as u32);

            // let i = _i * self.wave_len / 2usize.pow(_n as u32);
            let n = mip_offset(_n, len);
            even1 = mips[n + 1] + mips[n];
            odd1 = mips[n + 1] - mips[n];
            even2 = mips[n + 2] + mips[n + j - 1];
            odd2 = mips[n + 2] - mips[n + j - 1];
            c0[n] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            c1[n] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            c2[n] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            c3[n] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
        }
        // makes sure the end of waveforms are handled properly
        for _n in 0..self.mip_levels {
            let j = self.len / 2usize.pow(_n as u32);
            let n = mip_offset(_n, len);
            even1 = mips[n + j - 1] + mips[n + j - 2];
            odd1 = mips[n + j - 1] - mips[n + j - 2];
            even2 = mips[n] + mips[n + j - 3];
            odd2 = mips[n] - mips[n + j - 3];
            c0[n + j - 2] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            c1[n + j - 2] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            c2[n + j - 2] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            c3[n + j - 2] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;

            even1 = mips[n] + mips[n + j - 1];
            odd1 = mips[n] - mips[n + j - 1];
            even2 = mips[n + 1] + mips[n + j - 2];
            odd2 = mips[n + 1] - mips[n + j - 2];
            c0[n + j - 1] = even1 * 0.45868970870461956 + even2 * 0.04131401926395584;
            c1[n + j - 1] = odd1 * 0.48068024766578432 + odd2 * 0.17577925564495955;
            c2[n + j - 1] = even1 * -0.246185007019907091 + even2 * 0.24614027139700284;
            c3[n + j - 1] = odd1 * -0.36030925263849456 + odd2 * 0.10174985775982505;
        }
    }

    // doesn't *really* need to be circular, but better safe than sorry
    fn convolve_circular(&self, p_coeffs: &[f32], p_in: &[f32]) -> Vec<f32> {
        let h = p_coeffs;
        let x = p_in;
        let new_len = x.len() + (h.len() - 1) / 2;
        // let new_len = (p_coeffs.len() - 1) / 2 + p_coeffs.len();
        // let mut temp2 = vec![0.; x.len()+ h.len()];
        let mut convolved = vec![0.; new_len];
        // outer loop, per sample in output
        for n in (h.len() - 1) / 2..new_len {
            // for n in 0..x.len() {
            for m in 0..h.len() {
                convolved[n] +=
                    h[m] * x[((n as i32 - m as i32).rem_euclid(p_in.len() as i32)) as usize];
            }
        }
        //trimming group delay
        let mut truncated: Vec<f32>;
        truncated = convolved.split_off((p_coeffs.len() - 1) / 2);
        // trimming empty stuff at the end
        truncated.truncate(p_in.len());
        truncated
    }
}
impl<'a> Default for SampleInterp<'a> {
    fn default() -> SampleInterp<'a> {
        Self {
            // allocating a decent chunk of mem for source_y to avoid having to re-allocate later
            source_l: Vec::with_capacity(2048 * 8),
            source_r: Vec::with_capacity(2048 * 8),
            mips_l: Vec::with_capacity(2048 * 8),
            mips_r: Vec::with_capacity(2048 * 8),
            mip_levels: 9,
            len: 0,
            c0_l: Vec::with_capacity(2048 * 8 * 2),
            c1_l: Vec::with_capacity(2048 * 8 * 2),
            c2_l: Vec::with_capacity(2048 * 8 * 2),
            c3_l: Vec::with_capacity(2048 * 8 * 2),
            c0_r: Vec::with_capacity(2048 * 8 * 2),
            c1_r: Vec::with_capacity(2048 * 8 * 2),
            c2_r: Vec::with_capacity(2048 * 8 * 2),
            c3_r: Vec::with_capacity(2048 * 8 * 2),
            downsample_fir: &fir::DOWNSAMPLE_FIR,
        }
    }
}

// implements the recurrence relation "g(n) = 1 + 0.5*g(n-1), g(0)=0" but without having to calc it
// this recurrence relation describes how each mip mapped cycle is half as long as the previous
#[inline]
pub fn mip_offset(mip: usize, len: usize) -> usize {
    let amount = match mip {
        0 => 0.,
        1 => 1.,
        2 => 1.5,
        3 => 1.75,
        4 => 1.875,
        5 => 1.9375,
        6 => 1.96875,
        7 => 1.984375,
        8 => 1.9921875,
        9 => 1.99609375,
        10 => 1.998046875,
        _ => 0.,
    };
    (len as f32 * amount) as usize
}
// Implements the same recurrence relation, but without branching for potential simd optimization
pub fn _mip_offset2(mip: usize, len: usize) -> usize {
    let factor = (2 << (mip - 1)) as f32;
    // let factor = 2f32.powi(mip as i32 - 1);
    let amount = 1. + (factor - 1.) / factor;
    (len as f32 * amount) as usize
}
