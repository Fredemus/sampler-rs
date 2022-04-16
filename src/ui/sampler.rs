use std::sync::Arc;

use crate::parameters::SamplerParams;

use super::{draw_on_off_button, make_knob, plot, UiData, UiEvent, ICON_DOWN_OPEN};
use nih_plug::prelude::Param;
use vizia::*;
// use vst::plugin::PluginParameters;
pub fn draw_sampler_gui(cx: &mut Context) -> Handle<VStack> {
    let params = &UiData::params.get(cx);
    // let param_offset = 109;
    VStack::new(cx, move |cx| {
        HStack::new(cx, move |cx| {
            draw_on_off_button(cx, params.is_on.as_ptr());
            // Label::new(cx, "Sampler");
            draw_sample_selector(cx)
                .class("selector")
                .width(Pixels(150.));
        })
        .top(Pixels(0.))
        .height(Auto)
        // .child_space(Stretch(1.))
        .col_between(Pixels(5.));
        HStack::new(cx, move |cx| {
            make_pitch_field(
                cx,
                Pixels(80.),
                "Root".to_string(),
                params.root.as_ptr(),
                |p| &p.root,
            );
            make_pitch_field(
                cx,
                Pixels(80.),
                "Fine".to_string(),
                params.fine_tune.as_ptr(),
                |p| &p.fine_tune,
            );
            make_pitch_field(
                cx,
                Pixels(80.),
                "Crs".to_string(),
                params.coarse_tune.as_ptr(),
                |p| &p.coarse_tune,
            );
        })
        .top(Pixels(0.))
        .child_top(Pixels(0.))
        .height(Auto)
        .class("close_knobs");
        // placeholder for waveform
        // Element::new(cx).class("waveform").text("Waveform");
        plot::SamplePlot::new(cx)
            .class("waveform")
            .width(Stretch(1.))
            .overflow(Overflow::Visible)
            .top(Pixels(0.));
        HStack::new(cx, move |cx| {
            make_knob(cx, "Volume", false, params.volume.as_ptr(), |p| &p.volume);
            // make_knob(cx, "Pan", true, params.pan.as_ptr(), |p| &p.pan);
            make_knob(cx, "Pos", false, params.pos.as_ptr(), |p| &p.pos);
            VStack::new(cx, move |cx| {
                // TODO: Center these
                HStack::new(cx, move |cx| {
                    draw_on_off_button(cx, params.keytrack.as_ptr()).class("small_on_off");
                    Label::new(cx, "Keytrack");
                });
                // HStack::new(cx, move |cx| {
                //     draw_on_off_button(cx, params.is_looping.as_ptr()).class("small_on_off");
                //     Label::new(cx, "Loop");
                // });
            });
        })
        .top(Pixels(0.))
        .class("sparse_knobs");
    })
    .class("container")
}
fn draw_sample_selector(cx: &mut Context) -> Handle<HStack> {
    HStack::new(cx, move |cx| {
        Label::new(cx, "Sampler");
        // Dropdown to select filter circuit
        // Dropdown List
        Dropdown::new(
            cx,
            move |cx|
            // A Label and an Icon
            HStack::new(cx, move |cx|{
                // let choice = "Placeholder";
                Label::new(cx, UiData::params.map(move |params| params.get_sample_name()));
                Label::new(cx, ICON_DOWN_OPEN).class("arrow");
            })
            // .width(Stretch(1.))
            .width(Pixels(150.)),
            move |cx| {
                // List of options
                List::new(cx, UiData::samples, move |cx, _, item| {
                    VStack::new(cx, move |cx| {
                        let option = item.get(cx).clone();
                        let choice = UiData::params.get(cx).get_sample_name();
                        // let choice = "Kick drum";
                        let selected = format!("{}", option) == choice;
                        // Button which updates the chosen option
                        Label::new(cx, &item.get(cx).to_string())
                            .width(Stretch(1.0))
                            .class("item")
                            .checked(selected)
                            .on_press(move |cx| {
                                cx.emit(UiEvent::ChangeSample(item.get(cx).clone()));
                                cx.emit(PopupEvent::Close);
                            });
                    });
                });
            },
        )
        .width(Stretch(1.));
        // .width(Pixels(100.));
    })
}
fn make_pitch_field<'a, P, F>(
    cx: &mut Context,
    width: Units,
    name: String,
    param_ptr: nih_plug::param::internals::ParamPtr,
    params_to_param: F,
) where
    P: Param,
    F: 'static + Fn(&Arc<SamplerParams>) -> &P + Copy,
{
    // let context = UiData::gui_context.get(cx).as_ref();
    // let setter = ParamSetter::new(context.as_ref());
    // Label::new(cx, &state.get(cx).params.get_parameter_name(param_index))
    //     .class("small_label");
    let name = name.clone();
    Knob::custom(
        cx,
        params_to_param(&*UiData::params.get(cx)).default_normalized_value(),
        UiData::params.map(move |params| params_to_param(params).normalized_value()),
        move |cx, _| {
            let name = name.clone();
            HStack::new(cx, move |cx| {
                Label::new(cx, &name.clone())
                    .right(Stretch(1.))
                    .class("blue_label");
                Label::new(
                    cx,
                    UiData::params.map(move |params| params_to_param(params).to_string()),
                )
                .width(Pixels(40.));
            })
            .class("pitch_field")
        },
    )
    .on_changing(move |cx, val| {
        cx.emit(UiEvent::SetParam(param_ptr, val));
    })
    .height(Pixels(20.))
    .width(width);
}
