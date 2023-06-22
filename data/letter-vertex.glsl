#version 100

/*
 * Verda Ŝtelo - An anagram game in Esperanto for the web
 * Copyright (C) 2011, 2013  Neil Roberts
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

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
