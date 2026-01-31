use rupl::types::{Complex, Graph, GraphData};
use softbuffer::Surface;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};
fn main() -> Result<(), EventLoopError> {
    let mut app = App::default();
    app.plot
        .set_data(vec![GraphData::Width(points(-2.0, 2.0), -2.0, 2.0)]);
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.run_app(&mut app)
}
#[derive(Default)]
struct App {
    plot: Graph,
    surface_state: Option<Surface<Arc<Window>, Arc<Window>>>,
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = {
            let window = event_loop.create_window(Window::default_attributes());
            Arc::new(window.unwrap())
        };
        let context = softbuffer::Context::new(window.clone()).unwrap();
        self.surface_state = Some(Surface::new(&context, window.clone()).unwrap());
    }
    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::Resized(d) => {
                let Some(state) = self.surface_state.as_mut() else {
                    return;
                };
                state.window().request_redraw();
                state
                    .resize(
                        std::num::NonZeroU32::new(d.width).unwrap(),
                        std::num::NonZeroU32::new(d.height).unwrap(),
                    )
                    .unwrap();
            }
            WindowEvent::RedrawRequested => {
                let Some(state) = self.surface_state.as_mut() else {
                    return;
                };
                let (width, height) = {
                    let size = state.window().inner_size();
                    (size.width, size.height)
                };
                state
                    .resize(
                        std::num::NonZeroU32::new(width).unwrap(),
                        std::num::NonZeroU32::new(height).unwrap(),
                    )
                    .unwrap();
                #[cfg(not(feature = "skia-vulkan"))]
                {
                    let mut buffer = state.buffer_mut().unwrap();
                    self.plot.update(width, height, &mut buffer);
                    buffer.present().unwrap();
                }
            }
            WindowEvent::CloseRequested => el.exit(),
            _ => {}
        }
    }
    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.surface_state = None;
    }
}
fn points(start: f64, end: f64) -> Vec<Complex> {
    let len = 256;
    let delta = (end - start) / len as f64;
    (0..=len)
        .map(|i| {
            let x = start + i as f64 * delta;
            Complex::Real(f(x))
        })
        .collect()
}
fn f(x: f64) -> f64 {
    x * x * x - x
}
