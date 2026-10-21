<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
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
        <span class="eyebrow">DOCL / CATALOG</span>
        <h1 class="page-title">下载版本</h1>
        <p class="page-sub">原版客户端，BMCLAPI 镜像加速与 SHA1 校验</p>
      </div>
      <IconButton
        icon="refresh"
        label="刷新版本列表"
        variant="soft"
        :size="32"
        @click="refreshVersions(true)"
      />
    </header>

    <section v-if="progress" class="card card-pad download-progress reveal">
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

    <section class="card catalog reveal" style="--i: 1">
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

      <div v-if="store.loadingVersions" class="skeleton-list">
        <div v-for="n in 9" :key="n" class="skeleton-row">
          <span class="skeleton skeleton-row__title" />
          <span class="skeleton skeleton-row__meta" />
          <span class="skeleton skeleton-row__btn" />
        </div>
      </div>
      <div v-else-if="!filtered.length" class="list-empty">没有匹配的版本</div>
      <div v-else class="version-list">
        <div
          v-for="(item, index) in filtered"
          :key="item.id"
          class="version-row"
          :style="{ '--i': index }"
        >
          <span class="version-row__mark" />
          <div class="version-row__main">
            <strong class="version-row__id">{{ item.id }}</strong>
            <span class="tag" :class="{ 'tag--grass': item.kind === 'release' }">
              {{ kindLabel(item.kind) }}
            </span>
            <span v-if="item.installed" class="tag tag--grass">已安装</span>
            <span class="version-row__date mono">{{ formatDate(item.release_time) }}</span>
          </div>
          <div class="version-row__action">
            <IconButton
              v-if="!item.installed"
              icon="download"
              :label="store.installingId === item.id ? '正在安装' : `安装 ${item.id}`"
              variant="soft"
              :size="32"
              :disabled="!!store.installingId"
              @click="installVersion(item.id)"
            />
            <IconButton
              v-else
              icon="check"
              label="已安装，去版本管理"
              :size="32"
              @click="store.page = 'versions'"
            />
          </div>
        </div>
      </div>

      <div v-if="filtered.length < totalMatched" class="load-more">
        <WoodButton variant="ghost" size="sm" @click="limit += 200">
          <AppIcon name="chevron-down" :size="14" />
          加载更多（还有 {{ totalMatched - filtered.length }} 个）
        </WoodButton>
      </div>
    </section>

    <p class="muted small footnote">
      默认优先使用 BMCLAPI 镜像，连续失败后自动切换到官方源；资源文件与依赖库都会校验 SHA1。
    </p>
  </div>
</template>

<style scoped>
.catalog {
  overflow: hidden;
}

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 18px;
  border-bottom: 1px solid var(--line);
  background: linear-gradient(180deg, #fffdf6, #fbf4e7);
}

.tabs {
  display: flex;
  gap: 3px;
  padding: 3px;
  background: var(--paper-2);
  border: 1px solid var(--line);
  border-radius: 10px;
}

.tab {
  height: 29px;
  padding: 0 16px;
  border-radius: 7px;
  color: var(--ink-soft);
  font-size: 12.5px;
  font-weight: 700;
  transition: background var(--dur-2) ease, color var(--dur-2) ease,
    box-shadow var(--dur-2) ease;
}

.tab:hover {
  color: var(--walnut-deep);
}

.tab--active {
  background: var(--panel);
  color: var(--walnut-deep);
  box-shadow: var(--shadow-1);
}

.toolbar__search {
  width: 300px;
}

.version-list {
  max-height: 540px;
  overflow-y: auto;
  padding: 7px 9px;
}

.version-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 11px 14px 11px 18px;
  border-radius: 9px;
  transition: background var(--dur-2) ease, transform var(--dur-2) var(--ease-out-expo);
  animation: rise-in 460ms var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 18ms);
}

.version-row:hover {
  background: #f8efdf;
}

.version-row__mark {
  position: absolute;
  left: 6px;
  top: 50%;
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--line-strong);
  transform: translateY(-50%);
  transition: background var(--dur-2) ease, transform var(--dur-2) var(--ease-out-expo);
}

.version-row:hover .version-row__mark {
  background: var(--moss);
  transform: translateY(-50%) scale(1.5);
}

.version-row__main {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.version-row__id {
  font-family: var(--font-display);
  font-size: 16px;
  color: var(--ink);
  min-width: 128px;
  letter-spacing: 0.01em;
}

.version-row__date {
  font-size: 10.5px;
  color: var(--muted);
  letter-spacing: 0.06em;
}

.version-row__action {
  flex-shrink: 0;
}

.skeleton-list {
  padding: 8px 12px;
}

.skeleton-row {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 12px 8px;
}

.skeleton-row__title {
  width: 140px;
  height: 16px;
}

.skeleton-row__meta {
  width: 220px;
  height: 12px;
}

.skeleton-row__btn {
  margin-left: auto;
  width: 70px;
  height: 30px;
}

.list-empty {
  padding: 46px;
  text-align: center;
  color: var(--muted);
}

.load-more {
  display: flex;
  justify-content: center;
  padding: 13px;
  border-top: 1px dashed var(--line);
}

.download-progress {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.footnote {
  padding-left: 2px;
}
</style>
