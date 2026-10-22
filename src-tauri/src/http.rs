use std::time::Duration;

/// 读取 Windows「Internet 选项」里的系统代理，国内直连不到的 API
/// （CurseForge、部分 Mojang 接口）需要走用户已配置的代理。
fn system_proxy() -> Option<String> {
    #[cfg(windows)]
    {
        use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let key = hkcu
            .open_subkey_with_flags(
                r"Software\Microsoft\Windows\CurrentVersion\Internet Settings",
                KEY_READ,
            )
            .ok()?;
        let enabled: u32 = key.get_value("ProxyEnable").ok()?;
        if enabled == 0 {
            return None;
        }
        let server: String = key.get_value("ProxyServer").ok()?;
        let server = server.trim();
        if server.is_empty() {
            return None;
        }
        // 可能是 "127.0.0.1:7897" 或 "http=127.0.0.1:7897;https=127.0.0.1:7897"
        let address = if let Some((_, value)) = server
            .split(';')
            .filter_map(|part| part.split_once('='))
            .find(|(scheme, _)| scheme.eq_ignore_ascii_case("http"))
        {
            value
        } else if server.contains('=') {
            server
                .split(';')
                .filter_map(|part| part.split_once('='))
                .map(|(_, value)| value)
                .next()
                .unwrap_or(server)
        } else {
            server
        };
        let address = address.trim();
        if address.is_empty() {
            return None;
        }
        if address.contains("://") {
            Some(address.to_string())
        } else {
            Some(format!("http://{address}"))
        }
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// 系统里可能设置了代理但代理软件没启动，先探测一下可用性，
/// 否则所有请求都会因为连不上代理而失败。
fn proxy_reachable(address: &str) -> bool {
    use std::net::{TcpStream, ToSocketAddrs};
    let target = address
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    if let Ok(mut addrs) = target.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            return TcpStream::connect_timeout(&addr, Duration::from_millis(500)).is_ok();
        }
    }
    false
}

pub fn client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("DOCL/", env!("CARGO_PKG_VERSION"), " (Minecraft Launcher)"))
        .connect_timeout(Duration::from_secs(15))
        .pool_max_idle_per_host(16);

    if let Some(proxy) = system_proxy().filter(|address| proxy_reachable(address)) {
        if let Ok(mut proxy) = reqwest::Proxy::all(&proxy) {
            // 国内镜像与本地服务不需要走代理
            proxy = proxy.no_proxy(reqwest::NoProxy::from_string(
                "bmclapi2.bangbang93.com,bmclapi.bangbang93.com,localhost,127.0.0.1",
            ));
            builder = builder.proxy(proxy);
        }
    }

    builder.build().expect("无法创建 HTTP 客户端")
}
