
struct Globals {
    // Camera
    c_position: vec4<f32>,
    mat_view: mat4x4<f32>,
    mat_proj: mat4x4<f32>,

    // Light
    //position not used currently (global direction)
    l_position: vec4<f32>,
    l_color: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    diffuse_color_strength: vec4<f32>,
    specular_color_strength: vec4<f32>,
    l_direction: vec4<f32>,
}

// 64 u8 voxels (x,y, z shifted)
// chunk location agnostic
struct Data {
    data: mat4x4<u32>,
}

struct VertexInput {
    @location(0) position: mat4x4<array<f32, 4>>
};

@group(0) @binding(0)
var<uniform> globals: Globals;

// from Vertex/Mesh, max buffer size 147,456 / 64
@group(1) @binding(0)
var<uniform> data: array<Data, 2304>;
@group(1) @binding(1)
var<uniform> dst: array<VertexInput, 2304>;

@compute
@workgroup_size(8,8,1)
fn cs_main(
    @builtin(global_invocation_id)
    gid: vec3<u32>
){
    
}

