use anyhow::Result;

use log::error;
use pixels::{Pixels, SurfaceTexture};
use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget};
use winit::window::Window;
use winit_input_helper::WinitInputHelper;

use crate::WindowData;

pub struct App<UpdFn, DrawFn>
where
    UpdFn: 'static + FnMut(&Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
    DrawFn: 'static
        + FnMut(
            &Event<'_, ()>,
            &EventLoopWindowTarget<()>,
            &mut ControlFlow,
            &Window,
            &WindowData,
            &mut Pixels,
        ),
{
    event_loop: EventLoop<()>,
    on_update_fn: UpdFn,
    on_draw_fn: DrawFn,
    input: WinitInputHelper,
    window: Window,
    window_data: WindowData,
    pixels: Pixels,
}

impl<UpdFn, DrawFn> App<UpdFn, DrawFn>
where
    UpdFn: 'static + FnMut(&Event<'_, ()>, &EventLoopWindowTarget<()>, &mut ControlFlow),
    DrawFn: 'static
        + FnMut(
            &Event<'_, ()>,
            &EventLoopWindowTarget<()>,
            &mut ControlFlow,
            &Window,
            &WindowData,
            &mut Pixels,
        ),
{
    pub fn new<T: Clone>(
        title: &str,
        width: T,
        height: T,
        on_draw: DrawFn,
        on_update: UpdFn,
    ) -> Result<Self>
    where
        f64: From<T>,
        u32: From<T>,
    {
        let event_loop = EventLoop::new();
        let res = crate::create_window(title, width.clone(), height.clone(), &event_loop)?;
        let pixels = Pixels::new(
            width.into(),
            height.into(),
            SurfaceTexture::new(res.1.width, res.1.height, &res.0),
        )?;

        return Ok(Self {
            event_loop,
            on_update_fn: on_update,
            on_draw_fn: on_draw,
            input: WinitInputHelper::new(),
            window: res.0,
            window_data: res.1,
            pixels,
        });
    }

    pub fn run(mut self) -> ! {
        self.event_loop.run(move |event, target, control_flow| {
            if let Event::RedrawRequested(_) = event {
                self.on_draw_fn.call_mut((
                    &event,
                    target,
                    control_flow,
                    &self.window,
                    &self.window_data,
                    &mut self.pixels,
                ));

                if self
                    .pixels
                    .render()
                    .map_err(|e| error!("pixels.render() failed: {}", e))
                    .is_err()
                {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            if self.input.update(&event) {
                if let Some(size) = self.input.window_resized() {
                    self.pixels.resize_surface(size.width, size.height);
                }
                self.on_update_fn.call_mut((&event, target, control_flow));
                self.window.request_redraw();
            }
        });
    }
}
