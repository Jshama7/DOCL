<script setup lang="ts">
import { store, type Page } from "../store";

const items: { key: Page; label: string }[] = [
  { key: "home", label: "首页" },
  { key: "versions", label: "版本管理" },
  { key: "download", label: "下载版本" },
  { key: "accounts", label: "账号" },
  { key: "settings", label: "设置" },
];

function go(page: Page) {
  store.page = page;
}
</script>

<template>
  <aside class="sidebar">
    <button
      v-for="item in items"
      :key="item.key"
      class="nav-item"
      :class="{ 'nav-item--active': store.page === item.key }"
      @click="go(item.key)"
    >
      {{ item.label }}
    </button>

    <div class="sidebar__spacer" />

    <div class="sidebar__foot">
      <button
        class="sidebar__log"
        :class="{ 'sidebar__log--active': store.logOpen }"
        @click="store.logOpen = !store.logOpen"
      >
        <span class="statusbar__dot" :class="{ 'statusbar__dot--run': store.gameRunning }" />
        运行日志
      </button>
      <div class="sidebar__version">v0.1.0</div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar__log--active {
  border-color: var(--grass);
  color: var(--grass-dark);
  background: var(--grass-soft);
}
</style>
