<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import PlayerAvatar from "./PlayerAvatar.vue";
import { selectedAccount, store, type Page } from "../store";

const items: { key: Page; label: string; icon: string }[] = [
  { key: "home", label: "启动台", icon: "home" },
  { key: "versions", label: "版本", icon: "layers" },
  { key: "download", label: "下载", icon: "download" },
  { key: "multiplayer", label: "联机", icon: "globe" },
  { key: "accounts", label: "账号", icon: "user" },
  { key: "skins", label: "皮肤库", icon: "image" },
  { key: "settings", label: "设置", icon: "settings" },
];

const win = getCurrentWindow();

function go(page: Page) {
  store.page = page;
}

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
  <header class="topbar">
    <div class="topbar__brand">
      <span class="brand__mark" />
      <div class="brand__text">
        <strong>DOCL</strong>
        <span>MINECRAFT LAUNCHER</span>
      </div>
    </div>

    <nav class="topnav">
      <button
        v-for="item in items"
        :key="item.key"
        class="topnav__item"
        :class="{ 'topnav__item--active': store.page === item.key }"
        @click="go(item.key)"
      >
        <AppIcon :name="item.icon" :size="16" />
        <span>{{ item.label }}</span>
      </button>
    </nav>

    <div class="topbar__drag" data-tauri-drag-region @dblclick="toggleMaximize" />

    <div class="topbar__right">
      <button class="account-chip" @click="go('accounts')">
        <PlayerAvatar :account="selectedAccount" :size="28" :radius="8" />
        <span class="account-chip__name">{{ selectedAccount?.name ?? "未登录" }}</span>
      </button>

      <IconButton
        icon="log"
        label="运行日志"
        variant="soft"
        :active="store.logOpen"
        :size="34"
        @click="store.logOpen = !store.logOpen"
      />

      <div class="window-controls">
        <button class="window-btn" title="最小化" @click="minimize">
          <AppIcon name="minimize" :size="14" />
        </button>
        <button class="window-btn" title="最大化" @click="toggleMaximize">
          <AppIcon name="maximize" :size="14" />
        </button>
        <button class="window-btn window-btn--close" title="关闭" @click="close">
          <AppIcon name="close" :size="14" />
        </button>
      </div>
    </div>
  </header>
</template>

<style scoped>
.topbar {
  height: 62px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding-left: 18px;
  background: linear-gradient(180deg, var(--bar-a), var(--bar-b));
  border-bottom: 1px solid var(--line-strong);
  box-shadow: 0 1px 0 rgba(255, 255, 255, 0.7) inset, 0 4px 14px rgba(80, 58, 32, 0.05);
  position: relative;
  z-index: 20;
}

.topbar__brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-right: 16px;
  margin-right: 2px;
  border-right: 1px solid var(--line);
  height: 36px;
}

.brand__mark {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: linear-gradient(180deg, var(--moss) 0 42%, var(--walnut) 42% 100%);
  border: 1px solid var(--walnut-deep);
  box-shadow: 0 3px 8px rgba(70, 48, 26, 0.22), 0 1px 0 rgba(255, 255, 255, 0.4) inset;
  position: relative;
  transform: rotate(-3deg);
  transition: transform var(--dur-3) var(--ease-out-expo);
  flex-shrink: 0;
}

.topbar__brand:hover .brand__mark {
  transform: rotate(3deg) scale(1.05);
}

.brand__mark::after {
  content: "";
  position: absolute;
  inset: 5px;
  border: 1px solid rgba(255, 253, 246, 0.28);
  border-radius: 3px;
}

.brand__text {
  display: flex;
  flex-direction: column;
  line-height: 1.05;
}

.brand__text strong {
  font-family: var(--font-display);
  font-size: 19px;
  letter-spacing: 0.16em;
  color: var(--walnut-deep);
}

.brand__text span {
  font-family: var(--font-mono);
  font-size: 7.5px;
  letter-spacing: 0.16em;
  color: var(--muted);
  margin-top: 3px;
}

.topnav {
  display: flex;
  align-items: center;
  gap: 2px;
  height: 100%;
}

.topnav__item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 7px;
  height: 34px;
  padding: 0 13px;
  border-radius: 8px;
  color: var(--ink-soft);
  font-size: 13px;
  font-weight: 700;
  transition: background var(--dur-2) ease, color var(--dur-2) ease,
    transform var(--dur-2) var(--ease-out-expo);
}

.topnav__idx {
  font-size: 9px;
  color: #b3a18a;
  letter-spacing: 0.06em;
  transition: color var(--dur-2) ease;
}

.topnav__item:hover {
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--walnut-deep);
}

.topnav__item .icon {
  color: var(--muted);
  transition: color var(--dur-2) ease;
}

.topnav__item:hover .icon {
  color: var(--walnut);
}

.topnav__item--active .icon {
  color: var(--moss);
}

.topnav__item--active {
  background: var(--panel);
  color: var(--walnut-deep);
  box-shadow: var(--shadow-1);
}

.topnav__item--active .topnav__idx {
  color: var(--moss);
}

.topnav__item--active::after {
  content: "";
  position: absolute;
  left: 13px;
  right: 13px;
  bottom: -1px;
  height: 2px;
  border-radius: 2px 2px 0 0;
  background: var(--moss);
  animation: tab-in var(--dur-3) var(--ease-out-expo) both;
}

@keyframes tab-in {
  from {
    transform: scaleX(0.3);
    opacity: 0;
  }
  to {
    transform: scaleX(1);
    opacity: 1;
  }
}

.topbar__drag {
  flex: 1;
  height: 100%;
  min-width: 20px;
}

.topbar__right {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 100%;
  padding-right: 0;
}

.account-chip {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 38px;
  padding: 0 12px 0 6px;
  border-radius: 10px;
  border: 1px solid transparent;
  transition: background var(--dur-2) ease, border-color var(--dur-2) ease;
  max-width: 190px;
}

.account-chip:hover {
  background: rgba(var(--glass-rgb), 0.7);
  border-color: var(--line);
}

.account-chip__name {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.log-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  height: 32px;
  padding: 0 12px;
  border-radius: 9px;
  border: 1px solid var(--line);
  background: var(--panel-2);
  color: var(--ink-soft);
  font-size: 12.5px;
  font-weight: 600;
  transition: all var(--dur-2) ease;
}

.log-btn:hover {
  border-color: var(--walnut-light);
  color: var(--walnut-deep);
}

.log-btn--active {
  border-color: var(--moss);
  color: var(--moss);
  background: var(--moss-soft);
}

.window-controls {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 100%;
  margin-left: 6px;
  padding-right: 8px;
}

.window-controls::before {
  content: "";
  width: 1px;
  height: 18px;
  margin-right: 6px;
  background: var(--line);
}

.window-btn {
  width: 32px;
  height: 32px;
  border-radius: 9px;
  display: grid;
  place-items: center;
  color: var(--ink-soft);
  transition: background var(--dur-1) ease, color var(--dur-1) ease,
    transform var(--dur-1) ease, box-shadow var(--dur-1) ease;
}

.window-btn:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--walnut-deep);
}

.window-btn:active {
  transform: scale(0.92);
}

.window-btn:focus-visible {
  outline: none;
  box-shadow: 0 0 0 2px rgba(var(--accent-rgb), 0.4);
}

.window-btn--close:hover {
  background: #b4523a;
  color: #fffdf6;
  box-shadow: 0 4px 12px rgba(180, 82, 58, 0.28);
}

@media (max-width: 1120px) {
  .brand__text span {
    display: none;
  }

  .topnav__idx {
    display: none;
  }
}

@media (max-width: 1040px) {
  .account-chip__name {
    display: none;
  }

  .account-chip {
    padding: 0 6px;
  }
}
</style>
