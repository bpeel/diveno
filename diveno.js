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
  }
}

init_wasm().then(() => {
  let diveno = new Diveno();

  queue_download(diveno);
});
