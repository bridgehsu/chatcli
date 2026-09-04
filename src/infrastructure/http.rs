use std::time::Duration;

/// Builds a reusable HTTP client with the application's network timeouts.
///
/// An explicitly configured proxy takes precedence for this client; otherwise
/// reqwest can still use standard HTTP(S)_PROXY environment variables.
pub fn build_http_client(proxy_url: Option<&str>) -> Result<reqwest::Client, reqwest::Error> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(60))
        .tcp_nodelay(true);

    if let Some(proxy_url) = proxy_url.filter(|url| !url.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(proxy_url)?);
    }

    builder.build()
}
