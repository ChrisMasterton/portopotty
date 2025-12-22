#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

use netstat2::{
    get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState,
};
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessesToUpdate, System};

#[derive(Debug, Clone, Deserialize)]
struct PortRange {
    start: u16,
    end: u16,
}

#[derive(Debug, Clone, Serialize)]
struct ListenerInfo {
    port: u16,
    pid: u32,
    process_name: Option<String>,
    started_seconds_ago: Option<u64>,
}

fn in_any_range(port: u16, ranges: &[PortRange]) -> bool {
    ranges.iter().any(|r| {
        let (a, b) = if r.start <= r.end {
            (r.start, r.end)
        } else {
            (r.end, r.start)
        };
        port >= a && port <= b
    })
}

#[tauri::command]
fn scan_ports(ranges: Vec<PortRange>) -> Result<Vec<ListenerInfo>, String> {
    if ranges.is_empty() {
        return Ok(vec![]);
    }

    let af_flags = AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6;
    let proto_flags = ProtocolFlags::TCP;
    let sockets = get_sockets_info(af_flags, proto_flags).map_err(|e| e.to_string())?;

    let mut system = System::new_all();
    system.refresh_processes(ProcessesToUpdate::All, true);

    let mut out: Vec<ListenerInfo> = Vec::new();
    for socket in sockets {
        let (port, is_listen) = match socket.protocol_socket_info {
            ProtocolSocketInfo::Tcp(tcp) => (tcp.local_port, tcp.state == TcpState::Listen),
            _ => continue,
        };
        if !is_listen {
            continue;
        }
        if !in_any_range(port, &ranges) {
            continue;
        }

        for pid in socket.associated_pids {
            let pid_u32 = pid as u32;
            let pid_sys = Pid::from_u32(pid_u32);
            let proc = system.process(pid_sys);
            let process_name = proc.map(|p| p.name().to_string_lossy().to_string());
            let started_seconds_ago = proc.map(|p| p.run_time());
            out.push(ListenerInfo {
                port,
                pid: pid_u32,
                process_name,
                started_seconds_ago,
            });
        }
    }

    Ok(out)
}

#[tauri::command]
fn kill_pid(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("invalid pid".to_string());
    }

    #[cfg(windows)]
    {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("taskkill failed (exit {:?})", status.code()))
        }
    }

    #[cfg(not(windows))]
    {
        let term = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;

        if term.success() {
            return Ok(());
        }

        let kill = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;

        if kill.success() {
            Ok(())
        } else {
            Err(format!("kill failed (exit {:?})", kill.code()))
        }
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        #[cfg(target_os = "macos")]
                        let _ = app.set_activation_policy(ActivationPolicy::Regular);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        #[cfg(target_os = "macos")]
                        let _ = app.set_activation_policy(ActivationPolicy::Regular);
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } => {
                #[cfg(target_os = "macos")]
                let _ = window.app_handle().set_activation_policy(ActivationPolicy::Accessory);
                let _ = window.hide();
                api.prevent_close();
            }
            WindowEvent::Focused(false) => {
                if let Ok(minimized) = window.is_minimized() {
                    if minimized {
                        #[cfg(target_os = "macos")]
                        let _ = window.app_handle().set_activation_policy(ActivationPolicy::Accessory);
                        let _ = window.hide();
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![scan_ports, kill_pid])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
