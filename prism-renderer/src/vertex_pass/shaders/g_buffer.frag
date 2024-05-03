#version 450

layout(location = 0) in vec2 tex_coords;

layout(location = 0) out vec4 out_color;

layout(set = 1, binding = 0) uniform sampler2D albedo;

void main() {
    vec3 albedo_color = texture(albedo, tex_coords).xyz;
    out_color = vec4(albedo_color, 1.0f);
}