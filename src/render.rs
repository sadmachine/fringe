use crossterm::{
    cursor::MoveTo,
    queue,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{BeginSynchronizedUpdate, Clear, ClearType, EndSynchronizedUpdate},
};
use fringe::{
    app::{App, Run, Screen, CELEBRATION_TICKS},
    generator::Difficulty,
    model::{Board, Outcome, Pos, SpikePhase, Tile},
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
const WALL: Color = Color::Rgb {
    r: 82,
    g: 77,
    b: 204,
};
const PULSE_MOUNT: Color = Color::Rgb {
    r: 245,
    g: 166,
    b: 73,
};
const PLAYER_STYLE: (Color, Color, &str) = (BACKGROUND, GOLD, "oo");
type Style = (Color, Color, &'static str);

fn tile_style(tile: Tile, phase: SpikePhase) -> Style {
    match tile {
        Tile::Wall => (WALL, BACKGROUND, "██"),
        Tile::Floor => (BACKGROUND, BACKGROUND, "  "),
        Tile::Dot => (CYAN, BACKGROUND, ". "),
        Tile::Coin => (GOLD, BACKGROUND, "$ "),
        Tile::Spike => (BACKGROUND, PINK, "/\\"),
        Tile::TimedSpike => match phase {
            SpikePhase::Retracted => (INK, BACKGROUND, "=="),
            SpikePhase::Warning => (GOLD, BACKGROUND, "!!"),
            SpikePhase::Active => (BACKGROUND, PINK, "^^"),
        },
        Tile::TouchTrap => match phase {
            SpikePhase::Retracted => (INK, BACKGROUND, "[]"),
            SpikePhase::Warning => (GOLD, BACKGROUND, "!!"),
            SpikePhase::Active => (BACKGROUND, PINK, "^^"),
        },
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
}

fn wall_style(board: &Board, pos: Pos) -> [Style; 2] {
    // Edge order assigns left-facing strips to the left character and
    // right-facing strips to the right character, with caps between them.
    let mut edges = [None; 4];
    for (index, (dx, dy, glyph)) in [(-1, 0, "▏"), (0, -1, "▔"), (0, 1, "▁"), (1, 0, "▕")]
        .into_iter()
        .enumerate()
    {
        let (Some(x), Some(y)) = (pos.x.checked_add_signed(dx), pos.y.checked_add_signed(dy))
        else {
            continue;
        };
        if x < board.width && y < board.height {
            let color = match board.get(Pos { x, y }) {
                Tile::Spike | Tile::TimedSpike => Some(PULSE_MOUNT),
                Tile::TouchTrap => Some(CYAN),
                _ => None,
            };
            edges[index] = color.map(|color| (color, WALL, glyph));
        }
    }
    let mut marked = edges.into_iter().flatten();
    let Some(first) = marked.next() else {
        return [(WALL, BACKGROUND, "█"); 2];
    };
    // Two terminal characters can show two faces. Prefer a different hazard
    // color at junctions so a shared mount retains both types' markings.
    let second = marked
        .clone()
        .find(|edge| edge.0 != first.0)
        .or_else(|| marked.last());
    if let Some(second) = second {
        return [first, second];
    }
    let body = (WALL, WALL, " ");
    match first.2 {
        "▏" => [first, body],
        "▕" => [body, first],
        _ => [first; 2],
    }
}

fn paint(out: &mut impl Write, (fg, bg, glyph): Style) -> io::Result<()> {
    queue!(
        out,
        SetForegroundColor(fg),
        SetBackgroundColor(bg),
        Print(glyph)
    )
}

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

fn legend(out: &mut impl Write, width: u16, y: u16, phase: SpikePhase) -> io::Result<()> {
    let entries = [
        (PLAYER_STYLE, " you"),
        (tile_style(Tile::Dot, phase), "dot +1"),
        (tile_style(Tile::Coin, phase), "coin +10"),
        (tile_style(Tile::Spike, phase), " spikes"),
        (tile_style(Tile::TimedSpike, phase), " pulse"),
        (tile_style(Tile::TouchTrap, SpikePhase::Retracted), " touch"),
        (tile_style(Tile::Exit, phase), " exit"),
    ];
    let length = entries
        .iter()
        .map(|((_, _, glyph), label)| glyph.len() + label.len())
        .sum::<usize>()
        + (entries.len() - 1) * 3;
    queue!(out, MoveTo(width.saturating_sub(length as u16) / 2, y))?;
    for (index, ((fg, bg, glyph), label)) in entries.into_iter().enumerate() {
        if index > 0 {
            queue!(out, Print("   "))?;
        }
        paint(out, (fg, bg, glyph))?;
        queue!(
            out,
            SetForegroundColor(MUTED),
            SetBackgroundColor(BACKGROUND),
            Print(label)
        )?;
    }
    Ok(())
}

fn stage_clear(out: &mut impl Write, run: &Run, width: u16, top: u16) -> io::Result<()> {
    let panel_width = 54;
    let left = (width - panel_width) / 2;
    let panel_top = top + 7;
    // Cover the maze so the result reads as a new state, even without animation.
    for row in 0..11 {
        queue!(
            out,
            MoveTo(left, panel_top + row),
            SetForegroundColor(GOLD),
            SetBackgroundColor(BACKGROUND),
            Print(if row == 0 {
                format!("╔{}╗", "═".repeat(panel_width as usize - 2))
            } else if row == 10 {
                format!("╚{}╝", "═".repeat(panel_width as usize - 2))
            } else {
                format!("║{}║", " ".repeat(panel_width as usize - 2))
            })
        )?;
    }
    let sparkles = if run.clear_ticks == CELEBRATION_TICKS {
        "*        +        *        +        *"
    } else {
        match run.clear_ticks / 3 % 3 {
            0 => "  +      *    .       +    *      .  ",
            1 => "*     .      +    *      .     +    *",
            _ => "   *     +      .    *      +     .  ",
        }
    };
    center(out, width, panel_top + 1, CYAN, sparkles)?;
    center(
        out,
        width,
        panel_top + 3,
        GOLD,
        "* * *   STAGE CLEAR!   * * *",
    )?;
    center(
        out,
        width,
        panel_top + 5,
        INK,
        &format!("SCORE {:04}     SLIDES {}", run.game.score, run.game.slides),
    )?;
    center(out, width, panel_top + 7, GOLD, "Enter / N  next stage")?;
    center(
        out,
        width,
        panel_top + 8,
        MUTED,
        "R replay    M menu    Q quit",
    )?;
    center(out, width, panel_top + 9, CYAN, sparkles)
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
                    &format!("SCORE {:04}   SLIDES {}", game.score, game.slides),
                )?;
                let targets = [
                    (
                        GOLD,
                        format!("ANY%  PAR {}  ", run.stage.pars.any_percent()),
                    ),
                    (
                        MUTED,
                        format!("(min {})", run.stage.pars.any_percent_minimum),
                    ),
                    (
                        GOLD,
                        format!("    100%  PAR {}  ", run.stage.pars.full_clear()),
                    ),
                    (
                        MUTED,
                        format!("(min {})", run.stage.pars.full_clear_minimum),
                    ),
                ];
                let target_width: usize = targets.iter().map(|(_, text)| text.len()).sum();
                queue!(
                    out,
                    MoveTo(width.saturating_sub(target_width as u16) / 2, top + 3),
                    SetBackgroundColor(BACKGROUND)
                )?;
                for (color, text) in targets {
                    queue!(out, SetForegroundColor(color), Print(text))?;
                }
                let left = (width - game.board.width as u16 * 2) / 2;
                let board_top = top + 4 + (17 - game.board.height as u16) / 2;
                for y in 0..game.board.height {
                    queue!(out, MoveTo(left, board_top + y as u16))?;
                    for x in 0..game.board.width {
                        let pos = Pos { x, y };
                        if game.player == pos {
                            let (fg, bg, glyph) = PLAYER_STYLE;
                            paint(
                                out,
                                (
                                    fg,
                                    if game.outcome == Outcome::Died {
                                        PINK
                                    } else {
                                        bg
                                    },
                                    glyph,
                                ),
                            )?;
                        } else if game.board.get(pos) == Tile::Wall {
                            for style in wall_style(&game.board, pos) {
                                paint(out, style)?;
                            }
                        } else {
                            paint(
                                out,
                                tile_style(game.board.get(pos), game.spike_phase_at(pos)),
                            )?;
                        }
                    }
                }
                match game.outcome {
                    Outcome::Cleared => stage_clear(out, run, width, top)?,
                    Outcome::Died => center(
                        out,
                        width,
                        top + 21,
                        PINK,
                        "SPIKED!  R retry   N next seed   M menu",
                    )?,
                    Outcome::Playing if run.paused => {
                        center(out, width, top + 21, GOLD, "PAUSED  P / Space resumes")?
                    }
                    Outcome::Playing => legend(out, width, top + 21, game.spike_phase())?,
                }
                if game.outcome == Outcome::Playing {
                    let status = match game.spike_phase() {
                        SpikePhase::Retracted => "== SAFE",
                        SpikePhase::Warning => "!! WARNING",
                        SpikePhase::Active => "^^ ACTIVE",
                    };
                    center(
                        out,
                        width,
                        top + 22,
                        match game.spike_phase() {
                            SpikePhase::Retracted => INK,
                            SpikePhase::Warning => GOLD,
                            SpikePhase::Active => PINK,
                        },
                        &format!(
                            "PULSE {status}   [] touch = 2s delay   Wait safely between slides"
                        ),
                    )?;
                }
                if game.outcome != Outcome::Cleared {
                    center(
                        out,
                        width,
                        top + 23,
                        MUTED,
                        "Arrows/WASD/HJKL slide   P pause   R retry   N new   M menu   Q quit",
                    )?;
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use fringe::model::Game;

    #[test]
    fn wall_marks_follow_cardinal_neighbors_and_stay_visible_in_every_phase() {
        let mut board = Board::filled(5, 5, Tile::Wall);
        board.set(Pos { x: 1, y: 1 }, Tile::Floor);
        board.set(Pos { x: 2, y: 2 }, Tile::TimedSpike);
        board.set(Pos { x: 4, y: 2 }, Tile::TouchTrap);
        let mut game = Game::new(board, Pos { x: 1, y: 1 });
        for _ in 0..100 {
            for (pos, glyph) in [
                (Pos { x: 2, y: 1 }, "▁▁"),
                (Pos { x: 4, y: 1 }, "▁▁"),
                (Pos { x: 4, y: 3 }, "▔▔"),
                (Pos { x: 3, y: 2 }, "▏▕"),
                (Pos { x: 1, y: 3 }, "██"),
                (Pos { x: 0, y: 0 }, "██"),
            ] {
                let styles = wall_style(&game.board, pos);
                let actual: String = styles.iter().map(|style| style.2).collect();
                assert_eq!(actual, glyph, "{pos:?}");
                if glyph != "██" {
                    assert!(
                        styles.iter().all(|style| style.1 == WALL),
                        "marked wall must retain its solid body"
                    );
                }
            }
            game.tick();
        }
    }

    #[test]
    fn wall_strips_face_the_hazard_and_shared_mounts_keep_both_colors() {
        let center = Pos { x: 1, y: 1 };
        for (hazard, glyph) in [
            (Pos { x: 0, y: 1 }, "▏ "),
            (Pos { x: 2, y: 1 }, " ▕"),
            (Pos { x: 1, y: 0 }, "▔▔"),
            (Pos { x: 1, y: 2 }, "▁▁"),
        ] {
            let mut board = Board::filled(3, 3, Tile::Wall);
            board.set(hazard, Tile::TimedSpike);
            let styles = wall_style(&board, center);
            assert_eq!(
                styles.iter().map(|style| style.2).collect::<String>(),
                glyph
            );
            assert!(styles.iter().all(|style| style.1 == WALL));
        }

        let mut board = Board::filled(3, 3, Tile::Wall);
        board.set(Pos { x: 0, y: 1 }, Tile::TimedSpike);
        board.set(Pos { x: 2, y: 1 }, Tile::TimedSpike);
        board.set(Pos { x: 1, y: 0 }, Tile::TouchTrap);
        let styles = wall_style(&board, center);
        assert_eq!(styles[0], (PULSE_MOUNT, WALL, "▏"));
        assert_eq!(styles[1], (CYAN, WALL, "▔"));
    }
}
