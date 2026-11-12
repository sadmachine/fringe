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

    def board(self):
        lines = ["".join(row) for row in self.visible_rows]
        board_rows = [i for i, line in enumerate(lines) if "██" in line]
        assert board_rows, self.text()
        top, bottom = min(board_rows), max(board_rows)
        left = lines[top].index("██")
        width = len(re.match(r"█+", lines[top][left:]).group()) // 2
        tokens = {"██": "#", "  ": " ", ". ": " ", "$ ": " ", "/\\": "^", "<>": "E", "oo": "S"}
        return tuple("".join(tokens[line[left + 2*x:left + 2*x + 2]] for x in range(width)) for line in lines[top:bottom + 1])


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


def solution(board):
    start = next((x, y) for y, row in enumerate(board) for x, cell in enumerate(row) if cell == "S")
    queue = collections.deque([(start, [])])
    visited = {start}
    while queue:
        (x, y), path = queue.popleft()
        for key, dx, dy in [("w", 0, -1), ("d", 1, 0), ("s", 0, 1), ("a", -1, 0)]:
            nx, ny = x, y
            while board[ny + dy][nx + dx] != "#":
                nx, ny = nx + dx, ny + dy
                if board[ny][nx] == "^":
                    break
                if board[ny][nx] == "E":
                    return path + [(key, (nx, ny))]
            else:
                if (nx, ny) not in visited:
                    visited.add((nx, ny))
                    queue.append(((nx, ny), path + [(key, (nx, ny))]))
    raise AssertionError("screen maze has no safe solution")


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
        assert re.search(r"PAR ANY% [1-9]\d* / 100% [1-9]\d*", game.screen.text()), "both exact pars must be visible"
        initial = game.screen.board()
        game.send("p")
        game.wait_for(lambda s: "PAUSED" in s.text())
        game.send("wasd")
        game.read(0.2)
        assert game.screen.board() == initial, "paused controls moved the player"
        game.send("p")
        game.wait_for(lambda s: "PAUSED" not in s.text())

        game.set_size(40, 12)
        game.wait_for(lambda s: "Simulation suspended" in s.text())
        game.send("wasd")
        game.read(0.2)
        game.set_size(80, 24)
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial, "small-terminal controls moved the player"

        for key, (x, y) in solution(initial):
            game.send(key)
            game.wait_for(lambda s: "oo" in s.text() and s.board()[y][x] == "S")
        game.wait_for(lambda s: "STAGE CLEAR!" in s.text())
        game.send("r")
        game.wait_for(lambda s: "SLIDES 0" in s.text())
        assert game.screen.board() == initial, "retry changed the seeded maze"
        game.send("n")
        game.wait_for(lambda s: "SEED 43" in s.text())
        assert game.screen.board() != initial
        game.send("m")
        game.wait_for(lambda s: "RANDOM STAGES" in s.text())
        game.send("3\r")
        game.wait_for(lambda s: "HARD  /  SEED 43" in s.text() and "SLIDES 0" in s.text())
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
    print("PTY acceptance passed: menu, difficulty, win, replay, next seed, pause, resize, Q, Ctrl-C, cleanup, CLI errors")


if __name__ == "__main__":
    main()
