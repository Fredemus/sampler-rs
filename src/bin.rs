#![feature(portable_simd)]
use std::sync::Arc;

use nih_plug::context::GuiContext;
use vizia::{Application, WindowDescription};

mod parameters;
use parameters::SamplerParams;
// use crate::parameter::*;
mod editor;
mod resources;
pub mod utils;

mod ui;
use crate::editor::{WINDOW_HEIGHT, WINDOW_WIDTH};
use ui::*;

fn main() {
    let params = Arc::new(SamplerParams::default());
    // let state = Arc::new(EditorState::new(params.clone(), None));
    let param_set_guy = Arc::new(ParamSetGuy {});
    let window_description = WindowDescription::new()
        .with_inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
        .with_title("Sampler-rs")
        .with_always_on_top(true);

    Application::new(window_description, move |cx| {
        cx.add_stylesheet("src/style.css")
            .expect("no style sheet found");
        // plugin_gui(cx, Arc::clone(&params));
        plugin_gui(cx, Arc::clone(&params), param_set_guy.clone());
    })
    .run();
}
// dummmy GuiContext for the standalone version
struct ParamSetGuy();

impl GuiContext for ParamSetGuy {
    unsafe fn raw_begin_set_parameter(&self, _param: nih_plug::param::internals::ParamPtr) {}

    unsafe fn raw_set_parameter_normalized(
        &self,
        param: nih_plug::param::internals::ParamPtr,
        normalized: f32,
    ) {
        param.set_normalized_value(normalized);
    }

    unsafe fn raw_end_set_parameter(&self, _param: nih_plug::param::internals::ParamPtr) {}

    fn request_resize(&self) -> bool {
        todo!()
    }

    fn get_state(&self) -> nih_plug::prelude::PluginState {
        todo!()
    }

    fn set_state(&self, _state: nih_plug::prelude::PluginState) {
        // todo!()
    }
}
