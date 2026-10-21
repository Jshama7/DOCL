<script setup lang="ts">
import { ref } from "vue";
import { openFolder } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import {
  launchVersion,
  selectVersion,
  selectedVersion,
  store,
  uninstallVersion,
} from "../store";
import { formatBytes, formatDate, kindLabel } from "../utils";

const confirmId = ref<string | null>(null);
let confirmTimer: number | undefined;

function requestDelete(id: string) {
  if (confirmId.value === id) {
    confirmId.value = null;
    uninstallVersion(id);
    return;
  }
  confirmId.value = id;
  if (confirmTimer) window.clearTimeout(confirmTimer);
  confirmTimer = window.setTimeout(() => {
    if (confirmId.value === id) confirmId.value = null;
  }, 3200);
}

function openDir() {
  if (store.config?.game_dir) openFolder(store.config.game_dir);
}
</script>

<template>
  <div class="page">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / LIBRARY</span>
        <h1 class="page-title">版本管理</h1>
        <p class="page-sub">已安装的原版客户端，支持启动、切换与删除</p>
      </div>
      <div class="row">
        <IconButton icon="folder" label="打开游戏目录" :size="32" @click="openDir" />
        <WoodButton variant="primary" size="sm" sheen @click="store.page = 'download'">
          <AppIcon name="plus" :size="14" />
          安装新版本
        </WoodButton>
      </div>
    </header>

    <section v-if="!store.installed.length" class="empty card reveal">
      <span class="empty__no display">00</span>
      <div class="empty__text">
        <p class="empty__title">书架还是空的</p>
        <p class="muted small">前往下载页，选择一个正式版开始安装。</p>
      </div>
      <WoodButton variant="primary" sheen @click="store.page = 'download'">
        <AppIcon name="download" :size="15" />
        去下载
      </WoodButton>
    </section>

    <div v-else class="version-grid">
      <article
        v-for="(item, index) in store.installed"
        :key="item.id"
        class="version card card--hover reveal"
        :class="{ 'version--active': item.id === selectedVersion?.id }"
        :style="{ '--i': index }"
      >
        <span class="version__no display">{{ String(index + 1).padStart(2, "0") }}</span>

        <div class="version__head">
          <strong class="version__id display">{{ item.id }}</strong>
          <div class="row">
            <span v-if="item.id === selectedVersion?.id" class="tag tag--grass">当前</span>
            <span class="tag">{{ kindLabel(item.kind) }}</span>
          </div>
        </div>

        <div class="version__meta">
          <span><i>客户端</i>{{ formatBytes(item.jar_size) }}</span>
          <span><i>Java</i>{{ item.java_major }}</span>
          <span><i>安装于</i>{{ formatDate(new Date(item.last_modified * 1000).toISOString()) }}</span>
        </div>

        <div class="version__actions">
          <IconButton
            icon="play"
            :label="`启动 ${item.id}`"
            variant="primary"
            :size="34"
            @click="selectVersion(item.id); launchVersion(item.id)"
          />
          <IconButton
            icon="check"
            label="设为当前版本"
            :size="34"
            :disabled="item.id === selectedVersion?.id"
            @click="selectVersion(item.id)"
          />
          <IconButton
            icon="trash"
            :label="confirmId === item.id ? '确认删除' : '删除版本'"
            variant="danger"
            :size="34"
            @click="requestDelete(item.id)"
          />
        </div>
      </article>
    </div>
  </div>
</template>

<style scoped>
.empty {
  display: flex;
  align-items: center;
  gap: 22px;
  padding: 34px 30px;
}

.empty__no {
  font-size: 64px;
  line-height: 1;
  color: var(--line-strong);
}

.empty__text {
  flex: 1;
}

.empty__title {
  font-size: 17px;
  font-weight: 800;
  color: var(--walnut-deep);
}

.version-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.version {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 20px 22px;
  overflow: hidden;
}

.version::after {
  content: "";
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background: var(--line);
  transition: background var(--dur-2) ease;
}

.version--active::after {
  background: var(--moss);
}

.version__no {
  position: absolute;
  right: 16px;
  top: 6px;
  font-size: 64px;
  line-height: 1;
  color: rgba(var(--accent-rgb), 0.08);
  pointer-events: none;
}

.version__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  position: relative;
  z-index: 1;
}

.version__id {
  font-size: 30px;
  line-height: 1.1;
  color: var(--walnut-deep);
}

.version__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 18px;
  color: var(--ink-soft);
  font-size: 12.5px;
  padding-top: 10px;
  border-top: 1px dashed var(--line);
}

.version__meta i {
  font-style: normal;
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.12em;
  color: var(--muted);
  margin-right: 6px;
  text-transform: uppercase;
}

.version__actions {
  display: flex;
  gap: 6px;
  margin-top: 2px;
}
</style>
