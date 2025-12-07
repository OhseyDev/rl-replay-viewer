mod content;
mod rendering;

use winit::{
    event::{WindowEvent},
    event_loop::EventLoop,
    window::Window
};
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, StartCause};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{WindowAttributes, WindowId};

struct App {
    device: rendering::Device,
    view: Option<rendering::WindowView>,
}

impl App {
    fn new(event_loop: &EventLoop<()>) -> Self {
        let device = rendering::Device::new(event_loop, vec![]).expect("Failed to setup Vulkan - Aborting!");
        Self { device, view: None }
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        todo!()
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(_) = &self.view { return; }
        let view = rendering::WindowView::new(event_loop, &mut self.device).expect("Failed to setup Vulkan - Aborting!");
        self.view = Some(view);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        todo!()
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        todo!()
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, device_id: DeviceId, event: DeviceEvent) {
        todo!()
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        todo!()
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        todo!()
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        todo!()
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        todo!()
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).expect("App crashed!");
}
