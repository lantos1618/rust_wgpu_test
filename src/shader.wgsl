struct Uniforms {
    aspect_ratio: f32,
}
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@location(0) position: vec2<f32>) -> VertexOutput {
    var output: VertexOutput;
    // Apply aspect ratio correction to x coordinate
    var corrected_position = vec2<f32>(position.x * uniforms.aspect_ratio, position.y);
    output.position = vec4<f32>(corrected_position, 0.0, 1.0);
    return output;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0); // White color
}
