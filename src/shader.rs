//should've been named shade.rs

use std::borrow::Cow;
use std::time;

use anyhow::{anyhow, Context, Result};
use wgpu::util::DeviceExt;
use wgpu::{BindGroup, Buffer, CommandEncoder, ComputePipeline, Device, Queue};

pub struct ShaderData<'a, S>
where
    Cow<'a, str>: From<S>,
{
    pub name: &'a str,
    pub source: S,
}

pub struct DeviceData {
    pub device: Device,
    pub queue: Queue,
    pub command_encoder: CommandEncoder,
}

pub struct PipelineData {
    pub compute: ComputePipeline,
    pub bind_group: BindGroup,
    pub storage_buffer: Buffer,
    pub staging_buffer: Buffer,
    pub aux_buffer: Buffer,
}

pub async fn init_gpu() -> Result<DeviceData> {
    // Instantiates instance of WebGPU
    let instance = wgpu::Instance::new(wgpu::Backends::all());

    // Instantiates the general connection to the GPU
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .context("Failed to instantiate a connection to the GPU.")?;

    // Instantiates the feature specific connection to the GPU, defining some parameters,
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults(),
            },
            None,
        )
        .await
        .unwrap();

    let info = adapter.get_info();
    if info.vendor == 0x10005 {
        return Err(anyhow!(
            "LavaPipe is currently unsupported for Destructible shaders."
        ));
    }

    let command_encoder =
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    return Ok(DeviceData {
        device,
        queue,
        command_encoder,
    });
}

pub async fn exec_shader<'a, T, U: bytemuck::Pod>(
    shader: &ShaderData<'a, &'a str>,
    data: &'a [T],
    device_data: &mut DeviceData,
    pipeline_data: &PipelineData,
) -> Result<Vec<U>> {
    /*let mut cpass = device_data
        .command_encoder
        .begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
    {
        cpass.set_pipeline(&pipeline_data.compute);
        cpass.set_bind_group(0, &pipeline_data.bind_group, &[]);
        cpass.dispatch(8, 1, 1);
    }*/

    println!("1 {:?}", time::Instant::now());

    let slice_size = data.len() * std::mem::size_of::<T>();
    let size = slice_size as wgpu::BufferAddress;

    println!("2 {:?}", time::Instant::now());
    let mut encoder = device_data
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&pipeline_data.compute);
        cpass.set_bind_group(0, &pipeline_data.bind_group, &[]);
        cpass.insert_debug_marker(shader.name);
        cpass.dispatch(data.len() as u32, 1, 1);
    }

    println!("3 {:?}", time::Instant::now());
    // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.

    encoder.copy_buffer_to_buffer(
        &pipeline_data.storage_buffer,
        0,
        &pipeline_data.staging_buffer,
        0,
        size,
    );

    println!("4 {:?}", time::Instant::now());

    // Submits command encoder for processing
    device_data.queue.submit(Some(encoder.finish()));

    println!("5 {:?}", time::Instant::now());
    // Note that we're not calling `.await` here.
    let buffer_slice = pipeline_data.staging_buffer.slice(..);
    // Gets the future representing when `staging_buffer` can be read from
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    println!("6 {:?}", time::Instant::now());
    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device_data.device.poll(wgpu::Maintain::Wait);

    println!("7 {:?}", time::Instant::now());
    // Awaits until `buffer_future` can be read from
    if let Ok(()) = buffer_future.await {
        println!("8 {:?}", time::Instant::now());
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();

        println!("9 {:?}", time::Instant::now());
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = bytemuck::cast_slice(&data).to_vec();

        println!("10 {:?}", time::Instant::now());
        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        pipeline_data.staging_buffer.unmap(); // Unmaps buffer from memory

        println!("11 {:?}", time::Instant::now());
        // Returns data from buffer
        Ok(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
}

pub async fn create_shader<'a, T, U, V>(
    shader: &ShaderData<'a, &'a str>,
    data: &'a [T],
    aux_data: &'a [U],
    device_data: &DeviceData,
) -> Result<PipelineData>
where
    T: bytemuck::Pod,
    U: bytemuck::Pod,
    V: bytemuck::Pod + Clone,
{
    let cs_module = device_data
        .device
        .create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some(shader.name),
            source: wgpu::ShaderSource::Wgsl(Cow::from(shader.source)),
        });

    // Gets the size in bytes of the buffer.
    let slice_size = data.len() * std::mem::size_of::<T>();
    let size = slice_size as wgpu::BufferAddress;

    let aux_buffer = device_data
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Auxillary Buffer"),
            contents: bytemuck::cast_slice(&aux_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

    // Instantiates buffer without data.
    // `usage` of buffer specifies how it can be used:
    //   `BufferUsages::MAP_READ` allows it to be read (outside the shader).
    //   `BufferUsages::COPY_DST` allows it to be the destination of the copy.
    let staging_buffer = device_data.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    // Instantiates buffer with data (`numbers`).
    // Usage allowing the buffer to be:
    //   A storage buffer (can be bound within a bind group and thus available to a shader).
    //   The destination of a copy.
    //   The source of a copy.
    let storage_buffer = device_data
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Storage Buffer"),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });

    // A bind group defines how buffers are accessed by shaders.
    // It is to WebGPU what a descriptor set is to Vulkan.
    // `binding` here refers to the `binding` of a buffer in the shader (`layout(set = 0, binding = 0) buffer`).

    // A pipeline specifies the operation of a shader

    // Instantiates the pipeline.
    let compute_pipeline =
        device_data
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: None,
                layout: None,
                module: &cs_module,
                entry_point: "main",
            });

    // Instantiates the bind group, once again specifying the binding of buffers.
    let bind_group_layout = compute_pipeline.get_bind_group_layout(0);
    let bind_group = device_data
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: storage_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: aux_buffer.as_entire_binding(),
                },
            ],
        });

    /*
    // A command encoder executes one or many pipelines.
    // It is to WebGPU what a command buffer is to Vulkan.
    let mut encoder = device_data
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor { label: None });
        cpass.set_pipeline(&compute_pipeline);
        cpass.set_bind_group(0, &bind_group, &[]);
        cpass.insert_debug_marker(shader.name);
        cpass.dispatch(data.len() as u32, 1, 1);
    }
    // Sets adds copy operation to command encoder.
    // Will copy data from storage buffer on GPU to staging buffer on CPU.
    encoder.copy_buffer_to_buffer(&storage_buffer, 0, &staging_buffer, 0, size);

    // Submits command encoder for processing
    device_data.queue.submit(Some(encoder.finish()));

    // Note that we're not calling `.await` here.
    let buffer_slice = staging_buffer.slice(..);
    // Gets the future representing when `staging_buffer` can be read from
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);

    // Poll the device in a blocking manner so that our future resolves.
    // In an actual application, `device.poll(...)` should
    // be called in an event loop or on another thread.
    device_data.device.poll(wgpu::Maintain::Wait);

    // Awaits until `buffer_future` can be read from
    if let Ok(()) = buffer_future.await {
        // Gets contents of buffer
        let data = buffer_slice.get_mapped_range();
        // Since contents are got in bytes, this converts these bytes back to u32
        let result = bytemuck::cast_slice(&data).to_vec();

        // With the current interface, we have to make sure all mapped views are
        // dropped before we unmap the buffer.
        drop(data);
        staging_buffer.unmap(); // Unmaps buffer from memory

        // Returns data from buffer
        Ok(result)
    } else {
        panic!("failed to run compute on gpu!")
    }
    */
    return Ok(PipelineData {
        compute: compute_pipeline,
        bind_group,
        storage_buffer,
        staging_buffer,
        aux_buffer,
    });
}
