#import bevy_sprite::mesh2d_view_bind_group
#import bevy_sprite::mesh2d_struct

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef VERTEX_TANGENTS
    @location(3) tangent: vec4<f32>,
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
};

@group(0) @binding(0)
var<uniform> view: View;

@group(2) @binding(0)
var<uniform> mesh: Mesh2d;

@vertex
fn vertex_fn(vertex: Vertex) -> VertexOutput {
    let world_position = mesh.model * vec4<f32>(vertex.position, 1.0);

    var vout: VertexOutput;
    vout.uv = vertex.uv;
    vout.world_position = world_position;
    vout.clip_position = view.view_proj * world_position;
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
};

@fragment
fn fragment_fn(vin: FragmentInput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 1.0, 1.0);
}