use std::thread;

const PML_RESPONSE: &str = r#"<pml>
  <head>
    <meta http-equiv="Content-Type" content="text/x-playonline-pml;charset=UTF-8">
    <title>Fast</title>
  </head>
  <body>
    <timer name="fast" href="gameto:1" enable="1" delay="0">
  </body>
</pml>"#;

/// Start the proxy server in a background thread.
/// It serves one request then shuts down.
/// Returns a handle to join on.
pub fn start_proxy(port: u16) -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
    let addr = format!("0.0.0.0:{}", port);
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| format!("Failed to bind proxy on port {}: {}", port, e))?;

    log::info!("Proxy server listening on port {}", port);

    let handle = thread::spawn(move || {
        // Serve a single request then exit
        match server.recv() {
            Ok(request) => {
                log::info!("Proxy received request: {} {}", request.method(), request.url());
                let response = tiny_http::Response::from_string(PML_RESPONSE)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Content-Type"[..],
                            &b"text/x-playonline-pml;charset=UTF-8"[..],
                        )
                        .unwrap(),
                    );
                let _ = request.respond(response);
                log::info!("Proxy served PML redirect response");
            }
            Err(e) => {
                log::error!("Proxy error receiving request: {}", e);
            }
        }
    });

    Ok(handle)
}
