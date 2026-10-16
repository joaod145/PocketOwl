# Architecture

PocketOwl is a lightweight real-time system monitoring application composed of two main components: a Rust-based monitoring agent running on a Linux host and a Raspberry Pi Pico W running MicroPython.

The Linux host collects system metrics, encodes them into a compact binary packet, and transmits them to the Pico W over UDP and Wi-Fi. The Pico W receives, validates, processes, and displays the metrics on a Pimoroni Pico Display 240×135.

## System Overview

```text
          Linux Host
┌─────────────────────────────┐
│                             │
│          Rust Script        │
│                             │
│ CPU  RAM   Disk        Temp │
│ Network    RX/TX     Uptime │
└──────────────┬──────────────┘
               │
               │ UDP / Wi-Fi
               │
               ▼
       Raspberry Pi Pico W
┌─────────────────────────────┐
│                             │
│     MicroPython script      │
│                             │
│ Receive → Validate → Display│
│                             │
│     Pico Display 240×135    │
└─────────────────────────────┘
```

## Linux Host

The Linux host runs the monitoring agent implemented in Rust.

Its main responsibility is to collect system information with minimal overhead and periodically transmit the collected metrics to the Raspberry Pi Pico W.

The agent collects:

- CPU usage
- RAM usage
- Disk usage
- CPU temperature in Celsius
- Network Receive (RX) speed
- Network Transmit (TX) speed
- System uptime

### Metric Collection

The Rust agent reads system information directly from Linux interfaces such as `/proc` and `/sys`.

The main sources are:

| Metric | Linux source |
|---|---|
| CPU | `/proc/stat` |
| RAM | `/proc/meminfo` |
| Network | `/proc/net/dev` |
| Uptime | `/proc/uptime` |
| Temperature | `/sys/class/thermal/` |
| Disk | `statvfs()` |

Files that are accessed repeatedly are opened once and reused whenever possible. This avoids unnecessary file-open operations and reduces I/O overhead.

The temperature collector also discovers a valid thermal zone once and reuses the corresponding file instead of searching through all available thermal zones on every update.

Metrics that do not need to be updated every second, such as disk usage and temperature, can be sampled less frequently to further reduce system overhead.

### CPU and Network Measurements

CPU and network usage are calculated from differences between consecutive measurements.

For CPU usage, the agent compares the total and idle CPU counters between two samples.

Network throughput is calculated from the difference in received and transmitted bytes divided by the elapsed time between samples.

The resulting network values are encoded with one decimal place of precision.

For example:

```text
12.7 Mbps → 127
```

This allows the value to be transmitted efficiently using a 16-bit integer.

## UDP Communication

Communication between the Linux host and the Pico W uses UDP.

UDP was chosen because the application continuously sends the latest system state. If a packet is lost, there is no need to retransmit it because the next packet will contain a new measurement.

The host periodically sends a single binary packet containing all current metrics.

The Pico W listens on the configured UDP port:

```text
65432
```

The port is configurable and can be changed as long as the same port is configured on both devices.

## Binary Protocol

The communication protocol uses a compact fixed-size packet of **16 bytes**.

```text
Byte       Field
────────────────────────────
0          MAGIC
1          VERSION
2          CPU
3          RAM
4          DISK
5          TEMPERATURE
6-7        RX Mbps × 10
8-9        TX Mbps × 10
10-13      UPTIME
14         FLAGS
15         CHECKSUM
```

The fixed packet size avoids unnecessary serialization overhead and makes packet parsing on the Pico W simple and predictable.

### Packet Validation

The Pico W validates every received packet before processing it.

It checks:

1. Packet size
2. Magic value
3. Protocol version
4. Checksum

The checksum is calculated using a simple XOR over the first 15 bytes.

Invalid packets are discarded.

## Raspberry Pi Pico W

The Raspberry Pi Pico W runs the MicroPython application.

Its responsibilities are intentionally limited to:

1. Connecting to Wi-Fi
2. Receiving UDP packets
3. Validating packets
4. Decoding metric values
5. Updating the display
6. Detecting host connection loss

The Pico W does not perform system monitoring itself. It acts as a lightweight network client and display controller.

## Wi-Fi

The Pico W connects to the configured Wi-Fi network using the credentials defined in `main.py`.

```python
SSID = "your_wifi"
PASSWORD = "your_password"
```

The IP address is obtained automatically using DHCP.

The IP address is displayed on the screen during startup, allowing the user to configure the Rust host accordingly.

> **Note:** The Raspberry Pi Pico W supports **2.4 GHz Wi-Fi networks only** and cannot connect directly to 5 GHz Wi-Fi networks.

## Display

The project uses the **Pimoroni Pico Display 240×135**.

The MicroPython application uses the `picographics` library to control the display.

The screen presents the following information:

```text
CPU
RAM
DISK
TEMP
RX
TX
UP
```

The display also shows the Pico W IP address during startup.

When no valid packet is received within the configured timeout, the device displays a connection status indicating that the host is offline.

## Display Update Optimization

The MicroPython application keeps track of the values previously displayed.

Before redrawing the screen, it compares the current values with the previous state.

If no relevant value has changed, the display is not updated.

This avoids unnecessary display operations and reduces the workload on the Pico W.

## Connection Monitoring

The Pico W keeps track of the timestamp of the last valid packet received from the Linux host.

If no valid packet is received within the configured timeout:

```text
OFFLINE_TIMEOUT_MS = 3000
```

the host is considered offline.

The display then changes to:

```text
HOST OFFLINE
Waiting for UDP...
```

When a valid packet is received again, the connection is automatically considered active and the metrics are displayed again.

## Data Flow

The complete data flow can be summarized as:

```text
Linux System
     │
     ▼
Rust Metric Collectors
     │
     ▼
Metric Processing
     │
     ▼
16-byte Binary Packet
     │
     ▼
UDP / Wi-Fi
     │
     ▼
Raspberry Pi Pico W
     │
     ▼
Packet Validation
     │
     ▼
Metric Decoding
     │
     ▼
Pimoroni Pico Display
```

## Design Goals

### Low Resource Usage

The Rust agent avoids unnecessary allocations and repeated file operations where possible.

### Low Network Overhead

All metrics are transmitted in a single fixed-size 16-byte UDP packet.

### Simple Embedded Processing

The Pico W only needs to validate and decode a small binary packet before updating the display.

### Fault Tolerance

UDP packet loss does not affect the overall state of the application. The next packet provides a fresh snapshot of the system metrics.

The Pico W also detects communication loss automatically and reports the host as offline.