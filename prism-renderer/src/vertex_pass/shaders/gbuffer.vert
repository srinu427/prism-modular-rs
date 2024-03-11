#version 450

layout(location = 0) in vec4 inPosition;
layout(location = 1) in vec4 inNormal;
layout(location = 2) in vec4 inTangent;
layout(location = 3) in vec4 inBiTangent;
layout(location = 4) in vec4 inUVCoordinates;

layout(location = 0) out vec4 fragColor;

void main() {
    gl_Position = inPosition;
    fragColor = vec4(1.0, 1.0, 1.0, 1.0);
}