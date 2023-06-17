#version 300 es

void
main()
{
        gl_Position = vec4(float(gl_VertexID & 1) - 0.5,
                           float((gl_VertexID >> 1) & 1) - 0.5,
                           0.0,
                           1.0);
}
