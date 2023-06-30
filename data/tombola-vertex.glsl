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

uniform vec2 translation;
uniform vec2 scale;
uniform float rotation;

attribute vec2 position;
attribute vec2 tex_coord_attrib;

varying vec2 tex_coord;

void
main()
{
        float sin_angle = sin(rotation);
        float cos_angle = cos(rotation);
        vec2 rotated_position =
                vec2(cos_angle * position.x - sin_angle * position.y,
                     sin_angle * position.x + cos_angle * position.y);

        gl_Position = vec4(rotated_position * scale + translation, 0.0, 1.0);

        tex_coord = tex_coord_attrib;
}
