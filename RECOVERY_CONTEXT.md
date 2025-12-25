# Hardware Monitor - Recovery Context

## Current Status
- **Objective**: Rust GUI (eframe) to monitor CPU, RAM, GPU (PCIE RX/TX, VRAM), and Disk Bandwidth.
- **State**: Core logic written in `src/main.rs`. Currently stuck on compilation errors related to `windows-sys` PDH (Performance Data Helper) API.
- **Progress**: 
    - [x] CPU/RAM (sysinfo 0.33)
    - [x] GPU (nvml-wrapper 0.10)
    - [/] Disk (windows-sys 0.52/0.59) - Compilation issues with `PDH_FMT_COUNTER_VALUE` and union access.

## Compilation Errors
- `windows-sys` (0.52 and 0.59) imports for `PDH_FMT_COUNTER_VALUE` and `PdhOpenQueryW` have been inconsistent.
- Last error: `unresolved import windows_sys::Win32::System::Performance::PDH_FMT_COUNTER_...` (The name was truncated in logs).
- The user installed Rust *during* the session, so the shell environment might need path updates (though `cargo check` is running, implying `cargo` is found).

## Environment Info
- **Host**: Windows 11
- **Rust**: Recently installed.
- **Libraries**: `eframe`, `sysinfo`, `nvml-wrapper`, `windows-sys`.

## Next Steps for Future Me
1.  **Run full diagnostic**: Run `cargo check 2>&1 | Out-File -FilePath build_errors.txt` to see full types/names.
2.  **Verify PDH Types**: Check if `PDH_FMT_COUNTER_VALUE` is in `windows_sys::Win32::System::Performance` or a submodule.
3.  **Union Access**: In `windows-sys`, unions are typically handled via an `Anonymous` field or a named field like `u`.
4.  **Confirm Path**: Ensure `$env:Path` includes the cargo bin directory if tools start failing.
