# PocketOwl

A lightweight real-time Linux system monitor using Rust, Raspberry Pi Pico W, and MicroPython, with metrics transmitted over Wi-Fi via UDP and displayed on a Pimoroni Pico Display 240×135.

# Architecture

```text
┌─────────────────┐    UDP / Wi-Fi    ┌───────────────────────┐
│   Linux + Rust  │ ────────────────► │  Raspberry Pi Pico W  │
│     Metrics     │                   │      + Display        │
└─────────────────┘                   └───────────────────────┘
```

# Features

Real-time monitoring of:
- CPU
- RAM
- Disk
- Temperature
- Network RX/TX
- Uptime
- Connection status

Metrics are transmitted via UDP using a compact 16-byte binary protocol.

## User Guide

### 1. Install the `picographics` Module

The Raspberry Pi Pico W uses the **Pimoroni Pico Display 240×135**, which requires the `picographics` library.

First, install the appropriate **Pimoroni MicroPython firmware** for the Pico W. The firmware includes the `picographics` module required by the project.

After installing the firmware, verify that the module is available by running:

```python
import picographics
```

If no error is returned, the module is correctly installed.

### 2. Connect the Raspberry Pi Pico W

Connect the Raspberry Pi Pico W to the computer using USB.

Open **Thonny** and select:

```text
MicroPython (Raspberry Pi Pico)
```

as the interpreter.

The Pico W should then appear as the connected MicroPython device.

### 3. Configure the Wi-Fi

Open `main.py` and configure the Wi-Fi credentials:

```python
SSID = "your_wifi"
PASSWORD = "your_password"
```

The Pico W connects to the network during startup and obtains its IP address automatically through DHCP.

The IP address is displayed on the Pico Display during startup.

> **Note:** The Raspberry Pi Pico W supports **2.4 GHz Wi-Fi networks only**. It cannot connect directly to **5 GHz Wi-Fi networks**.

### 4. Upload the MicroPython Program

Save the MicroPython program to the Pico W as:

```text
main.py
```

In Thonny, use:

```text
File → Save As → Raspberry Pi Pico
```

and save the file as `main.py`.

The Pico W will automatically execute `main.py` when it starts.

### 5. Configure the Rust Host

On the Linux host, configure the destination IP address in `src/main.rs`:

```rust
const PICO_IP: &str = "picoIP";
const PICO_PORT: u16 = 65432;
```

Replace `PICO_IP` with the IP address displayed by the Raspberry Pi Pico W.

The host and Pico W must be connected to the same Wi-Fi network.

> **Note:** The UDP port used by the project does not have to be **65432**. You can use any available port of your choice. If you change the port, make sure to update it in **both** `src/main.rs` and `main.py`:
>
> ```rust
> const PICO_PORT: u16 = n;
> ```
>
> ```python
> UDP_PORT = n
> ```
>
> Both values must use the **same available UDP port** for communication to work correctly.

### 6. Build the Rust Agent

From the project directory, build the optimized release version:

```bash
cargo build --release
```

The executable will be generated at:

```text
target/release/host
```

### 7. Run the PocketOwl

Start the Rust agent with:

```bash
./target/release/host
```

The PocketOwl will start collecting system metrics and transmitting them to the Pico W using UDP.

### 8. Verify the Display

Once the Pico W receives valid packets, the display will show:

```text
CPU: 3%
RAM: 16%
DISK: 38%
TEMP: 51 C
RX: 12.4 Mbps
TX: 2.8 Mbps
UP: 9h 31m
```

If no packets are received within the configured timeout, the display will show:

```text
HOST OFFLINE
Waiting for UDP...
```

When communication is restored, the metrics will automatically appear again.

### 9. Running Without Thonny

Once `main.py` has been saved to the Pico W, **Thonny is no longer required**.

The Pico W can be disconnected from the computer and powered from a USB power supply.

On startup, the Pico W will:

1. Start MicroPython.
2. Execute `main.py`.
3. Connect to Wi-Fi.
4. Display its IP address.
5. Start listening for UDP packets.
6. Display the received system metrics.

The Rust agent only needs to be running on the Linux host.

# Repository Structure

```text
.
├── LICENSE
├── README.md
├── documentation/
├── host/
│   ├── Cargo.lock
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
└── pico/
    ├── displays/
    └── src/
        └── main.py
```

## `host/`

Contains the Rust monitoring agent running on the Linux host.

### `Cargo.toml`

Defines the Rust project configuration.

### `Cargo.lock`

Contains the locked versions of the Rust dependencies.

### `main.rs`

Contains the Rust agent responsible for collecting system metrics and transmitting them to the Raspberry Pi Pico W via UDP.

## `pico/`

### `src/`

Contains the main MicroPython application.

#### `main.py`

Contains the MicroPython program responsible for connecting to Wi-Fi, receiving and processing the UDP packets, and displaying the system metrics on the Pico Display.

### `displays/`

Contains alternative display layouts for the 240×135 Pico Display. These layouts are provided as concept/prototype implementations and can be used as alternative interfaces for displaying the system metrics.