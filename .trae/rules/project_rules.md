# DOCL 项目规则

## 发版流程（每次上架按此顺序执行）

1. **同步版本号 4 处**：`package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`update.json`。
   `Cargo.lock` 中 `mc-launcher` 的版本随 `cargo check` 自动更新。
2. **打包**：用 `scripts/build-release.ps1`（带 `--remap-path-prefix`，会自检 exe 中不含本机用户名路径），不要直接裸跑 `pnpm tauri build`。
   安装包产物在 `src-tauri/target/release/bundle/nsis/`。
3. **便携包**：`scripts/build-portable.ps1`，产出 `dist-portable/DOCL_<version>_portable.zip`。
   若已用 build-release 编好，可复用同一 exe 只做 zip 部分，避免二次编译。
4. **推送与发布**：`git push` 后打 tag `v<version>`，再 `gh release create` 上传
   `DOCL_<version>_x64-setup.exe` 与 `DOCL_<version>_portable.zip`。
5. **清 CDN 缓存（必做，容易漏）**：

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/purge-cdn.ps1 -Version <version>
   ```

   `update.json` 的国内默认清单走 `https://cdn.jsdelivr.net/gh/dawo4/DOCL@main/update.json`，
   jsDelivr 对 `@main` 有约 12 小时缓存。不清缓存的话，刚发版时客户端「检查更新」仍显示旧版本号。
   脚本会 purge 并回读 CDN 上的 `version`；若仍是旧值，等 1 分钟再跑一次（脚本以退出码 1 提示）。
6. **验证**：确认以下三处版本一致后即可宣布发布完成
   - GitHub raw：`https://raw.githubusercontent.com/dawo4/DOCL/main/update.json`
   - jsDelivr：`https://cdn.jsdelivr.net/gh/dawo4/DOCL@main/update.json`
   - Release 资产文件名与 `update.json` 里的 `url` / `portable_url` 完全对应

## 备注

- jsDelivr 只代理仓库内文本文件，不代理 Release 附件；安装包下载仍走 GitHub 直链。
- 构建/发布类命令用内置终端执行，不要用 MCP 的 PowerShell 工具（会卡在授权确认上）。
- **git 需要走系统代理**：本机直连 `github.com:443` 会超时/重置，Windows 系统代理为
  `127.0.0.1:7897`。推送时用临时参数，不要修改全局/仓库 git 配置：

  ```powershell
  git -c http.proxy=http://127.0.0.1:7897 -c https.proxy=http://127.0.0.1:7897 push origin main
  ```

  端口以系统代理实际设置为准（`HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings` 的 `ProxyServer`）。
- 本机 git 未配置全局身份。提交时用 `-c` 临时指定历史作者，避免写入配置：

  ```powershell
  git -c user.name="dawo4" -c user.email="dawo4@users.noreply.github.com" commit -m "..."
  ```
