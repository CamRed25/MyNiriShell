// System statistics reader — CPU, memory, network, volume.
// Zero GTK4 imports. Called on a background thread; results are sent to the
// main thread for display.

use std::fs;
use std::io::{self, BufRead, BufReader};
use std::sync::Mutex;
use std::time::Instant;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SysInfoError {
    #[error("failed to read {path}: {source}")]
    Io { path: &'static str, source: io::Error },
    #[error("failed to parse {path}: unexpected format")]
    Parse { path: &'static str },
}

/// A snapshot of current system statistics, ready for display.
#[derive(Debug, Clone, Default)]
pub struct SysSnapshot {
    /// CPU usage 0–100 %.
    pub cpu_percent: f32,
    /// Memory in use, bytes.
    pub memory_used: u64,
    /// Total memory, bytes.
    pub memory_total: u64,
    /// Network upload, bytes/s.
    pub net_up: u64,
    /// Network download, bytes/s.
    pub net_down: u64,
    /// Master volume 0–100.
    pub volume: u8,
}

// ── CPU ───────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct CpuTick {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
}

impl CpuTick {
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait + self.irq + self.softirq
    }
    fn idle(&self) -> u64 {
        self.idle + self.iowait
    }
}

fn read_cpu_tick() -> Result<CpuTick, SysInfoError> {
    let content = fs::read_to_string("/proc/stat")
        .map_err(|e| SysInfoError::Io { path: "/proc/stat", source: e })?;
    let line = content.lines().next().ok_or(SysInfoError::Parse { path: "/proc/stat" })?;
    let mut parts = line.split_whitespace().skip(1);
    let mut next = || -> Result<u64, SysInfoError> {
        parts
            .next()
            .and_then(|s| s.parse().ok())
            .ok_or(SysInfoError::Parse { path: "/proc/stat" })
    };
    Ok(CpuTick {
        user: next()?,
        nice: next()?,
        system: next()?,
        idle: next()?,
        iowait: next()?,
        irq: next()?,
        softirq: next()?,
    })
}

// ── Memory ────────────────────────────────────────────────────────────────────

fn read_memory() -> Result<(u64, u64), SysInfoError> {
    let f = fs::File::open("/proc/meminfo")
        .map_err(|e| SysInfoError::Io { path: "/proc/meminfo", source: e })?;
    let reader = BufReader::new(f);

    let mut total_kb = 0u64;
    let mut available_kb = 0u64;
    let mut found = 0u8;

    for line in reader.lines() {
        let line = line.map_err(|e| SysInfoError::Io { path: "/proc/meminfo", source: e })?;
        if line.starts_with("MemTotal:") {
            total_kb = parse_kb_field(&line)
                .ok_or(SysInfoError::Parse { path: "/proc/meminfo" })?;
            found += 1;
        } else if line.starts_with("MemAvailable:") {
            available_kb = parse_kb_field(&line)
                .ok_or(SysInfoError::Parse { path: "/proc/meminfo" })?;
            found += 1;
        }
        if found == 2 {
            break;
        }
    }

    let total = total_kb * 1024;
    let used = total.saturating_sub(available_kb * 1024);
    Ok((used, total))
}

fn parse_kb_field(line: &str) -> Option<u64> {
    // e.g. "MemTotal:       16298340 kB"
    line.split_whitespace().nth(1)?.parse().ok()
}

// ── Network ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Default)]
struct NetTick {
    rx: u64,
    tx: u64,
}

fn read_net_tick() -> Result<NetTick, SysInfoError> {
    let f = fs::File::open("/proc/net/dev")
        .map_err(|e| SysInfoError::Io { path: "/proc/net/dev", source: e })?;
    let reader = BufReader::new(f);

    let mut total = NetTick::default();

    // First two lines are headers.
    for (idx, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| SysInfoError::Io { path: "/proc/net/dev", source: e })?;
        if idx < 2 {
            continue;
        }
        let (iface, rest) = match line.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };
        // Skip loopback.
        if iface.trim() == "lo" {
            continue;
        }
        let mut fields = rest.split_whitespace();
        let rx: u64 = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        // skip 7 receive fields, then tx bytes is field 9
        for _ in 0..7 {
            fields.next();
        }
        let tx: u64 = fields.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        total.rx = total.rx.saturating_add(rx);
        total.tx = total.tx.saturating_add(tx);
    }

    Ok(total)
}

// ── Volume ────────────────────────────────────────────────────────────────────

fn read_volume() -> u8 {
    // `pactl get-sink-volume @DEFAULT_SINK@` outputs e.g.:
    //   Volume: front-left: 65536 /  100% / 0.00 dB,   front-right: ...
    let output = std::process::Command::new("pactl")
        .args(["get-sink-volume", "@DEFAULT_SINK@"])
        .output();
    let Ok(out) = output else { return 0 };
    let text = String::from_utf8_lossy(&out.stdout);
    // Find first "NN%"
    for token in text.split_whitespace() {
        if let Some(pct_str) = token.strip_suffix('%') {
            if let Ok(v) = pct_str.parse::<u8>() {
                return v;
            }
        }
    }
    0
}

// ── Stateful sampler ──────────────────────────────────────────────────────────

/// Holds the previous sample so we can compute deltas.
pub struct Sampler {
    prev_cpu: CpuTick,
    prev_net: NetTick,
    prev_time: Instant,
}

impl Sampler {
    pub fn new() -> Self {
        let cpu = read_cpu_tick().unwrap_or_default();
        let net = read_net_tick().unwrap_or_default();
        Self { prev_cpu: cpu, prev_net: net, prev_time: Instant::now() }
    }

    /// Take a new reading and return a [`SysSnapshot`].
    pub fn sample(&mut self) -> SysSnapshot {
        let now = Instant::now();
        let elapsed = now.duration_since(self.prev_time).as_secs_f64().max(0.001);
        self.prev_time = now;

        // CPU %
        let new_cpu = read_cpu_tick().unwrap_or(self.prev_cpu);
        let total_delta =
            new_cpu.total().saturating_sub(self.prev_cpu.total()) as f64;
        let idle_delta =
            new_cpu.idle().saturating_sub(self.prev_cpu.idle()) as f64;
        let cpu_pct = if total_delta > 0.0 {
            ((1.0 - idle_delta / total_delta) * 100.0).clamp(0.0, 100.0) as f32
        } else {
            0.0
        };
        self.prev_cpu = new_cpu;

        // Memory
        let (mem_used, mem_total) = read_memory().unwrap_or((0, 0));

        // Network bytes/s
        let new_net = read_net_tick().unwrap_or(self.prev_net);
        let net_up =
            (new_net.tx.saturating_sub(self.prev_net.tx) as f64 / elapsed) as u64;
        let net_down =
            (new_net.rx.saturating_sub(self.prev_net.rx) as f64 / elapsed) as u64;
        self.prev_net = new_net;

        // Volume (cheap subprocess, once per 2-second tick)
        let volume = read_volume();

        SysSnapshot { cpu_percent: cpu_pct, memory_used: mem_used, memory_total: mem_total,
            net_up, net_down, volume }
    }
}

// Thread-safe shared sampler — wrapped in Mutex so the GTK timer can call it.
static SAMPLER: Mutex<Option<Sampler>> = Mutex::new(None);

/// Initialise the global sampler (call once at startup, before first tick).
pub fn init_sampler() {
    let mut guard = SAMPLER.lock().unwrap();
    *guard = Some(Sampler::new());
}

/// Take a new system snapshot. Returns `None` if the sampler isn't initialised.
pub fn sample() -> Option<SysSnapshot> {
    SAMPLER.lock().unwrap().as_mut().map(|s| s.sample())
}

/// Adjust the default audio sink volume by `delta_pct` percent (positive = louder).
pub fn set_volume_delta(delta_pct: i8) {
    let arg = format!("{:+}%", delta_pct);
    let _ = std::process::Command::new("pactl")
        .args(["set-sink-volume", "@DEFAULT_SINK@", &arg])
        .spawn();
}
