#version 300 es

out vec2 tex_coord;

void
main()
{
        gl_Position = vec4(float(gl_VertexID & 1) - 0.5,
                           float((gl_VertexID >> 1) & 1) - 0.5,
                           0.0,
                           1.0);
        tex_coord = vec2(gl_Position.s + 0.5,
                         1.0 - (gl_Position.t + 0.5));
}
