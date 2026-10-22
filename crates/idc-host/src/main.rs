//! Tarayıcı eklentisi ile IDC uygulaması arasındaki yerel mesajlaşma köprüsü.
//!
//! Tarayıcı bu programı başlatır ve stdin/stdout üzerinden `4 bayt uzunluk + JSON` mesajları
//! gönderir. Köprü her mesajı `127.0.0.1` üzerindeki uygulamaya iletir, cevabı geri yazar.
//! Uygulama çalışmıyorsa arka planda başlatır. Günlükler yalnız stderr'e yazılır.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const APP_DIR_LINUX: &str = "internet-download-center";
const APP_DIR_WINDOWS: &str = "InternetDownloadCenter";

fn config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("IDC_DATA_DIR") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())) {
        if dir.join("portable.txt").exists() {
            return dir.join("data");
        }
    }
    let name = if cfg!(windows) { APP_DIR_WINDOWS } else { APP_DIR_LINUX };
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join(name)
}

#[derive(serde::Deserialize)]
struct Ipc {
    port: u16,
    token: String,
}

fn read_ipc() -> Option<Ipc> {
    let data = std::fs::read(config_dir().join("ipc.json")).ok()?;
    serde_json::from_slice(&data).ok()
}

fn read_message(r: &mut impl Read) -> Option<Vec<u8>> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).ok()?;
    let n = u32::from_ne_bytes(len) as usize;
    if n > 64 * 1024 * 1024 {
        return None;
    }
    let mut buf = vec![0u8; n];
    r.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn write_message(w: &mut impl Write, data: &[u8]) -> std::io::Result<()> {
    w.write_all(&(data.len() as u32).to_ne_bytes())?;
    w.write_all(data)?;
    w.flush()
}

/// Uygulamaya tek bir HTTP isteği gönderir.
fn post(ipc: &Ipc, body: &[u8], timeout: Duration) -> std::io::Result<Vec<u8>> {
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], ipc.port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2))?;
    s.set_read_timeout(Some(timeout))?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
    let head = format!(
        "POST /api/ext HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nX-IDC-Token: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        ipc.port,
        ipc.token,
        body.len()
    );
    s.write_all(head.as_bytes())?;
    s.write_all(body)?;
    let mut resp = Vec::new();
    s.read_to_end(&mut resp)?;
    let split = resp.windows(4).position(|w| w == b"\r\n\r\n").ok_or_else(|| std::io::Error::other("geçersiz cevap"))?;
    let head = String::from_utf8_lossy(&resp[..split]).to_string();
    let status_ok = head.lines().next().map(|l| l.contains(" 200")).unwrap_or(false);
    if !status_ok {
        return Err(std::io::Error::other(format!("uygulama cevabı: {}", head.lines().next().unwrap_or(""))));
    }
    Ok(resp[split + 4..].to_vec())
}

fn app_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let names: &[&str] = if cfg!(windows) {
        &["internet-download-center.exe", "Internet Download Center.exe"]
    } else {
        &["internet-download-center"]
    };
    for n in names {
        let p = dir.join(n);
        if p.is_file() {
            return Some(p);
        }
    }
    // AppImage içinden kopyalanmış köprü uygulamanın yerini bir dosyada tutar.
    if let Ok(p) = std::fs::read_to_string(config_dir().join("app-path.txt")) {
        let p = PathBuf::from(p.trim());
        if p.is_file() {
            return Some(p);
        }
    }
    if cfg!(unix) {
        for p in ["/usr/bin/internet-download-center", "/usr/local/bin/internet-download-center"] {
            if std::path::Path::new(p).is_file() {
                return Some(PathBuf::from(p));
            }
        }
    }
    None
}

/// Uygulamayı tarayıcıdan tamamen ayrık başlatır (tarayıcının boruları/iş nesnesi miras alınmaz).
fn launch_app() -> bool {
    let Some(path) = app_path() else {
        eprintln!("idc-host: uygulama bulunamadı");
        return false;
    };
    let mut c = std::process::Command::new(&path);
    c.arg("--background").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            c.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        return c.spawn().is_ok();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
        c.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_BREAKAWAY_FROM_JOB);
        if c.spawn().is_ok() {
            return true;
        }
        let mut c2 = std::process::Command::new(&path);
        c2.arg("--background").creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
        return c2.spawn().is_ok();
    }
    #[allow(unreachable_code)]
    false
}

fn timeout_for(msg: &serde_json::Value) -> Duration {
    match msg.get("type").and_then(|t| t.as_str()) {
        Some("media_probe") => Duration::from_secs(150),
        Some("download") => Duration::from_secs(25),
        _ => Duration::from_secs(20),
    }
}

/// Mesajı iletir; uygulama kapalıysa başlatıp yeniden dener.
fn forward(body: &[u8]) -> Vec<u8> {
    let msg: serde_json::Value = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
    let timeout = timeout_for(&msg);
    if let Some(ipc) = read_ipc() {
        match post(&ipc, body, timeout) {
            Ok(r) => return r,
            Err(e) => eprintln!("idc-host: {e}"),
        }
    }
    // Merhaba mesajında uygulamayı başlatma: tarayıcı açılışında her seferinde IDC açılmasın.
    let is_hello = msg.get("type").and_then(|t| t.as_str()) == Some("hello");
    let launch_on_hello = msg.get("launch").and_then(|v| v.as_bool()).unwrap_or(false);
    if !is_hello || launch_on_hello {
        let before = std::fs::metadata(config_dir().join("ipc.json")).and_then(|m| m.modified()).ok();
        if launch_app() {
            let start = Instant::now();
            while start.elapsed() < Duration::from_secs(12) {
                std::thread::sleep(Duration::from_millis(200));
                let now = std::fs::metadata(config_dir().join("ipc.json")).and_then(|m| m.modified()).ok();
                if now.is_some() && now != before {
                    if let Some(ipc) = read_ipc() {
                        if let Ok(r) = post(&ipc, body, timeout) {
                            return r;
                        }
                    }
                }
            }
        }
    }
    serde_json::to_vec(&serde_json::json!({ "type": "error", "message": "app_unavailable" })).unwrap()
}

fn main() {
    // Tarayıcı ilk argüman olarak eklenti kaynağını verir; başka bir şeyle çalıştırılırsa bilgi göster.
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("idc-host {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    while let Some(msg) = read_message(&mut input) {
        let reply = forward(&msg);
        if write_message(&mut output, &reply).is_err() {
            break;
        }
    }
}
