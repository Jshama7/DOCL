<script setup lang="ts">
import { store, selectedAccount, type Page } from "../store";

const items: { key: Page; label: string; idx: string }[] = [
  { key: "home", label: "启动台", idx: "01" },
  { key: "versions", label: "版本管理", idx: "02" },
  { key: "download", label: "下载版本", idx: "03" },
  { key: "accounts", label: "账号", idx: "04" },
  { key: "settings", label: "设置", idx: "05" },
];

function go(page: Page) {
  store.page = page;
}
</script>

<template>
  <aside class="sidebar">
    <div class="brand">
      <span class="brand__mark" />
      <div class="brand__text">
        <strong>DOCL</strong>
        <span>MINECRAFT LAUNCHER</span>
      </div>
    </div>

    <div class="side-label">导航</div>

    <button
      v-for="(item, index) in items"
      :key="item.key"
      class="nav-item"
      :class="{ 'nav-item--active': store.page === item.key }"
      :style="{ '--i': index }"
      @click="go(item.key)"
    >
      <span class="nav-item__idx">{{ item.idx }}</span>
      <span>{{ item.label }}</span>
    </button>

    <div class="sidebar__spacer" />

    <button class="side-account" @click="go('accounts')">
      <span class="side-account__avatar">
        {{ selectedAccount ? selectedAccount.name.slice(0, 1).toUpperCase() : "?" }}
      </span>
      <span class="side-account__text">
        <strong>{{ selectedAccount?.name ?? "未登录" }}</strong>
        <span>
          {{
            selectedAccount
              ? selectedAccount.kind === "microsoft"
                ? "微软账号"
                : "离线账号"
              : "点击添加账号"
          }}
        </span>
      </span>
    </button>

    <div class="sidebar__foot">
      <button
        class="sidebar__log"
        :class="{ 'sidebar__log--active': store.logOpen }"
        @click="store.logOpen = !store.logOpen"
      >
        <span class="statusbar__dot" :class="{ 'statusbar__dot--run': store.gameRunning }" />
        运行日志
      </button>
      <div class="sidebar__version">DOCL · v0.1.0</div>
    </div>
  </aside>
</template>

<style scoped>
.nav-item {
  animation-delay: calc(var(--i, 0) * 40ms);
}
</style>
