#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_view_bindings

struct Vertex {
    @location(0) position: vec3<f32>,
#ifdef SKINNED
    @location(4) joint_indexes: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif
};

@group(1) @binding(0)
var<uniform> mesh: Mesh;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
};

#ifdef SKINNED
@group(1) @binding(1)
var<uniform> joint_matrices: SkinnedMesh;
#import bevy_pbr::skinning
#endif

@vertex
fn vertex_fn(vertex: Vertex) -> VertexOutput {
#ifdef SKINNED
    let model = skin_model(vertex.joint_indexes, vertex.joint_weights);
#else
    let model = mesh.model;
#endif

    let world_position = model * vec4<f32>(vertex.position, 1.0);
    var vout: VertexOutput;
    vout.clip_position = view.view_proj * world_position;

    return vout;
}

@fragment
fn fragment_fn() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
