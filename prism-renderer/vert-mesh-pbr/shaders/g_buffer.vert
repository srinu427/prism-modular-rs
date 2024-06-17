#version 450

layout(location = 0) in vec4 in_position;
layout(location = 1) in vec4 in_normal;
layout(location = 2) in vec4 in_tangent;
layout(location = 3) in vec4 in_uv_coordinates;

layout(location = 0) out vec4 frag_color;

layout(set = 0, binding = 0) uniform CameraTransform{
    mat4 view;
    mat4 proj;
} cam_transform;

void main() {
    vec4 lol = cam_transform.view * in_position;
    gl_Position = vec4(lol.x, lol.y, 1.0, 1.0);
    frag_color = vec4(1.0, 1.0, 1.0, 1.0);
}