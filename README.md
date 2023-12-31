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

## Keys

The game is meant to be played with a host who makes sure the rules are followed. This means the program is more just a tool to host the game and it doesn’t enforce the rules. The host can be lenient and let teams off for simple mistakes. To manage the game the host needs to remember some keyboard shortcuts. The game is split into three pages: the left team bingo grid, the word puzzle and the right team bingo grid. The keys do different things depending on which page is visible:

### Any page

| Key | Action |
| --- | ------ |
| Left, Right | Switch between the pages |
| Up, Down | Add or subtract 10 points to the appropriate team. This is intended to be able to undo mistakes. |
| Space | In normal mode this switches teams. When a word is solved the points will be added to the current team. In Super Diveno mode this starts or pauses the countdown. |

### Word puzzle page

| Key | Action |
| --- | ------ |
| Any letter key | Add a letter to the current guess. You can type an X to add a hat to the previous letter. On the website version, if you have a dead key in your keyboard layout you can use that to type a hat too. It doesn’t work in the native SDL version though. |
| Enter | Enter the current guess. If it’s not a word in the dictionary it will be rejected. |
| Backspace | Remove the last letter in the current guess. |
| Delete | Reject a guess. Normally you would do this after a team suggests an invalid word before passing over to the other team. |
| Page down | Add a letter hint. Normally you would do this before passing to the othear team. |
| Home | Pick a new word and reset the word grid. |
| ` or $ | Toggle Super Diveno mode. In Super Diveno mode a countdown timer will be shown instead of the player scores and the number of guessed words will be counted. |

### Bingo grid page

| Key | Action |
| --- | ------ |
| Enter | Spin the tombola and pick a ball. If the ball has a number then it will turn red on the bingo grid. If that forms a line then the corresponding team will receive 100 points. |
| Home | Pick a new bingo grid for the chosen team and put all the balls back in the tombola. |
