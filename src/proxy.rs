use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const PML_BODY: &str = r#"<pml><head><meta http-equiv="Content-Type" content="text/x-playonline-pml;charset=UTF-8"><title>Fast</title></head><body><timer name="fast" href="gameto:1" enable="1" delay="0"></body></pml>"#;

fn handle_connection(mut stream: TcpStream) {
    // Read the request (we don't care about the contents)
    let mut buf = [0u8; 1024];
    let _ = stream.read(&mut buf);

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/x-playonline-pml;charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        PML_BODY.len(),
        PML_BODY,
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
    log::info!("Proxy served PML redirect response");
}

/// Start the proxy server in a background thread.
/// It serves one request then shuts down.
/// Uses SO_REUSEADDR so the port can be rebound immediately after closing.
pub fn start_proxy(port: u16) -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
    let listener = bind_with_reuse(port)?;

    log::info!("Proxy server listening on port {}", port);

    let handle = thread::spawn(move || {
        match listener.accept() {
            Ok((stream, addr)) => {
                log::info!("Proxy accepted connection from {}", addr);
                handle_connection(stream);
            }
            Err(e) => {
                log::error!("Proxy error accepting connection: {}", e);
            }
        }
        // listener drops here, socket closes
    });

    Ok(handle)
}

#[cfg(windows)]
fn bind_with_reuse(port: u16) -> Result<TcpListener, Box<dyn std::error::Error>> {
    use std::os::windows::io::FromRawSocket;
    use windows_sys::Win32::Networking::WinSock::{
        socket, bind as wsa_bind, listen, setsockopt,
        AF_INET, SOCK_STREAM, IPPROTO_TCP, SOL_SOCKET, SO_REUSEADDR,
        SOCKADDR_IN, SOCKET_ERROR, INVALID_SOCKET, WSAStartup, WSADATA,
    };

    unsafe {
        // Ensure Winsock is initialized
        let mut wsa_data: WSADATA = std::mem::zeroed();
        WSAStartup(0x0202, &mut wsa_data);

        let sock = socket(AF_INET as i32, SOCK_STREAM, IPPROTO_TCP);
        if sock == INVALID_SOCKET {
            return Err(format!("Failed to create socket for port {}", port).into());
        }

        // Set SO_REUSEADDR so we can rebind immediately after the previous proxy closes
        let optval: i32 = 1;
        if setsockopt(
            sock as usize,
            SOL_SOCKET,
            SO_REUSEADDR,
            &optval as *const i32 as *const u8,
            std::mem::size_of::<i32>() as i32,
        ) == SOCKET_ERROR
        {
            return Err("Failed to set SO_REUSEADDR".into());
        }

        let mut addr: SOCKADDR_IN = std::mem::zeroed();
        addr.sin_family = AF_INET;
        addr.sin_port = port.to_be();
        // 0.0.0.0
        addr.sin_addr.S_un.S_addr = 0;

        if wsa_bind(
            sock as usize,
            &addr as *const SOCKADDR_IN as *const _,
            std::mem::size_of::<SOCKADDR_IN>() as i32,
        ) == SOCKET_ERROR
        {
            return Err(format!("Failed to bind proxy on port {}", port).into());
        }

        if listen(sock as usize, 1) == SOCKET_ERROR {
            return Err(format!("Failed to listen on port {}", port).into());
        }

        Ok(TcpListener::from_raw_socket(sock as u64))
    }
}

#[cfg(not(windows))]
fn bind_with_reuse(port: u16) -> Result<TcpListener, Box<dyn std::error::Error>> {
    use std::net::SocketAddr;
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse()?;
    let listener = TcpListener::bind(addr)?;
    Ok(listener)
}
