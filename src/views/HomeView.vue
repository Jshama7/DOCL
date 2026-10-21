<script setup lang="ts">
import { computed } from "vue";
import { openFolder } from "../api";
import WoodButton from "../components/WoodButton.vue";
import WoodProgress from "../components/WoodProgress.vue";
import {
  launchVersion,
  selectedAccount,
  selectedVersion,
  selectVersion,
  store,
} from "../store";
import { formatBytes, formatSpeed, kindLabel } from "../utils";

const version = selectedVersion;
const account = selectedAccount;

const isInstalling = computed(
  () => !!store.installingId && store.installingId === version.value?.id
);

const progress = computed(() => {
  if (!store.progress || store.progress.stage === "java") return null;
  if (!store.installingId) return null;
  return store.progress;
});

const percent = computed(() => {
  const p = progress.value;
  if (!p) return 0;
  if (p.bytes_total > 0) return Math.min(100, (p.bytes_done / p.bytes_total) * 100);
  if (p.files_total > 0) return Math.min(100, (p.files_done / p.files_total) * 100);
  return 0;
});

const speed = computed(() => {
  const p = progress.value;
  if (!p) return "";
  return `${formatSpeed(p.bytes_done, p.elapsed_ms)} · ${p.files_done}/${p.files_total} 个文件`;
});

const javaLabel = computed(() => {
  const v = version.value;
  if (!v) return "—";
  const java = store.javaList.find((j) => j.major === v.java_major);
  return java ? `Java ${java.major}` : `缺少 Java ${v.java_major}`;
});

function openGameDir() {
  if (store.config?.game_dir) openFolder(store.config.game_dir);
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1 class="page-title">今天想玩点什么</h1>
        <p class="page-sub">本地版本、账号与运行状态一览</p>
      </div>
      <span class="tag">{{ account ? (account.kind === "microsoft" ? "正版账号" : "离线账号") : "未登录" }}</span>
    </header>

    <section class="hero card">
      <div class="hero__main">
        <span class="tag tag--grass">{{ version ? kindLabel(version.kind) : "未安装" }}</span>
        <h2 class="hero__title">{{ version?.id ?? "还没有可用的游戏版本" }}</h2>
        <p class="hero__desc">
          {{
            version
              ? `客户端 ${formatBytes(version.jar_size)} · 需要 Java ${version.java_major}`
              : "先到下载页选一个版本，安装完成后就能从这里直接启动。"
          }}
        </p>
        <div class="hero__chips">
          <span class="chip">{{ javaLabel }}</span>
          <span class="chip">账号 {{ account?.name ?? "未登录" }}</span>
          <span class="chip">内存 {{ store.config?.memory_mb ?? 0 }} MB</span>
        </div>
      </div>
      <div class="hero__action">
        <WoodButton v-if="!version" variant="primary" size="lg" @click="store.page = 'download'">
          去下载版本
        </WoodButton>
        <template v-else>
          <WoodButton
            variant="primary"
            size="lg"
            :disabled="isInstalling"
            @click="launchVersion()"
          >
            {{ store.gameRunning ? "停止游戏" : "启动游戏" }}
          </WoodButton>
          <WoodButton variant="ghost" size="sm" @click="openGameDir">打开游戏目录</WoodButton>
        </template>
      </div>
    </section>

    <section v-if="progress" class="card card-pad progress-card">
      <div class="row-between">
        <div class="row">
          <span class="tag tag--grass">安装中</span>
          <strong>{{ store.installingId }}</strong>
        </div>
        <span class="muted small">{{ speed }}</span>
      </div>
      <WoodProgress :value="percent" />
      <p class="muted small">{{ progress.message }}</p>
    </section>

    <div class="grid-3">
      <div class="card card-pad stat">
        <span class="stat__value">{{ store.installed.length }}</span>
        <span class="stat__label">已安装版本</span>
      </div>
      <div class="card card-pad stat">
        <span class="stat__value">{{ store.javaList.length }}</span>
        <span class="stat__label">可用 Java</span>
      </div>
      <div class="card card-pad stat">
        <span class="stat__value">{{ store.accounts.length }}</span>
        <span class="stat__label">账号</span>
      </div>
    </div>

    <section class="card card-pad">
      <div class="row-between">
        <div class="section-title">快速切换版本</div>
        <WoodButton variant="ghost" size="sm" @click="store.page = 'versions'">
          管理版本
        </WoodButton>
      </div>
      <div v-if="store.installed.length" class="chips">
        <button
          v-for="item in store.installed"
          :key="item.id"
          class="chip chip--button"
          :class="{ 'chip--active': item.id === version?.id }"
          @click="selectVersion(item.id)"
        >
          {{ item.id }}
          <span class="chip__kind">{{ kindLabel(item.kind) }}</span>
        </button>
      </div>
      <p v-else class="muted small">暂无已安装版本。</p>
    </section>
  </div>
</template>

<style scoped>
.hero {
  display: flex;
  align-items: stretch;
  justify-content: space-between;
  gap: 24px;
  padding: 26px 26px 24px;
  background: linear-gradient(135deg, #fffdf8 0%, #faf3e6 62%, #f4ead6 100%);
}

.hero__main {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-width: 0;
}

.hero__title {
  font-size: 30px;
  font-weight: 800;
  letter-spacing: 0.01em;
  color: var(--wood-dark);
  line-height: 1.25;
}

.hero__desc {
  color: var(--ink-soft);
  font-size: 13px;
}

.hero__chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;
}

.chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 11px;
  border-radius: 7px;
  background: rgba(255, 253, 248, 0.75);
  border: 1px solid var(--line);
  color: var(--ink-soft);
  font-size: 12.5px;
}

.chip--button {
  transition: border-color 0.15s ease, color 0.15s ease, background 0.15s ease;
}

.chip--button:hover {
  border-color: var(--wood-light);
  color: var(--wood-dark);
}

.chip--active {
  background: var(--grass-soft);
  border-color: #cfdcc5;
  color: var(--grass-dark);
  font-weight: 600;
}

.chip__kind {
  color: var(--muted);
  font-size: 11px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 14px;
}

.hero__action {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  justify-content: center;
  gap: 10px;
  min-width: 190px;
}

.progress-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.stat__value {
  font-size: 26px;
  font-weight: 800;
  color: var(--wood-dark);
  line-height: 1.1;
}

.stat__label {
  font-size: 12.5px;
  color: var(--muted);
}
</style>
