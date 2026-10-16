use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::net::UdpSocket;
use std::os::raw::{c_char, c_int, c_ulong};
use std::thread;
use std::time::{Duration, Instant};

// ============================================================
// CONFIG
// ============================================================

// network 
const PICO_IP: &str = "picoIP";
const PICO_PORT: u16 = 65432;

// time intervals for metric updates
const UPDATE_INTERVAL: Duration = Duration::from_secs(1);
const DISK_UPDATE_INTERVAL: Duration = Duration::from_secs(30);
const TEMP_UPDATE_INTERVAL: Duration = Duration::from_secs(5);

// paths to Linux system files from which metrics are collected
const PROC_STAT: &str = "/proc/stat";
const PROC_NET_DEV: &str = "/proc/net/dev";
const PROC_UPTIME: &str = "/proc/uptime";
const TEMP_GLOB_PREFIX: &str = "/sys/class/thermal/thermal_zone";

// parameters of the communication protocol
const MAGIC: u8 = 0xA7;
const PROTOCOL_VERSION: u8 = 2;
const PACKET_SIZE: usize = 16;

// fixed-size buffers
const CPU_BUFFER_SIZE: usize = 4096;
const MEM_BUFFER_SIZE: usize = 4096;
const NET_BUFFER_SIZE: usize = 8192;
const UPTIME_BUFFER_SIZE: usize = 128;
const TEMP_BUFFER_SIZE: usize = 64;

// ============================================================
// Linux statvfs()
// ============================================================

#[repr(C)] // arrange this structure in memory in the same way as
struct Statvfs { // a C structure to obtain information about disk usage
    f_bsize: c_ulong,
    f_frsize: c_ulong,
    f_blocks: c_ulong,
    f_bfree: c_ulong,
    f_bavail: c_ulong,
    f_files: c_ulong,
    f_ffree: c_ulong,
    f_favail: c_ulong,
    f_fsid: c_ulong,
    f_flag: c_ulong,
    f_namemax: c_ulong,
    f_spare: [c_int; 6],
}

unsafe extern "C" {
    fn statvfs(path: *const c_char, buf: *mut Statvfs) -> c_int;
} // call the Linux system's statvfs() function

// ============================================================
// CPU
// ============================================================

#[derive(Clone, Copy)] // stores the CPU counters read from /proc/stat
struct CpuStats {
    total: u64, // total amount of CPU time across all
    idle: u64, // time the CPU was idle, including I/O wait
}

fn read_cpu_stats( // reads the CPU statistics
    file: &mut File,
    buffer: &mut [u8; CPU_BUFFER_SIZE],
) -> io::Result<CpuStats> {
    file.seek(SeekFrom::Start(0))?;

    let size = file.read(buffer)?;
    let data = &buffer[..size];

    if data.len() < 4
        || data[0] != b'c'
        || data[1] != b'p'
        || data[2] != b'u'
        || data[3] != b' '
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid /proc/stat",
        ));
    }

    let mut pos = 4;

    let user = read_number(data, &mut pos);
    let nice = read_number(data, &mut pos);
    let system = read_number(data, &mut pos);
    let idle = read_number(data, &mut pos);
    let iowait = read_number(data, &mut pos);
    let irq = read_number(data, &mut pos);
    let softirq = read_number(data, &mut pos);
    let steal = read_number(data, &mut pos);

    let total = user // calculate the total CPU time by adding all relevant states
        .saturating_add(nice)
        .saturating_add(system)
        .saturating_add(idle)
        .saturating_add(iowait)
        .saturating_add(irq)
        .saturating_add(softirq)
        .saturating_add(steal);

    Ok(CpuStats { // return the total CPU time and the idle time
        total,
        idle: idle.saturating_add(iowait),
    })
}

#[inline] // calculates CPU usage as a percentage
fn calculate_cpu_usage(previous: CpuStats, current: CpuStats) -> u8 {
    let total_delta = current.total.saturating_sub(previous.total);
    let idle_delta = current.idle.saturating_sub(previous.idle);

    if total_delta == 0 {
        return 0;
    }

    let busy_delta = total_delta.saturating_sub(idle_delta); // calculate the amount of CPU time spent doing actual work

    (((busy_delta as u128) * 100 + (total_delta as u128 / 2)) // convert the busy/total ratio into a percentage
        / total_delta as u128)
        .min(100) as u8
}

// ============================================================
// RAM
// ============================================================

struct MemoryReader {
    file: File,
    buffer: [u8; MEM_BUFFER_SIZE], // fixed-size buffer used to read /proc/meminfo
    total_kb: u64, // total amount of system RAM
}

impl MemoryReader { // creates a new MemoryReader
    fn new() -> io::Result<Self> {
        let file = File::open("/proc/meminfo")?;

        let mut reader = Self {
            file,
            buffer: [0u8; MEM_BUFFER_SIZE],
            total_kb: 0,
        };

        reader.total_kb = reader.read_total()?;
        Ok(reader)
    }

    fn read_total(&mut self) -> io::Result<u64> { // reads the total amount of system RAM
        self.file.seek(SeekFrom::Start(0))?;

        let size = self.file.read(&mut self.buffer)?;
        let data = &self.buffer[..size];

        let mut pos = 0;

        while pos < data.len() {
            let start = pos;

            while pos < data.len() && data[pos] != b'\n' {
                pos += 1;
            }

            let line = &data[start..pos];

            if line.starts_with(b"MemTotal:") {
                return Ok(value_after_colon(line));
            }

            pos += 1;
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "MemTotal not found",
        ))
    }

    fn usage_percent(&mut self) -> io::Result<u8> { // calculates the current RAM usage as a percentage.
        self.file.seek(SeekFrom::Start(0))?;

        let size = self.file.read(&mut self.buffer)?;
        let data = &self.buffer[..size];

        let mut pos = 0;

        while pos < data.len() {
            let start = pos;

            while pos < data.len() && data[pos] != b'\n' {
                pos += 1;
            }

            let line = &data[start..pos];

            if line.starts_with(b"MemAvailable:") { // calculate used memory
                let available = value_after_colon(line);
                let used = self.total_kb.saturating_sub(available);

                let percentage = // convert the used memory into a percentage
                    ((used as u128) * 100 + (self.total_kb as u128 / 2))
                        / self.total_kb as u128;

                return Ok(percentage.min(100) as u8);
            }

            pos += 1;
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "MemAvailable not found",
        ))
    }
}

// ============================================================
// DISk
// ============================================================

fn read_disk_usage() -> io::Result<u8> { // reads the disk usage of the root filesystem
    let path = b"/\0";

    let mut stat = Statvfs { // structure that will be filled by the statvfs() call
        f_bsize: 0,
        f_frsize: 0,
        f_blocks: 0,
        f_bfree: 0,
        f_bavail: 0,
        f_files: 0,
        f_ffree: 0,
        f_favail: 0,
        f_fsid: 0,
        f_flag: 0,
        f_namemax: 0,
        f_spare: [0; 6],
    };

    let result = unsafe {
        statvfs(
            path.as_ptr() as *const c_char,
            &mut stat,
        )
    };

    if result != 0 {
        return Err(io::Error::last_os_error());
    }

    if stat.f_blocks == 0 {
        return Ok(0);
    }

    let available = stat.f_bavail; // get the number of blocks available to the user
    let used = stat.f_blocks.saturating_sub(available); // calculate the number of blocks currently in use

    let percentage = // convert the number of used blocks into a percentage
        ((used as u128) * 100 + (stat.f_blocks as u128 / 2))
            / stat.f_blocks as u128;

    Ok(percentage.min(100) as u8)
}

// ============================================================
// NETWORK
// ============================================================

#[derive(Clone, Copy)] // stores the cumulative number of bytes received and transmitted
struct NetworkStats {
    rx_bytes: u64, // total number of bytes received across all network interfaces
    tx_bytes: u64, // total number of bytes transmitted across all network interfaces
}

fn read_network_stats( // reads network statistics
    file: &mut File,
    buffer: &mut [u8; NET_BUFFER_SIZE],
) -> io::Result<NetworkStats> {
    file.seek(SeekFrom::Start(0))?;

    let size = file.read(buffer)?; // read the current contents of /proc/net/dev
    let data = &buffer[..size]; // only process the bytes that were actually read

    let mut rx_total = 0u64;
    let mut tx_total = 0u64;

    let mut pos = 0;

    while pos < data.len() {
        let start = pos;

        while pos < data.len() && data[pos] != b'\n' {
            pos += 1;
        }

        let line = &data[start..pos];

        if let Some(colon) = find_byte(line, b':') {
            let interface = &line[..colon];

            if interface != b" lo" && interface != b"lo" {
                let values = &line[colon + 1..];
                let mut field = 0;

                let rx = read_number(values, &mut field);

                for _ in 0..7 {
                    let _ = read_number(values, &mut field);
                }

                let tx = read_number(values, &mut field);

                rx_total = rx_total.saturating_add(rx);
                tx_total = tx_total.saturating_add(tx);
            }
        }

        pos += 1;
    }

    Ok(NetworkStats {
        rx_bytes: rx_total,
        tx_bytes: tx_total,
    })
}


#[inline] // calculates network speed in Mbps
fn calculate_mbps_x10(
    previous: u64,
    current: u64,
    elapsed: Duration,
) -> u16 {
    let bytes = current.saturating_sub(previous);
    let nanos = elapsed.as_nanos();

    if nanos == 0 {
        return 0;
    }

    let value = (bytes as u128 * 80_000u128 + nanos / 2) // convert the number of transferred bytes into Mbps × 10
        / nanos;

    value.min(u16::MAX as u128) as u16
}

// ============================================================
// TEMP
// ============================================================

struct TemperatureReader {
    file: Option<File>,
    buffer: [u8; TEMP_BUFFER_SIZE],
    temperature: u8, // current temperature in degrees Celsius
}

impl TemperatureReader { // reads the CPU/system temperature
    fn new() -> Self {
        let mut zone = 0u32;

        while zone < 32 {
            let path = format!(
                "{}{}/temp",
                TEMP_GLOB_PREFIX,
                zone
            );

            if let Ok(mut file) = File::open(&path) {
                let mut buffer = [0u8; TEMP_BUFFER_SIZE];

                if let Ok(size) = file.read(&mut buffer) {
                    let data = &buffer[..size];
                    let mut pos = 0;
                    let milli_c = read_number(data, &mut pos);

                    if milli_c > 0 {
                        let celsius = (milli_c / 1000).min(254) as u8; // convert millicelsius to Celsius

                        let _ = file.seek(SeekFrom::Start(0));

                        return Self {
                            file: Some(file),
                            buffer,
                            temperature: celsius,
                        };
                    }
                }
            }

            zone += 1;
        }

        Self {
            file: None,
            buffer: [0u8; TEMP_BUFFER_SIZE],
            temperature: 0xFF,
        }
    }

    fn update(&mut self) { // reads the temperature again
        let Some(file) = self.file.as_mut() else {
            self.temperature = 0xFF;
            return;
        };

        if file.seek(SeekFrom::Start(0)).is_err() {
            self.temperature = 0xFF;
            return;
        }

        let Ok(size) = file.read(&mut self.buffer) else {
            self.temperature = 0xFF;
            return;
        };

        let data = &self.buffer[..size];

        let mut pos = 0;
        let milli_c = read_number(data, &mut pos);

        if milli_c > 0 {
            let celsius = (milli_c / 1000).min(254) as u8;
            self.temperature = celsius;
        } else {
            self.temperature = 0xFF;
        }
    }

    #[inline]
    fn value(&self) -> u8 { // return the current temperature value
        self.temperature
    }

    #[inline]
    fn is_valid(&self) -> bool {
        self.temperature != 0xFF
    }
}

// ============================================================
// UPTIME
// ============================================================

fn read_uptime( // reads the system uptime
    file: &mut File,
    buffer: &mut [u8; UPTIME_BUFFER_SIZE],
) -> io::Result<u32> {
    file.seek(SeekFrom::Start(0))?;

    let size = file.read(buffer)?;
    let data = &buffer[..size];

    let mut pos = 0;
    let seconds = read_number(data, &mut pos);

    Ok(seconds.min(u32::MAX as u64) as u32)
}

// ============================================================
// FLAGS
// ============================================================

// validate packet
const FLAG_TEMP_VALID: u8 = 1 << 0;
const FLAG_UPTIME_VALID: u8 = 1 << 1;

// ============================================================
// CHECKSUM
// ============================================================

#[inline]
fn calculate_checksum(packet: &[u8; PACKET_SIZE - 1]) -> u8 { // calculates checksum over the first 15 bytes of the packet
    let mut checksum = 0u8;

    for byte in packet {
        checksum ^= *byte;
    }

    checksum
}

#[inline]
fn encode_packet( // builds the binary packet that will be sent to the Pico
    cpu: u8,
    ram: u8,
    disk: u8,
    temperature: u8,
    rx_x10: u16,
    tx_x10: u16,
    uptime: u32,
    flags: u8,
) -> [u8; PACKET_SIZE] {
    let rx = rx_x10.to_be_bytes();
    let tx = tx_x10.to_be_bytes();
    let up = uptime.to_be_bytes();

    let mut packet = [0u8; PACKET_SIZE - 1];

    packet[0] = MAGIC;
    packet[1] = PROTOCOL_VERSION;
    packet[2] = cpu;
    packet[3] = ram;
    packet[4] = disk;
    packet[5] = temperature;
    packet[6] = rx[0];
    packet[7] = rx[1];
    packet[8] = tx[0];
    packet[9] = tx[1];
    packet[10] = up[0];
    packet[11] = up[1];
    packet[12] = up[2];
    packet[13] = up[3];
    packet[14] = flags;

    let checksum = calculate_checksum(&packet);

    let mut result = [0u8; PACKET_SIZE];
    result[..PACKET_SIZE - 1].copy_from_slice(&packet);
    result[15] = checksum;

    result
}

// ============================================================
// PARSER
// ============================================================

#[inline]
fn read_number(data: &[u8], position: &mut usize) -> u64 {
    while *position < data.len()
        && data[*position].is_ascii_whitespace()
    {
        *position += 1;
    }

    let mut value = 0u64;

    while *position < data.len() {
        let byte = data[*position];

        if !byte.is_ascii_digit() {
            break;
        }

        value = value
            .saturating_mul(10)
            .saturating_add((byte - b'0') as u64);

        *position += 1;
    }

    value
}

#[inline]
fn value_after_colon(line: &[u8]) -> u64 {
    if let Some(position) = find_byte(line, b':') {
        let mut position = position + 1;
        return read_number(line, &mut position);
    }

    0
}

#[inline]
fn find_byte(data: &[u8], target: u8) -> Option<usize> {
    let mut index = 0;

    while index < data.len() {
        if data[index] == target {
            return Some(index);
        }

        index += 1;
    }

    None
}

// ============================================================
// MAIN
// ============================================================

fn main() -> io::Result<()> {

    // open Linux /proc files once
    let mut cpu_file = File::open(PROC_STAT)?;
    let mut network_file = File::open(PROC_NET_DEV)?;
    let mut uptime_file = File::open(PROC_UPTIME)?;

    let mut cpu_buffer = [0u8; CPU_BUFFER_SIZE];
    let mut network_buffer = [0u8; NET_BUFFER_SIZE];
    let mut uptime_buffer = [0u8; UPTIME_BUFFER_SIZE];

    // --------------------------------------------------------
    // RAM
    // --------------------------------------------------------

    let mut memory = MemoryReader::new()?;

    // --------------------------------------------------------
    // UDP
    // --------------------------------------------------------

    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let destination = (PICO_IP, PICO_PORT);

    // --------------------------------------------------------
    // Disk
    // --------------------------------------------------------

    let mut disk_percent = read_disk_usage()?;
    let mut last_disk_update = Instant::now();

    // --------------------------------------------------------
    // Temp
    // --------------------------------------------------------

    let mut temperature_reader = TemperatureReader::new();

    let mut last_temperature_update = Instant::now();

    // --------------------------------------------------------
    // first CPU and network counter readings
    // --------------------------------------------------------

    let mut previous_cpu =
        read_cpu_stats(&mut cpu_file, &mut cpu_buffer)?;

    let mut previous_network =
        read_network_stats(
            &mut network_file,
            &mut network_buffer,
        )?;

    let mut previous_time = Instant::now();

    // --------------------------------------------------------
    // first measurement window
    // --------------------------------------------------------

    thread::sleep(UPDATE_INTERVAL);

    // --------------------------------------------------------
    // MAIN LOOP
    // --------------------------------------------------------

    // the application now continuously collects metrics, creates a packet and sends it to the Pico
    loop {
        let cycle_start = Instant::now();

        // ====================================================
        // CPU
        // ====================================================

        let current_cpu =
            read_cpu_stats(&mut cpu_file, &mut cpu_buffer)?;

        let cpu = calculate_cpu_usage(
            previous_cpu,
            current_cpu,
        );

        // ====================================================
        // RAM
        // ====================================================

        let ram = memory.usage_percent()?;

        // ====================================================
        // DISK
        // ====================================================

        if last_disk_update.elapsed() >= DISK_UPDATE_INTERVAL {
            disk_percent = read_disk_usage()?;
            last_disk_update = Instant::now();
        }

        // ====================================================
        // TEMP
        // ====================================================

        if last_temperature_update.elapsed() >= TEMP_UPDATE_INTERVAL {
            temperature_reader.update();
            last_temperature_update = Instant::now();
        }

        let temperature = temperature_reader.value();

        // ====================================================
        // NETWORK
        // ====================================================

        let current_network =
            read_network_stats(
                &mut network_file,
                &mut network_buffer,
            )?;

        let elapsed = previous_time.elapsed();

        let rx_x10 = calculate_mbps_x10(
            previous_network.rx_bytes,
            current_network.rx_bytes,
            elapsed,
        );

        let tx_x10 = calculate_mbps_x10(
            previous_network.tx_bytes,
            current_network.tx_bytes,
            elapsed,
        );

        // ====================================================
        // UPTIME
        // ====================================================

        let uptime = read_uptime(
            &mut uptime_file,
            &mut uptime_buffer,
        )?;

        // ====================================================
        // FLAGS
        // ====================================================

        let mut flags = FLAG_UPTIME_VALID;

        if temperature_reader.is_valid() {
            flags |= FLAG_TEMP_VALID;
        }

        // ====================================================
        // PACKET
        // ====================================================

        let packet = encode_packet(
            cpu,
            ram,
            disk_percent,
            temperature,
            rx_x10,
            tx_x10,
            uptime,
            flags,
        );

        // ====================================================
        // UDP
        // ====================================================

        socket.send_to(&packet, destination)?;

        // ====================================================
        // update previous counters
        // ====================================================

        previous_cpu = current_cpu;
        previous_network = current_network;
        previous_time = Instant::now();

        // NOTE:
        // Maintain a 1 Hz update frequency.
        //
        // Metric collection and UDP transmission take time. If the cycle
        // finishes before UPDATE_INTERVAL, sleep for the remaining time.
        //
        // This keeps the monitoring loop at roughly one cycle per second
        // while avoiding unnecessary timing drift.

        let elapsed_cycle = cycle_start.elapsed();

        if elapsed_cycle < UPDATE_INTERVAL {
            thread::sleep(
                UPDATE_INTERVAL - elapsed_cycle
            );
        }
    }
}