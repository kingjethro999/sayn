use crate::dependencies::DependencyManager;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct DevServerManager;

impl DevServerManager {
    pub fn is_port_available(port: u16) -> bool {
        TcpListener::bind(("127.0.0.1", port)).is_ok()
    }

    pub fn find_available_port(starting_port: u16) -> u16 {
        for port in starting_port..starting_port + 100 {
            if Self::is_port_available(port) {
                return port;
            }
        }
        starting_port
    }

    /// Start the Next.js dev server using the detected package manager.
    pub fn start_dev_server(
        gen_root: &Path,
        port: u16,
    ) -> std::io::Result<Child> {
        let pm = DependencyManager::detect_package_manager()
            .unwrap_or("pnpm");

        Command::new(pm)
            .arg("run")
            .arg("dev")
            .arg("--")
            .arg("-p")
            .arg(port.to_string())
            .current_dir(gen_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_port_availability() {
        let port = DevServerManager::find_available_port(3000);
        assert!(port >= 3000);
    }
}
