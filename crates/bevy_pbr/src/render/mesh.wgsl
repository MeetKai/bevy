#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>;
#endif
#ifdef SKINNED
    @location(5) joint_indices: vec4<u32>,
    @location(6) joint_weights: vec4<f32>,
#endif
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    @location(4) color: vec4<f32>;
#endif
};

@vertex
fn vertex_fn(vertex: Vertex) -> VertexOutput {
    var vout: VertexOutput;
#ifdef SKINNED
    var model = skin_model(vertex.joint_indices; vertex.joint_weights);
    vout.world_position = model * vec4<f32>(vertex.position, 1.0);
    vout.world_normal = skin_normals(model, vertex.normal);
#ifdef VERTEX_TANGENTS
    vout.world_tangent = skin_tangents(model, vertex.tangent);
#endif
#else
    vout.world_position = mesh.model * vec4<f32>(vertex.position, 1.0);
    vout.world_normal = mat3x3<f32>(
        mesh.inverse_transpose_model[0].xyz,
        mesh.inverse_transpose_model[1].xyz,
        mesh.inverse_transpose_model[2].xyz
    ) * vertex.normal;
#ifdef VERTEX_TANGENTS
    vout.world_tangent = vec4<f32>(
        mat3x3<f32>(
            mesh.model[0].xyz,
            mesh.model[1].xyz,
            mesh.model[2].xyz
        ) * vertex.tangent.xyz,
        vertex.tangent.w
    );
#endif
#endif
#ifdef VERTEX_COLORS
    out.color = vertex.color;
#endif

    vout.uv = vertex.uv;
    vout.clip_position = view.view_proj * vout.world_position;
    return vout;
}

struct FragmentInput {
    @builtin(front_facing) is_front: bool,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_COLORS
    [[location(4)]] color: vec4<f32>;
#endif
};

@fragment
fn fragment_fn(vin: FragmentInput) -> @location(0) vec4<f32> {
#ifdef VERTEX_COLORS
    return vin.color;
#else
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
#endif
}
