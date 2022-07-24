#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    #import bevy_pbr::mesh_vertex_output
};

@vertex
fn vertex_fn(vertex: Vertex) -> VertexOutput {
    var vout: VertexOutput;
#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
    vout.world_normal = skin_normals(model, vertex.normal);
#else
    var model = mesh.model;
    vout.world_normal = mesh_normal_local_to_world(vertex.normal);
#endif
    vout.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
#ifdef VERTEX_UVS
    vout.uv = vertex.uv;
#endif
#ifdef VERTEX_TANGENTS
    vout.world_tangent = mesh_tangent_local_to_world(model, vertex.tangent);
#endif
#ifdef VERTEX_COLORS
    vout.color = vertex.color;
#endif

    vout.clip_position = mesh_position_world_to_clip(vout.world_position);
    return vout;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    #import bevy_pbr::mesh_vertex_output
};

@fragment
fn fragment_fn(frag_in: FragmentInput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    return frag_in.color;
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}
