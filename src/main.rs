use std::borrow::Cow;
use std::num::NonZeroU64;

use wgpu::util::DeviceExt;
use wgpu::wgt::{BufferDescriptor, CommandEncoderDescriptor};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, BufferBindingType, CommandBuffer, ComputePassDescriptor, ComputePipeline, ComputePipelineDescriptor, PipelineCompilationOptions, PipelineLayout, PipelineLayoutDescriptor, ShaderStages
};
use wgpu::{
    Buffer, BufferUsages, Device, DeviceDescriptor, ExperimentalFeatures, Features, Instance,
    Limits, MemoryHints, Queue, ShaderModule, ShaderModuleDescriptor, ShaderSource, Trace,
    util::BufferInitDescriptor,
};

const SHADER_SOURCE: &'static str = include_str!("shader.wgsl");

fn main() {
    env_logger::init();
    let (device, queue) = get_device_and_queue();

    let shader_module = create_shader_module(&device, "test", SHADER_SOURCE);

    let data = [1.; 32];
    let input_buffer = create_input_buffer(&device, &data);
    let size = input_buffer.size();
    let output_buffer = create_output_buffer(
        &device,
        size,
        BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    );
    let download_buffer = create_output_buffer(
        &device,
        size,
        BufferUsages::MAP_READ | BufferUsages::COPY_DST,
    );

    let bind_group_layout = create_bind_group_layout(&device);
    let bind_group = create_bind_group(&device, &bind_group_layout, &input_buffer, &output_buffer);

    let pipeline_layout = create_pipeline_layout(&device, &bind_group_layout);
    let pipeline = create_pipeline(&device, &pipeline_layout, &shader_module);

    let workgroup_count = data.len().div_ceil(64) as u32;
    let command_buffer = create_command_buffer(&device, &pipeline, &bind_group, workgroup_count, &output_buffer, &download_buffer);
}

fn create_command_buffer(
    device: &Device,
    pipeline: &ComputePipeline,
    bind_group: &BindGroup,
    workgroup_count: u32,
    output_buffer: &Buffer,
    download_buffer: &Buffer,
) -> CommandBuffer {
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor { label: Some("encoder") });
    let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("compute pass"),
        timestamp_writes: None,
    });
    compute_pass.set_pipeline(&pipeline);
    compute_pass.set_bind_group(0, bind_group, &[]);
    compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
    drop(compute_pass);
    // TODO: split in multiple command encoders
    encoder.copy_buffer_to_buffer(output_buffer, 0, download_buffer, 0, output_buffer.size());
    encoder.finish()
}

fn create_pipeline(
    device: &Device,
    layout: &PipelineLayout,
    shader_module: &ShaderModule,
) -> ComputePipeline {
    device.create_compute_pipeline(&ComputePipelineDescriptor {
        label: Some("pipeline"),
        layout: Some(layout),
        module: shader_module,
        entry_point: Some("call"),
        compilation_options: PipelineCompilationOptions::default(),
        cache: None,
    })
}

fn create_pipeline_layout(device: &Device, bind_group_layout: &BindGroupLayout) -> PipelineLayout {
    device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    })
}

fn create_bind_group(
    device: &Device,
    layout: &BindGroupLayout,
    input: &Buffer,
    output: &Buffer,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("bind group"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    })
}

fn create_bind_group_layout(device: &Device) -> BindGroupLayout {
    device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("bind group layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(4).unwrap()),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::COMPUTE,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(4).unwrap()),
                },
                count: None,
            },
        ],
    })
}

fn create_input_buffer(device: &Device, data: &[f32]) -> Buffer {
    device.create_buffer_init(&BufferInitDescriptor {
        label: Some("input"),
        contents: bytemuck::cast_slice(data),
        usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
    })
}

fn create_output_buffer(device: &Device, size: u64, usage: BufferUsages) -> Buffer {
    device.create_buffer(&BufferDescriptor {
        label: Some("output"),
        size,
        usage,
        mapped_at_creation: false,
    })
}

fn create_shader_module(device: &Device, label: &str, shader_source: &str) -> ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label: Some(label),
        source: ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
    })
}

async fn get_device_and_queue_async() -> (Device, Queue) {
    let instance = Instance::default();
    let adapter = wgpu::util::initialize_adapter_from_env(&instance, None)
        .expect("No suitable GPU adapters found on the system");
    let info = adapter.get_info();
    println!(
        "Using {:#?} {} with {:#?} backend",
        info.device_type, info.name, info.backend
    );
    let downlevel_capabilities = adapter.get_downlevel_capabilities();
    if !downlevel_capabilities
        .flags
        .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
    {
        panic!("Adapter does not support compute shaders");
    }
    let device_and_queue = adapter
        .request_device(&DeviceDescriptor {
            label: None,
            required_features: Features::empty(),
            required_limits: Limits::downlevel_defaults(),
            memory_hints: MemoryHints::Performance,
            trace: Trace::Off,
            experimental_features: ExperimentalFeatures::disabled(),
        })
        .await
        .unwrap();
    device_and_queue
}

fn get_device_and_queue() -> (Device, Queue) {
    futures::executor::block_on(get_device_and_queue_async())
}
