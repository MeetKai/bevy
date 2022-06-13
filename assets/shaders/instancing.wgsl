#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

@group(1) @binding(0)
var<uniform> mesh: Mesh;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,

    @location(3) i_pos_scale: vec4<f32>,
    @location(4) i_color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vertex_fn(vertex: Vertex) -> VertexOutput {
    let position = vertex.position * vertex.i_pos_scale.w + vertex.i_pos_scale.xyz;
    let world_position = mesh.model * vec4<f32>(position, 1.0);

    var vout: VertexOutput;
    vout.clip_position = view.view_proj * world_position;
    vout.color = vertex.i_color;
    return vout;
}

@fragment
fn fragment_fn(vin: VertexOutput) -> @location(0) vec4<f32> {
    return vin.color;
}
