use std::borrow::Cow;

use wgpu::{
    Device, DeviceDescriptor, ExperimentalFeatures, Features, Instance, Limits, MemoryHints, Queue,
    ShaderModule, ShaderModuleDescriptor, ShaderSource, Trace,
};

const SHADER_SOURCE: &'static str = include_str!("shader.wgsl");

fn main() {
    env_logger::init();
    let (device, queue) = get_device_and_queue();
    let shader_module = create_shader_module(&device, "test", SHADER_SOURCE);
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
