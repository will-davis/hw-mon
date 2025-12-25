use eframe::egui;
use sysinfo::System;
use nvml_wrapper::Nvml;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use windows_sys::Win32::System::Performance::{
    self as pdh, PdhAddCounterW, PdhCollectQueryData, PdhGetFormattedCounterValue, PdhOpenQueryW,
    PDH_FMT_COUNTERVALUE, PDH_FMT_DOUBLE,
};

struct HardwareMetrics {
    cpu_usage: f32,
    ram_used_gb: f32,
    ram_total_gb: f32,
    gpu_name: String,
    gpu_pcie_tx: u64, // Bytes/s
    gpu_pcie_rx: u64, // Bytes/s
    gpu_vram_used_mb: u64,
    gpu_vram_total_mb: u64,
    disk_read_bps: u64,
    disk_write_bps: u64,
}

impl Default for HardwareMetrics {
    fn default() -> Self {
        Self {
            cpu_usage: 0.0,
            ram_used_gb: 0.0,
            ram_total_gb: 0.0,
            gpu_name: "Detecting...".to_string(),
            gpu_pcie_tx: 0,
            gpu_pcie_rx: 0,
            gpu_vram_used_mb: 0,
            gpu_vram_total_mb: 0,
            disk_read_bps: 0,
            disk_write_bps: 0,
        }
    }
}

// --- Rust Memory Ownership & Threading Commentary ---
// 1. Arc (Atomic Reference Counted): 
//    We wrap HardwareMetrics in an Arc so multiple threads can "own" a reference to the same data.
//    Arc keeps track of the number of references; when the last reference is dropped, the data is deallocated.
// 2. Mutex (Mutual Exclusion):
//    Arc provides shared ownership, but not mutability. Mutex provides "Interior Mutability" by ensuring
//    only one thread can lock and modify the metrics at a time.
// 3. move closures:
//    The 'move' keyword in std::thread::spawn(move || ...) transfers ownership of the 'metrics_clone' (an Arc)
//    from the main thread into the new monitoring thread.
// 4. Borrowing in update():
//    The GUI thread calls .lock().unwrap() to 'borrow' the metrics for its frame. 
//    This borrow is short-lived; it ends when 'metrics' goes out of scope at the end of update().
// ----------------------------------------------------

struct MonitorApp {
    metrics: Arc<Mutex<HardwareMetrics>>,
}

impl MonitorApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let metrics = Arc::new(Mutex::new(HardwareMetrics::default()));
        
        // Clone the Arc. Cloning an Arc doesn't copy the data, just increments the reference count.
        // Both 'metrics' and 'metrics_clone' now point to the same memory on the heap.
        let metrics_clone = metrics.clone();

        // Spawn a monitoring thread to poll hardware without blocking the GUI.
        // 'move' moves the metrics_clone Arc into this thread's scope.
        std::thread::spawn(move || {
            let mut sys = System::new_all();
            let nvml = Nvml::init().ok();
            
            // Initialization for Windows Disk Counters (PDH)
            let mut query: isize = 0;
            let mut read_counter: isize = 0;
            let mut write_counter: isize = 0;
            
            unsafe {
                if pdh::PdhOpenQueryW(std::ptr::null(), 0, &mut query) == 0 {
                    let read_path: Vec<u16> = r"\PhysicalDisk(_Total)\Disk Read Bytes/sec".encode_utf16().chain(Some(0)).collect();
                    let write_path: Vec<u16> = r"\PhysicalDisk(_Total)\Disk Write Bytes/sec".encode_utf16().chain(Some(0)).collect();
                    
                    unsafe {
                        pdh::PdhAddCounterW(query, read_path.as_ptr(), 0, &mut read_counter);
                        pdh::PdhAddCounterW(query, write_path.as_ptr(), 0, &mut write_counter);
                    }
                }
            }

            loop {
                sys.refresh_all();
                
                // Lock the mutex to get mutable access to the metrics.
                // This blocks other threads (like the UI thread) until we drop 'm'.
                let mut m = metrics_clone.lock().unwrap();
                
                // CPU & RAM usage (sysinfo)
                m.cpu_usage = sys.global_cpu_usage();
                m.ram_used_gb = sys.used_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
                m.ram_total_gb = sys.total_memory() as f32 / 1024.0 / 1024.0 / 1024.0;
                
                // GPU (nvml)
                // Note: We track PCIe utilization here because NVML provides high-fidelity access 
                // to NVIDIA-specific bus metrics. Standard Windows PDH counters usually don't 
                // expose generic PCIe bus utilization for all devices in a unified way.
                if let Some(ref n) = nvml {
                    if let Ok(device) = n.device_by_index(0) {
                        m.gpu_name = device.name().unwrap_or_else(|_| "Unknown GPU".to_string());
                        if let Ok(pcie) = device.pcie_throughput(nvml_wrapper::enum_wrappers::device::PcieUtilCounter::Send) {
                            m.gpu_pcie_tx = pcie as u64 * 1024; // reported in KB/s
                        }
                        if let Ok(pcie) = device.pcie_throughput(nvml_wrapper::enum_wrappers::device::PcieUtilCounter::Receive) {
                            m.gpu_pcie_rx = pcie as u64 * 1024;
                        }
                        if let Ok(mem) = device.memory_info() {
                            m.gpu_vram_used_mb = mem.used / 1024 / 1024;
                            m.gpu_vram_total_mb = mem.total / 1024 / 1024;
                        }
                    }
                }
                
                // Disk metrics using PDH
                unsafe {
                    if query != 0 && pdh::PdhCollectQueryData(query) == 0 {
                        let mut read_value: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
                        if pdh::PdhGetFormattedCounterValue(read_counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut read_value) == 0 {
                            m.disk_read_bps = read_value.Anonymous.doubleValue as u64;
                        }
                        let mut write_value: PDH_FMT_COUNTERVALUE = std::mem::zeroed();
                        if pdh::PdhGetFormattedCounterValue(write_counter, PDH_FMT_DOUBLE, std::ptr::null_mut(), &mut write_value) == 0 {
                            m.disk_write_bps = write_value.Anonymous.doubleValue as u64;
                        }
                    }
                }
                
                // Mutex guard 'm' is dropped here, releasing the lock.
                drop(m); 
                std::thread::sleep(Duration::from_millis(500));
            }
        });

        Self { metrics }
    }
}

impl eframe::App for MonitorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Here we borrow the metrics data for the duration of this function.
        // Rust ensures that while we have this lock, no other thread can mutate the data.
        let metrics = self.metrics.lock().unwrap();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hardware Bandwidth Monitor");
            
            ui.separator();
            
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(format!("CPU Usage: {:.1}%", metrics.cpu_usage));
                    ui.label(format!("RAM: {:.1}/{:.1} GB", metrics.ram_used_gb, metrics.ram_total_gb));
                });
                
                ui.add_space(20.0);
                
                ui.vertical(|ui| {
                    ui.set_max_width(360.0);
                    ui.label(format!("GPU: {}", metrics.gpu_name));
                    ui.label(format!("VRAM: {}/{} MB", metrics.gpu_vram_used_mb, metrics.gpu_vram_total_mb));
                    ui.label(format!("PCIe TX (Send): {:.2} MB/s", metrics.gpu_pcie_tx as f32 / 1024.0 / 1024.0));
                    ui.label(format!("PCIe RX (Receive): {:.2} MB/s", metrics.gpu_pcie_rx as f32 / 1024.0 / 1024.0));
                });
            });
            
            ui.separator();
            ui.heading("Storage Bandwidth (Total Disk I/O)");
            ui.label(format!("Global Read: {:.2} MB/s", metrics.disk_read_bps as f32 / 1024.0 / 1024.0));
            ui.label(format!("Global Write: {:.2} MB/s", metrics.disk_write_bps as f32 / 1024.0 / 1024.0));
            
            ui.add_space(10.0);
            ui.weak("Monitor detects bottlenecks in data movement between NVMe, RAM, and GPU.");
        });

        ctx.request_repaint_after(Duration::from_millis(500));
    }
}

fn load_icon() -> Option<egui::IconData> {
    let icon_path = "assets/favicon.ico";
    if let Ok(image) = image::open(icon_path) {
        let image = image.to_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        Some(egui::IconData { rgba, width, height })
    } else {
        None
    }
}

fn main() -> eframe::Result {
    let mut native_options = eframe::NativeOptions::default();
    
    // Set the favicon if available
    if let Some(icon) = load_icon() {
        native_options.viewport.icon = Some(Arc::new(icon));
    }

    eframe::run_native(
        "Bandwidth Monitor",
        native_options,
        Box::new(|cc| Ok(Box::new(MonitorApp::new(cc)))),
    )
}
