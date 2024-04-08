
// vertex

struct Camera {
    position: vec4<f32>,
    mat_view: mat4x4<f32>,
    mat_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    //position not used currently (global direction)
    position: vec4<f32>,
    color: vec4<f32>,
    ambient_color_strength: vec4<f32>,
    diffuse_color_strength: vec4<f32>,
    specular_color_strength: vec4<f32>,
    direction: vec4<f32>,
}
@group(0) @binding(1)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) world_normal: vec4<f32>,
    @location(2) world_position: vec4<f32>,
};

@vertex
fn vs_main(vert: VertexInput) -> VertexOutput {
	var out: VertexOutput;
	out.position = camera.mat_proj * camera.mat_view * vert.position;
	out.color = vert.color;
    out.world_normal = vert.normal;
    out.world_position = vert.position;
	return out;
}

// fragment

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ambient_color = light.ambient_color_strength.rgb * light.ambient_color_strength.a;
    let diffuse_color = light.diffuse_color_strength.rgb * light.diffuse_color_strength.a * dot(in.world_normal.xyz, light.direction.xyz);
    let view_dir = normalize(camera.position.xyz - in.world_position.xyz);
    let half_dir = normalize(view_dir + light.direction.xyz);
    let specular_color = pow(max(dot(half_dir, in.world_normal.xyz), 0.0), 32.0) * diffuse_color;
    let ret = (ambient_color + diffuse_color + specular_color) * in.color.rgb;
    /* let ret = (ambient_color + diffuse_color) * in.color.rgb; */
    /* let ret = (specular_color) * in.color.rgb; */
    return vec4<f32>(ret, in.color.a);
}

