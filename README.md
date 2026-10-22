# DOCL

Minecraft 第三方启动器，面向 Windows，采用 Tauri 2 + Vue 3 + Rust 构建。

界面为浅色木纹玻璃风，中文优先，强调「装得上、跑得起来、出问题能看懂」：下载、启动、联机、诊断都在一个窗口里完成。

> 本项目与 Mojang Studios / Microsoft 无关，不包含任何游戏本体文件。使用前需拥有正版 Minecraft 账号。

## 功能

- **版本管理**：原版与 Forge / NeoForge / Fabric / Quilt 安装，逐版本覆盖内存、Java、窗口、JVM 参数、启动前命令；支持目录隔离与多游戏目录切换
- **下载中心**：游戏版本、加载器、整合包、资源包、光影、数据包，支持 BMCLAPI 与官方源切换、并发下载
- **社区资源**：Modrinth / CurseForge 搜索与筛选，按当前实例的游戏版本与加载器过滤；模组更新检查
- **多人游戏**：联机大厅、局域网房间发现、服务器状态 ping、一键开服与服务器管理
- **皮肤**：账号皮肤与披风切换、本地皮肤库、内置皮肤编辑器（含 64x64 与旧版 64x32）
- **实例内容**：模组 / 资源包 / 光影 / 存档 / 截图 / 投影文件管理，模组启用禁用、拖入导入
- **诊断**：崩溃报告解析与处置建议、启动日志抽屉、Java 运行时检测
- **整合包**：导入 CurseForge / Modrinth / 通用 ZIP，导出为 ZIP 与 `.mrpack`
- **存档种子预览**：解析 `level.dat` 读取真实种子，一键跳转在线种子地图
- **设置中心**：改完即生效并自动保存，更新策略可选，配置可导入导出备份
- **自动更新**：可配置更新清单，下载完整安装包后启动安装程序

## 下载

前往 [Releases](https://github.com/dawo4/DOCL/releases) 下载 `DOCL_x.y.z_x64-setup.exe`。

安装包暂未做代码签名，Windows SmartScreen 可能提示「已保护你的电脑」，点击「更多信息」后选择「仍要运行」即可。

系统要求：Windows 10 1809 及以上，需 WebView2 运行时（Windows 11 已内置）。

## 自动更新

客户端默认不启用自动检查。在「设置 → 更新」中填入更新清单地址后生效，例如：

```text
https://raw.githubusercontent.com/dawo4/DOCL/main/update.json
https://cdn.jsdelivr.net/gh/dawo4/DOCL@main/update.json
```

清单为 JSON，格式如下：

```json
{
  "version": "1.0.0",
  "url": "https://example.com/DOCL_1.0.0_x64-setup.exe",
  "notes": "本次更新内容"
}
```

版本号逐段数值比较，高于当前版本才会提示。更新策略支持「自动下载」「仅提示」「关闭」三档。

## 从源码构建

环境要求：

- Windows 10/11
- Node.js 20+、pnpm
- Rust stable（MSVC 工具链）
- WebView2 运行时

```bash
pnpm install
pnpm tauri dev      # 开发
pnpm tauri build    # 打包，产物在 src-tauri/target/release/bundle/
```

发版时需同时修改 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 三处版本号，并同步更新 `update.json`。

## 目录结构

```text
src/                   Vue 前端（视图、组件、状态、api 封装）
src-tauri/src/         Rust 后端（下载、安装、启动、联机、诊断等模块）
src-tauri/tauri.conf.json   窗口、打包、NSIS 配置
update.json            更新清单
```

## 许可证

[AGPL-3.0](LICENSE)。

修改后分发（包括以网络服务形式提供）需以同等许可公开源代码。
