import init_wasm, { Diveno } from "./pkg/diveno.js";

function queue_download(diveno) {
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
        queue_download(diveno);
      })
      .catch(error => {
        console.log(`Error loading “${next_filename}”: ${error}`);
      });
  } else {
    let canvas = document.getElementById("canvas");
    canvas.style.display = "block";
    let rect = canvas.getBoundingClientRect();
    diveno.update_fb_size(rect.width, rect.height);
    diveno.redraw();
  }
}

init_wasm().then(() => {
  let diveno = new Diveno();

  queue_download(diveno);
});
