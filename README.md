# Base16 Shell Preview

A tool to preview and enable [Base16 Shell](https://github.com/chriskempson/base16-shell) themes in your terminal.

![Preview GIF](https://raw.githubusercontent.com/nvllsvm/base16-shell-preview/master/preview.gif)

## Installation

Base16 Shell must also be present.

```shell
cargo install base16-shell-preview
```

## Usage

Before running, ensure that the `BASE16_SHELL` environment variable is set to the local
[Base16 Shell](https://github.com/chriskempson/base16-shell) path or that a theme is already symlinked to `~/.base16_theme`.

```
base16-shell-preview -h
usage: base16-shell-preview [-h] [--version] [--sort-bg] [-l|--list] [theme]

keys:
  up/down      move 1
  pgup/pgdown  move page
  home/end     go to beginning/end
  q            quit
  enter        enable theme and quit

optional arguments:
  -h, --help  show this help message and exit
  --version   show program's version number and exit
  --sort-bg   sort themes by background (darkest to lightest)
  -l, --list  print a list of themes and exit
  theme        set this theme and exit
```
