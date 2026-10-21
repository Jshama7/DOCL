<script setup lang="ts">
import { ref } from "vue";
import { openFolder } from "../api";
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
        <h1 class="page-title">版本管理</h1>
        <p class="page-sub">已安装的原版客户端，支持启动、切换与删除</p>
      </div>
      <div class="row">
        <WoodButton variant="ghost" size="sm" @click="openDir">游戏目录</WoodButton>
        <WoodButton variant="primary" size="sm" @click="store.page = 'download'">
          安装新版本
        </WoodButton>
      </div>
    </header>

    <div v-if="!store.installed.length" class="card empty">
      <p class="empty__title">还没有安装任何版本</p>
      <p class="muted small">前往下载页，选择一个正式版开始安装。</p>
      <WoodButton variant="primary" @click="store.page = 'download'">去下载</WoodButton>
    </div>

    <div v-else class="grid-2">
      <article
        v-for="item in store.installed"
        :key="item.id"
        class="card card-pad version"
        :class="{ 'version--active': item.id === selectedVersion?.id }"
      >
        <div class="version__head">
          <div class="version__title">
            <strong>{{ item.id }}</strong>
            <span v-if="item.id === selectedVersion?.id" class="tag tag--grass">当前</span>
          </div>
          <span class="tag">{{ kindLabel(item.kind) }}</span>
        </div>

        <div class="version__meta">
          <span>客户端 {{ formatBytes(item.jar_size) }}</span>
          <span>Java {{ item.java_major }}</span>
          <span>{{ formatDate(new Date(item.last_modified * 1000).toISOString()) }}</span>
        </div>

        <div class="version__actions">
          <WoodButton
            variant="primary"
            size="sm"
            @click="selectVersion(item.id); launchVersion(item.id)"
          >
            启动
          </WoodButton>
          <WoodButton
            variant="ghost"
            size="sm"
            :disabled="item.id === selectedVersion?.id"
            @click="selectVersion(item.id)"
          >
            设为当前
          </WoodButton>
          <WoodButton
            variant="danger"
            size="sm"
            @click="requestDelete(item.id)"
          >
            {{ confirmId === item.id ? "确认删除" : "删除" }}
          </WoodButton>
        </div>
      </article>
    </div>
  </div>
</template>

<style scoped>
.empty {
  padding: 46px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
}

.empty__title {
  font-size: 16px;
  font-weight: 700;
  color: var(--wood-dark);
}

.version {
  display: flex;
  flex-direction: column;
  gap: 12px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.version:hover {
  border-color: var(--wood-light);
  box-shadow: var(--shadow);
}

.version--active {
  border-color: #c9d9bf;
  background: linear-gradient(180deg, #fffdf8, #fbf8ee);
}

.version__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.version__title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 17px;
  color: var(--wood-dark);
}

.version__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 14px;
  color: var(--muted);
  font-size: 12.5px;
}

.version__actions {
  display: flex;
  gap: 8px;
  padding-top: 4px;
  border-top: 1px dashed var(--line);
}
</style>
