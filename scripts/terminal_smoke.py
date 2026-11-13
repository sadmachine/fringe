#!/usr/bin/env python3
"""Exercise the real game through a PTY; no third-party Python packages needed."""

import codecs
import collections
import fcntl
import os
import pathlib
import re
import select
import signal
import struct
import subprocess
import sys
import termios
import time

ROOT = pathlib.Path(__file__).resolve().parent.parent
BINARY = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "target/debug/fringe"
CSI = re.compile(r"\x1b\[([0-?]*)([ -/]*)([@-~])")


def child_terminal():
    # Crossterm may query /dev/tty; it must refer to this slave, not our caller.
    os.setsid()
    fcntl.ioctl(0, termios.TIOCSCTTY, 0)


class Screen:
    def __init__(self, width=80, height=24):
        self.resize(width, height)
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def resize(self, width, height):
        self.width, self.height = width, height
        self.rows = [[" "] * width for _ in range(height)]
        self.visible_rows = [row.copy() for row in self.rows]
        self.x = self.y = 0

    def feed(self, data):
        self.pending += self.decoder.decode(data)
        while self.pending:
            if self.pending.startswith("\x1b["):
                match = CSI.match(self.pending)
                if not match:
                    return
                values, _, command = match.groups()
                self.pending = self.pending[match.end():]
                if command in "Hf":
                    numbers = [int(n or 1) for n in values.split(";")]
                    self.y, self.x = numbers[0] - 1, (numbers[1] if len(numbers) > 1 else 1) - 1
                elif command == "J" and values in ("2", "3"):
                    self.rows = [[" "] * self.width for _ in range(self.height)]
                elif command == "l" and values == "?2026":
                    self.visible_rows = [row.copy() for row in self.rows]
                continue
            if self.pending[0] == "\x1b":
                if len(self.pending) < 2:
                    return
                self.pending = self.pending[2:]
                continue
            char, self.pending = self.pending[0], self.pending[1:]
            if char == "\r":
                self.x = 0
            elif char == "\n":
                self.y += 1
            elif char >= " ":
                if 0 <= self.y < self.height and 0 <= self.x < self.width:
                    self.rows[self.y][self.x] = char
                self.x += 1

    def text(self):
        return "\n".join("".join(row) for row in self.visible_rows)

    def board_tokens(self):
        lines = ["".join(row) for row in self.visible_rows]
        wall_pattern = r"(?:██|[▏▕▔▁][▏▕▔▁ ]| [▏▕▔▁])+"
        board_rows = [i for i, line in enumerate(lines) if re.search(wall_pattern, line)]
        assert board_rows, self.text()
        top, bottom = min(board_rows), max(board_rows)
        border = re.search(wall_pattern, lines[top])
        left = border.start()
        width = len(border.group()) // 2
        return tuple(tuple(line[left + 2*x:left + 2*x + 2] for x in range(width)) for line in lines[top:bottom + 1])

    def board(self):
        tokens = {"██": "#", "  ": " ", ". ": " ", "$ ": " ", "{}": "B", "/\\": "^", "==": "T", "[]": "C", "!!": "T", "^^": "T", "<>": "E", "oo": "S", "OO": "S", "**": "S"}
        return tuple("".join("#" if any(c in "▏▕▔▁" for c in token) else tokens[token] for token in row) for row in self.board_tokens())


class Game:
    def __init__(self):
        self.master, self.slave = os.openpty()
        self.original_termios = termios.tcgetattr(self.slave)
        self.screen = Screen()
        self.set_size(80, 24, notify=False)
        self.process = subprocess.Popen(
            [str(BINARY), "--seed", "42", "--difficulty", "easy"],
            stdin=self.slave, stdout=self.slave, stderr=subprocess.PIPE,
            env={**os.environ, "TERM": "xterm-256color"}, close_fds=True,
            preexec_fn=child_terminal,
        )
        self.raw = bytearray()

    def set_size(self, width, height, notify=True):
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", height, width, 0, 0))
        self.screen.resize(width, height)
        if notify:
            os.kill(self.process.pid, signal.SIGWINCH)

    def read(self, timeout=0.1):
        if select.select([self.master], [], [], timeout)[0]:
            data = os.read(self.master, 65536)
            self.raw.extend(data)
            self.screen.feed(data)

    def wait_for(self, predicate, timeout=5):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            self.read()
            if predicate(self.screen):
                # Drain the rest of the synchronized frame before inspecting the board.
                self.read(0.03)
                return
            if self.process.poll() is not None:
                break
        raise AssertionError(f"screen condition timed out\n{self.screen.text()}")

    def send(self, keys):
        os.write(self.master, keys.encode())

    def read_for(self, duration):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            self.read(min(0.1, max(0, deadline - time.monotonic())))

    def close(self, key):
        self.send(key)
        deadline = time.monotonic() + 5
        while self.process.poll() is None and time.monotonic() < deadline:
            self.read()
        assert self.process.poll() == 0, self.screen.text()
        self.read(0.03)
        assert termios.tcgetattr(self.slave) == self.original_termios, "terminal modes were not restored"
        assert b"\x1b[?1049l" in self.raw, "alternate screen was not left"
        assert b"\x1b[?25h" in self.raw, "cursor was not restored"
        assert self.process.stderr.read() == b""

    def dispose(self):
        if self.process.poll() is None:
            self.process.kill()
            self.process.wait()
        self.process.stderr.close()
        os.close(self.master)
        os.close(self.slave)


def solution(board, goal="exit", shield=0):
    start = next((x, y) for y, row in enumerate(board) for x, cell in enumerate(row) if cell == "S")
    queue = collections.deque([((*start, shield), [])])
    visited = {(*start, shield)}
    while queue:
        (x, y, inventory), path = queue.popleft()
        for key, dx, dy in [("w", 0, -1), ("d", 1, 0), ("s", 0, 1), ("a", -1, 0)]:
            nx, ny = x, y
            charge = inventory
            protected = False
            touched = False
            pulse_hit = False
            while board[ny + dy][nx + dx] != "#":
                nx, ny = nx + dx, ny + dy
                if board[ny][nx] == "B" and charge == 0:
                    charge = 1
                if goal == "pulse" and board[ny][nx] == "T" and charge == 1:
                    pulse_hit = True
                if board[ny][nx] == "^":
                    if charge == 1:
                        charge, protected = 2, True
                    elif not protected:
                        break
                touched |= board[ny][nx] == "C"
                if board[ny][nx] == "E":
                    if goal != "exit":
                        break
                    return path + [(key, (nx, ny))]
            else:
                if (nx, ny) == (x, y) or board[ny][nx] == "^":
                    continue
                if ((goal == "touch" and touched) or (goal == "shield" and charge == 1)
                        or (goal == "pulse" and pulse_hit)):
                    return path + [(key, (nx, ny))]
                if (nx, ny, charge) not in visited:
                    visited.add((nx, ny, charge))
                    queue.append(((nx, ny, charge), path + [(key, (nx, ny))]))
    raise AssertionError(f"screen maze has no safe {goal} route")


def touch_positions(board):
    return [(x, y) for y, row in enumerate(board) for x, cell in enumerate(row) if cell == "C"]


def ready_for_slide(game, board):
    traps = touch_positions(board)
    game.wait_for(lambda s: all(s.board_tokens()[y][x] == "[]" for x, y in traps))
    # Observe a fresh safe window, rather than guessing wall-clock delays.
    game.wait_for(lambda s: "PULSE ^^ ACTIVE" in s.text())
    game.wait_for(lambda s: "PULSE == SAFE" in s.text())


def assert_wall_marks(screen, board):
    glyphs = screen.board_tokens()
    for y, row in enumerate(board):
        for x, cell in enumerate(row):
            if cell != "#":
                continue
            faces = [(x-1, y, "▏"), (x, y-1, "▔"), (x, y+1, "▁"), (x+1, y, "▕")]
            edges = [mark for nx, ny, mark in faces if 0 <= ny < len(board) and 0 <= nx < len(row)
                     and board[ny][nx] in ("^", "T", "C")]
            actual = glyphs[y][x]
            if not edges:
                assert actual == "██", f"unmarked wall changed at {(x, y)}"
            else:
                assert any(c in edges for c in actual), f"missing spike-facing edge at {(x, y)}"
                assert all(c == " " or c in edges for c in actual), f"edge faces away from spikes at {(x, y)}"


def clear_stage(game, board):
    path = solution(board)
    for index, (key, (x, y)) in enumerate(path):
        ready_for_slide(game, board)
        game.send(key)
        if index == len(path) - 1:
            game.wait_for(lambda s: "STAGE CLEAR!" in s.text())
        else:
            game.wait_for(lambda s: s.board()[y][x] == "S")


def main():
    # Fragmented reads must expose only complete synchronized frames.
    screen = Screen()
    screen.feed(b"\x1b[?2026h\x1b[2J\x1b[1;1HOLD\x1b[?2026l")
    assert screen.text().startswith("OLD")
    screen.feed(b"\x1b[?2026h\x1b[2J\x1b[1;1HNEW")
    assert screen.text().startswith("OLD"), "a partial frame became visible"
    screen.feed(b"\x1b[?2026l")
    assert screen.text().startswith("NEW")

    for args, expected_code in [(["--help"], 0), (["--seed", "-1"], 1), (["--difficulty", "bad"], 1), ([], 1)]:
        result = subprocess.run([str(BINARY), *args], stdin=subprocess.DEVNULL, capture_output=True)
        assert result.returncode == expected_code
        assert b"\x1b" not in result.stdout + result.stderr, "non-interactive launch emitted terminal controls"

    game = Game()
    try:
        game.wait_for(lambda s: "RANDOM STAGES" in s.text())
        game.send("1\r")
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        targets = re.search(r"ANY%  PAR ([1-9]\d*)  \(min ([1-9]\d*)\)\s+100%  PAR ([1-9]\d*)  \(min ([1-9]\d*)\)", game.screen.text())
        assert targets, "par and minimum for both objectives must fit the 80x24 HUD"
        any_par, any_minimum, full_par, full_minimum = map(int, targets.groups())
        assert any_par > any_minimum and full_par > full_minimum
        assert full_minimum >= any_minimum
        initial = game.screen.board()
        assert any("T" in row for row in initial), "stage must contain a timed wall"
        assert any("C" in row for row in initial), "stage must contain a touch trap"
        assert sum(row.count("B") for row in initial) == 1, "stage needs one shield pickup"
        assert "SHIELD --" in game.screen.text()
        assert "{}shield" in game.screen.text(), "shield legend must fit at 80x24"
        assert_wall_marks(game.screen, initial)
        assert any_minimum == len(solution(initial)), "HUD minimum must match the actual shortest route"
        game.wait_for(lambda s: "PULSE !! WARNING" in s.text())
        assert "!!" in game.screen.text(), "warning must use a distinct glyph"
        game.send("p")
        game.wait_for(lambda s: "PAUSED" in s.text())
        game.send("wasd")
        game.read_for(1.0)
        assert game.screen.board() == initial, "paused controls moved the player"
        assert "PULSE !! WARNING" in game.screen.text(), "pause advanced hazards"
        game.send("p")
        game.wait_for(lambda s: "PAUSED" not in s.text())

        game.set_size(40, 12)
        game.wait_for(lambda s: "Simulation suspended" in s.text())
        game.send("wasd")
        game.read_for(1.0)
        game.set_size(80, 24)
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial, "small-terminal controls moved the player"
        assert "PULSE !! WARNING" in game.screen.text(), "resize suspension advanced hazards"
        assert_wall_marks(game.screen, initial)

        for key, (x, y) in solution(initial, goal="shield"):
            ready_for_slide(game, initial)
            game.send(key)
            game.wait_for(lambda s: s.board()[y][x] == "S")
        assert "SHIELD ARMED" in game.screen.text(), "pickup must arm automatically"
        assert "{}" not in "".join("".join(row) for row in game.screen.board_tokens())
        assert any("OO" in row for row in game.screen.board_tokens()), "armed player needs a distinct shape"
        game.send("p")
        game.wait_for(lambda s: "PAUSED" in s.text())
        armed_board = game.screen.board()
        game.read_for(0.3)
        assert "SHIELD ARMED" in game.screen.text() and game.screen.board() == armed_board
        game.set_size(40, 12)
        game.wait_for(lambda s: "Simulation suspended" in s.text())
        game.read_for(0.3)
        game.set_size(80, 24)
        game.wait_for(lambda s: "PAUSED" in s.text())
        assert "SHIELD ARMED" in game.screen.text() and game.screen.board() == armed_board
        game.send("p")
        game.wait_for(lambda s: "PAUSED" not in s.text())
        ready_for_slide(game, initial)
        hit_path = solution(game.screen.board(), goal="pulse", shield=1)
        for index, (key, (x, y)) in enumerate(hit_path):
            ready_for_slide(game, initial)
            if index == len(hit_path) - 1:
                game.wait_for(lambda s: "PULSE ^^ ACTIVE" in s.text())
            game.send(key)
            game.wait_for(lambda s: s.board()[y][x] == "S")
        assert "SHIELD --" in game.screen.text(), "lethal crossing must spend the charge"
        assert "SPIKED!" not in game.screen.text(), "shield must save the player"
        game.send("r")
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial and "SHIELD --" in game.screen.text()

        for key, (x, y) in solution(initial, goal="touch"):
            ready_for_slide(game, initial)
            game.send(key)
            game.wait_for(lambda s: s.board()[y][x] == "S")
        traps = touch_positions(initial)
        armed = next((x, y) for x, y in traps if game.screen.board_tokens()[y][x] == "!!")
        tx, ty = armed
        assert_wall_marks(game.screen, initial)
        game.send("p")
        game.wait_for(lambda s: "PAUSED" in s.text())
        game.read_for(2.2)
        assert game.screen.board_tokens()[ty][tx] == "!!", "pause advanced the touch countdown"
        game.send("p")
        game.wait_for(lambda s: "PAUSED" not in s.text())
        game.set_size(40, 12)
        game.wait_for(lambda s: "Simulation suspended" in s.text())
        game.read_for(2.2)
        game.set_size(80, 24)
        game.wait_for(lambda s: "PULSE" in s.text())
        assert game.screen.board_tokens()[ty][tx] == "!!", "resize advanced the touch countdown"
        game.wait_for(lambda s: s.board_tokens()[ty][tx] == "^^")
        assert_wall_marks(game.screen, initial)
        game.wait_for(lambda s: s.board_tokens()[ty][tx] == "[]")
        assert_wall_marks(game.screen, initial)
        game.send("r")
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial, "retry did not reset touch traps"

        clear_stage(game, initial)
        rows = game.screen.text().splitlines()
        assert "STAGE CLEAR!" in rows[6], "win title must be prominent in the maze area"
        assert "╔" in rows[4] and "╚" in rows[20], "win panel must fit the minimum terminal"
        assert "Enter / N  next stage" in rows[17] and "R replay" in rows[18]
        result = re.search(r"SCORE (\d+)\s+SLIDES (\d+)", rows[8]).groups()
        slides = int(result[1])
        assert slides == any_minimum
        assert rows[11].strip(" ║").split() == ["GOAL", "SLIDES", "PAR", "MIN"]
        assert "+ over target, - under target" in rows[14]

        def comparison(actual, target):
            return [str(target), f"({actual - target:+d})"]

        any_row = rows[12].strip(" ║").split()
        assert any_row == ["ANY%", str(slides), *comparison(slides, any_par),
                           *comparison(slides, any_minimum)], any_row
        collected = re.search(r"DOTS (\d+)/(\d+)\s+COINS (\d+)/(\d+)", rows[9])
        assert collected, "collection totals must fit inside the result panel"
        dots, all_dots, coins, all_coins = map(int, collected.groups())
        complete = dots == all_dots and coins == all_coins
        full_row = rows[13].strip(" ║").split()
        if complete:
            assert full_row == ["100%", str(slides), *comparison(slides, full_par),
                                *comparison(slides, full_minimum)], full_row
            assert "100% COMPLETE" in rows[15]
        else:
            assert full_row == ["100%", "--", str(full_par), str(full_minimum)], full_row
            assert "100% NOT COMPLETED" in rows[15]
        assert "OPTIMAL" in rows[15], "matching a minimum should still earn recognition"
        result_rows = rows[8:19]
        first_frame = game.screen.text()
        game.wait_for(lambda s: s.text() != first_frame, timeout=1)
        assert "STAGE CLEAR!" in game.screen.text(), "sparkles must keep the result visible"
        game.send("wasdp")
        deadline = time.monotonic() + 1.4
        while time.monotonic() < deadline:
            game.read()
        settled = game.screen.text()
        assert settled.splitlines()[8:19] == result_rows, "celebration changed performance results"
        assert "STAGE CLEAR!" in settled and "PAUSED" not in settled
        output_size = len(game.raw)
        game.read(0.2)
        assert len(game.raw) == output_size, "settled celebration must stop redrawing"
        game.set_size(40, 12)
        game.wait_for(lambda s: "Simulation suspended" in s.text())
        game.set_size(100, 30)
        game.wait_for(lambda s: "STAGE CLEAR!" in s.text())
        assert "STAGE CLEAR!" in game.screen.text().splitlines()[9]
        game.set_size(80, 24)
        game.wait_for(lambda s: "STAGE CLEAR!" in s.text())
        assert game.screen.text().splitlines()[8:19] == result_rows, "resize changed performance results"
        game.send("r")
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial, "retry changed the seeded maze"
        clear_stage(game, initial)
        game.send("\r")
        game.wait_for(lambda s: "SEED 43" in s.text())
        assert "STAGE CLEAR!" not in game.screen.text(), "Enter must dismiss the animation immediately"
        assert "SHIELD --" in game.screen.text(), "new stages must start without a shield"
        assert game.screen.board() != initial
        game.send("n")
        game.wait_for(lambda s: "SEED 44" in s.text())
        game.send("m")
        game.wait_for(lambda s: "RANDOM STAGES" in s.text())
        game.send("3\r")
        game.wait_for(lambda s: "HARD  /  SEED 44" in s.text() and "SLIDES 0" in s.text())
        assert len(game.screen.board()) == 17
        game.close("q")
    finally:
        game.dispose()

    game = Game()
    try:
        game.wait_for(lambda s: "RANDOM STAGES" in s.text())
        game.close("\x03")
    finally:
        game.dispose()
    print("PTY acceptance passed: shield pickup/damage/reset and pause/resize, pulse spikes, touch traps, distinct wall mountings, hazard pause/resize freeze, exact slide minimum, menu, difficulty, win celebration, replay, next seed, Q, Ctrl-C, cleanup, CLI errors")


if __name__ == "__main__":
    main()
