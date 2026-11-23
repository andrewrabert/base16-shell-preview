#!/usr/bin/env python3
import argparse
import curses
import functools
import os
import pathlib
import signal
import subprocess
import sys

__version__ = '1.1.0'

THEME_PATH = pathlib.Path('~/.base16_theme').expanduser()
FISH_THEME_PATH = pathlib.Path('~/.base16_theme.fish').expanduser()

SHELL = '/bin/sh'

NUM_COLORS = 22


class Theme:
    def __init__(self, path):
        self.path = pathlib.Path(path)

        self.name = self.path.stem
        prefix = 'base16-'
        if self.name.startswith(prefix):
            self.name = self.name[len(prefix):]

    def apply(self):
        subprocess.run([SHELL, self.path], check=True)

    def install(self):
        try:
            THEME_PATH.unlink()
        except FileNotFoundError:
            pass
        THEME_PATH.symlink_to(self.path)

        try:
            FISH_THEME_PATH.unlink()
        except FileNotFoundError:
            pass
        fish_path = self.path.parent.parent.joinpath(
            'fish', f'{self.path.stem}.fish')
        if fish_path.is_file():
            FISH_THEME_PATH.symlink_to(fish_path)

        hooks_dir = os.environ.get('BASE16_SHELL_HOOKS')
        if hooks_dir is not None:
            env = os.environ.copy()
            env['BASE16_THEME'] = self.name
            for path in pathlib.Path(hooks_dir).iterdir():
                if path.is_file():
                    subprocess.run(
                        [path],
                        env=env,
                        stderr=subprocess.DEVNULL,
                        stdout=subprocess.DEVNULL,
                        check=True
                    )

    @property
    @functools.lru_cache()
    def bg_color(self):
        background_line = [
            line
            for line in self.path.read_text().split('\n')
            if line.startswith('color00')
        ][0]
        background_str = background_line.split('"')[1].replace('/', '')
        return hex(int(background_str, 16))


class ScrollListWindow:
    def __init__(self, data, lines, left_cols, right_cols):
        self.data = data

        self.lines = lines
        self.left_cols = left_cols
        self.right_cols = right_cols

        self.offset = 0
        self.selected = 0
        self.left_window = curses.newwin(lines, left_cols)
        self.right_window = curses.newwin(lines, right_cols, 0, self.left_cols)

    def up(self):
        self.index -= 1
        self.render()

    @property
    def index(self):
        return self.offset + self.selected

    @index.setter
    def index(self, index):
        if index < 0:
            index = 0
        elif index >= len(self.data):
            index = len(self.data) - 1

        if index < (self.index):
            diff = self.index - index

            available = self.selected
            self.selected -= min(diff, available)
        elif index > (self.index):
            diff = index - self.index

            available = self.lines - 1 - self.selected
            self.selected += min(diff, available)

        self.offset = index - self.selected

    def down(self):
        self.index += 1
        self.render()

    def up_page(self):
        self.selected = 0
        self.index -= self.lines
        self.render()

    def down_page(self):
        self.selected = self.lines - 1
        self.index += self.lines
        self.render()

    def top(self):
        self.index = 0
        self.render()

    def bottom(self):
        self.index = len(self.data) - 1
        self.render()

    def render(self):
        self._render_left()
        self._render_right()
        self.left_window.refresh()
        self.right_window.refresh()

    def _render_left(self):
        end = self.offset + self.lines
        line = 0
        for i, value in enumerate(self.data):
            if i < self.offset:
                continue
            elif i == end:
                break

            if line == self.selected:
                self.value = value
                attrs = curses.A_REVERSE
            else:
                attrs = 0

            self.left_window.addstr(
                line, 0, (self.left_cols - 1) * ' ', attrs)
            self.left_window.addstr(
                line, 0, self.format_left(value)[:self.left_cols-1],
                attrs)
            line += 1

    def _render_right(self):
        for i in range(self.lines):
            curses.init_pair(i, i, -1)
            text = f'color{i:02d} '
            spaces = self.right_cols - len(text) - 1

            self.right_window.addstr(i, len(text), spaces*' ',
                                     curses.color_pair(i) + curses.A_REVERSE)
            self.right_window.addstr(i, 0, text, curses.color_pair(i))


class ThemePreviewer(ScrollListWindow):

    def __init__(self, *args, selected_theme, **kwargs):
        super().__init__(*args, **kwargs)
        self.theme_indexes = {}
        for i, theme in enumerate(self.data):
            self.theme_indexes[theme.name] = i
        if selected_theme:
            self.index = self.theme_indexes[selected_theme.name]

    @staticmethod
    def format_left(theme):
        return theme.name

    @staticmethod
    def format_right(theme):
        pass

    def render(self):
        super().render()
        self.value.apply()


def run_curses_app(stdscr, themes):
    stdscr.refresh()
    stdscr.keypad(True)
    curses.start_color()
    curses.use_default_colors()
    curses.curs_set(0)
    curses.noecho()

    scroll_list_cols = 35
    preview_cols = 42

    total_cols = scroll_list_cols + preview_cols

    win = ThemePreviewer(
        themes,
        NUM_COLORS,
        scroll_list_cols,
        preview_cols,
        selected_theme=get_installed_theme(),
    )

    win.render()

    movement_map = {
        curses.KEY_DOWN: win.down,
        curses.KEY_UP: win.up,
        curses.KEY_PPAGE: win.up_page,
        curses.KEY_NPAGE: win.down_page,
        curses.KEY_HOME: win.top,
        curses.KEY_END: win.bottom,
    }

    while True:
        win.render()
        c = stdscr.getch()

        if c in movement_map:
            movement_map[c]()
        elif c == ord('q'):
            end_run()
            return
        elif c == ord('\n'):
            win.value.install()
            end_run()
            return
        elif c == curses.KEY_RESIZE:
            if curses.LINES < win.lines:
                raise ValueError('Terminal has less than 22 lines.')
            elif curses.COLS < total_cols:
                raise ValueError(f'Terminal has less than {total_cols} cols.')


def get_installed_theme():
    if THEME_PATH.exists():
        return Theme(THEME_PATH.readlink())


def end_run(*_):
    theme = get_installed_theme()
    if theme:
        theme.apply()


def _exit_signal_handler(*_):
    end_run()
    sys.exit()


def main():
    parser = argparse.ArgumentParser(
        'base16-shell-preview',
        formatter_class=argparse.RawTextHelpFormatter,
        description="""
keys:
  up/down      move 1
  pgup/pgdown  move page
  home/end     go to beginning/end
  q            quit
  enter        enable theme and quit
""")
    parser.add_argument(
        '--version',
        action='version',
        version=__version__
    )
    parser.add_argument(
        '--sort-bg',
        action='store_true',
        help='sort themes by background (darkest to lightest)'
    )
    parser.add_argument(
        '-l', '--list',
        action='store_true',
        help='print a list of themes and exit'
    )
    parser.add_argument(
        'theme',
        nargs='?',
        help='set this theme and exit'
    )
    args = parser.parse_args()

    for signalnum in [signal.SIGINT, signal.SIGTERM]:
        signal.signal(signalnum, _exit_signal_handler)

    base16_shell_dir = os.environ.get('BASE16_SHELL')
    if base16_shell_dir is not None:
        base16_shell_dir = pathlib.Path(base16_shell_dir)
    else:
        if THEME_PATH.is_symlink():
            base16_shell_dir = THEME_PATH.resolve().parent.parent
        else:
            parser.error(
                'please set the BASE16_SHELL environment variable '
                'to the local repository path.'
            )

    sort_key = 'bg_color' if args.sort_bg else 'name'
    scripts_dir = base16_shell_dir.joinpath('scripts')
    themes = sorted(
        [Theme(path)
         for path in scripts_dir.iterdir()
         if path.is_file()],
        key=lambda x: (getattr(x, sort_key), x.name)
    )

    if args.theme:
        found_theme = None
        for theme in themes:
            if theme.name == args.theme:
                found_theme = theme
                break
        if found_theme:
            found_theme.apply()
            found_theme.install()
        else:
            parser.error('theme not found')
    elif args.list:
        print('\n'.join(theme.name for theme in themes))
    else:
        curses.wrapper(run_curses_app, themes)


if __name__ == '__main__':
    main()
