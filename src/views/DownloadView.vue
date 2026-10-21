<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodProgress from "../components/WoodProgress.vue";
import { cancelInstall, installVersion, refreshVersions, store } from "../store";
import { formatBytes, formatDate, formatSpeed, kindLabel } from "../utils";

const filter = ref<"release" | "snapshot" | "all">("release");
const query = ref("");
const limit = ref(240);

const tabs = [
  { key: "release", label: "正式版" },
  { key: "snapshot", label: "快照" },
  { key: "all", label: "全部" },
] as const;

onMounted(() => {
  if (!store.versions.length) refreshVersions();
});

const filtered = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return store.versions
    .filter((v) => (filter.value === "all" ? true : v.kind === filter.value))
    .filter((v) => (keyword ? v.id.toLowerCase().includes(keyword) : true))
    .slice(0, limit.value);
});

const totalMatched = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return store.versions
    .filter((v) => (filter.value === "all" ? true : v.kind === filter.value))
    .filter((v) => (keyword ? v.id.toLowerCase().includes(keyword) : true)).length;
});

const progress = computed(() => {
  if (!store.installingId || !store.progress) return null;
  if (store.progress.stage === "java") return null;
  return store.progress;
});

const percent = computed(() => {
  const p = progress.value;
  if (!p) return 0;
  if (p.bytes_total > 0) return Math.min(100, (p.bytes_done / p.bytes_total) * 100);
  if (p.files_total > 0) return Math.min(100, (p.files_done / p.files_total) * 100);
  return 0;
});
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <h1 class="page-title">下载版本</h1>
        <p class="page-sub">原版客户端，支持 BMCLAPI 镜像加速与 SHA1 校验</p>
      </div>
      <WoodButton variant="ghost" size="sm" @click="refreshVersions(true)">刷新列表</WoodButton>
    </header>

    <section v-if="progress" class="card card-pad download-progress">
      <div class="row-between">
        <div class="row">
          <span class="tag tag--grass">安装中</span>
          <strong>{{ store.installingId }}</strong>
        </div>
        <div class="row">
          <span class="muted small">
            {{ formatSpeed(progress.bytes_done, progress.elapsed_ms) }} ·
            {{ formatBytes(progress.bytes_done) }} / {{ formatBytes(progress.bytes_total) }}
          </span>
          <WoodButton variant="danger" size="sm" @click="cancelInstall">取消</WoodButton>
        </div>
      </div>
      <WoodProgress :value="percent" />
      <p class="muted small">{{ progress.message }}</p>
    </section>

    <section class="card">
      <div class="toolbar">
        <div class="tabs">
          <button
            v-for="tab in tabs"
            :key="tab.key"
            class="tab"
            :class="{ 'tab--active': filter === tab.key }"
            @click="filter = tab.key; limit = 240"
          >
            {{ tab.label }}
          </button>
        </div>
        <div class="toolbar__search">
          <WoodInput v-model="query" placeholder="搜索版本号，例如 1.20 / 26.3" />
        </div>
      </div>

      <div v-if="store.loadingVersions" class="list-empty">正在获取版本清单…</div>
      <div v-else-if="!filtered.length" class="list-empty">没有匹配的版本</div>
      <div v-else class="version-list">
        <div v-for="item in filtered" :key="item.id" class="version-row">
          <div class="version-row__main">
            <strong>{{ item.id }}</strong>
            <span class="tag" :class="{ 'tag--grass': item.kind === 'release' }">
              {{ kindLabel(item.kind) }}
            </span>
            <span v-if="item.installed" class="tag tag--grass">已安装</span>
            <span class="muted small">{{ formatDate(item.release_time) }}</span>
          </div>
          <div class="version-row__action">
            <WoodButton
              v-if="!item.installed"
              variant="soft"
              size="sm"
              :disabled="!!store.installingId"
              @click="installVersion(item.id)"
            >
              {{ store.installingId === item.id ? "安装中…" : "安装" }}
            </WoodButton>
            <WoodButton
              v-else
              variant="ghost"
              size="sm"
              @click="store.page = 'versions'"
            >
              去管理
            </WoodButton>
          </div>
        </div>
      </div>

      <div v-if="filtered.length < totalMatched" class="load-more">
        <WoodButton variant="ghost" size="sm" @click="limit += 200">
          加载更多（还有 {{ totalMatched - filtered.length }} 个）
        </WoodButton>
      </div>
    </section>

    <p class="muted small">
      下载源会按设置自动选择，默认优先使用 BMCLAPI 镜像；失败时自动回退官方源。资源文件与依赖库均会校验 SHA1。
    </p>
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px;
  border-bottom: 1px solid var(--line);
}

.tabs {
  display: flex;
  gap: 4px;
  padding: 3px;
  background: var(--panel-2);
  border: 1px solid var(--line);
  border-radius: 9px;
}

.tab {
  height: 28px;
  padding: 0 14px;
  border-radius: 6px;
  color: var(--ink-soft);
  font-size: 13px;
  font-weight: 600;
  transition: all 0.15s ease;
}

.tab:hover {
  color: var(--wood-dark);
}

.tab--active {
  background: var(--panel);
  color: var(--wood-dark);
  box-shadow: var(--shadow-soft);
}

.toolbar__search {
  width: 280px;
}

.version-list {
  max-height: 520px;
  overflow-y: auto;
  padding: 6px 8px;
}

.version-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 12px;
  border-radius: 8px;
  transition: background 0.13s ease;
}

.version-row:hover {
  background: #f8f1e3;
}

.version-row__main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.version-row__main strong {
  color: var(--ink);
  font-size: 14px;
  min-width: 120px;
}

.version-row__action {
  flex-shrink: 0;
}

.list-empty {
  padding: 42px;
  text-align: center;
  color: var(--muted);
}

.load-more {
  display: flex;
  justify-content: center;
  padding: 12px;
  border-top: 1px dashed var(--line);
}

.download-progress {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
</style>
