# sampler-rs
A basic resampling sampler plugin.

## Build Instructions

To run the standalone GUI:
```bash
cargo run sampler --bin sampler_standalone
```

To build the the project as a clap plugin:
```bash
cargo xtask bundle sampler --release
```

## File instrutions
For the sampler to work the folder `dirs::home_dir()`/Documents/sampler-rs/samples/ must exist and have a file called `Hard kick 1.wav`
On windows this is equivalent to `C:\Users\USER_NAME\Documents\sampler-rs\samples`. See [dirs::home_dir()](https://docs.rs/dirs/1.0.4/dirs/fn.home_dir.html)
