#version 450

layout(location = 0) in vec4 frag_color;

layout(location = 0) out vec4 out_color;

layout(set = 0, binding = 0) uniform sampler2D albedo;

void main() {
    out_color = frag_color;
}