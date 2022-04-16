use std::cell::RefCell;
use std::rc::Rc;

use femtovg::ImageFlags;
use femtovg::ImageId;
use femtovg::RenderTarget;
use femtovg::{Paint, Path};
use vizia::*;

use crate::ui::UiData;

pub struct SamplePlot {
    // image: Option<ImageId>,
    image: Rc<RefCell<Option<ImageId>>>,
}

impl SamplePlot {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {
            image: Rc::new(RefCell::new(None)),
        }
        .build2(cx, |_| {})
    }
}

impl View for SamplePlot {
    fn draw(&self, cx: &mut Context, canvas: &mut Canvas) {
        if let Some(ui_data) = cx.data::<UiData>() {
            // TODO - Make this configurable
            // let width = 360;
            // let height = 200;
            let width = 512;
            let height = 100;
            let amps = ui_data.params.get_sample();
            // let len = amps.len();

            let max = 1.;
            let min = -1.;

            let bounds = cx.cache.get_bounds(cx.current);

            let image_id = if let Some(image_id) = *self.image.borrow() {
                image_id
            } else {
                canvas
                    .create_image_empty(
                        width,
                        height,
                        femtovg::PixelFormat::Rgb8,
                        ImageFlags::FLIP_Y,
                    )
                    .expect("Failed to create image")
            };
            *self.image.borrow_mut() = Some(image_id);
            canvas.set_render_target(RenderTarget::Image(image_id));

            let background_color = cx
                .style
                .background_color
                .get(cx.current)
                .cloned()
                .unwrap_or_default();
            let color = cx
                .style
                .font_color
                .get(cx.current)
                .cloned()
                .unwrap_or_default();

            // Fill background
            canvas.clear_rect(0, 0, width as u32, height as u32, background_color.into());

            // println!("max: {}", amps.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() );
            // println!("min: {}", amps.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap());
            let ds_factor = amps.len() / width;
            // dbg!(ds_factor);
            // let ds_factor = 1;
            let mut path = Path::new();
            let amp = amps[0].clamp(min, max);
            let y = height as f32 * ((amp - min) / (max - min));

            path.move_to(0.0, height as f32 - y + 1.0);
            let line_width = 2.0;
            let height_adjusted = height as f32 - 100. * ((line_width) / height as f32);
            for i in 1..width {
                let amp = amps[i * ds_factor].clamp(min, max);
                let y = amp * height_adjusted;

                path.line_to(i as f32, (height as f32 - y) / 2.);
            }
            let mut path2 = path.clone();

            // Draw plot
            let mut paint = Paint::color(color.into());
            paint.set_line_width(line_width);
            canvas.stroke_path(&mut path, paint);

            // making a cool background thingy
            // Graph background
            let mut mid_color = femtovg::Color::from(color);
            mid_color.set_alpha(10);
            let mut edge_color = femtovg::Color::from(color);
            edge_color.set_alpha(64);
            let bg = Paint::linear_gradient_stops(
                0.0,
                0.0,
                0.0,
                height as f32,
                // femtovg::Color::rgba(0, 160, 192, 0),
                // femtovg::Color::rgba(0, 160, 192, 64),
                &[(0.0, edge_color), (0.5, mid_color), (1.0, edge_color)],
            );
            path2.line_to(width as f32, height as f32 / 2.);
            path2.line_to(0., height as f32 / 2.);
            canvas.fill_path(&mut path2, bg);

            canvas.set_render_target(RenderTarget::Screen);

            let mut path = Path::new();
            path.rect(bounds.x, bounds.y, bounds.w, bounds.h);
            canvas.fill_path(
                &mut path,
                Paint::image(image_id, bounds.x, bounds.y, bounds.w, bounds.h, 0.0, 1.0),
            );
        }
    }
}
