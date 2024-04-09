use glam::*;

// util functions {{{

// math{{{

pub fn positive_modulo(n: i32, m: i32) -> i32 { (n % m + m) % m }

pub fn floor_div(x: i32, y: i32) -> i32
{
    let mut q = x / y;
    let r = x % y;
    if r != 0 && (r < 0) != (y < 0) { q -= 1; }
    q
}

pub fn dfloor_div(x: f64, y: f64) -> i32
{
    let mut q = (x / y) as i32;
    let r = x % y;
    if r != 0.0 && (r < 0.0) != (y < 0.0) { q -= 1; }
    q
}

// fn isign(a: i32) -> i32 {if a > 0 {1} else if a < 0 {-1} else {0}}
// fn dsign(a: f64) -> f64 {if a > 0.0 {1.0} else if a < 0.0 {-1.0} else {0.0}}

pub fn isign(a: i32) -> i32 {if a > 0 {1} else {-1}}
pub fn dsign(a: f64) -> f64 {if a > 0.0 {1.0} else {-1.0}}

pub fn is_intersection(a: f64, b: f64) -> bool {dsign(a) != dsign(b)}

//}}}

// distance functions{{{

pub fn df_torus(pos: DVec3, ax: DVec3, t: DVec2) -> f64
{
    let l = (pos * (dvec3(1.0, 1.0, 1.0) - ax)).length();
    let l2 = (pos * ax).length();
    (dvec2(l - t.x, l2)).length() - t.y
}

pub fn df_sphere(pos: DVec3, r: f64) -> f64
{
    pos.length() - r
}

pub fn df_plane(pos: DVec3, n: DVec3, h: f64) -> f64
{
    pos.dot(n.normalize()) + h
}

pub fn df_cylinder(pos: DVec3, ax: DVec3, c: f64) -> f64
{
    (pos * ax).length() - c
}

//}}}

// matrix{{{

pub fn mat_projection (fov: f64, ratio: f64, near: f64, far: f64) -> DMat4
{
    let fov_rad = 1.0 / (fov * 0.5 / 180.0 * std::f64::consts::PI).tan();

    // dmat4(
    //     ratio * fov_rad, 0.0,     0.0,                                0.0,
    //     0.0,             fov_rad, 0.0,                                0.0,
    //     0.0,             0.0,     (far + near) / (near - far),       -1.0,
    //     0.0,             0.0,     (2.0 * far * near) / (near - far),  0.0
    // )

    // dmat4(
    //     ratio * fov_rad, 0.0,     0.0,                           0.0,
    //     0.0,             fov_rad, 0.0,                           0.0,
    //     0.0,             0.0,     -far / (far - near),          -1.0,
    //     0.0,             0.0,     (-far * near) / (far - near),  0.0
    // )

    // infinite far, reversed z (1..0), RH
    // dmat4(
    //     dvec4(ratio * fov_rad,  0.0,     0.0,                           0.0),
    //     dvec4(0.0,              fov_rad, 0.0,                           0.0),
    //     dvec4(0.0,              0.0,     0.0,                          -1.0),
    //     dvec4(0.0,              0.0,     near,                          0.0),
    // )

    // infinite far, reversed z (1..0), LH
    dmat4(
        dvec4(ratio * fov_rad,  0.0,     0.0,                           0.0),
        dvec4(0.0,              fov_rad, 0.0,                           0.0),
        dvec4(0.0,              0.0,     0.0,                           1.0),
        dvec4(0.0,              0.0,     near,                          0.0),
    )

}

pub fn mat_rotation_x (theta: f64) -> DMat4
{
    dmat4(
        dvec4(1.0,  0.0,         0.0,         0.0),
        dvec4(0.0,  theta.cos(), theta.sin(), 0.0),
        dvec4(0.0, -theta.sin(), theta.cos(), 0.0),
        dvec4(0.0,  0.0,         0.0,         1.0),
    )
}

pub fn mat_rotation_y (theta: f64) -> DMat4
{
    dmat4(
        dvec4(theta.cos(), 0.0, -theta.sin(), 0.0),
        dvec4(0.0,         1.0, 0.0,          0.0),
        dvec4(theta.sin(), 0.0, theta.cos(),  0.0),
        dvec4(0.0,         0.0, 0.0,          1.0),
    )
}

pub fn mat_rotation_z (theta: f64) -> DMat4
{
    dmat4(
        dvec4(theta.cos(), -theta.sin(), 0.0, 0.0),
        dvec4(theta.sin(),  theta.cos(), 0.0, 0.0),
        dvec4(0.0,          0.0,         1.0, 0.0),
        dvec4(0.0,          0.0,         0.0, 1.0),
    )
}

pub fn mat_rotation (theta: DVec3) -> DMat4
{
    mat_rotation_z(theta.z) * mat_rotation_y(theta.y) * mat_rotation_x(theta.x)
}

pub fn mat_translation (t: DVec3) -> DMat4
{
    dmat4(
        dvec4(1.0, 0.0, 0.0, 0.0),
        dvec4(0.0, 1.0, 0.0, 0.0),
        dvec4(0.0, 0.0, 1.0, 0.0),
        dvec4(t.x, t.y, t.z, 1.0),
    )
}

pub fn mat_scale (s: DVec3) -> DMat4
{
    dmat4(
        dvec4(s.x, 0.0, 0.0, 0.0),
        dvec4(0.0, s.y, 0.0, 0.0),
        dvec4(0.0, 0.0, s.z, 0.0),
        dvec4(0.0, 0.0, 0.0, 1.0),
    )
}

pub fn mat_quick_inv (mat: DMat4) -> DMat4
{
    dmat4(
        dvec4(mat.col(0)[0], mat.col(1)[0], mat.col(2)[0], 0.0),
        dvec4(mat.col(0)[1], mat.col(1)[1], mat.col(2)[1], 0.0),
        dvec4(mat.col(0)[2], mat.col(1)[2], mat.col(2)[2], 0.0),
        dvec4(
            -(mat.col(3)[0] * mat.col(0)[0] + mat.col(3)[1] * mat.col(0)[1] + mat.col(3)[2] * mat.col(0)[2]),
            -(mat.col(3)[0] * mat.col(1)[0] + mat.col(3)[1] * mat.col(1)[1] + mat.col(3)[2] * mat.col(1)[2]),
            -(mat.col(3)[0] * mat.col(2)[0] + mat.col(3)[1] * mat.col(2)[1] + mat.col(3)[2] * mat.col(2)[2]),
            1.0
        ),
    )
}

pub fn mat_look_at (pos: DVec3, rot: DMat4) -> DMat4
{
    mat_translation(pos) * rot
}

//}}}

pub fn intersect_plane (plane_dot: f64, normal: DVec3, line_start: DVec4, line_end: DVec4) -> (DVec4, f64)
{
    let ad = normal.dot(line_start.truncate());
    let bd = normal.dot(line_end.truncate());
    let t = (plane_dot - ad) / (bd - ad);
    (line_start + (line_end - line_start) * t, t)
}

pub fn dist_plane (plane_dot: f64, normal: DVec3, point: DVec3) -> f64
{
    normal.dot(point) - plane_dot
}

pub fn to_dvec3 (vec: IVec3) -> DVec3 { dvec3(vec.x as f64, vec.y as f64, vec.z as f64) }
pub fn to_dvec4 (vec: IVec4) -> DVec4 { dvec4(vec.x as f64, vec.y as f64, vec.z as f64, vec.w as f64) }

//}}}

