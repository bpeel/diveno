#version 300 es

precision mediump float;

layout(location = 0) out vec4 color;

in vec2 tex_coord;

uniform sampler2D tex;

void
main()
{
        color = texture2D(tex, tex_coord);
}
