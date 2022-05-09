struct CustomMaterial {
    color: vec4<f32>,
};
@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@fragment
fn fragment_fn() -> @location(0) vec4<f32> {
    return material.color;
}
