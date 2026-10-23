/// BMCLAPI 镜像地址重写。
/// 文档：https://bmclapidoc.bangbang93.com/
const MIRROR: &str = "https://bmclapi2.bangbang93.com";

const HOST_MAPPINGS: &[(&str, &str)] = &[
    (
        "https://launchermeta.mojang.com/",
        "https://bmclapi2.bangbang93.com/",
    ),
    (
        "https://launcher.mojang.com/",
        "https://bmclapi2.bangbang93.com/",
    ),
    (
        "https://piston-meta.mojang.com/",
        "https://bmclapi2.bangbang93.com/",
    ),
    (
        "https://piston-data.mojang.com/",
        "https://bmclapi2.bangbang93.com/",
    ),
    (
        "https://libraries.minecraft.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://maven.minecraftforge.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://resources.download.minecraft.net/",
        "https://bmclapi2.bangbang93.com/assets/",
    ),
    (
        "https://meta.fabricmc.net/",
        "https://bmclapi2.bangbang93.com/fabric-meta/",
    ),
    (
        "https://maven.fabricmc.net/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
    (
        "https://meta.quiltmc.org/",
        "https://bmclapi2.bangbang93.com/quilt-meta/",
    ),
    (
        "https://maven.quiltmc.org/repository/release/",
        "https://bmclapi2.bangbang93.com/maven/",
    ),
];

pub fn rewrite(url: &str) -> String {
    for (from, to) in HOST_MAPPINGS {
        if let Some(rest) = url.strip_prefix(from) {
            return format!("{to}{rest}");
        }
    }
    // 未收录的主机不做改写，避免把外部 API 拼成镜像子路径
    url.to_string()
}

pub fn official(url: &str) -> String {
    if let Some(rest) = url.strip_prefix(MIRROR) {
        // 还原为官方地址；多数资源可直接用官方主机
        if rest.starts_with("/assets/") {
            return format!(
                "https://resources.download.minecraft.net/{}",
                rest.trim_start_matches("/assets/")
            );
        }
        if rest.starts_with("/maven/") {
            return format!(
                "https://libraries.minecraft.net/{}",
                rest.trim_start_matches("/maven/")
            );
        }
        return format!("https://launchermeta.mojang.com{rest}");
    }
    url.to_string()
}
