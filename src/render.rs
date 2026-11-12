use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use fringe::{
    app::{App, Screen},
    generator::Difficulty,
    model::{Outcome, Pos, Tile},
};
use std::io::{self, Write};

const BACKGROUND: Color = Color::Rgb { r: 10, g: 9, b: 24 };
const INK: Color = Color::Rgb {
    r: 221,
    g: 231,
    b: 241,
};
const CYAN: Color = Color::Rgb {
    r: 64,
    g: 224,
    b: 232,
};
const GOLD: Color = Color::Rgb {
    r: 255,
    g: 220,
    b: 72,
};
const PINK: Color = Color::Rgb {
    r: 255,
    g: 80,
    b: 140,
};
const MUTED: Color = Color::Rgb {
    r: 132,
    g: 142,
    b: 168,
};

fn center(out: &mut impl Write, width: u16, y: u16, color: Color, text: &str) -> io::Result<()> {
    let text: String = text
        .chars()
        .take(width.saturating_sub(2) as usize)
        .collect();
    let x = width.saturating_sub(text.chars().count() as u16) / 2;
    queue!(
        out,
        MoveTo(x, y),
        SetForegroundColor(color),
        SetBackgroundColor(BACKGROUND),
        Print(text)
    )
}

pub fn draw(out: &mut impl Write, app: &App, size: (u16, u16)) -> io::Result<()> {
    let (width, height) = size;
    queue!(
        out,
        BeginSynchronizedUpdate,
        SetBackgroundColor(BACKGROUND),
        Clear(ClearType::All)
    )?;
    if !crate::terminal::fits(size) {
        center(
            out,
            width,
            height.saturating_sub(1) / 2,
            GOLD,
            "Resize to 80 x 24 to play",
        )?;
        if height > 3 {
            center(
                out,
                width,
                height / 2 + 1,
                MUTED,
                "Simulation suspended. Q quits.",
            )?;
        }
    } else {
        let top = (height - 24) / 2;
        match &app.screen {
            Screen::Menu => {
                center(out, width, top + 3, CYAN, "F  R  I  N  G  E")?;
                center(out, width, top + 5, MUTED, "Slide into the unknown.")?;
                center(out, width, top + 8, INK, "RANDOM STAGES")?;
                for (i, difficulty) in Difficulty::ALL.iter().enumerate() {
                    let selected = *difficulty == app.difficulty;
                    let description = match difficulty {
                        Difficulty::Easy => "Find your footing",
                        Difficulty::Normal => "Read the turns",
                        Difficulty::Hard => "Watch every wall",
                    };
                    let line = format!(
                        "{}  {}  {:<6}   {}",
                        if selected { ">" } else { " " },
                        i + 1,
                        difficulty.to_string().to_uppercase(),
                        description
                    );
                    center(
                        out,
                        width,
                        top + 10 + i as u16 * 2,
                        if selected { GOLD } else { MUTED },
                        &line,
                    )?;
                }
                center(
                    out,
                    width,
                    top + 17,
                    INK,
                    "Arrows select   Enter starts   Q quits",
                )?;
                center(
                    out,
                    width,
                    top + 19,
                    MUTED,
                    "Every stage is generated, then checked for a safe exit.",
                )?;
                center(out, width, top + 21, MUTED, &format!("SEED {}", app.seed))?;
            }
            Screen::Playing(run) => {
                let game = &run.game;
                center(out, width, top, CYAN, "F R I N G E")?;
                center(
                    out,
                    width,
                    top + 1,
                    MUTED,
                    &format!(
                        "{}  /  SEED {}",
                        run.stage.difficulty.to_string().to_uppercase(),
                        run.stage.seed
                    ),
                )?;
                center(
                    out,
                    width,
                    top + 2,
                    INK,
                    &format!(
                        "SCORE {:04}   SLIDES {}   PAR ANY% {} / 100% {}",
                        game.score,
                        game.slides,
                        run.stage.pars.any_percent,
                        run.stage.pars.full_clear
                    ),
                )?;
                let left = (width - game.board.width as u16 * 2) / 2;
                let board_top = top + 4 + (17 - game.board.height as u16) / 2;
                for y in 0..game.board.height {
                    queue!(out, MoveTo(left, board_top + y as u16))?;
                    for x in 0..game.board.width {
                        let pos = Pos { x, y };
                        let (fg, bg, glyph) = if game.player == pos {
                            (
                                BACKGROUND,
                                if game.outcome == Outcome::Died {
                                    PINK
                                } else {
                                    GOLD
                                },
                                "oo",
                            )
                        } else {
                            match game.board.get(pos) {
                                Tile::Wall => (
                                    Color::Rgb {
                                        r: 82,
                                        g: 77,
                                        b: 204,
                                    },
                                    BACKGROUND,
                                    "██",
                                ),
                                Tile::Floor => (BACKGROUND, BACKGROUND, "  "),
                                Tile::Dot => (CYAN, BACKGROUND, ". "),
                                Tile::Coin => (GOLD, BACKGROUND, "$ "),
                                Tile::Spike => (PINK, BACKGROUND, "/\\"),
                                Tile::Exit => (
                                    CYAN,
                                    Color::Rgb {
                                        r: 15,
                                        g: 53,
                                        b: 61,
                                    },
                                    "<>",
                                ),
                            }
                        };
                        queue!(
                            out,
                            SetForegroundColor(fg),
                            SetBackgroundColor(bg),
                            Print(glyph)
                        )?;
                    }
                }
                let (color, message) = match game.outcome {
                    Outcome::Cleared => (
                        GOLD,
                        "STAGE CLEAR!  Enter / N next seed   R replay   M menu",
                    ),
                    Outcome::Died => (PINK, "SPIKED!  R retry   N next seed   M menu"),
                    Outcome::Playing if run.paused => (GOLD, "PAUSED  P / Space resumes"),
                    Outcome::Playing => (
                        MUTED,
                        "oo you   . dot +1   $ coin +10   /\\ spikes   <> exit",
                    ),
                };
                center(out, width, top + 21, color, message)?;
                center(
                    out,
                    width,
                    top + 23,
                    MUTED,
                    "Arrows/WASD/HJKL slide   P pause   R retry   N new   M menu   Q quit",
                )?;
            }
            Screen::Error(message) => {
                center(out, width, top + 7, PINK, "GENERATION FAILED")?;
                center(
                    out,
                    width,
                    top + 9,
                    INK,
                    &format!("{} / SEED {}", app.difficulty, app.seed),
                )?;
                // This message is internally generated, but wrap it to keep recovery visible.
                let mut line = String::new();
                let mut row = top + 11;
                for word in message.split_whitespace() {
                    if line.len() + word.len() + 1 > 70 {
                        center(out, width, row, MUTED, &line)?;
                        line.clear();
                        row += 1;
                    }
                    if !line.is_empty() {
                        line.push(' ');
                    }
                    line.push_str(word);
                }
                center(out, width, row, MUTED, &line)?;
                center(
                    out,
                    width,
                    top + 17,
                    GOLD,
                    "N / Enter retry with next seed   M menu   Q quit",
                )?;
            }
        }
    }
    queue!(out, EndSynchronizedUpdate)?;
    out.flush()
}
