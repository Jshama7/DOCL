<script setup lang="ts">
import { computed } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { store } from "../store";

const labels: Record<string, string> = {
  home: "首页",
  versions: "版本管理",
  download: "下载版本",
  accounts: "账号",
  settings: "设置",
};

const pageLabel = computed(() => labels[store.page] ?? "");
const win = getCurrentWindow();

function minimize() {
  win.minimize();
}
function toggleMaximize() {
  win.toggleMaximize();
}
function close() {
  win.close();
}
</script>

<template>
  <header class="titlebar">
    <div class="titlebar__brand">
      <span class="titlebar__mark" />
      <span>木野启动器</span>
    </div>
    <span class="titlebar__page">{{ pageLabel }}</span>
    <div class="titlebar__drag" data-tauri-drag-region @dblclick="toggleMaximize" />
    <div class="titlebar__actions">
      <button class="titlebar__btn" title="最小化" @click="minimize">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M2 6h8" stroke="currentColor" stroke-width="1.3" />
        </svg>
      </button>
      <button class="titlebar__btn" title="最大化" @click="toggleMaximize">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <rect x="2.4" y="2.4" width="7.2" height="7.2" fill="none" stroke="currentColor" stroke-width="1.2" />
        </svg>
      </button>
      <button class="titlebar__btn titlebar__btn--close" title="关闭" @click="close">
        <svg width="12" height="12" viewBox="0 0 12 12">
          <path d="M2.5 2.5l7 7M9.5 2.5l-7 7" stroke="currentColor" stroke-width="1.2" />
        </svg>
      </button>
    </div>
  </header>
</template>
