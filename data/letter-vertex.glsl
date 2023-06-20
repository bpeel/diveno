#version 100

uniform mat4 mvp;

attribute vec2 position;
attribute vec2 tex_coord;
attribute vec2 rotation;

varying vec2 v_tex_coord;

void
main()
{
        float radius = position.y - rotation.x;
        float y = radius * cos(rotation.y) + rotation.x;
        float z = radius * sin(rotation.y);
        gl_Position = mvp * vec4(position.x, y, z, 1.0);
        v_tex_coord = tex_coord;
}
