#![deny(clippy::all)]
#![forbid(unsafe_code)]

use std::time;

use anyhow::Result;
use descructible::app::App;
use descructible::shader::{create_shader, exec_shader, init_gpu, ShaderData};
use rand::Rng;

const RANDFILL: &str = include_str!("../shader/randfill.wgsl");

fn main() -> Result<()> {
    env_logger::init();
    let mut rng = rand::thread_rng();
    let mut device_data = pollster::block_on(init_gpu())?;
    let shader_data = ShaderData {
        name: "randfill",
        source: RANDFILL,
    };
    let data = &[0.0, 1.0, 2.0, 3.0];
    let shader = pollster::block_on(create_shader::<f32, f32, f32>(
        &shader_data,
        &[0.0, 1.0, 2.0, 3.0],
        &[rng.gen_range(0.0f32..10.0)],
        &device_data,
    ))?;

    let funnygame = App::new(
        "Funny Game",
        600u32,
        1080u32,
        move |_, _, _, _, window_data, pixels| {
            let frame = pixels.get_frame();
            let now = time::Instant::now();
            let out = pollster::block_on(exec_shader::<f32, f32>(
                &shader_data,
                data,
                &mut device_data,
                &shader,
            ))
            .unwrap();
            println!("{:?}", out);
            println!(
                "finished compute shader in {}us",
                (time::Instant::now() - now).to_owned().subsec_micros()
            );
            /*for f in res {
                frame[rng.gen_range(0..window_data.width * window_data.height * 4) as usize] =
                    (f.round() % 255.0) as u8;
            }*/
            frame[rng.gen_range(0..window_data.width * window_data.height * 4) as usize] =
                rng.gen_range(0..=255);
        },
        move |_, _, _| {},
    )?;

    funnygame.run();
}
