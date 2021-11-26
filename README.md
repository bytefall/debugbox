# DebugBox

DebugBox is an external debugger for [DOSBox](https://www.dosbox.com/).

Basically DOSBox has built-in [debugger](https://www.vogons.org/viewtopic.php?t=3944) and it's good enough, but as a console application it has limited usability.

## Features

- uses [D-Bus](https://www.freedesktop.org/wiki/Software/dbus/) for inter-process communication with DOSBox.

## Build steps

### Requirements

1. Rust
	- Install [Rust](https://www.rust-lang.org/tools/install).
2. D-Bus
	- `apt install dbus libdbus-1-dev`).

### DebugBox

Execute
- `cargo build` to compile in *debug* mode or
- `cargo build --release` for *release* mode.

### DOSBox

1. Clone DOSBox repository.
2. Replace `dosbox-0.74/src/debug/debug.cpp` with the file `dosbox/0.74/debug.cpp` (which comes with DebugBox).
3. Configure with debug feature enabled: `./configure --enable-debug`.
4. Add `libdbus` to the Makefile:

```
CPPFLAGS =  -I/usr/include/SDL -D_GNU_SOURCE=1 -D_REENTRANT -I/usr/include/dbus-1.0 -I/usr/lib/dbus-1.0/include -I/usr/lib/x86_64-linux-gnu/dbus-1.0/include
```

```
LIBS = -lSDL_sound -lasound -lm -ldl -lpthread -L/usr/lib/x86_64-linux-gnu -lSDL -lcurses -ldbus-1 -lpng -lz -lX11 -lGL
```

## Test from the shell

Start the DOSBox and hit `Alt + Pause` to break in.

Get the value of `EAX`:
```
busctl --user call com.dosbox /cpu/regs/eax com.dosbox get
```

Resume execution with the following command:
```
busctl --user call com.dosbox /cpu com.dosbox run
```
