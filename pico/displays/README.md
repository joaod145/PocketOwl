# Alternative Displays

> [!WARNING]
> These display layouts have been tested, but not exhaustively. They are primarily intended as concept/prototype designs, and the code may still contain bugs, limitations, or unexpected behaviour.

## 1. Compact Dashboard

```text
┌────────────────────────┐
│ CPU  42%      RAM  67% │
│                        │
│ DISK 38%     TEMP  51C │
│                        │
│ RX  12.7      TX  3.4  │
│                        │
│ ● ONLINE       2h 31m  │
└────────────────────────┘
```
- [display1.py](./display1.py)

## 2. Usage Bars

```text
┌────────────────────────┐
│ CPU  42%               │
│ ████████░░░░░░░░░░░░   │
│                        │
│ RAM  67%               │
│ █████████████░░░░░░░   │
│                        │
│ DISK 38%               │
│ ███████░░░░░░░░░░░░░   │
└────────────────────────┘
```
- [display2.py](./display2.py)

## 3. System Monitor

A layout inspired by traditional system monitoring tools such as `htop`:

```text
┌────────────────────────┐
│ POCKETOWL      ONLINE ●│
│                        │
│ CPU  ███████░░  42%    │
│ RAM  ██████████ 67%    │
│ DISK ██████░░░░ 38%    │
│                        │
│ TEMP 51C               │
│ NET  ↓12.7 ↑3.4 Mbps   │
│ UP   2d 04h            │
└────────────────────────┘
```
- [display3.py](./display3.py)

## 4. Recommended Combination

A combination of **dashboard, usage bars, and a small history graph** could provide a more informative interface without making the MicroPython implementation unnecessarily complex.

```text
┌────────────────────────┐
│ POCKETOWL        ●     │
│                        │
│ CPU   42%   RAM   67%  │
│ ███████░  ██████████░  │
│                        │
│ DISK  38%   TEMP  51°C │
│ ██████░░  ██████████░  │
│                        │
│ ↓ 12.7    ↑ 3.4 Mbps   │
│ UP 2d 04h              │
└────────────────────────┘
```
- [display4.py](./display4.py)