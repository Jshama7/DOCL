<script setup lang="ts">
import { computed, markRaw, onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import TitleBar from "./components/TitleBar.vue";
import DownloadCenter from "./components/DownloadCenter.vue";
import LogDrawer from "./components/LogDrawer.vue";
import CrashDialog from "./components/CrashDialog.vue";
import DropImportDialog from "./components/DropImportDialog.vue";
import AppIcon from "./components/AppIcon.vue";
import HomeView from "./views/HomeView.vue";
import VersionsView from "./views/VersionsView.vue";
import DownloadView from "./views/DownloadView.vue";
import AccountsView from "./views/AccountsView.vue";
import SkinsView from "./views/SkinsView.vue";
import InstanceView from "./views/InstanceView.vue";
import MultiplayerView from "./views/MultiplayerView.vue";
import SettingsView from "./views/SettingsView.vue";
import { api, type DroppedItem } from "./api";
import {
  initApp,
  notify,
  selectedAccount,
  selectedVersion,
  startupUpdate,
  store,
  type Page,
} from "./store";

const appVersion = ref("");
const dropActive = ref(false);
const dropItems = ref<DroppedItem[]>([]);
let unlistenDrop: (() => void) | undefined;

onMounted(async () => {
  window.addEventListener("docl-open-error", (event) => {
    notify((event as CustomEvent<string>).detail, "err");
  });
  await initApp();
  void loadVersion();
  void checkStartupUpdate();
  void listenDrop();
});

onBeforeUnmount(() => {
  unlistenDrop?.();
});

/// 拖入文件后交给后端识别分类，识别结果决定弹窗里怎么装。
async function listenDrop() {
  try {
    unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        dropActive.value = true;
        return;
      }
      dropActive.value = false;
      if (payload.type === "drop") void handleDrop(payload.paths);
    });
  } catch {
    // 拖放不可用时不影响其他功能
  }
}

async function handleDrop(paths: string[]) {
  if (!paths.length) return;
  if (store.installingId) {
    notify("已有任务正在进行，请稍后再导入", "err");
    return;
  }
  try {
    const items = await api.dropInspect(paths);
    if (!items.length) {
      notify("没有找到可导入的文件", "err");
      return;
    }
    dropItems.value = items;
  } catch (err) {
    notify(`识别文件失败：${String(err)}`, "err");
  }
}

async function loadVersion() {
  try {
    appVersion.value = await api.appVersion();
  } catch {
    // 版本号只是展示用，读取失败不影响其他功能
  }
}

/// 按设置里的更新策略在启动时静默检查；失败不打扰用户。
async function checkStartupUpdate() {
  const config = store.config;
  if (!config || config.update_mode === "off" || !config.update_url.trim()) return;
  try {
    const info = await api.updateCheck(config.update_url);
    if (!info.available) return;
    if (config.update_mode === "auto") {
      const path = await api.updateDownload(info.url, info.latest);
      startupUpdate.value = { info, path };
      notify(`已下载新版本 v${info.latest}，可在 设置 → 更新 中安装`);
    } else {
      startupUpdate.value = { info, path: null };
      notify(`发现新版本 v${info.latest}，可在 设置 → 更新 中查看`);
    }
  } catch {
    // 静默检查失败交给设置页手动重试
  }
}

const appStyle = computed(
  () =>
    ({
      "--bg-blur": `${store.config?.background_blur ?? 4}px`,
      "--glass-blur": `${store.config?.glass_blur ?? 18}px`,
      "--glass-alpha": String((store.config?.glass_opacity ?? 62) / 100),
      "--bg-dim": String((store.config?.background_dim ?? 0) / 100),
    }) as Record<string, string>
);

const views = {
  home: markRaw(HomeView),
  versions: markRaw(VersionsView),
  download: markRaw(DownloadView),
  accounts: markRaw(AccountsView),
  skins: markRaw(SkinsView),
  instance: markRaw(InstanceView),
  multiplayer: markRaw(MultiplayerView),
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
    :data-radius="store.config?.ui_radius ?? 'default'"
    :data-density="store.config?.ui_density ?? 'normal'"
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
      <span class="wallpaper__dim" />
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
        <span>DOCL {{ appVersion || "1.1.0" }}</span>
      </div>
    </footer>

    <DownloadCenter />
    <LogDrawer />
    <CrashDialog />

    <Transition name="drop">
      <div v-if="dropActive" class="drop-hint">
        <div class="drop-hint__card">
          <AppIcon name="package" :size="30" />
          <strong>松开以导入</strong>
          <span>自动识别模组、资源包、光影、数据包、存档与整合包</span>
        </div>
      </div>
    </Transition>

    <DropImportDialog
      v-if="dropItems.length"
      :items="dropItems"
      @close="dropItems = []"
      @done="dropItems = []"
    />

    <Transition name="toast">
      <div v-if="store.toast" class="toast" :class="{ 'toast--err': store.toast.kind === 'err' }">
        {{ store.toast.text }}
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.drop-hint {
  position: fixed;
  inset: 0;
  z-index: 78;
  display: grid;
  place-items: center;
  background: rgba(52, 38, 24, 0.26);
  backdrop-filter: blur(4px);
  pointer-events: none;
}

.drop-hint__card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 30px 46px;
  border-radius: 18px;
  border: 2px dashed var(--moss);
  background: var(--panel);
  box-shadow: 0 24px 60px rgba(50, 34, 18, 0.28);
  color: var(--walnut-deep);
}

.drop-hint__card strong {
  font-size: 17px;
  letter-spacing: 0.04em;
}

.drop-hint__card span {
  font-size: 12px;
  color: var(--muted);
}

.drop-enter-active,
.drop-leave-active {
  transition: opacity 140ms ease;
}

.drop-enter-from,
.drop-leave-to {
  opacity: 0;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 160ms ease;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
}
</style>
