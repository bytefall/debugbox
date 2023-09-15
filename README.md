# DebugBox

DebugBox is an external debugger for [DOSBox](https://www.dosbox.com/).

Basically DOSBox has built-in [debugger](https://www.vogons.org/viewtopic.php?t=3944) and it's good enough, but as a console application it has limited usability.

## Features

- uses [D-Bus](https://www.freedesktop.org/wiki/Software/dbus/) for inter-process communication with DOSBox.

## Build steps

### Requirements

1. Rust
	- Install [Rust](https://www.rust-lang.org/tools/install).
2. Docker
	- [Docker](https://docs.docker.com/engine/install/) to build DOSBox

### DebugBox

Execute
- `cargo build` to compile in *debug* mode or
- `cargo build --release` for *release* mode.

### DOSBox

You can build DOSBox locally but it's more convenient to do this in Docker:

```sh
cd ./dosbox
docker build -t dosbox .
docker run --rm --entrypoint bash dosbox -c 'cat /app/dosbox-staging/src/dosbox; sleep 1' > dosbox-dbus
chmod +x dosbox-dbus
```

## Test from the shell

Start the DOSBox and hit `Alt + Pause` (`Fn + Alt + P` on modern keyboards) to break in.

Get the value of `EAX`:
```
busctl --user call com.dosbox /cpu/regs/eax com.dosbox get
```

Resume execution with the following command:
```
busctl --user call com.dosbox /cpu com.dosbox run
```
