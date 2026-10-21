<script setup lang="ts">
import { computed, onMounted } from "vue";
import TitleBar from "./components/TitleBar.vue";
import SideNav from "./components/SideNav.vue";
import LogDrawer from "./components/LogDrawer.vue";
import HomeView from "./views/HomeView.vue";
import VersionsView from "./views/VersionsView.vue";
import DownloadView from "./views/DownloadView.vue";
import AccountsView from "./views/AccountsView.vue";
import SettingsView from "./views/SettingsView.vue";
import { initApp, selectedAccount, selectedVersion, store } from "./store";

onMounted(initApp);

const javaLabel = computed(() => {
  const version = selectedVersion.value;
  if (!version) return "未选择版本";
  const java = store.javaList.find((j) => j.major === version.java_major);
  return java ? `Java ${java.major}` : `缺少 Java ${version.java_major}`;
});

const statusText = computed(() => {
  if (store.gameRunning) return "游戏运行中";
  if (store.installingId) return `正在安装 ${store.installingId}`;
  return "空闲";
});
</script>

<template>
  <div class="app" :class="{ 'no-anim': store.config && !store.config.animations }">
    <TitleBar />

    <div class="shell">
      <SideNav />
      <main class="main">
        <HomeView v-if="store.page === 'home'" />
        <VersionsView v-else-if="store.page === 'versions'" />
        <DownloadView v-else-if="store.page === 'download'" />
        <AccountsView v-else-if="store.page === 'accounts'" />
        <SettingsView v-else-if="store.page === 'settings'" />
      </main>
    </div>

    <footer class="statusbar">
      <span class="statusbar__dot" :class="{ 'statusbar__dot--run': store.gameRunning || !!store.installingId }" />
      <span>{{ statusText }}</span>
      <span v-if="selectedVersion">版本 {{ selectedVersion.id }}</span>
      <span>{{ javaLabel }}</span>
      <span>账号 {{ selectedAccount?.name ?? "未登录" }}</span>
      <div class="statusbar__right">
        <span>{{ store.installed.length }} 个已安装版本</span>
        <span>{{ store.javaList.length }} 个 Java</span>
      </div>
    </footer>

    <LogDrawer />

    <transition name="toast">
      <div v-if="store.toast" class="toast" :class="{ 'toast--err': store.toast.kind === 'err' }">
        {{ store.toast.text }}
      </div>
    </transition>
  </div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.16s ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
}
</style>
