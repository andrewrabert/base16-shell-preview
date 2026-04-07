use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use crossterm::cursor::Show;
use crossterm::event::{self, Event, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Stylize};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use ratatui::DefaultTerminal;

const NUM_COLORS: u16 = 22;
const LEFT_COLS: u16 = 35;
const RIGHT_COLS: u16 = 42;
const TOTAL_COLS: u16 = LEFT_COLS + RIGHT_COLS;

#[derive(Parser)]
#[command(
    name = "base16-shell-preview",
    version,
    about = "Browse and preview Base16 Shell themes in your terminal.",
    after_help = "\
keys:
  up/down      move 1
  pgup/pgdown  move page
  home/end     go to beginning/end
  q            quit
  enter        enable theme and quit"
)]
struct Cli {
    /// Sort themes by background (darkest to lightest)
    #[arg(long = "sort-bg")]
    sort_bg: bool,

    /// Print a list of themes and exit
    #[arg(short, long)]
    list: bool,

    /// Set this theme and exit
    theme: Option<String>,
}

struct Theme {
    path: PathBuf,
    name: String,
}

impl Theme {
    fn new(path: PathBuf) -> Self {
        let mut name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if let Some(stripped) = name.strip_prefix("base16-") {
            name = stripped.to_owned();
        }
        Self { path, name }
    }

    fn apply(&self) -> io::Result<()> {
        let status = Command::new("/bin/sh").arg(&self.path).status()?;
        if !status.success() {
            return Err(io::Error::other("theme script failed"));
        }
        Ok(())
    }

    fn install(&self) -> io::Result<()> {
        let theme_path = theme_path();
        let _ = fs::remove_file(&theme_path);
        symlink(&self.path, &theme_path)?;

        if let Ok(hooks_dir) = env::var("BASE16_SHELL_HOOKS") {
            let hooks_path = Path::new(&hooks_dir);
            if hooks_path.is_dir() {
                for entry in fs::read_dir(hooks_path)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let _ = Command::new(&path)
                            .env("BASE16_THEME", &self.name)
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
            }
        }

        Ok(())
    }

    fn bg_color(&self) -> Option<u64> {
        let content = fs::read_to_string(&self.path).ok()?;
        let line = content.lines().find(|l| l.starts_with("color00"))?;
        let hex_str = line.split('"').nth(1)?.replace('/', "");
        u64::from_str_radix(&hex_str, 16).ok()
    }
}

struct App {
    themes: Vec<Theme>,
    offset: usize,
    selected: usize,
    should_install: bool,
}

impl App {
    fn new(themes: Vec<Theme>) -> Self {
        let mut app = Self {
            themes,
            offset: 0,
            selected: 0,
            should_install: false,
        };

        if let Some(installed) = get_installed_theme()
            && let Some(pos) = app.themes.iter().position(|t| t.name == installed.name)
        {
            app.set_index(pos);
        }

        app
    }

    fn index(&self) -> usize {
        self.offset + self.selected
    }

    fn set_index(&mut self, index: usize) {
        let index = index.min(self.themes.len().saturating_sub(1));
        let current = self.index();

        if index < current {
            let diff = current - index;
            let available = self.selected;
            self.selected -= diff.min(available);
        } else if index > current {
            let diff = index - current;
            let available = NUM_COLORS as usize - 1 - self.selected;
            self.selected += diff.min(available);
        }

        self.offset = index - self.selected;
    }

    fn current_theme(&self) -> &Theme {
        &self.themes[self.index()]
    }

    fn up(&mut self) {
        let i = self.index().saturating_sub(1);
        self.set_index(i);
    }

    fn down(&mut self) {
        let i = self.index() + 1;
        self.set_index(i);
    }

    fn up_page(&mut self) {
        self.selected = 0;
        let i = self.index().saturating_sub(NUM_COLORS as usize);
        self.set_index(i);
    }

    fn down_page(&mut self) {
        self.selected = NUM_COLORS as usize - 1;
        let i = self.index() + NUM_COLORS as usize;
        self.set_index(i);
    }

    fn top(&mut self) {
        self.set_index(0);
    }

    fn bottom(&mut self) {
        self.set_index(self.themes.len().saturating_sub(1));
    }
}

struct AppWidget<'a> {
    app: &'a App,
}

impl Widget for AppWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::horizontal([
            Constraint::Length(LEFT_COLS),
            Constraint::Length(RIGHT_COLS),
        ])
        .split(area);

        self.render_left(chunks[0], buf);
        self.render_right(chunks[1], buf);
    }
}

impl AppWidget<'_> {
    fn render_left(&self, area: Rect, buf: &mut Buffer) {
        let end = (self.app.offset + NUM_COLORS as usize).min(self.app.themes.len());
        let max_width = (area.width - 1) as usize;

        for (row, theme) in self.app.themes[self.app.offset..end].iter().enumerate() {
            let name: String = theme.name.chars().take(max_width).collect();
            let padded = format!("{:<width$}", name, width = max_width);

            let line = if row == self.app.selected {
                Line::from(padded.reversed())
            } else {
                Line::from(padded)
            };

            buf.set_line(area.x, area.y + row as u16, &line, area.width);
        }
    }

    fn render_right(&self, area: Rect, buf: &mut Buffer) {
        for i in 0..NUM_COLORS.min(area.height) {
            let color = Color::Indexed(i as u8);
            let label = format!("color{:02} ", i);
            let swatch_width = (area.width as usize).saturating_sub(label.len() + 1);
            let swatch = " ".repeat(swatch_width);

            let line = Line::from(vec![
                label.fg(color),
                swatch.fg(color).reversed(),
            ]);

            buf.set_line(area.x, area.y + i, &line, area.width);
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, term_signal: Arc<AtomicBool>) -> io::Result<()> {
    let mut last_applied: Option<usize> = None;
    let mut dirty = true;

    loop {
        if term_signal.load(Ordering::Relaxed) {
            break;
        }

        let idx = app.index();
        if last_applied != Some(idx) {
            app.current_theme().apply()?;
            terminal.clear()?;
            last_applied = Some(idx);
            dirty = true;
        }

        if dirty {
            terminal.draw(|frame| {
                let area = Rect::new(0, 0, TOTAL_COLS, NUM_COLORS);
                frame.render_widget(AppWidget { app }, area);
            })?;
            dirty = false;
        }

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Down => app.down(),
                    KeyCode::Up => app.up(),
                    KeyCode::PageUp => app.up_page(),
                    KeyCode::PageDown => app.down_page(),
                    KeyCode::Home => app.top(),
                    KeyCode::End => app.bottom(),
                    KeyCode::Char('q') => break,
                    KeyCode::Enter => {
                        app.should_install = true;
                        break;
                    }
                    _ => {}
                },
                Event::Resize(cols, rows) => {
                    if rows < NUM_COLORS {
                        return Err(io::Error::other(
                            format!("Terminal has less than {} lines.", NUM_COLORS),
                        ));
                    }
                    if cols < TOTAL_COLS {
                        return Err(io::Error::other(
                            format!("Terminal has less than {} cols.", TOTAL_COLS),
                        ));
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn theme_path() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_default()).join(".base16_theme")
}

fn get_installed_theme() -> Option<Theme> {
    let path = theme_path();
    if path.is_symlink() {
        fs::read_link(&path).ok().map(Theme::new)
    } else {
        None
    }
}

fn end_run() {
    if let Some(theme) = get_installed_theme() {
        let _ = theme.apply();
    }
}

fn find_base16_shell_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = env::var("BASE16_SHELL") {
        return Ok(PathBuf::from(dir));
    }

    let path = theme_path();
    if path.is_symlink()
        && let Ok(resolved) = fs::canonicalize(&path)
        && let Some(parent) = resolved.parent().and_then(|p| p.parent())
    {
        return Ok(parent.to_path_buf());
    }

    Err("please set the BASE16_SHELL environment variable to the local repository path.".into())
}

fn main() {
    let cli = Cli::parse();

    let base16_shell_dir = match find_base16_shell_dir() {
        Ok(dir) => dir,
        Err(msg) => {
            eprintln!("error: {}", msg);
            std::process::exit(2);
        }
    };

    let scripts_dir = base16_shell_dir.join("scripts");
    let mut themes: Vec<Theme> = match fs::read_dir(&scripts_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .map(Theme::new)
            .collect(),
        Err(err) => {
            eprintln!("error: cannot read scripts directory: {}", err);
            std::process::exit(2);
        }
    };

    if cli.sort_bg {
        themes.sort_by_cached_key(|t| (t.bg_color().unwrap_or(0), t.name.clone()));
    } else {
        themes.sort_by(|a, b| a.name.cmp(&b.name));
    }

    if let Some(theme_name) = &cli.theme {
        match themes.iter().find(|t| t.name == *theme_name) {
            Some(theme) => {
                if let Err(err) = theme.apply() {
                    eprintln!("error: {}", err);
                    std::process::exit(1);
                }
                if let Err(err) = theme.install() {
                    eprintln!("error: {}", err);
                    std::process::exit(1);
                }
            }
            None => {
                eprintln!("error: theme not found");
                std::process::exit(2);
            }
        }
        return;
    }

    if cli.list {
        for theme in &themes {
            println!("{}", theme.name);
        }
        return;
    }

    let term_signal = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term_signal))
        .expect("failed to register SIGINT handler");
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term_signal))
        .expect("failed to register SIGTERM handler");

    let mut terminal = ratatui::init();
    let mut app = App::new(themes);

    let result = run(&mut terminal, &mut app, term_signal);

    ratatui::restore();
    let _ = crossterm::execute!(io::stdout(), Show);

    if app.should_install {
        if let Err(err) = app.current_theme().install() {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
    } else {
        end_run();
    }

    if let Err(err) = result {
        eprintln!("error: {}", err);
        std::process::exit(1);
    }
}
