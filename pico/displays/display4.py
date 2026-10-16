import network
import socket
import time
import gc

from picographics import PicoGraphics, DISPLAY_PICO_DISPLAY, PEN_RGB332


# ============================================================
# CONFIG
# ============================================================

SSID = "ssid"
PASSWORD = "pass"

UDP_PORT = 65432

PACKET_SIZE = 16
MAGIC = 0xA7
PROTOCOL_VERSION = 2

OFFLINE_TIMEOUT_MS = 3000

IP_DISPLAY_TIME_MS = 3000


# ============================================================
# DISPLAY
# ============================================================

display = PicoGraphics(
    DISPLAY_PICO_DISPLAY,
    pen_type=PEN_RGB332,
    rotate=0
)

WIDTH, HEIGHT = display.get_bounds()

BLACK = display.create_pen(0, 0, 0)
WHITE = display.create_pen(255, 255, 255)
GREEN = display.create_pen(0, 255, 0)
YELLOW = display.create_pen(255, 255, 51)
CYAN = display.create_pen(130, 213, 252)
ORANGE = display.create_pen(232, 112, 42)
PINK = display.create_pen(255, 123, 239)

display.set_font("bitmap8")


# ============================================================
# USAGE BAR
# ============================================================

def draw_usage_bar(x, y, width, height, value, pen):

    display.set_pen(WHITE)

    display.rectangle(
        x,
        y,
        width,
        height
    )

    if value < 0:
        value = 0

    if value > 100:
        value = 100

    filled_width = int(
        (width * value) / 100
    )

    if filled_width > 0:

        display.set_pen(pen)

        display.rectangle(
            x,
            y,
            filled_width,
            height
        )


# ============================================================
# CHECKSUM
# ============================================================

def calculate_checksum(data):

    checksum = 0

    for byte in data:
        checksum ^= byte

    return checksum


# ============================================================
# UPTIME
# ============================================================

def draw_uptime():

    seconds = uptime

    days = seconds // 86400
    seconds %= 86400

    hours = seconds // 3600
    seconds %= 3600

    minutes = seconds // 60

    if days > 0:
        return "{}d {}h".format(
            days,
            hours
        )

    if hours > 0:
        return "{}h {}m".format(
            hours,
            minutes
        )

    return "{}m".format(minutes)


# ============================================================
# WI-FI
# ============================================================

wlan = network.WLAN(network.STA_IF)

wlan.active(True)

try:
    wlan.config(
        pm=network.WLAN.PM_PERFORMANCE
    )
except:
    pass

wlan.connect(
    SSID,
    PASSWORD
)


# ============================================================
# START SCREEN
# ============================================================

display.set_pen(BLACK)
display.clear()

display.set_pen(WHITE)

display.text(
    "Connecting Wi-Fi...",
    0,
    50,
    WIDTH,
    2
)

display.update()


while not wlan.isconnected():

    time.sleep_ms(100)


# ============================================================
# IP
# ============================================================

ip_address = wlan.ifconfig()[0]

print(
    "Wi-Fi OK:",
    ip_address
)


# ============================================================
# Display IP before any data
# ============================================================

display.set_pen(BLACK)
display.clear()

display.set_pen(CYAN)

display.text(
    "PICO DISPLAY",
    0,
    25,
    WIDTH,
    2
)

display.set_pen(WHITE)

display.text(
    "IP ADDRESS:",
    0,
    55,
    WIDTH,
    2
)

display.set_pen(GREEN)

display.text(
    ip_address,
    0,
    75,
    WIDTH,
    2
)

display.update()

time.sleep_ms(
    IP_DISPLAY_TIME_MS
)


# ============================================================
# UDP
# ============================================================

sock = socket.socket(
    socket.AF_INET,
    socket.SOCK_DGRAM
)

sock.settimeout(1.0)

sock.bind(
    (
        "0.0.0.0",
        UDP_PORT
    )
)

print(
    "UDP:",
    UDP_PORT
)


# ============================================================
# STATUS
# ============================================================

cpu = 0
ram = 0
disk = 0
temperature = 0

rx_x10 = 0
tx_x10 = 0

uptime = 0

flags = 0

online = False

last_packet_ms = time.ticks_ms()


# ============================================================
# PREVIOUSLY DRAWN STATE
# ============================================================

last_cpu = -1
last_ram = -1
last_disk = -1
last_temperature = -2

last_rx_x10 = -1
last_tx_x10 = -1

last_uptime = -1

last_online = False


# ============================================================
# DISPLAY - DASHBOARD
# ============================================================

def update_display():

    global last_cpu
    global last_ram
    global last_disk
    global last_temperature
    global last_rx_x10
    global last_tx_x10
    global last_uptime
    global last_online

    # --------------------------------------------------------
    # Avoid display refresh if nothing has changed
    # --------------------------------------------------------

    if (
        cpu == last_cpu
        and ram == last_ram
        and disk == last_disk
        and temperature == last_temperature
        and rx_x10 == last_rx_x10
        and tx_x10 == last_tx_x10
        and uptime == last_uptime
        and online == last_online
    ):
        return

    # --------------------------------------------------------
    # Clear display
    # --------------------------------------------------------

    display.set_pen(BLACK)
    display.clear()

    if online:

        # ====================================================
        # HEADER
        # ====================================================

        display.set_pen(WHITE)

        display.text(
            "POCKETOWL",
            0,
            2,
            WIDTH,
            2
        )

        display.set_pen(GREEN)

        display.circle(
            232,
            8,
            3
        )

        # ====================================================
        # CPU
        # ====================================================

        display.set_pen(ORANGE)

        display.text(
            "CPU",
            0,
            25,
            60,
            2
        )

        display.set_pen(WHITE)

        display.text(
            "{}%".format(cpu),
            48,
            25,
            60,
            2
        )

        # ====================================================
        # RAM
        # ====================================================

        display.set_pen(CYAN)

        display.text(
            "RAM",
            125,
            25,
            60,
            2
        )

        display.set_pen(WHITE)

        display.text(
            "{}%".format(ram),
            174,
            25,
            66,
            2
        )

        # ====================================================
        # CPU BAR
        # ====================================================

        draw_usage_bar(
            0,
            42,
            105,
            7,
            cpu,
            ORANGE
        )

        # ====================================================
        # RAM BAR
        # ====================================================

        draw_usage_bar(
            125,
            42,
            105,
            7,
            ram,
            CYAN
        )

        # ====================================================
        # DISK
        # ====================================================

        display.set_pen(GREEN)

        display.text(
            "DISK",
            0,
            61,
            60,
            2
        )

        display.set_pen(WHITE)

        display.text(
            "{}%".format(disk),
            48,
            61,
            60,
            2
        )

        # ====================================================
        # TEMPERATURE
        # ====================================================

        display.set_pen(PINK)

        if flags & 1:

            temperature_text = "{}C".format(
                temperature
            )

        else:

            temperature_text = "--"

        display.text(
            "TEMP",
            125,
            61,
            60,
            2
        )

        display.set_pen(WHITE)

        display.text(
            temperature_text,
            174,
            61,
            66,
            2
        )

        # ====================================================
        # DISK BAR
        # ====================================================

        draw_usage_bar(
            0,
            78,
            105,
            7,
            disk,
            GREEN
        )

        # ====================================================
        # TEMP BAR
        # ====================================================

        temperature_percent = temperature

        if temperature_percent > 100:
            temperature_percent = 100

        if temperature_percent < 0:
            temperature_percent = 0

        if flags & 1:

            draw_usage_bar(
                125,
                78,
                105,
                7,
                temperature_percent,
                PINK
            )

        else:

            display.set_pen(WHITE)

            display.rectangle(
                125,
                78,
                105,
                7
            )

        # ====================================================
        # NETWORK
        # ====================================================

        rx_value = "{}.{}".format(
            rx_x10 // 10,
            rx_x10 % 10
        )

        tx_value = "{}.{}".format(
            tx_x10 // 10,
            tx_x10 % 10
        )

        display.set_pen(YELLOW)

        display.text(
            "Rx {} Mbps".format(rx_value),
            0,
            98,
            110,
            2
        )

        display.text(
            "Tx {} Mbps".format(tx_value),
            125,
            98,
            115,
            2
        )

        # ====================================================
        # UPTIME
        # ====================================================

        display.set_pen(WHITE)

        display.text(
            "UP " + draw_uptime(),
            0,
            117,
            WIDTH,
            2
        )

    else:

        # ====================================================
        # HOST OFFLINE
        # ====================================================

        display.set_pen(PINK)

        display.text(
            "POCKETOWL",
            0,
            30,
            WIDTH,
            2
        )

        display.set_pen(PINK)

        display.text(
            "HOST OFFLINE",
            0,
            55,
            WIDTH,
            2
        )

        display.set_pen(WHITE)

        display.text(
            "Waiting for UDP...",
            0,
            78,
            WIDTH,
            2
        )

        display.set_pen(CYAN)

        display.text(
            "IP: " + ip_address,
            0,
            110,
            WIDTH,
            2
        )

    # --------------------------------------------------------
    # Update display
    # --------------------------------------------------------

    display.update()

    # --------------------------------------------------------
    # Save displayed state
    # --------------------------------------------------------

    last_cpu = cpu
    last_ram = ram
    last_disk = disk
    last_temperature = temperature

    last_rx_x10 = rx_x10
    last_tx_x10 = tx_x10

    last_uptime = uptime

    last_online = online


# ============================================================
# PACKET
# ============================================================

def process_packet(data):

    global cpu
    global ram
    global disk
    global temperature
    global rx_x10
    global tx_x10
    global uptime
    global flags
    global online
    global last_packet_ms

    if len(data) != PACKET_SIZE:
        return False

    # --------------------------------------------------------
    # MAGIC
    # --------------------------------------------------------

    if data[0] != MAGIC:
        return False

    # --------------------------------------------------------
    # VERSION
    # --------------------------------------------------------

    if data[1] != PROTOCOL_VERSION:
        return False

    # --------------------------------------------------------
    # CHECKSUM
    # --------------------------------------------------------

    if calculate_checksum(data[:15]) != data[15]:
        return False

    # --------------------------------------------------------
    # METRICS
    # --------------------------------------------------------

    cpu = data[2]

    ram = data[3]

    disk = data[4]

    temperature = data[5]

    rx_x10 = (
        (data[6] << 8)
        | data[7]
    )

    tx_x10 = (
        (data[8] << 8)
        | data[9]
    )

    uptime = (
        (data[10] << 24)
        | (data[11] << 16)
        | (data[12] << 8)
        | data[13]
    )

    flags = data[14]

    online = True

    last_packet_ms = time.ticks_ms()

    return True


# ============================================================
# INITIAL DISPLAY
# ============================================================

display.set_pen(BLACK)

display.clear()

display.set_pen(PINK)

display.text(
    "HOST OFFLINE",
    0,
    35,
    WIDTH,
    2
)

display.set_pen(WHITE)

display.text(
    "Waiting for UDP...",
    0,
    60,
    WIDTH,
    2
)

display.set_pen(CYAN)

display.text(
    "IP: " + ip_address,
    0,
    110,
    WIDTH,
    2
)

display.update()

gc.collect()


# ============================================================
# MAIN LOOP
# ============================================================

while True:

    try:

        data, _ = sock.recvfrom(
            PACKET_SIZE
        )

        process_packet(data)

        update_display()

    except OSError:

        pass

    now = time.ticks_ms()

    if online:

        if time.ticks_diff(
            now,
            last_packet_ms
        ) > OFFLINE_TIMEOUT_MS:

            online = False

            update_display()

    time.sleep_ms(10)
