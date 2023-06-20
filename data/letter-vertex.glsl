#version 100

#define M_PI 3.1415926535897932384626433832795
#define TILE_SIZE (2.0 / 10.0)

uniform mat4 mvp;

attribute vec2 position;
attribute vec2 tex_coord;
attribute vec2 rotation;

varying vec2 v_tex_coord;

void
main()
{
        float radius = position.y - rotation.x;
        float angle = rotation.y * M_PI;
        float y = radius * cos(angle) + rotation.x;
        float z = radius * sin(angle);

        // Make the tile bob towards the user as it turns
        z += (0.5 - abs(fract(rotation.y) - 0.5)) * 2.0 * TILE_SIZE;

        gl_Position = mvp * vec4(position.x, y, z, 1.0);
        v_tex_coord = tex_coord;
}
