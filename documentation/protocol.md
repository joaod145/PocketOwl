# Communication Protocol

PocketOwl uses a lightweight binary protocol over UDP to transfer system metrics from the Linux host to the Raspberry Pi Pico W.

The protocol is designed to minimize network overhead, simplify packet parsing on the microcontroller, and provide a fixed and predictable packet structure.

## Transport

Communication uses:

- **Protocol:** UDP
- **Transport:** Wi-Fi
- **Packet size:** 16 bytes
- **Default port:** `65432`
- **Byte order:** Big Endian for multi-byte numeric fields

The Linux host sends the packets, while the Raspberry Pi Pico W listens for incoming packets.

```text
Linux Host
    │
    │ UDP
    │
    ▼
Pico W : 65432
```

UDP was selected because the application continuously transmits the current system state.

Reliable delivery is not required. If a packet is lost, the next packet will contain a new and more recent measurement.

## Packet Structure

Every packet contains exactly **16 bytes**.

```text
┌──────┬───────────────┬─────────────────────────────────────┐
│ Byte │ Field         │ Description                         │
├──────┼───────────────┼─────────────────────────────────────┤
│  0   │ MAGIC         │ Protocol identification             │
│  1   │ VERSION       │ Protocol version                    │
│  2   │ CPU           │ CPU usage (%)                       │
│  3   │ RAM           │ RAM usage (%)                       │
│  4   │ DISK          │ Disk usage (%)                      │
│  5   │ TEMP          │ Temperature (°C)                    │
│ 6-7  │ RX            │ Network RX speed × 10               │
│ 8-9  │ TX            │ Network TX speed × 10               │
│10-13 │ UPTIME        │ System uptime (seconds)             │
│  14  │ FLAGS         │ Status flags                        │
│  15  │ CHECKSUM      │ Packet integrity check              │
└──────┴───────────────┴─────────────────────────────────────┘
```

## Byte 0 — MAGIC

```text
Value: 0xA7
Size:  1 byte
```

The `MAGIC` value identifies packets belonging to the PocketOwl protocol.

The Pico W checks this value before processing the packet.

```python
if data[0] != MAGIC:
    return False
```

This prevents unrelated UDP traffic from being interpreted as a valid monitoring packet.

## Byte 1 — VERSION

```text
Value: 0x02
Size:  1 byte
```

The protocol version identifies the packet format being used.

The current implementation uses:

```python
PROTOCOL_VERSION = 2
```

The Pico W rejects packets using an unsupported protocol version.

This allows the packet format to evolve in future versions while maintaining compatibility checks.

## Byte 2 — CPU

```text
Size: 1 byte
Range: 0–100
Unit: %
```

Contains the current CPU utilization as an integer percentage.

Example:

```text
CPU usage = 37%
Byte 2    = 0x25
```

The value is calculated by the Rust agent using CPU counters obtained from:

```text
/proc/stat
```

The agent compares consecutive CPU measurements to calculate the percentage of time spent doing useful work.

## Byte 3 — RAM

```text
Size: 1 byte
Range: 0–100
Unit: %
```

Contains the current RAM utilization.

The Rust agent obtains the total memory and available memory from:

```text
/proc/meminfo
```

The percentage is calculated from:

```text
Used = Total - Available
```

Example:

```text
RAM usage = 62%
Byte 3    = 0x3E
```

## Byte 4 — DISK

```text
Size: 1 byte
Range: 0–100
Unit: %
```

Contains the percentage of disk space currently in use.

The Rust agent obtains this information using the Linux `statvfs()` system call.

The monitored filesystem is:

```text
/
```

Example:

```text
Disk usage = 48%
Byte 4     = 0x30
```

Disk usage does not need to be measured at the same frequency as CPU or network activity, so the collector can update it less frequently.

## Byte 5 — TEMP

```text
Size: 1 byte
Range: 0–254
Unit: °C
```

Contains the system temperature as an integer number of degrees Celsius.

Linux temperature information is obtained from:

```text
/sys/class/thermal/
```

The raw Linux temperature is normally represented in millicelsius.

For example:

```text
51000 millicelsius
        ↓
       51 °C
```

The protocol reserves:

```text
0xFF
```

to indicate that a valid temperature could not be obtained.

Therefore:

```text
0–254 → Valid temperature
255    → Temperature unavailable
```

The Pico W uses the corresponding flag to determine whether the temperature should be displayed.

## Bytes 6–7 — RX

```text
Size: 2 bytes
Encoding: unsigned 16-bit integer
Byte order: Big Endian
Unit: Mbps × 10
```

Contains the network receive speed.

The value is multiplied by 10 before transmission to preserve one decimal place without using floating-point numbers.

Example:

```text
12.7 Mbps
   ↓
  127
   ↓
 0x007F
```

The packet therefore contains:

```text
Byte 6 = 0x00
Byte 7 = 0x7F
```

On the Pico W:

```python
rx_x10 = (data[6] << 8) | data[7]
```

The displayed value is reconstructed using:

```python
whole = rx_x10 // 10
decimal = rx_x10 % 10
```

Result:

```text
12.7 Mbps
```

## Bytes 8–9 — TX

```text
Size: 2 bytes
Encoding: unsigned 16-bit integer
Byte order: Big Endian
Unit: Mbps × 10
```

Works in the same way as RX, but represents transmitted network traffic.

Example:

```text
3.4 Mbps
   ↓
  34
   ↓
 0x0022
```

The Pico W reconstructs the value using:

```python
tx_x10 = (data[8] << 8) | data[9]
```

## Bytes 10–13 — UPTIME

```text
Size: 4 bytes
Encoding: unsigned 32-bit integer
Byte order: Big Endian
Unit: seconds
```

Contains the Linux host uptime in seconds.

The value is obtained from:

```text
/proc/uptime
```

Example:

```text
Uptime = 86400 seconds
```

corresponds to:

```text
1 day
```

The Pico W receives the 32-bit value and reconstructs it using:

```python
uptime = (
    (data[10] << 24)
    | (data[11] << 16)
    | (data[12] << 8)
    | data[13]
)
```

The value is then converted into a human-readable representation such as:

```text
2h 15m
```

or:

```text
3d 4h
```

## Byte 14 — FLAGS

```text
Size: 1 byte
```

The `FLAGS` field contains status information about the values included in the packet.

Currently, two flags are defined.

### Bit 0 — Temperature Valid

```text
Value: 0x01
```

Indicates whether the temperature value is valid.

```text
0 → Temperature unavailable
1 → Temperature valid
```

The flag is set by the Rust agent when a valid temperature has been obtained.

### Bit 1 — Uptime Valid

```text
Value: 0x02
```

Indicates whether the uptime value is valid.

```text
0 → Uptime unavailable
1 → Uptime valid
```

The current implementation sets this flag when the uptime is successfully read.

### Flags Example

If both temperature and uptime are valid:

```text
FLAGS = 0x03
```

because:

```text
0x01 | 0x02 = 0x03
```

If only uptime is valid:

```text
FLAGS = 0x02
```

## Byte 15 — CHECKSUM

```text
Size: 1 byte
Algorithm: XOR
```

The final byte contains a simple XOR checksum calculated over the first 15 bytes.

Conceptually:

```text
CHECKSUM =
    BYTE0 XOR
    BYTE1 XOR
    BYTE2 XOR
    ...
    BYTE14
```

The Rust implementation:

```rust
fn calculate_checksum(packet: &[u8; 15]) -> u8 {
    let mut checksum = 0u8;

    for byte in packet {
        checksum ^= *byte;
    }

    checksum
}
```

The Pico W calculates the checksum again after receiving the packet:

```python
if calculate_checksum(data[:15]) != data[15]:
    return False
```

If the calculated checksum does not match the transmitted checksum, the packet is discarded.

The checksum is intended as a lightweight application-level validation mechanism rather than a replacement for UDP/IP error detection.

## Complete Example

A packet containing:

```text
CPU       = 37%
RAM       = 62%
DISK      = 48%
TEMP      = 51 °C
RX        = 12.7 Mbps
TX        = 3.4 Mbps
UPTIME    = 86400 seconds
FLAGS     = temperature + uptime valid
```

could be represented as:

```text
┌───────┬────────────┐
│ Byte  │ Value      │
├───────┼────────────┤
│ 0     │ 0xA7       │
│ 1     │ 0x02       │
│ 2     │ 0x25       │
│ 3     │ 0x3E       │
│ 4     │ 0x30       │
│ 5     │ 0x33       │
│ 6     │ 0x00       │
│ 7     │ 0x7F       │
│ 8     │ 0x00       │
│ 9     │ 0x22       │
│ 10    │ 0x00       │
│ 11    │ 0x01       │
│ 12    │ 0x51       │
│ 13    │ 0x80       │
│ 14    │ 0x03       │
│ 15    │ CHECKSUM   │
└───────┴────────────┘
```

The final checksum depends on all preceding bytes.

## Packet Processing

The Pico W processes packets in the following order:

```text
UDP packet received
        │
        ▼
Check packet size
        │
        ▼
Check MAGIC
        │
        ▼
Check VERSION
        │
        ▼
Calculate checksum
        │
        ▼
Compare checksum
        │
        ├── Invalid ──► Discard
        │
        ▼
Decode metrics
        │
        ▼
Update application state
        │
        ▼
Update display
```

This ensures that only valid packets can modify the displayed system state.

## Connection Timeout

UDP does not provide connection state, so the Pico W implements a simple application-level timeout.

Every valid packet updates:

```python
last_packet_ms
```

If no valid packet is received for more than:

```python
OFFLINE_TIMEOUT_MS = 3000
```

the host is considered offline.

The Pico W then displays:

```text
HOST OFFLINE
Waiting for UDP...
```

Receiving a valid packet immediately restores the online state.

## Port Configuration

The default UDP port is:

```text
65432
```

On the Linux host:

```rust
const PICO_PORT: u16 = 65432;
```

On the Pico W:

```python
UDP_PORT = 65432
```

The port is not fixed by the protocol itself. It can be changed to any available UDP port.

If the port is changed, **both values must be updated to the same port**.

## Protocol Characteristics

The protocol intentionally favors simplicity and efficiency over complexity.

### Fixed Packet Size

Every packet is exactly 16 bytes, making allocation and parsing predictable.

### No Floating-Point Values

Network speeds are represented as integers multiplied by 10, avoiding floating-point serialization.

### No Strings

Metric names and other textual data are not transmitted.

### No JSON

The protocol avoids the overhead associated with text-based formats such as JSON.

### Stateless Transport

Each packet contains a complete snapshot of the current system state.

The Pico W does not need to reconstruct a stream of incremental updates.

### UDP

Packet loss is acceptable because subsequent packets contain fresh metric values.

## Protocol Versioning

The protocol currently uses:

```text
VERSION = 2
```

Future versions can change the packet structure while retaining the version field.

A Pico W running an older implementation can reject packets from an unsupported protocol version instead of attempting to decode an incompatible structure.

This provides a simple mechanism for future protocol evolution.