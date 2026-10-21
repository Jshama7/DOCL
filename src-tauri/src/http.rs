use std::time::Duration;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("DOCL/0.1.0 (Minecraft Launcher)")
        .connect_timeout(Duration::from_secs(15))
        .pool_max_idle_per_host(16)
        .build()
        .expect("无法创建 HTTP 客户端")
}
