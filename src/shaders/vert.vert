#version 140

in vec2 position;
in vec2 uv;

out vec2 v_uv;

uniform mat4 p;
uniform vec2 pos;
uniform vec2 rots;

void main() {
    mat4 trans;
    trans[0] = vec4(1, 0, 0, pos.x);
    trans[1] = vec4(0, 1, 0, pos.y);
    trans[2] = vec4(0, 0, 1, 0);
    trans[3] = vec4(0, 0, 0, 1);
    trans = transpose(trans);

    mat4 rot;
    rot[0] = vec4(rots.y, -rots.x, 0, 0);
    rot[1] = vec4(rots.x, rots.y, 0, 0);
    rot[2] = vec4(0, 0, 1, 0);
    rot[3] = vec4(0, 0, 0, 1);
    rot = transpose(rot);

    gl_Position = p * trans * rot * vec4(position, 0.0, 1.0);
    v_uv = uv;
}