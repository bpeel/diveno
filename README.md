# Diveno

This is a work-in-progress tool to host a game of [MOTUS/Lingo](https://en.wikipedia.org/wiki/Lingo_(American_game_show)) in Esperanto over video conferencing.

## Native build

You can run the game natively using SDL if your GL driver supports the GLES2.0 API by typing:

```bash
cargo run
```

## WASM build

You can also run the game as a website using WebGL+WASM. You need to install `wasm-pack` and then you can compile the program like this:

```bash
wasm-pack build --target=web
```

Then you need to run a local webserver pointing to the repository directory. One way to do this is with Python’s twisted server like this:

```
pip3 install twisted
python3 -m twisted web --path="$PWD"
```
