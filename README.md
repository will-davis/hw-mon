# Hardware Monitor

A high-performance hardware bandwidth and utilization monitor for Windows 11, built with Rust.

## Features

- **CPU Monitoring**: Real-time utilization tracking using `sysinfo`.
- **RAM Monitoring**: Total and used memory tracking.
- **GPU Monitoring**: NVMe/GPU bandwidth (PCIE RX/TX) and VRAM usage via `nvml-wrapper`.
- **Disk Monitoring**: Bandwidth utilization using Windows Performance Data Helper (PDH) via `windows-sys`.
- **Modern GUI**: Built with `eframe` (egui) for a responsive and lightweight interface.

## Tech Stack

- **Language**: [Rust](https://www.rust-lang.org/)
- **GUI Framework**: [eframe / egui](https://github.com/emilk/egui)
- **Monitoring APIs**:
  - `sysinfo`: CPU and RAM metrics.
  - `nvml-wrapper`: NVIDIA GPU metrics.
  - `windows-sys`: Windows PDH for Disk bandwidth metrics.

## Status

**Currently in Development.**
The core logic for CPU, RAM, and GPU monitoring is implemented. Integration of Disk monitoring via PDH is currently being refined to resolve compilation abstractions with Windows FFI types.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable recommended)
- Windows 11 (uses Windows-specific APIs for Disk monitoring)

### Build & Run

```powershell
cargo run --release
```
