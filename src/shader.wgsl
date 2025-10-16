@group(0) @binding(0)
var<storage, read> input: array<f32>;
@group(0) @binding(1)
var<storage, read_write> output: array<f32>;

fn neg(in: f32) -> f32 {
    return -in;
}

@compute
@workgroup_size(64)
fn call(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;

    if (i > arrayLength(&output)) {
        return;
    }

    output[i] = neg(input[i]);
}