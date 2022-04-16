use baseview::WindowHandle;
use nih_plug::context::GuiContext;
use nih_plug::plugin::{Editor, ParentWindowHandle};

use vizia::*;
// include style from css file
const STYLE: &str = include_str!("style.css");

// use crate::resources::{load_preset, save_preset, Table};
// use crate::synth::mod_matrix::ModulateMulti;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
// use std::path::PathBuf;
use std::sync::Arc;

// pub const WINDOW_WIDTH: u32 = 1024;
// pub const WINDOW_HEIGHT: u32 = 768;
pub const WINDOW_WIDTH: u32 = 512;
pub const WINDOW_HEIGHT: u32 = 384;
// pub const WINDOW_WIDTH: u32 = 1280;
// pub const WINDOW_HEIGHT: u32 = 960;
// const WINDOW_WIDTH_F: f32 = WINDOW_WIDTH as f32;
// const WINDOW_HEIGHT_F: f32 = WINDOW_HEIGHT as f32;

#[allow(dead_code)]
pub fn create_vizia_editor<U>(update: U) -> Option<Box<dyn Editor>>
where
    U: Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync,
{
    Some(Box::new(ViziaEditor {
        update: Arc::new(update),
    }))
}
pub struct ViziaEditor {
    update: Arc<dyn Fn(&mut Context, Arc<dyn GuiContext>) + 'static + Send + Sync>,
}

impl Editor for ViziaEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn std::any::Any + Send + Sync> {
        let update = self.update.clone();

        let window_description =
            WindowDescription::new().with_inner_size(WINDOW_WIDTH, WINDOW_HEIGHT);
        let window = Application::new(window_description, move |cx| {
            cx.add_theme(STYLE);

            (update)(cx, context.clone());
        })
        .open_parented(&parent);

        Box::new(ViziaEditorHandle { window })
    }

    fn size(&self) -> (u32, u32) {
        (WINDOW_WIDTH, WINDOW_HEIGHT)
    }

    fn set_scale_factor(&self, _factor: f32) -> bool {
        todo!()
    }

    fn param_values_changed(&self) {
        // TODO: Should anything happen here?
    }
}

struct ViziaEditorHandle {
    window: WindowHandle,
}

unsafe impl Send for ViziaEditorHandle {}
unsafe impl Sync for ViziaEditorHandle {}

impl Drop for ViziaEditorHandle {
    fn drop(&mut self) {
        self.window.close();
    }
}

struct VstParent(*mut ::std::ffi::c_void);
#[cfg(target_os = "macos")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::macos::MacOSHandle;

        RawWindowHandle::MacOS(MacOSHandle {
            ns_view: self.0 as *mut ::std::ffi::c_void,
            ..MacOSHandle::empty()
        })
    }
}
#[cfg(target_os = "windows")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::windows::WindowsHandle;

        RawWindowHandle::Windows(WindowsHandle {
            hwnd: self.0,
            ..WindowsHandle::empty()
        })
    }
}
#[cfg(target_os = "linux")]
unsafe impl HasRawWindowHandle for VstParent {
    fn raw_window_handle(&self) -> RawWindowHandle {
        use raw_window_handle::unix::XcbHandle;

        RawWindowHandle::Xcb(XcbHandle {
            window: self.0 as u32,
            ..XcbHandle::empty()
        })
    }
}
