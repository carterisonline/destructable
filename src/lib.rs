#![deny(clippy::all)]
#![forbid(unsafe_code)]
#![feature(fn_traits)]

use anyhow::Result;
use winit::dpi::{LogicalPosition, LogicalSize, PhysicalSize};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub mod app;
pub mod shader;

pub struct WindowData {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}
pub fn create_window<T>(
    title: &str,
    width: T,
    height: T,
    event_loop: &EventLoop<()>,
) -> Result<(Window, WindowData)>
where
    f64: From<T>,
{
    let window = WindowBuilder::new()
        .with_visible(false)
        .with_title(title)
        .build(event_loop)?;

    let scale_factor = window.scale_factor();
    let width: f64 = width.into();
    let height: f64 = height.into();
    let (monitor_width, monitor_height) = if let Some(monitor) = window.current_monitor() {
        let size = monitor.size().to_logical(scale_factor);
        (size.width, size.height)
    } else {
        (width, height)
    };

    let scale = (monitor_height / height * 2.0 / 3.0).round().max(1.0);

    let min_size = PhysicalSize::new(width, height).to_logical::<f64>(scale_factor);
    let default_size = LogicalSize::new(width * scale, height * scale);
    let center = LogicalPosition::new(
        (monitor_width - width * scale) / 2.0,
        (monitor_height - height * scale) / 2.0,
    );

    window.set_inner_size(default_size);
    window.set_min_inner_size(Some(min_size));
    window.set_outer_position(center);
    window.set_visible(true);

    let size = default_size.to_physical::<f64>(scale_factor);

    return Ok((
        window,
        WindowData {
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            scale_factor,
        },
    ));
}
