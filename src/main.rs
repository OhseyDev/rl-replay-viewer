mod content;
mod rendering;

use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::Window
};

async fn run(event_loop: EventLoop<()>, window: &Window) {
    event_loop.run(move |event, target| {
        if let Event::WindowEvent { window_id: _, event } = event {
            match event {
                WindowEvent::Resized(new_size) => {
                    todo!("Not implemented yet");
                }
                WindowEvent::RedrawRequested => {
                    todo!("Not implemented yet");
                }
                WindowEvent::CloseRequested => {
                    target.exit();
                }
                _ => {}
            }
        }
    }).unwrap();
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = winit::window::WindowBuilder::new().with_title("RL Replay Viewer").build(&event_loop).unwrap();
    let device = crate::rendering::Device::new().expect("Unable to initialize vulkan!");
    pollster::block_on(run(event_loop, &window));
}
