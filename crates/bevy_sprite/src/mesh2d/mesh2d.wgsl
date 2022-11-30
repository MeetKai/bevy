#import bevy_sprite::mesh2d_view_bindings
#import bevy_sprite::mesh2d_bindings

// NOTE: Bindings must come before functions that use them!
#import bevy_sprite::mesh2d_functions

struct Vertex {
#ifdef VERTEX_POSITIONS
    @location(0) position: vec3<f32>,
#endif
#ifdef VERTEX_NORMALS
    @location(1) normal: vec3<f32>,
#endif
#ifdef VERTEX_UVS
    @location(2) uv: vec2<f32>,
#endif
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    #import bevy_sprite::mesh2d_vertex_output
}

@vertex
fn vertex_fn(v_in: Vertex) -> VertexOutput {
    var vout: VertexOutput;

#ifdef VERTEX_UVS
    vout.uv = v_in.uv;
#endif

#ifdef VERTEX_POSITIONS
    vout.world_position = mesh2d_position_local_to_world(mesh.model, vec4<f32>(v_in.position, 1.0));
    vout.clip_position = mesh2d_position_world_to_clip(vout.world_position);
#endif

#ifdef VERTEX_NORMALS
    out.world_normal = mesh2d_normal_local_to_world(vertex.normal);
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh2d_tangent_local_to_world(mesh.model, vertex.tangent);
#endif

#ifdef VERTEX_COLORS
    vout.color = v_in.color;
#endif
    return vout;
}

struct FragmentInput {
    #import bevy_sprite::mesh2d_vertex_output
};

@fragment
fn fragment_fn(frag_in: FragmentInput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    return frag_in.color;
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}
