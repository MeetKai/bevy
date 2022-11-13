// If using this WGSL snippet as an #import, a dedicated
// "joint_matricies" uniform of type SkinnedMesh must be added in the
// main shader.

#define_import_path bevy_pbr::skinning

fn skin_model(
    indexes: vec4<u32>,
    weights: vec4<f32>,
) -> mat4x4<f32> {
    return weights.x * joint_matrices.data[indexes.x]
        + weights.y * joint_matrices.data[indexes.y]
        + weights.z * joint_matrices.data[indexes.z]
        + weights.w * joint_matrices.data[indexes.w];
}

fn inverse_transpose_3x3(m_in: mat3x3<f32>) -> mat3x3<f32> {
    let x = cross(m_in[1], m_in[2]);
    let y = cross(m_in[2], m_in[0]);
    let z = cross(m_in[0], m_in[1]);
    let det = dot(m_in[2], z);
    return mat3x3<f32>(
        x / det,
        y / det,
        z / det
    );
}

fn skin_normals(
    model: mat4x4<f32>,
    normal: vec3<f32>,
) -> vec3<f32> {
    return normalize(
        inverse_transpose_3x3(
            mat3x3<f32>(
                model[0].xyz,
                model[1].xyz,
                model[2].xyz
            )
        ) * normal
    );
}
