// Diveno – A word game in Esperanto
// Copyright (C) 2023  Neil Roberts
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import init_wasm, { Diveno } from "./pkg/diveno.js";

function start_game(diveno) {
  let canvas = document.getElementById("canvas");
  canvas.style.display = "block";

  let redrawQueued = false;

  function redrawCb() {
    redrawQueued = false;

    if (diveno.redraw())
      queueRedraw();
  }

  function queueRedraw() {
    if (redrawQueued)
      return;

    redrawQueued = true;
    window.requestAnimationFrame(redrawCb);
  }

  function handleSizeChange() {
    let rect = canvas.getBoundingClientRect();
    canvas.width = rect.width;
    canvas.height = rect.height;
    diveno.update_fb_size(rect.width, rect.height);
    queueRedraw();
  }

  handleSizeChange();

  let observer = new ResizeObserver(handleSizeChange);

  observer.observe(canvas);

  window.addEventListener("keydown", (event) => {
    let needRedraw = false;

    if (event.key == "Enter")
      needRedraw = diveno.press_enter();
    else if (event.key == "Backspace")
      needRedraw = diveno.press_backspace();
    else if (event.key == "Dead")
      needRedraw = diveno.press_dead_key();
    else if (event.key.length == 1) {
      let code = event.key.charCodeAt(0);
      if (code >= 65) {
        needRedraw = diveno.press_letter_key(event.key);
      }
    }
    if (needRedraw)
      queueRedraw();
  });
}

function queue_data_download(diveno) {
  let next_filename = diveno.next_data_filename();

  if (next_filename) {
    fetch("data/" + next_filename)
      .then(response => {
        if (!response.ok) {
          throw new Error(`HTTP error! Status: ${response.status}`);
        }

        return response.text();
      })
      .then(response => {
        diveno.data_loaded(response);
        queue_data_download(diveno);
      })
      .catch(error => {
        console.log(`Error loading “${next_filename}”: ${error}`);
      });
  } else if (diveno.is_ready()) {
    start_game(diveno);
  }
}

function queue_image_download(diveno) {
  let next_filename = diveno.next_image_filename();

  if (next_filename) {
    let image = new Image();

    image.onload = function() {
      diveno.image_loaded(image);
      queue_image_download(diveno);
    };

    image.src = "data/" + next_filename;
  } else if (diveno.is_ready()) {
    start_game(diveno);
  }
}

init_wasm().then(() => {
  let diveno = new Diveno();

  queue_data_download(diveno);
  queue_image_download(diveno);
});
