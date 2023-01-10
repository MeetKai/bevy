use bevy_math::Mat4;

//Gets y fov in radians from a perspective matrix
pub fn fov_from_mat4(mat: Mat4) -> f32 {
    let f = mat.y_axis.y;
    let fov_y_radians = 2.0 * (1.0 / f).atan();
    fov_y_radians
}
