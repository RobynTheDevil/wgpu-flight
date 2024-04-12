
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

struct VertexInput {
    @location(0) position: vec4<f32>
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec4<f32>,
    @location(2) world_position: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: Globals;

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
	var out: VertexOutput;
	out.position = globals.mat_proj * globals.mat_view * vert.position;
	out.color = vec4(0.1, 0.3, 0.5, 1.0);
    out.world_normal = vec4(1.0, 0.0, 0.0, 1.0);
    out.world_position = vert.position;
	return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_color = globals.ambient_color_strength.rgb * globals.ambient_color_strength.a;
    let diffuse_color = globals.diffuse_color_strength.rgb * globals.diffuse_color_strength.a * dot(in.world_normal.xyz, globals.l_direction.xyz);
    let view_dir = normalize(globals.c_position.xyz - in.world_position.xyz);
    let half_dir = normalize(view_dir + globals.l_direction.xyz);
    let specular_color = pow(max(dot(half_dir, in.world_normal.xyz), 0.0), 32.0) * diffuse_color;
    let ret = (ambient_color + diffuse_color + specular_color) * in.color.rgb;
    /* let ret = (ambient_color + diffuse_color) * in.color.rgb; */
    /* let ret = (specular_color) * in.color.rgb; */
    return vec4<f32>(ret, in.color.a);
}

