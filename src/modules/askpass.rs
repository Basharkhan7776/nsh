use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Overwrite bytes with zeros securely before dropping or clearing.
pub fn secure_wipe_string(s: &mut String) {
    let bytes = unsafe { s.as_mut_vec() };
    for byte in bytes.iter_mut() {
        unsafe {
            std::ptr::write_volatile(byte, 0);
        }
    }
    s.clear();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthPromptType {
    SudoPassword,
    SshPassword,
    SshKeyPassphrase,
    DevicePin,
    GitCredentials,
    HostVerification,
    GenericPassword,
}

/// Classify prompt string into prompt type, dialog title, description, label, and whether input is masked.
pub fn classify_prompt(prompt: &str) -> (AuthPromptType, String, String, String, bool) {
    let trimmed = prompt.trim();
    let lower = trimmed.to_ascii_lowercase();

    // 1. Host key verification
    if lower.contains("continue connecting (yes/no")
        || lower.contains("continue connecting (yes/no/")
        || lower.contains("authenticity of host")
    {
        // Extract host or fingerprint if present
        let desc = if let Some(start) = trimmed.find('\'') {
            if let Some(end) = trimmed[start + 1..].find('\'') {
                format!("Host: {}", &trimmed[start + 1..start + 1 + end])
            } else {
                trimmed.to_string()
            }
        } else {
            trimmed.to_string()
        };
        return (
            AuthPromptType::HostVerification,
            "Host Key Verification".to_string(),
            desc,
            "Continue (yes/no):".to_string(),
            false, // unmasked
        );
    }

    // 2. SSH key passphrase
    if lower.contains("passphrase") {
        let desc = if let Some(start) = trimmed.find('\'') {
            if let Some(end) = trimmed[start + 1..].find('\'') {
                format!("Key: {}", &trimmed[start + 1..start + 1 + end])
            } else {
                "SSH Private Key Passphrase".to_string()
            }
        } else if let Some(idx) = trimmed.find("/.") {
            let rest = &trimmed[idx..];
            let end = rest.find(':').unwrap_or(rest.len());
            format!("Key: {}", rest[..end].trim())
        } else {
            "SSH Private Key Passphrase".to_string()
        };
        return (
            AuthPromptType::SshKeyPassphrase,
            "SSH Key Passphrase".to_string(),
            desc,
            "Passphrase:".to_string(),
            true, // masked
        );
    }

    // 3. Hardware device PIN / Authenticator PIN
    if lower.contains("pin") || lower.contains("authenticator") || lower.contains("device") {
        return (
            AuthPromptType::DevicePin,
            "Device Security PIN".to_string(),
            if trimmed.is_empty() {
                "Authenticator PIN required".to_string()
            } else {
                trimmed.to_string()
            },
            "PIN:".to_string(),
            true, // masked
        );
    }

    // 4. Sudo password
    if lower.starts_with("[sudo] password for") || lower.starts_with("password for root") {
        let user = if let Some(idx) = lower.find("for ") {
            let rest = &trimmed[idx + 4..];
            let end = rest.find(':').unwrap_or(rest.len());
            rest[..end].trim()
        } else {
            "root"
        };
        return (
            AuthPromptType::SudoPassword,
            "Authentication Required".to_string(),
            format!("User: {}", user),
            "Password:".to_string(),
            true, // masked
        );
    }

    // 5. Git credentials (username or password for HTTP/HTTPS repositories)
    if lower.starts_with("username for")
        || (lower.starts_with("password for")
            && (lower.contains("http://")
                || lower.contains("https://")
                || lower.contains("git@")
                || lower.contains(".git")))
    {
        let is_username = lower.starts_with("username");
        let target = if let Some(start) = trimmed.find('\'') {
            if let Some(end) = trimmed[start + 1..].find('\'') {
                &trimmed[start + 1..start + 1 + end]
            } else {
                trimmed
            }
        } else {
            trimmed
        };
        return (
            AuthPromptType::GitCredentials,
            "Git Authentication".to_string(),
            format!("Repository: {}", target),
            if is_username { "Username:".to_string() } else { "Password:".to_string() },
            !is_username, // masked for password, unmasked for username
        );
    }

    // 6. SSH VPS password
    if lower.contains("'s password") || lower.contains("password for") {
        let desc = if let Some(idx) = trimmed.find("'s password") {
            format!("Host: {}", &trimmed[..idx].trim())
        } else if let Some(idx) = trimmed.find("for ") {
            let rest = &trimmed[idx + 4..];
            let end = rest.find(':').unwrap_or(rest.len());
            format!("Target: {}", rest[..end].trim())
        } else {
            trimmed.to_string()
        };
        return (
            AuthPromptType::SshPassword,
            "SSH Authentication".to_string(),
            desc,
            "Password:".to_string(),
            true, // masked
        );
    }

    // 7. Generic password prompt
    (
        AuthPromptType::GenericPassword,
        "Authentication Required".to_string(),
        if trimmed.is_empty() {
            "Password required to continue".to_string()
        } else {
            trimmed.to_string()
        },
        "Password:".to_string(),
        true, // masked
    )
}

/// Request received from an external process asking for authentication input.
pub struct AskPassPromptEvent {
    pub prompt: String,
    pub prompt_type: AuthPromptType,
    pub title: String,
    pub description: String,
    pub label: String,
    pub is_masked: bool,
    pub response_tx: Sender<Option<String>>,
}

/// Local AskPass IPC Server that coordinates with OpenSSH/Git/Sudo child processes.
pub struct AskPassServer {
    pub socket_path: PathBuf,
    running: Arc<AtomicBool>,
}

impl AskPassServer {
    /// Start an AskPass Unix domain socket server and return the receiver for prompt events.
    pub fn start() -> std::io::Result<(Self, Receiver<AskPassPromptEvent>)> {
        let tmp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let socket_path = tmp_dir.join(format!("nsh-askpass-{}-{}.sock", pid, timestamp));

        if socket_path.exists() {
            let _ = fs::remove_file(&socket_path);
        }

        let listener = UnixListener::bind(&socket_path)?;
        // Restrict permissions to owner only (0o600)
        let _ = fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o600));

        let (event_tx, event_rx) = channel::<AskPassPromptEvent>();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let socket_path_clone = socket_path.clone();

        thread::spawn(move || {
            let _ = listener.set_nonblocking(true);
            while running_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let tx = event_tx.clone();
                        thread::spawn(move || {
                            Self::handle_client(stream, tx);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            if socket_path_clone.exists() {
                let _ = fs::remove_file(&socket_path_clone);
            }
        });

        let socket_path_str = socket_path.to_string_lossy().to_string();
        unsafe {
            std::env::set_var("NSH_ASKPASS_SOCKET", &socket_path_str);
        }

        Ok((
            Self {
                socket_path,
                running,
            },
            event_rx,
        ))
    }

    fn handle_client(mut stream: UnixStream, event_tx: Sender<AskPassPromptEvent>) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(60)));
        let mut reader = BufReader::new(&stream);
        let mut prompt_line = String::new();

        if reader.read_line(&mut prompt_line).is_err() || prompt_line.trim().is_empty() {
            return;
        }

        let prompt = prompt_line.trim_end_matches(['\r', '\n']).to_string();
        let (prompt_type, title, description, label, is_masked) = classify_prompt(&prompt);

        let (resp_tx, resp_rx) = channel::<Option<String>>();
        let event = AskPassPromptEvent {
            prompt,
            prompt_type,
            title,
            description,
            label,
            is_masked,
            response_tx: resp_tx,
        };

        if event_tx.send(event).is_err() {
            return;
        }

        // Await user response from main event loop (timeout: 120 seconds)
        if let Ok(Some(secret)) = resp_rx.recv_timeout(Duration::from_secs(120)) {
            let mut output = secret;
            let _ = stream.write_all(output.as_bytes());
            let _ = stream.write_all(b"\n");
            let _ = stream.flush();
            secure_wipe_string(&mut output);
        }
        // If cancelled or None, stream drops without response, signaling cancellation to askpass client
    }
}

impl Drop for AskPassServer {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if self.socket_path.exists() {
            let _ = fs::remove_file(&self.socket_path);
        }
        unsafe {
            std::env::remove_var("NSH_ASKPASS_SOCKET");
        }
    }
}

/// Prompt directly on /dev/tty when running outside an active socket listener
pub fn prompt_tty_fallback(prompt: &str) -> Option<String> {
    use std::fs::OpenOptions;
    use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
    use crossterm::event::{read, Event, KeyCode, KeyModifiers, KeyEventKind};

    let tty_file = OpenOptions::new().read(true).write(true).open("/dev/tty").ok()?;
    let mut tty_write = tty_file;

    let (_, title, description, label, is_masked) = classify_prompt(prompt);

    // Clean monochrome box
    let _ = write!(tty_write, "\r\n┌─────────────────────────────────────────────────────────────┐\r\n");
    let title_display = if title.len() > 44 { &title[..44] } else { &title };
    let _ = write!(tty_write, "│ {:<44} [Esc Cancel]│\r\n", title_display);
    let desc_cut = if description.len() > 48 { &description[..48] } else { &description };
    let _ = write!(tty_write, "│ Target:    {:<48} │\r\n", desc_cut);
    let _ = write!(tty_write, "├─────────────────────────────────────────────────────────────┤\r\n");
    let _ = write!(tty_write, "│ {:<11} ", label);
    let _ = tty_write.flush();

    let _ = enable_raw_mode();
    let mut input = String::new();

    let res = loop {
        match read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                match key.code {
                    KeyCode::Enter => {
                        break Some(input);
                    }
                    KeyCode::Esc => {
                        break None;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break None;
                    }
                    KeyCode::Backspace => {
                        if !input.is_empty() {
                            input.pop();
                            let _ = write!(tty_write, "\x08 \x08");
                            let _ = tty_write.flush();
                        }
                    }
                    KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        let count = if is_masked { input.chars().count() } else { input.len() };
                        let _ = write!(tty_write, "{}{}{}", "\x08".repeat(count), " ".repeat(count), "\x08".repeat(count));
                        let _ = tty_write.flush();
                        input.clear();
                    }
                    KeyCode::Char(c) => {
                        input.push(c);
                        if is_masked {
                            let _ = write!(tty_write, "•");
                        } else {
                            let _ = write!(tty_write, "{}", c);
                        }
                        let _ = tty_write.flush();
                    }
                    _ => {}
                }
            }
            Err(_) => break None,
            _ => {}
        }
    };

    let _ = disable_raw_mode();
    let _ = write!(tty_write, "\r\n└─────────────────────────────────────────────────────────────┘\r\n");
    let _ = tty_write.flush();

    res
}

/// Client routine executed when nsh is invoked as an askpass handler.
pub fn run_askpass_client(prompt: &str) -> std::process::ExitCode {
    if let Ok(socket_path) = std::env::var("NSH_ASKPASS_SOCKET") {
        if Path::new(&socket_path).exists() {
            if let Ok(mut stream) = UnixStream::connect(&socket_path) {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(120)));
                let _ = stream.write_all(prompt.as_bytes());
                let _ = stream.write_all(b"\n");
                let _ = stream.flush();

                let mut reader = BufReader::new(stream);
                let mut response = String::new();
                if reader.read_line(&mut response).is_ok() && !response.is_empty() {
                    let secret = response.trim_end_matches(['\r', '\n']);
                    print!("{}", secret);
                    let _ = std::io::stdout().flush();
                    secure_wipe_string(&mut response);
                    return std::process::ExitCode::SUCCESS;
                }
            }
        }
    }

    // Fallback 1: prompt directly on controlling terminal /dev/tty
    if let Some(mut secret) = prompt_tty_fallback(prompt) {
        print!("{}", secret);
        let _ = std::io::stdout().flush();
        secure_wipe_string(&mut secret);
        return std::process::ExitCode::SUCCESS;
    }

    // Fallback 2: try zenity GUI dialog if DISPLAY is present
    let has_display =
        std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
    if has_display {
        let mut cmd = std::process::Command::new("zenity");
        let (_, title, _, _, is_masked) = classify_prompt(prompt);
        if is_masked {
            cmd.arg("--password");
        } else {
            cmd.arg("--entry");
        }
        cmd.arg(format!("--title={}", title));
        cmd.arg(format!("--text={}", prompt.trim()));

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let secret = String::from_utf8_lossy(&output.stdout)
                    .trim_end_matches(['\r', '\n'])
                    .to_string();
                print!("{}", secret);
                let _ = std::io::stdout().flush();
                return std::process::ExitCode::SUCCESS;
            }
        }
    }

    std::process::ExitCode::FAILURE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_ssh_key_passphrase() {
        let (pt, title, desc, label, masked) =
            classify_prompt("Enter passphrase for key '/home/user/.ssh/id_ed25519': ");
        assert_eq!(pt, AuthPromptType::SshKeyPassphrase);
        assert_eq!(title, "SSH Key Passphrase");
        assert!(desc.contains("/home/user/.ssh/id_ed25519"));
        assert_eq!(label, "Passphrase:");
        assert!(masked);
    }

    #[test]
    fn test_classify_ssh_password() {
        let (pt, title, desc, label, masked) =
            classify_prompt("root@192.168.1.10's password: ");
        assert_eq!(pt, AuthPromptType::SshPassword);
        assert_eq!(title, "SSH Authentication");
        assert!(desc.contains("root@192.168.1.10"));
        assert_eq!(label, "Password:");
        assert!(masked);
    }

    #[test]
    fn test_classify_device_pin() {
        let (pt, title, _, label, masked) =
            classify_prompt("Enter PIN for authenticator: ");
        assert_eq!(pt, AuthPromptType::DevicePin);
        assert_eq!(title, "Device Security PIN");
        assert_eq!(label, "PIN:");
        assert!(masked);
    }

    #[test]
    fn test_classify_host_verification_unmasked() {
        let (pt, title, _, label, masked) = classify_prompt(
            "The authenticity of host 'github.com (140.82.121.4)' can't be established. Are you sure you want to continue connecting (yes/no/[fingerprint])? "
        );
        assert_eq!(pt, AuthPromptType::HostVerification);
        assert_eq!(title, "Host Key Verification");
        assert_eq!(label, "Continue (yes/no):");
        assert!(!masked, "Host key verification must not be masked");
    }

    #[test]
    fn test_classify_sudo_password() {
        let (pt, title, desc, label, masked) =
            classify_prompt("[sudo] password for bashar: ");
        assert_eq!(pt, AuthPromptType::SudoPassword);
        assert_eq!(title, "Authentication Required");
        assert!(desc.contains("bashar"));
        assert_eq!(label, "Password:");
        assert!(masked);
    }

    #[test]
    fn test_classify_git_username() {
        let (pt, title, desc, label, masked) =
            classify_prompt("Username for 'https://github.com': ");
        assert_eq!(pt, AuthPromptType::GitCredentials);
        assert_eq!(title, "Git Authentication");
        assert!(desc.contains("https://github.com"));
        assert_eq!(label, "Username:");
        assert!(!masked, "Username must not be masked");
    }

    #[test]
    fn test_askpass_server_client_roundtrip() {
        let (server, rx) = AskPassServer::start().expect("Failed to start AskPassServer");
        let sock_str = server.socket_path.to_str().unwrap().to_string();

        let client_thread = thread::spawn(move || {
            let mut stream = UnixStream::connect(&sock_str).expect("Failed to connect");
            stream.write_all(b"Enter passphrase for key '/test/key': \n").unwrap();
            stream.flush().unwrap();

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            line.trim_end().to_string()
        });

        // Server receives prompt
        let event = rx.recv_timeout(Duration::from_secs(5)).expect("Did not receive event");
        assert_eq!(event.prompt_type, AuthPromptType::SshKeyPassphrase);
        assert_eq!(event.title, "SSH Key Passphrase");

        // Respond with secret
        event.response_tx.send(Some("unlocked_secret_123".to_string())).unwrap();

        let received = client_thread.join().unwrap();
        assert_eq!(received, "unlocked_secret_123");
    }

    #[test]
    fn test_askpass_server_client_cancellation() {
        let (server, rx) = AskPassServer::start().expect("Failed to start AskPassServer");
        let sock_str = server.socket_path.to_str().unwrap().to_string();

        let client_thread = thread::spawn(move || {
            let mut stream = UnixStream::connect(&sock_str).expect("Failed to connect");
            stream.write_all(b"user@host's password: \n").unwrap();
            stream.flush().unwrap();

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            let n = reader.read_line(&mut line).unwrap();
            n // Expect 0 (EOF on cancellation)
        });

        let event = rx.recv_timeout(Duration::from_secs(5)).expect("Did not receive event");
        // User cancels prompt
        event.response_tx.send(None).unwrap();

        let bytes_read = client_thread.join().unwrap();
        assert_eq!(bytes_read, 0, "Client should receive EOF on cancellation");
    }

    #[test]
    fn test_secure_wipe_string() {
        let mut secret = "my_sensitive_password".to_string();
        secure_wipe_string(&mut secret);
        assert_eq!(secret, "");
        assert!(secret.is_empty());
    }
}
