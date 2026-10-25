﻿﻿﻿﻿﻿﻿﻿# 发版收尾：清掉 jsdelivr 对 update.json 的 CDN 缓存。
# jsdelivr 对 @main 分支引用缓存很久，发版后不清缓存国内更新源会一直读到旧版本。
# 用法: powershell -ExecutionPolicy Bypass -File scripts/purge-cdn.ps1
# 可选: -Version 1.5.0 指定期望版本做校验（默认只回读显示）

param(
  [string]$Version = ""
)

$ErrorActionPreference = "Stop"

# 命令行联网走系统代理；代理不可达时直连再试一次
$targets = @(@{ Proxy = "http://127.0.0.1:7897" }, @{ Proxy = $null })

$purged = $false
foreach ($t in $targets) {
  try {
    if ($t.Proxy) { $env:HTTPS_PROXY = $t.Proxy } else { Remove-Item Env:HTTPS_PROXY -ErrorAction SilentlyContinue }
    Write-Host "purging via jsdelivr..." -ForegroundColor Cyan
    [void](Invoke-RestMethod "https://purge.jsdelivr.net/gh/dawo4/DOCL@main/update.json" -Proxy $t.Proxy -TimeoutSec 20)
    Start-Sleep -Seconds 3
    $live = (Invoke-RestMethod "https://cdn.jsdelivr.net/gh/dawo4/DOCL@main/update.json" -Proxy $t.Proxy -TimeoutSec 20).version
    Write-Host "国内源当前版本: $live" -ForegroundColor Green
    if ($Version -and $live -ne $Version) {
      throw "国内源版本($live)与期望($Version)不一致，稍后重试或检查 update.json 是否已推送"
    }
    $purged = $true
    break
  } catch {
    Write-Warning "尝试失败: $_"
  }
}

if (-not $purged) { throw "purge 失败：代理与直连均不可用" }
