use glam::*;
use crate::math::hasher::*;

pub mod hasher;
pub mod octree;
pub mod direction;
pub mod generator;

// util functions {{{

// math{{{

#[inline]
pub fn positive_modulo(n: i32, m: i32) -> i32 { (n % m + m) % m }

#[inline]
pub fn floor_div(x: i32, y: i32) -> i32
{
    let mut q = x / y;
    let r = x % y;
    if r != 0 && (r < 0) != (y < 0) { q -= 1; }
    q
}

#[inline]
pub fn dfloor_div(x: f64, y: f64) -> i32
{
    let mut q = (x / y) as i32;
    let r = x % y;
    if r != 0.0 && (r < 0.0) != (y < 0.0) { q -= 1; }
    q
}

// fn isign(a: i32) -> i32 {if a > 0 {1} else if a < 0 {-1} else {0}}
// fn dsign(a: f64) -> f64 {if a > 0.0 {1.0} else if a < 0.0 {-1.0} else {0.0}}

#[inline]
pub fn isign(a: i32) -> i32 {if a > 0 {1} else {-1}}
#[inline]
pub fn dsign(a: f64) -> f64 {if a > 0.0 {1.0} else {-1.0}}

#[inline]
pub fn is_intersection(a: f64, b: f64) -> bool {dsign(a) != dsign(b)}

//}}}

// distance functions{{{

#[inline]
pub fn df_torus(pos: DVec3, ax: DVec3, t: DVec2) -> f64
{
    let l = (pos * (dvec3(1.0, 1.0, 1.0) - ax)).length();
    let l2 = (pos * ax).length();
    (dvec2(l - t.x, l2)).length() - t.y
}

#[inline]
pub fn df_sphere(pos: DVec3, r: f64) -> f64
{
    pos.length() - r
}

#[inline]
pub fn df_plane(pos: DVec3, n: DVec3, h: f64) -> f64
{
    pos.dot(n.normalize()) + h
}

#[inline]
pub fn df_cylinder(pos: DVec3, ax: DVec3, c: f64) -> f64
{
    (pos * ax).length() - c
}

//}}}

// matrix{{{

#[inline]
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

#[inline]
pub fn mat_rotation_x (theta: f64) -> DMat4
{
    dmat4(
        dvec4(1.0,  0.0,         0.0,         0.0),
        dvec4(0.0,  theta.cos(), theta.sin(), 0.0),
        dvec4(0.0, -theta.sin(), theta.cos(), 0.0),
        dvec4(0.0,  0.0,         0.0,         1.0),
    )
}

#[inline]
pub fn mat_rotation_y (theta: f64) -> DMat4
{
    dmat4(
        dvec4(theta.cos(), 0.0, -theta.sin(), 0.0),
        dvec4(0.0,         1.0, 0.0,          0.0),
        dvec4(theta.sin(), 0.0, theta.cos(),  0.0),
        dvec4(0.0,         0.0, 0.0,          1.0),
    )
}

#[inline]
pub fn mat_rotation_z (theta: f64) -> DMat4
{
    dmat4(
        dvec4(theta.cos(), -theta.sin(), 0.0, 0.0),
        dvec4(theta.sin(),  theta.cos(), 0.0, 0.0),
        dvec4(0.0,          0.0,         1.0, 0.0),
        dvec4(0.0,          0.0,         0.0, 1.0),
    )
}

#[inline]
pub fn mat_rotation (theta: DVec3) -> DMat4
{
    mat_rotation_z(theta.z) * mat_rotation_y(theta.y) * mat_rotation_x(theta.x)
}

#[inline]
pub fn mat_translation (t: DVec3) -> DMat4
{
    dmat4(
        dvec4(1.0, 0.0, 0.0, 0.0),
        dvec4(0.0, 1.0, 0.0, 0.0),
        dvec4(0.0, 0.0, 1.0, 0.0),
        dvec4(t.x, t.y, t.z, 1.0),
    )
}

#[inline]
pub fn mat_scale (s: DVec3) -> DMat4
{
    dmat4(
        dvec4(s.x, 0.0, 0.0, 0.0),
        dvec4(0.0, s.y, 0.0, 0.0),
        dvec4(0.0, 0.0, s.z, 0.0),
        dvec4(0.0, 0.0, 0.0, 1.0),
    )
}

#[inline]
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

#[inline]
pub fn mat_look_at (pos: DVec3, rot: DMat4) -> DMat4
{
    mat_translation(pos) * rot
}

//}}}

#[inline]
pub fn intersect_plane (plane_dot: f64, normal: DVec3, line_start: DVec4, line_end: DVec4) -> (DVec4, f64)
{
    let ad = normal.dot(line_start.truncate());
    let bd = normal.dot(line_end.truncate());
    let t = (plane_dot - ad) / (bd - ad);
    (line_start + (line_end - line_start) * t, t)
}

#[inline]
pub fn dist_plane (plane_dot: f64, normal: DVec3, point: DVec3) -> f64
{
    normal.dot(point) - plane_dot
}

#[inline]
pub fn to_dvec3 (vec: IVec3) -> DVec3 { dvec3(vec.x as f64, vec.y as f64, vec.z as f64) }
#[inline]
pub fn to_dvec4 (vec: IVec4) -> DVec4 { dvec4(vec.x as f64, vec.y as f64, vec.z as f64, vec.w as f64) }

//}}}

// coord {{{
    
#[inline]
pub fn key2coord(key: &SeaHashKey) -> IVec3
{
    ivec3(
        i32::from_ne_bytes(key[0..4].try_into().unwrap()),
        i32::from_ne_bytes(key[4..8].try_into().unwrap()),
        i32::from_ne_bytes(key[8..12].try_into().unwrap()),
    )
}

#[inline]
pub fn coord2key(coord: IVec3) -> SeaHashKey
{
    let (x, y, z) = (
        coord.x.to_ne_bytes(),
        coord.y.to_ne_bytes(),
        coord.z.to_ne_bytes(),
    );
    [
        x[0], x[1], x[2], x[3],
        y[0], y[1], y[2], y[3],
        z[0], z[1], z[2], z[3],
    ]
}

#[inline]
pub fn coord2ind(coord: IVec3, size: i32) -> i32 {
    coord.x + (coord.y + coord.z * size) * size
}

#[inline]
pub fn ind2coord(ind: i32, size: i32) -> IVec3 {
    let x = ind % size;
    let r = ind / size;
    let y = r % size;
    let z = r / size;
    ivec3(x, y, z)
}

#[inline]
pub fn coord2pos(coord: IVec3, chunk_coord: IVec3, size: i32) -> DVec3 {
    to_dvec3(coord + chunk_coord * size)
}

#[inline]
pub fn pos2ind(pos: DVec3, size: i32) -> i32 {
    let v = ivec3(
        positive_modulo(pos.x as i32, size),
        positive_modulo(pos.y as i32, size),
        positive_modulo(pos.z as i32, size)
    );
    v.x + (v.y + v.z * size) * size
}

#[inline]
pub fn pos2coord(pos: DVec3, size: i32) -> IVec3 {
    ivec3(
        pos.x as i32 % size,
        pos.y as i32 % size,
        pos.z as i32 % size
    )
}

#[inline]
pub fn pos2chunk(pos: DVec3, size: i32) -> IVec3 {
    ivec3(
        floor_div(pos.x as i32, size),
        floor_div(pos.y as i32, size),
        floor_div(pos.z as i32, size)
    )
}

#[inline]
pub fn chunk2pos(chunk_coord: IVec3, size: i32) -> DVec3 {
    to_dvec3(chunk_coord * size)
}

#[inline]
pub fn key2mixed(key: SeaHashKey, size: i32) -> (IVec3, IVec3) {
    coord2mixed(key2coord(&key), size)
}

#[inline]
pub fn key2mixedkey(key: SeaHashKey, size: i32) -> (SeaHashKey, SeaHashKey) {
    let (a, b) = key2mixed(key, size);
    (coord2key(a), coord2key(b))
}

#[inline]
pub fn coord2mixed(mixed_coord: IVec3, size: i32) -> (IVec3, IVec3) {
    let coord = ivec3(
        mixed_coord.x % size,
        mixed_coord.y % size,
        mixed_coord.z % size,
    );
    let chunk = mixed_coord - coord;
    (chunk, coord)
}

// }}}

