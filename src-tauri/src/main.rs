#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, WindowEvent,
};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

use netstat2::{get_sockets_info, AddressFamilyFlags, ProtocolFlags, ProtocolSocketInfo, TcpState};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    process::{Command, Output},
    thread,
    time::Duration,
};
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
    container_id: Option<String>,
    container_name: Option<String>,
}

#[derive(Debug, Clone)]
struct DockerPublishedContainer {
    id: String,
    name: String,
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
    let docker_ports = docker_published_containers_by_port();

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
            let container = docker_ports.get(&port);
            out.push(ListenerInfo {
                port,
                pid: pid_u32,
                process_name,
                started_seconds_ago,
                container_id: container.map(|c| c.id.clone()),
                container_name: container.map(|c| c.name.clone()),
            });
        }
    }

    Ok(out)
}

fn docker_published_containers_by_port() -> HashMap<u16, DockerPublishedContainer> {
    let output = match docker_output(&["ps", "--format", "{{json .}}"]) {
        Ok(output) => output,
        Err(_) => return HashMap::new(),
    };

    if !output.status.success() {
        return HashMap::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers = HashMap::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let id = value
            .get("ID")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let name = value
            .get("Names")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let ports = value
            .get("Ports")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        if id.is_empty() {
            continue;
        }

        for port in host_ports_from_docker_ports(ports) {
            containers
                .entry(port)
                .or_insert_with(|| DockerPublishedContainer {
                    id: id.clone(),
                    name: name.clone(),
                });
        }
    }

    containers
}

fn host_ports_from_docker_ports(ports: &str) -> Vec<u16> {
    ports
        .split(',')
        .filter_map(|segment| segment.trim().split("->").next())
        .flat_map(parse_docker_host_ports)
        .collect()
}

fn parse_docker_host_ports(host: &str) -> Vec<u16> {
    let Some(port_part) = host.trim().rsplit(':').next() else {
        return Vec::new();
    };
    let port_part = port_part.trim();
    if let Some((start, end)) = port_part.split_once('-') {
        let Ok(start) = start.parse::<u16>() else {
            return Vec::new();
        };
        let Ok(end) = end.parse::<u16>() else {
            return Vec::new();
        };
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        return (start..=end).collect();
    }
    port_part
        .parse::<u16>()
        .map(|port| vec![port])
        .unwrap_or_default()
}

#[tauri::command]
fn disconnect_listener(port: u16, pid: u32) -> Result<String, String> {
    if let Some(container) = docker_published_containers_by_port().get(&port) {
        stop_docker_container(container)?;
        if !wait_until_port_closes(port, 1_500) {
            return Err(format!(
                "stopped container {}, but port {} is still listening",
                container.name, port
            ));
        }
        return Ok(format!("stopped container {}", container.name));
    }

    kill_pid(pid)?;
    if !wait_until_port_closes(port, 1_500) {
        return Err(format!(
            "killed pid {}, but port {} is still listening",
            pid, port
        ));
    }
    Ok(format!("killed pid {}", pid))
}

fn stop_docker_container(container: &DockerPublishedContainer) -> Result<(), String> {
    let stop = docker_output(&["stop", "--timeout", "2", &container.id])
        .map_err(|e| format!("docker stop failed to start: {e}"))?;

    if stop.status.success() {
        return Ok(());
    }

    let kill = docker_output(&["kill", &container.id])
        .map_err(|e| format!("docker kill failed to start: {e}"))?;

    if kill.status.success() {
        Ok(())
    } else {
        let stop_err = String::from_utf8_lossy(&stop.stderr);
        let kill_err = String::from_utf8_lossy(&kill.stderr);
        Err(format!(
            "docker stop failed for {} (exit {:?}): {}; docker kill failed (exit {:?}): {}",
            container.name,
            stop.status.code(),
            stop_err.trim(),
            kill.status.code(),
            kill_err.trim()
        ))
    }
}

fn docker_output(args: &[&str]) -> std::io::Result<Output> {
    let candidates = [
        "docker",
        "/opt/homebrew/bin/docker",
        "/usr/local/bin/docker",
        "/Applications/Docker.app/Contents/Resources/bin/docker",
    ];
    let mut last_error = None;

    for candidate in candidates {
        match Command::new(candidate).args(args).output() {
            Ok(output) => return Ok(output),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "docker command not found")
    }))
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
        let term = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;

        if !term.success() {
            return Err(format!("kill -TERM failed (exit {:?})", term.code()));
        }

        if wait_until_pid_exits(pid, 800) {
            return Ok(());
        }

        let kill = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status()
            .map_err(|e| e.to_string())?;

        if kill.success() && wait_until_pid_exits(pid, 800) {
            Ok(())
        } else if kill.success() {
            Err(format!("PID {pid} accepted SIGKILL but is still running"))
        } else {
            Err(format!("kill failed (exit {:?})", kill.code()))
        }
    }
}

#[cfg(not(windows))]
fn wait_until_pid_exits(pid: u32, timeout_ms: u64) -> bool {
    let attempts = (timeout_ms / 100).max(1);
    for _ in 0..attempts {
        if !pid_is_running(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    !pid_is_running(pid)
}

#[cfg(not(windows))]
fn pid_is_running(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn wait_until_port_closes(port: u16, timeout_ms: u64) -> bool {
    let attempts = (timeout_ms / 100).max(1);
    for _ in 0..attempts {
        if !port_has_listener(port) {
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
    !port_has_listener(port)
}

fn port_has_listener(port: u16) -> bool {
    let Ok(sockets) = get_sockets_info(AddressFamilyFlags::IPV4 | AddressFamilyFlags::IPV6, ProtocolFlags::TCP) else {
        return false;
    };

    sockets.into_iter().any(|socket| match socket.protocol_socket_info {
        ProtocolSocketInfo::Tcp(tcp) => tcp.local_port == port && tcp.state == TcpState::Listen,
        _ => false,
    })
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
                let _ = window
                    .app_handle()
                    .set_activation_policy(ActivationPolicy::Accessory);
                let _ = window.hide();
                api.prevent_close();
            }
            WindowEvent::Focused(false) => {
                if let Ok(minimized) = window.is_minimized() {
                    if minimized {
                        #[cfg(target_os = "macos")]
                        let _ = window
                            .app_handle()
                            .set_activation_policy(ActivationPolicy::Accessory);
                        let _ = window.hide();
                    }
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            scan_ports,
            disconnect_listener,
            kill_pid
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_ipv4_and_ipv6_published_docker_ports() {
        let ports = "0.0.0.0:7777->8080/tcp, [::]:27017->27017/tcp";

        assert_eq!(host_ports_from_docker_ports(ports), vec![7777, 27017]);
    }

    #[test]
    fn ignores_unpublished_container_ports() {
        assert!(host_ports_from_docker_ports("80/tcp, 443/tcp").is_empty());
    }

    #[test]
    fn expands_published_port_ranges() {
        assert_eq!(
            host_ports_from_docker_ports("0.0.0.0:8000-8003->80-83/tcp"),
            vec![8000, 8001, 8002, 8003]
        );
    }

    #[test]
    fn parses_localhost_published_ports() {
        assert_eq!(
            host_ports_from_docker_ports("127.0.0.1:5432->5432/tcp"),
            vec![5432]
        );
    }
}
