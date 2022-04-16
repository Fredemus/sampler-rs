use std::sync::Arc;

use nih_plug::context::GuiContext;
use nih_plug::param::internals::ParamPtr;
use nih_plug::prelude::Param;
use vizia::*;

use crate::parameters::SamplerParams;
use crate::resources::samples;
use crate::resources::SampleName;
mod plot;

const ICON_DOWN_OPEN: &str = "\u{e75c}";
mod sampler;

pub fn plugin_gui(cx: &mut Context, params: Arc<SamplerParams>, context: Arc<dyn GuiContext>) {
    UiData {
        gui_context: context.clone(),
        params: params.clone(),
        // presets: vec![],
        samples: samples().unwrap(),
    }
    .build(cx);
    VStack::new(cx, |cx| {
        sampler::draw_sampler_gui(cx);
    })
    .class("container")
    .row_between(Pixels(0.))
    .child_top(Pixels(0.));
}

fn make_knob<'a, P, F>(
    cx: &'a mut Context,
    name: &str,
    centered: bool,
    param_ptr: nih_plug::param::internals::ParamPtr,
    params_to_param: F,
) -> Handle<'a, VStack>
where
    P: Param,
    F: 'static + Fn(&Arc<SamplerParams>) -> &P + Copy,
    // L: Lens<Target = ParamPtr>,
{
    // let context = UiData::gui_context.get(cx);
    // let setter = ParamSetter::new(context.as_ref());
    // create binding in the context for whether to render the parameter value
    VStack::new(cx, move |cx| {
        if cx.data::<LabelData>().is_none() {
            LabelData { visible: false }.build(cx);
        }
        // Binding::new(cx, UiData::params, move |cx, state| {
        // Label::new(cx, &state.get(cx).params.get_parameter_name(param_index))
        Label::new(cx, name).class("small_label");
        Knob::custom(
            cx,
            params_to_param(&*UiData::params.get(cx)).default_normalized_value(),
            // params.get(cx).get_parameter(param_index),
            UiData::params.map(move |params| params_to_param(params).normalized_value()),
            move |cx, lens| {
                TickKnob::new(
                    cx,
                    Percentage(80.0),
                    // Percentage(20.0),
                    Pixels(2.),
                    Percentage(80.0),
                    270.0,
                    KnobMode::Continuous,
                )
                .value(lens.clone())
                .class("tick");
                ArcTrack::new(
                    cx,
                    centered,
                    Percentage(100.0),
                    Percentage(10.),
                    270.,
                    KnobMode::Continuous,
                )
                .value(lens)
                .class("track")
            },
        )
        .on_changing(move |cx, val| {
            cx.emit(UiEvent::AllParams(param_ptr, val));
        })
        .on_press(move |cx| {
            cx.emit(LabelEvent::Show(true));
            cx.emit(UiEvent::BeginSet(param_ptr));
        })
        .on_release(move |cx| {
            cx.emit(LabelEvent::Show(false));
            cx.emit(UiEvent::EndSet(param_ptr));
        });
        Binding::new(cx, LabelData::visible, move |cx, visible| {
            let vis = *visible.get(cx);
            HStack::new(cx, move |cx| {
                Label::new(
                    cx,
                    UiData::params.map(move |params| params_to_param(params).name().to_owned()),
                )
                .class("blue_label")
                .overflow(Overflow::Visible);
                // Label::new(cx, &state.get(cx).params.get_parameter_text(param_index));
                Label::new(
                    cx,
                    UiData::params.map(move |params| params_to_param(params).to_string()),
                )
                .overflow(Overflow::Visible);
            })
            .translate((0., 45.))
            .visibility(Visibility::from(vis))
            .col_between(Pixels(10.))
            .z_order(100)
            .height(Auto)
            .class("ghost_label")
            .position_type(PositionType::SelfDirected)
            .overflow(Overflow::Visible);
        });
        // });
    })
    .width(Pixels(50.))
    .child_space(Stretch(1.0))
    .row_between(Pixels(5.0))
}
fn draw_on_off_button(cx: &mut Context, param_ptr: ParamPtr) -> Handle<Button> {
    Button::new(
        cx,
        move |cx| {
            cx.emit(UiEvent::ToggleParam(param_ptr));
        },
        |cx| Label::new(cx, " "),
    )
    .bind(
        UiData::params.map(move |_p| unsafe { param_ptr.normalized_value() }),
        move |handle, val| {
            let selected = *val.get(handle.cx) > 0.;
            handle.checked(selected);
        },
    )
}

#[derive(Lens)]
pub struct UiData {
    pub gui_context: Arc<dyn GuiContext>,
    params: Arc<SamplerParams>,
    samples: Vec<SampleName>,
}
#[derive(Debug)]
pub enum UiEvent {
    AllParams(ParamPtr, f32),
    BeginSet(ParamPtr),
    EndSet(ParamPtr),
    ToggleParam(ParamPtr),
    ChangeSample(SampleName),
    // TODO: SavePreset should take a name sometime in the future
}
impl Data for SampleName {
    fn same(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl UiData {}
impl Model for UiData {
    fn event(&mut self, _cx: &mut Context, event: &mut Event) {
        if let Some(param_change_event) = event.message.downcast() {
            match param_change_event {
                // TODO: Rename to SetParam
                UiEvent::AllParams(param_ptr, new_value) => {
                    unsafe {
                        self.gui_context
                            .raw_set_parameter_normalized(*param_ptr, *new_value)
                    };
                }
                UiEvent::BeginSet(param_ptr) => {
                    unsafe { self.gui_context.raw_begin_set_parameter(*param_ptr) };
                }
                UiEvent::EndSet(param_ptr) => {
                    unsafe { self.gui_context.raw_end_set_parameter(*param_ptr) };
                }
                UiEvent::ToggleParam(param_ptr) => {
                    // let setter = ParamSetter::new(self.gui_context.as_ref());
                    unsafe { self.gui_context.raw_begin_set_parameter(*param_ptr) };
                    let norm_val = unsafe { param_ptr.normalized_value() };
                    let new_value = if norm_val > 0. { 0. } else { 1. };
                    // println!("param is {norm_val}");
                    // println!("Setting param to {new_value}");
                    unsafe {
                        self.gui_context
                            .raw_set_parameter_normalized(*param_ptr, new_value)
                    };
                    unsafe { self.gui_context.raw_end_set_parameter(*param_ptr) };
                }

                UiEvent::ChangeSample(table) => {
                    // println!("loading sample {}", table);
                    *self.params.sample_name.write().unwrap() = table.clone();
                    self.params.source_changed.set_release(true);
                    // self.params.load_sample();
                }
            }
        }
    }
}
#[derive(Lens)]
pub struct LabelData {
    visible: bool,
}
#[derive(Debug)]
pub enum LabelEvent {
    Show(bool),
}
impl Model for LabelData {
    fn event(&mut self, _cx: &mut Context, event: &mut Event) {
        if let Some(label_event) = event.message.downcast() {
            match label_event {
                LabelEvent::Show(visible) => {
                    self.visible = *visible;
                }
            }
        }
    }
}
