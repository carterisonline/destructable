
[[block]]
struct Data {
    inner: [[stride(4)]] array<f32>;
};

[[block]]
struct Aux {
    rand_seed: f32;
};

[[group(0), binding(0)]] var<storage, read_write> data: Data;
[[group(0), binding(1)]] var<uniform> aux: Aux;

fn rand(n: f32, seed: f32) -> f32 {
    return fract(sin(n) * seed);
}

[[stage(compute), workgroup_size(1)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    data.inner[global_id.x] = rand(data.inner[global_id.x], aux.rand_seed);
}