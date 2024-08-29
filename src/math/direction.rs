use glam::*;

pub struct IDirection;
pub struct DDirection;

impl DDirection
{
    pub const ZERO    : DVec3 = DVec3{x: 0.0, y: 0.0, z: 0.0};
    pub const RIGHT   : DVec3 = DVec3{x: 1.0, y: 0.0, z: 0.0};
    pub const LEFT    : DVec3 = DVec3{x:-1.0, y: 0.0, z: 0.0};
    pub const UP      : DVec3 = DVec3{x: 0.0, y: 1.0, z: 0.0};
    pub const DOWN    : DVec3 = DVec3{x: 0.0, y:-1.0, z: 0.0};
    pub const FORWARD : DVec3 = DVec3{x: 0.0, y: 0.0, z: 1.0};
    pub const BACK    : DVec3 = DVec3{x: 0.0, y: 0.0, z:-1.0};

    pub const EDGE_PAIRS: &[(DVec3, DVec3)] = &[
        ( DVec3{x:0.0, y:0.0, z:0.0}, DVec3{x:1.0, y:0.0, z:0.0} ), // 0 +x
        ( DVec3{x:0.0, y:0.0, z:0.0}, DVec3{x:0.0, y:1.0, z:0.0} ), // 0 +y
        ( DVec3{x:0.0, y:0.0, z:0.0}, DVec3{x:0.0, y:0.0, z:1.0} ), // 0 +z
        ( DVec3{x:1.0, y:0.0, z:0.0}, DVec3{x:0.0, y:1.0, z:0.0} ), // x +y
        ( DVec3{x:1.0, y:0.0, z:0.0}, DVec3{x:0.0, y:0.0, z:1.0} ), // x +z
        ( DVec3{x:0.0, y:1.0, z:0.0}, DVec3{x:1.0, y:0.0, z:0.0} ), // y +x
        ( DVec3{x:0.0, y:1.0, z:0.0}, DVec3{x:0.0, y:0.0, z:1.0} ), // y +z
        ( DVec3{x:0.0, y:0.0, z:1.0}, DVec3{x:1.0, y:0.0, z:0.0} ), // z +x
        ( DVec3{x:0.0, y:0.0, z:1.0}, DVec3{x:0.0, y:1.0, z:0.0} ), // z +y
        ( DVec3{x:1.0, y:1.0, z:0.0}, DVec3{x:0.0, y:0.0, z:1.0} ), // xy + z
        ( DVec3{x:1.0, y:0.0, z:1.0}, DVec3{x:0.0, y:1.0, z:0.0} ), // xz + y
        ( DVec3{x:0.0, y:1.0, z:1.0}, DVec3{x:1.0, y:0.0, z:0.0} ), // yz + x
    ];

}

impl IDirection
{
    pub const ZERO    : IVec3 = IVec3{x: 0, y: 0, z: 0};
    pub const RIGHT   : IVec3 = IVec3{x: 1, y: 0, z: 0};
    pub const LEFT    : IVec3 = IVec3{x:-1, y: 0, z: 0};
    pub const UP      : IVec3 = IVec3{x: 0, y: 1, z: 0};
    pub const DOWN    : IVec3 = IVec3{x: 0, y:-1, z: 0};
    pub const FORWARD : IVec3 = IVec3{x: 0, y: 0, z: 1};
    pub const BACK    : IVec3 = IVec3{x: 0, y: 0, z:-1};

    pub const UNIT_DIRS : &[IVec3] = &[
        Self::ZERO,
        Self::RIGHT,
        Self::UP,
        Self::FORWARD,
    ];

    pub const POSITIVE_DIRS : &[IVec3] = &[
        Self::ZERO,
        Self::RIGHT,
        Self::UP,
        Self::FORWARD,
        IVec3{
            x: Self::RIGHT.x + Self::UP.x,
            y: Self::RIGHT.y + Self::UP.y,
            z: Self::RIGHT.z + Self::UP.z,
        },
        IVec3{
            x: Self::RIGHT.x + Self::FORWARD.x,
            y: Self::RIGHT.y + Self::FORWARD.y,
            z: Self::RIGHT.z + Self::FORWARD.z,
        },
        IVec3{
            x: Self::FORWARD.x + Self::UP.x,
            y: Self::FORWARD.y + Self::UP.y,
            z: Self::FORWARD.z + Self::UP.z,
        },
        IVec3{
            x: Self::RIGHT.x + Self::FORWARD.x + Self::UP.x,
            y: Self::RIGHT.y + Self::FORWARD.y + Self::UP.y,
            z: Self::RIGHT.z + Self::FORWARD.z + Self::UP.z,
        },
    ];

    pub const NEGATIVE_DIRS : &[IVec3] = &[
        Self::ZERO,
        Self::LEFT,
        Self::DOWN,
        Self::BACK,
        IVec3{
            x: Self::LEFT.x + Self::DOWN.x,
            y: Self::LEFT.y + Self::DOWN.y,
            z: Self::LEFT.z + Self::DOWN.z,
        },
        IVec3{
            x: Self::LEFT.x + Self::BACK.x,
            y: Self::LEFT.y + Self::BACK.y,
            z: Self::LEFT.z + Self::BACK.z,
        },
        IVec3{
            x: Self::BACK.x + Self::DOWN.x,
            y: Self::BACK.y + Self::DOWN.y,
            z: Self::BACK.z + Self::DOWN.z,
        },
        IVec3{
            x: Self::LEFT.x + Self::BACK.x + Self::DOWN.x,
            y: Self::LEFT.y + Self::BACK.y + Self::DOWN.y,
            z: Self::LEFT.z + Self::BACK.z + Self::DOWN.z,
        },
    ];

    // index into positive or negative directions
    pub const EDGE_INDS : &[(usize, usize)] = &[
        (0, 1), // 0 +x
        (0, 2), // 0 +y
        (0, 3), // 0 +z
        (1, 4), // x +y
        (1, 5), // x +z
        (2, 4), // y +x
        (2, 6), // y +z
        (3, 5), // z +x
        (3, 6), // z +y
        (4, 7), // xy + z
        (5, 7), // xz + y
        (6, 7), // yz + x
    ];

    pub const EDGE_INDS_X : &[(usize, usize)] = &[
        (0, 1), // 0 +x
        (2, 4), // y +x
        (3, 5), // z +x
        (6, 7), // yz + x
    ];

    pub const EDGE_INDS_Y : &[(usize, usize)] = &[
        (0, 2), // 0 +y
        (1, 4), // x +y
        (3, 6), // z +y
        (5, 7), // xz + y
    ];

    pub const EDGE_INDS_Z : &[(usize, usize)] = &[
        (0, 3), // 0 +z
        (1, 5), // x +z
        (2, 6), // y +z
        (4, 7), // xy + z
    ];

    pub const EDGE_PAIRS : &[(IVec3, IVec3)] = &[
        ( IVec3{x:0, y:0, z:0}, IVec3{x:1, y:0, z:0} ), // 0 +x
        ( IVec3{x:0, y:0, z:0}, IVec3{x:0, y:1, z:0} ), // 0 +y
        ( IVec3{x:0, y:0, z:0}, IVec3{x:0, y:0, z:1} ), // 0 +z
        ( IVec3{x:1, y:0, z:0}, IVec3{x:0, y:1, z:0} ), // x +y
        ( IVec3{x:1, y:0, z:0}, IVec3{x:0, y:0, z:1} ), // x +z
        ( IVec3{x:0, y:1, z:0}, IVec3{x:1, y:0, z:0} ), // y +x
        ( IVec3{x:0, y:1, z:0}, IVec3{x:0, y:0, z:1} ), // y +z
        ( IVec3{x:0, y:0, z:1}, IVec3{x:1, y:0, z:0} ), // z +x
        ( IVec3{x:0, y:0, z:1}, IVec3{x:0, y:1, z:0} ), // z +y
        ( IVec3{x:1, y:1, z:0}, IVec3{x:0, y:0, z:1} ), // xy + z
        ( IVec3{x:1, y:0, z:1}, IVec3{x:0, y:1, z:0} ), // xz + y
        ( IVec3{x:0, y:1, z:1}, IVec3{x:1, y:0, z:0} ), // yz + x
    ];

    // swap 3 and 4 (zyx bits to order in positive dirs))
    pub const BITWISE_TO_DIRS : &[usize] = &[0, 1, 2, 4, 3, 5, 6, 7];

    pub const SFP_INDS : &[(usize, usize, usize)] = &[
        (3, 6, 2),
        (1, 5, 3),
        (2, 4, 1),
    ];

}

