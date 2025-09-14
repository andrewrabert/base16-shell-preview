# Changelog

## 1.1.0 (2025-02-12)
- Add `-l`, `--list` option to print themes and exit
- Add optional positional argument to set current theme and exit
- Start preview listing on current theme when possible

## 1.0.1 (2025-02-12)
- Fix curses.endwin exception on exit

## 1.0.0 (2019-10-20)
- Improve scrolling performance
- Check exit code of subprocesses
- Drop support for Python < 3.6

## 0.6.3 (2019-08-14)
- Fix error when current theme symlink is broken

## 0.6.2 (2019-08-10)
- Restore original theme upon SIGINT or SIGTERM

## 0.6.1 (2019-08-08)
- Add empty __init__.py to package

## 0.6.0 (2019-06-07)
- Add --sort-bg flag to sort on background color (starting with darkest)
- Fix exiting when no theme selected and ~/.base16_shell does not exist

## 0.5.1 (2019-06-06)
- Read Base16 Shell path from BASE16_SHELL when no theme is active

## 0.5.0 (2019-06-06)
- Create symlink when activating theme (works with all shells)
- Add support for env var BASE16_SHELL_HOOKS script directory
- Add support for running the module (ex. `python -m base16_shell_preview`)
- Fix execution when script not installed

## 0.4.0 (2019-06-05)
- Add --version flag
- Fix fish shell

## 0.3.0 (2018-03-07)
- Add page up/page down
- Add home/end

## 0.2.0 (2018-02-04)
- Permanently enable theme with enter key
- Add -h / --help argument

## 0.1.0 (2018-02-03)
- Initial release.