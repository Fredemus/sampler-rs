use std::{fs, io, path::PathBuf};
const I24_MAX: i32 = 2_i32.pow(23) - 1;
// use dirs;
use hound::{self, WavReader};
use lazy_static::lazy_static;
use std::fmt;

const FOLDER_PATH: &str = r"Documents/sampler-rs/";

// SampleName is just a wrapper around a PathBuf, so we can use it with Vizia (needs to impl a Data trait to be lensed)
#[derive(Clone, Debug)]
pub struct SampleName(pub PathBuf);

impl SampleName {
    //
    pub fn load_sample(self) -> Result<Sample, hound::Error> {
        let path = dirs::home_dir()
            .unwrap()
            .join(PathBuf::from(FOLDER_PATH).join("samples").join(self.0));
        let mut reader = WavReader::open(path)?;
        let spec = reader.spec();
        if spec.bits_per_sample == 32 {
            let read = reader.samples().collect::<Result<Vec<f32>, hound::Error>>();
            if let Ok(samples) = read {
                // samples.push(spec.sample_rate as f32);
                return Ok(Sample {
                    data: samples,
                    sample_rate: spec.sample_rate as f32,
                    channels: spec.channels as usize,
                });
            } else {
                return Err(hound::Error::FormatError("Failed reading samples"));
            }
        } else if spec.bits_per_sample == 16 {
            let read = reader.samples().collect::<Result<Vec<i16>, hound::Error>>();
            if let Ok(samples) = read {
                let float_samples: Vec<f32> = samples
                    .iter()
                    .map(|val| *val as f32 / i16::MAX as f32)
                    .collect();
                // float_samples.push(spec.sample_rate as f32);
                return Ok(Sample {
                    data: float_samples,
                    sample_rate: spec.sample_rate as f32,
                    channels: spec.channels as usize,
                });
            } else {
                return Err(hound::Error::FormatError("Failed reading samples"));
            }
        } else if spec.bits_per_sample == 24 {
            let read = reader.samples().collect::<Result<Vec<i32>, hound::Error>>();
            if let Ok(samples) = read {
                // println!("max val: {}", samples.iter().max().unwrap());
                let float_samples: Vec<f32> = samples
                    .iter()
                    .map(|val| *val as f32 / I24_MAX as f32)
                    .collect();
                // float_samples.push(spec.sample_rate as f32);
                return Ok(Sample {
                    data: float_samples,
                    sample_rate: spec.sample_rate as f32,
                    channels: spec.channels as usize,
                });
            } else {
                return Err(hound::Error::FormatError("Failed reading samples"));
            }
        } else {
            return Err(hound::Error::FormatError("unsupported sample type"));
        }
    }
}
/// Helper struct to share details about the data from the wav file
pub struct Sample {
    pub data: Vec<f32>,
    pub channels: usize,
    pub sample_rate: f32,
}

pub fn samples() -> io::Result<Vec<SampleName>> {
    SAMPLES
        .iter()
        .map(|builtin| Ok(builtin.clone()))
        // .chain(preset_table_dir_tables())
        .map(|path_buf| path_buf.map(SampleName))
        .collect()
}

lazy_static! {
    static ref SAMPLES: Vec<PathBuf> = {
        macro_rules! _include_folder {
            ( $path:expr ) => {{
                let abs_path = dirs::home_dir().unwrap().join(FOLDER_PATH).join($path);
                let mut table_vec: Vec<PathBuf> = Vec::new();
                for entry in fs::read_dir(abs_path).unwrap() {
                    let entry = entry.unwrap();
                    let pathy = PathBuf::from(entry.path().file_name().unwrap());
                    table_vec.push(pathy);
                }
                table_vec
            }};
        }
        _include_folder!("samples")
    };
}

impl fmt::Display for SampleName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut string = format!("{:?}", self.0.file_stem().unwrap());
        string = string.replace('"', "");
        write!(f, "{}", string)
    }
}

#[test]
fn print_samples() {
    let tables = samples().unwrap();
    println!("{:#?}", tables);
}
