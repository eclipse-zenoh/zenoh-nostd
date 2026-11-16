#!/usr/bin/env python3
"""POSIX text terminal: Ctrl-C goes to the device; Ctrl-] closes the terminal.

Uses only the standard library. Opening/closing does not deliberately toggle
DTR/RTS or invoke a flashing monitor's reset sequence.
"""
import argparse
import errno
import fcntl
import os
import select
import signal
import sys
import termios
import tty


def write_all(fd, data):
    while data:
        try:
            written = os.write(fd, data)
            data = data[written:]
        except BlockingIOError:
            select.select([], [fd], [])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("port")
    parser.add_argument("--baud", default=115200, type=int)
    args = parser.parse_args()
    baud = getattr(termios, f"B{args.baud}", None)
    if baud is None:
        parser.error("unsupported baud rate")
    try:
        fd = os.open(args.port, os.O_RDWR | os.O_NOCTTY | os.O_NONBLOCK)
    except OSError as error:
        parser.exit(1, f"Cannot open {args.port}: {error.strerror}\n")
    saved_stdin = None
    try:
        fcntl.ioctl(fd, termios.TIOCEXCL)
        settings = termios.tcgetattr(fd)
        settings[0] = settings[1] = settings[3] = 0
        settings[2] = termios.CS8 | termios.CREAD | termios.CLOCAL
        settings[4] = settings[5] = baud
        settings[6][termios.VMIN] = 0
        settings[6][termios.VTIME] = 0
        termios.tcsetattr(fd, termios.TCSANOW, settings)
        if sys.stdin.isatty():
            saved_stdin = termios.tcgetattr(sys.stdin.fileno())
            tty.setraw(sys.stdin.fileno())
        def stop(signum, _frame):
            raise SystemExit(128 + signum)
        signal.signal(signal.SIGTERM, stop)
        signal.signal(signal.SIGINT, stop)
        print("Ctrl-C cancels the device command; Ctrl-] exits this terminal.\r", file=sys.stderr, flush=True)
        while True:
            ready, _, _ = select.select([fd, sys.stdin.fileno()], [], [])
            if fd in ready:
                try:
                    data = os.read(fd, 4096)
                except OSError as error:
                    if error.errno in (errno.EIO, errno.ENXIO):
                        raise SystemExit("Serial device disconnected") from error
                    raise
                if not data:
                    raise SystemExit("Serial device disconnected")
                write_all(sys.stdout.fileno(), data)
            if sys.stdin.fileno() in ready:
                data = os.read(sys.stdin.fileno(), 1024)
                if not data:
                    break
                before_exit, separator, _ = data.partition(b"\x1d")
                write_all(fd, before_exit)
                if separator:
                    break
    finally:
        if saved_stdin is not None:
            termios.tcsetattr(sys.stdin.fileno(), termios.TCSADRAIN, saved_stdin)
        os.close(fd)


if __name__ == "__main__":
    main()
