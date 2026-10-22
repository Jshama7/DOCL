<script setup lang="ts">
import { computed, markRaw, onMounted } from "vue";
import TitleBar from "./components/TitleBar.vue";
import DownloadCenter from "./components/DownloadCenter.vue";
import LogDrawer from "./components/LogDrawer.vue";
import HomeView from "./views/HomeView.vue";
import VersionsView from "./views/VersionsView.vue";
import DownloadView from "./views/DownloadView.vue";
import AccountsView from "./views/AccountsView.vue";
import InstanceView from "./views/InstanceView.vue";
import SettingsView from "./views/SettingsView.vue";
import { initApp, selectedAccount, selectedVersion, store, type Page } from "./store";

onMounted(initApp);

const appStyle = computed(
  () =>
    ({
      "--bg-blur": `${store.config?.background_blur ?? 4}px`,
      "--glass-blur": `${store.config?.glass_blur ?? 18}px`,
      "--glass-alpha": String((store.config?.glass_opacity ?? 62) / 100),
    }) as Record<string, string>
);

const views = {
  home: markRaw(HomeView),
  versions: markRaw(VersionsView),
  download: markRaw(DownloadView),
  accounts: markRaw(AccountsView),
  instance: markRaw(InstanceView),
  settings: markRaw(SettingsView),
};

const currentView = computed(() => views[store.page as Page]);

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
  <div
    class="app"
    :class="{
      'no-anim': store.config && !store.config.animations,
      glass: !!store.backgroundUrl || !!store.backgroundPreset,
    }"
    :style="appStyle"
  >
    <div
      v-if="store.backgroundUrl || store.backgroundPreset"
      class="wallpaper"
      :class="store.backgroundPreset ? `wallpaper--${store.backgroundPreset}` : ''"
      aria-hidden="true"
    >
      <img v-if="store.backgroundUrl" :src="store.backgroundUrl" alt="" />
      <span v-else class="wallpaper__builtin" />
      <span class="wallpaper__tint" />
    </div>

    <div class="ambient" aria-hidden="true">
      <span class="ambient__orb ambient__orb--a" />
      <span class="ambient__orb ambient__orb--b" />
      <span class="ambient__grain" />
    </div>

    <TitleBar />

    <main class="main">
      <Transition name="page" mode="out-in">
        <component :is="currentView" :key="store.page" />
      </Transition>
    </main>

    <footer class="statusbar">
      <span
        class="statusbar__dot"
        :class="{ 'statusbar__dot--run': store.gameRunning || !!store.installingId }"
      />
      <span>{{ statusText }}</span>
      <span v-if="selectedVersion">版本 {{ selectedVersion.id }}</span>
      <span>{{ javaLabel }}</span>
      <span>账号 {{ selectedAccount?.name ?? "未登录" }}</span>
      <div class="statusbar__right">
        <span>{{ store.installed.length }} VERSIONS</span>
        <span>{{ store.javaList.length }} JAVA</span>
        <span>DOCL 0.1.0</span>
      </div>
    </footer>

    <DownloadCenter />
    <LogDrawer />

    <Transition name="toast">
      <div v-if="store.toast" class="toast" :class="{ 'toast--err': store.toast.kind === 'err' }">
        {{ store.toast.text }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition: opacity 160ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
}
</style>
