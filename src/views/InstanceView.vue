<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  api,
  openFolder,
  type InstanceFile,
  type InstanceInfo,
  type InstanceMeta,
  type InstancePaths,
} from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodSelect from "../components/WoodSelect.vue";
import {
  launchVersion,
  notify,
  openCommunity,
  refreshInstanceMetas,
  store,
} from "../store";
import { formatBytes, formatDate, kindLabel } from "../utils";

type Tab =
  | "overview"
  | "launch"
  | "mods"
  | "resourcepacks"
  | "shaderpacks"
  | "saves"
  | "screenshots";

const tabs: { key: Tab; label: string; icon: string; no: string }[] = [
  { key: "overview", label: "概览", icon: "cube", no: "01" },
  { key: "launch", label: "启动设置", icon: "settings", no: "02" },
  { key: "mods", label: "Mod", icon: "grid", no: "03" },
  { key: "resourcepacks", label: "资源包", icon: "image", no: "04" },
  { key: "shaderpacks", label: "光影", icon: "sun", no: "05" },
  { key: "saves", label: "存档", icon: "database", no: "06" },
  { key: "screenshots", label: "截图", icon: "folder", no: "07" },
];

const contentTabs: Tab[] = [
  "mods",
  "resourcepacks",
  "shaderpacks",
  "saves",
  "screenshots",
];

const tab = ref<Tab>("overview");
const info = ref<InstanceInfo | null>(null);
const paths = ref<InstancePaths | null>(null);
const loading = ref(true);
const saving = ref(false);
const files = ref<InstanceFile[]>([]);
const filesLoading = ref(false);
const fileQuery = ref("");
const confirmName = ref<string | null>(null);
let confirmTimer: number | undefined;

const iconPresets = [
  { value: "auto", label: "自动", color: "#8b9d6f" },
  { value: "grass", label: "草方块", color: "#6f8f5a" },
  { value: "diamond", label: "钻石", color: "#5fa7a0" },
  { value: "gold", label: "黄金", color: "#cf9f45" },
  { value: "redstone", label: "红石", color: "#b8543f" },
  { value: "nether", label: "下界", color: "#7c4a3f" },
  { value: "end", label: "末地", color: "#6f5f86" },
];

const categoryOptions = [
  { value: "auto", label: "自动" },
  { value: "survival", label: "生存" },
  { value: "creative", label: "创造" },
  { value: "pvp", label: "PvP" },
  { value: "modpack", label: "整合包" },
  { value: "tech", label: "生电" },
  { value: "build", label: "建筑" },
  { value: "other", label: "其他" },
];

const fullscreenOptions = [
  { value: "inherit", label: "跟随全局" },
  { value: "on", label: "开启全屏" },
  { value: "off", label: "窗口模式" },
];

const form = reactive({
  display_name: "",
  description: "",
  icon: "auto",
  category: "auto",
  favorite: false,
  memory: "",
  javaPath: "",
  javaArgs: "",
  width: "",
  height: "",
  fullscreen: "inherit",
});

const javaOptions = computed(() => [
  { value: "", label: "跟随全局设置" },
  ...store.javaList.map((java) => ({
    value: java.path,
    label: `Java ${java.major} · ${java.source}`,
  })),
]);

const iconColor = computed(
  () => iconPresets.find((preset) => preset.value === form.icon)?.color ?? "#8b9d6f"
);

const loaderLabel = computed(() => {
  const loader = info.value?.loader ?? "vanilla";
  if (loader === "vanilla") return "原版";
  return loader.charAt(0).toUpperCase() + loader.slice(1);
});

const isContentTab = computed(() => contentTabs.includes(tab.value));
const contentLabel = computed(
  () => tabs.find((item) => item.key === tab.value)?.label ?? ""
);
const contentKind = computed(() =>
  isContentTab.value ? (tab.value as string) : ""
);
const filteredFiles = computed(() => {
  const keyword = fileQuery.value.trim().toLowerCase();
  if (!keyword) return files.value;
  return files.value.filter((file) =>
    file.display_name.toLowerCase().includes(keyword)
  );
});
const canDownloadMore = computed(
  () => tab.value === "mods" || tab.value === "resourcepacks" || tab.value === "shaderpacks"
);

const contentHint = computed(() => {
  switch (tab.value) {
    case "mods":
      return "禁用会保留文件并加上 .disabled 后缀，游戏不会加载";
    case "resourcepacks":
      return "支持 .zip 资源包与文件夹形式的资源包";
    case "shaderpacks":
      return "光影包需要游戏内安装对应光影加载器";
    case "saves":
      return "每个文件夹就是一个存档世界";
    case "screenshots":
      return "游戏内截图会保存在这里";
    default:
      return "";
  }
});

onMounted(async () => {
  if (!store.instanceId) {
    store.page = "versions";
    return;
  }
  await loadAll();
});

watch(tab, async (value) => {
  fileQuery.value = "";
  confirmName.value = null;
  if (contentTabs.includes(value)) {
    await loadFiles();
  }
});

async function loadAll() {
  loading.value = true;
  try {
    const id = store.instanceId!;
    const [detail, folder] = await Promise.all([
      api.instanceInfo(id),
      api.instancePaths(id),
    ]);
    info.value = detail;
    paths.value = folder;
    applyForm(detail);
  } catch (err) {
    notify(`读取版本信息失败：${String(err)}`, "err");
    store.page = "versions";
  } finally {
    loading.value = false;
  }
}

function applyForm(detail: InstanceInfo) {
  form.display_name =
    detail.display_name && detail.display_name !== detail.id
      ? detail.display_name
      : "";
  form.description = detail.description ?? "";
  form.icon = detail.icon || "auto";
  form.category = detail.category || "auto";
  form.favorite = detail.favorite;
  form.memory = detail.memory_mb ? String(detail.memory_mb) : "";
  form.javaPath = detail.java_path ?? "";
  form.javaArgs = detail.java_args ?? "";
  form.width = detail.window_width ? String(detail.window_width) : "";
  form.height = detail.window_height ? String(detail.window_height) : "";
  form.fullscreen =
    detail.fullscreen === null ? "inherit" : detail.fullscreen ? "on" : "off";
}

function numberOrNull(value: string, min: number) {
  const parsed = Number(value);
  return Number.isFinite(parsed) && parsed >= min ? Math.round(parsed) : null;
}

function buildMeta(): InstanceMeta {
  return {
    display_name: form.display_name.trim() || null,
    description: form.description.trim(),
    icon: form.icon,
    category: form.category,
    favorite: form.favorite,
    memory_mb: numberOrNull(form.memory, 512),
    java_path: form.javaPath || null,
    java_args: form.javaArgs.trim() || null,
    window_width: numberOrNull(form.width, 320),
    window_height: numberOrNull(form.height, 240),
    fullscreen:
      form.fullscreen === "inherit" ? null : form.fullscreen === "on",
  };
}

async function saveMeta() {
  if (!store.instanceId || saving.value) return;
  saving.value = true;
  try {
    const detail = await api.instanceSaveMeta(store.instanceId, buildMeta());
    info.value = detail;
    applyForm(detail);
    await refreshInstanceMetas();
    notify("版本设置已保存");
  } catch (err) {
    notify(`保存失败：${String(err)}`, "err");
  } finally {
    saving.value = false;
  }
}

async function toggleFavorite() {
  form.favorite = !form.favorite;
  await saveMeta();
}

async function loadFiles() {
  if (!store.instanceId) return;
  filesLoading.value = true;
  try {
    files.value = await api.instanceFiles(store.instanceId, contentKind.value);
  } catch (err) {
    notify(`读取文件失败：${String(err)}`, "err");
    files.value = [];
  } finally {
    filesLoading.value = false;
  }
}

async function refreshFiles() {
  await loadFiles();
  if (store.instanceId) {
    try {
      info.value = await api.instanceInfo(store.instanceId);
    } catch {
      // 忽略计数刷新失败
    }
  }
}

async function openContentFolder() {
  if (!paths.value || !isContentTab.value) return;
  const target = paths.value[tab.value as keyof InstancePaths];
  if (target) openFolder(target);
}

function openVersionFolder() {
  if (paths.value) openFolder(paths.value.version);
}

function openGameFolder() {
  if (paths.value) openFolder(paths.value.game_dir);
}

async function toggleFile(file: InstanceFile) {
  if (!store.instanceId) return;
  try {
    await api.instanceToggleFile(store.instanceId, contentKind.value, file.name);
    await loadFiles();
  } catch (err) {
    notify(`切换失败：${String(err)}`, "err");
  }
}

function requestDelete(file: InstanceFile) {
  if (confirmName.value === file.name) {
    confirmName.value = null;
    removeFile(file);
    return;
  }
  confirmName.value = file.name;
  if (confirmTimer) window.clearTimeout(confirmTimer);
  confirmTimer = window.setTimeout(() => {
    confirmName.value = null;
  }, 3200);
}

async function removeFile(file: InstanceFile) {
  if (!store.instanceId) return;
  try {
    await api.instanceDeleteFile(store.instanceId, contentKind.value, file.name);
    await refreshFiles();
    notify(`已删除 ${file.display_name}`);
  } catch (err) {
    notify(`删除失败：${String(err)}`, "err");
  }
}

function importFilters() {
  switch (tab.value) {
    case "mods":
      return [{ name: "Mod", extensions: ["jar", "zip"] }];
    case "resourcepacks":
    case "shaderpacks":
      return [{ name: "压缩包", extensions: ["zip"] }];
    case "screenshots":
      return [{ name: "图片", extensions: ["png", "jpg", "jpeg"] }];
    default:
      return undefined;
  }
}

async function importContent() {
  if (!store.instanceId) return;
  const isSaves = tab.value === "saves";
  const selection = await openDialog({
    multiple: true,
    directory: isSaves,
    title: isSaves ? "选择要导入的存档文件夹" : `选择要导入的${contentLabel.value}`,
    filters: isSaves ? undefined : importFilters(),
  });
  const selected = Array.isArray(selection)
    ? selection
    : selection
      ? [selection]
      : [];
  if (!selected.length) return;
  try {
    const count = await api.instanceImportFiles(
      store.instanceId,
      contentKind.value,
      selected
    );
    await refreshFiles();
    notify(count ? `已导入 ${count} 个文件` : "没有可导入的文件");
  } catch (err) {
    notify(`导入失败：${String(err)}`, "err");
  }
}

function contentIcon() {
  switch (tab.value) {
    case "mods":
      return "grid";
    case "resourcepacks":
      return "image";
    case "shaderpacks":
      return "sun";
    case "saves":
      return "database";
    case "screenshots":
      return "folder";
    default:
      return "package";
  }
}

function goTab(next: Tab) {
  tab.value = next;
}

function downloadMore() {
  if (!info.value) return;
  const map: Record<string, string> = {
    mods: "mods",
    resourcepacks: "resourcepacks",
    shaderpacks: "shaders",
  };
  const kind = map[tab.value];
  if (!kind) return;
  openCommunity(kind, info.value.id, info.value.game_version, info.value.loader);
}

function launch() {
  if (info.value) launchVersion(info.value.id);
}
</script>

<template>
  <div class="page instance">
    <header class="page-head ins-head">
      <IconButton
        icon="chevron-left"
        label="返回版本列表"
        :size="34"
        @click="store.page = 'versions'"
      />
      <span class="ins-tile" :style="{ '--tile': iconColor }">
        <AppIcon name="cube" :size="24" />
      </span>
      <div class="ins-head__text">
        <span class="eyebrow">DOCL / INSTANCE</span>
        <h1 class="page-title">{{ info?.display_name ?? "版本设置" }}</h1>
        <p class="page-sub">
          {{ info?.id }}
          <template v-if="info"> · {{ info.game_version }} · {{ kindLabel(info.kind) }} · {{ loaderLabel }}</template>
        </p>
      </div>
      <div class="row ins-head__actions">
        <IconButton
          icon="star"
          :label="form.favorite ? '取消收藏' : '收藏版本'"
          :variant="form.favorite ? 'primary' : 'soft'"
          :size="34"
          @click="toggleFavorite"
        />
        <IconButton
          icon="folder"
          label="打开版本目录"
          variant="soft"
          :size="34"
          @click="openVersionFolder"
        />
        <WoodButton
          variant="primary"
          sheen
          @click="launch"
        >
          <AppIcon :name="store.gameRunning ? 'stop' : 'play'" :size="15" />
          {{ store.gameRunning ? "停止游戏" : "启动游戏" }}
        </WoodButton>
      </div>
    </header>

    <div v-if="loading" class="ins-loading card card-pad">
      <span class="skeleton" />
      <span class="muted small">正在读取版本信息…</span>
    </div>

    <div v-else-if="info" class="ins-layout">
      <aside class="ins-side card">
        <button
          v-for="item in tabs"
          :key="item.key"
          class="ins-side__item"
          :class="{ 'ins-side__item--active': tab === item.key }"
          @click="goTab(item.key)"
        >
          <AppIcon :name="item.icon" :size="16" />
          <span class="ins-side__label">{{ item.label }}</span>
          <span
            v-if="item.key === 'mods' || item.key === 'resourcepacks' || item.key === 'shaderpacks' || item.key === 'saves' || item.key === 'screenshots'"
            class="ins-side__count mono"
          >
            {{ item.key === "mods" ? info.counts.mods : item.key === "resourcepacks" ? info.counts.resourcepacks : item.key === "shaderpacks" ? info.counts.shaderpacks : item.key === "saves" ? info.counts.saves : info.counts.screenshots }}
          </span>
          <span v-else class="ins-side__no mono">{{ item.no }}</span>
        </button>

        <div class="ins-side__meta">
          <span class="ins-side__key">版本目录</span>
          <span class="ins-side__path mono">{{ info.path }}</span>
          <span v-if="info.favorite" class="tag tag--grass ins-side__fav">已收藏</span>
        </div>
      </aside>

      <div class="ins-main">
        <!-- 概览 -->
        <template v-if="tab === 'overview'">
          <section class="card card-pad group reveal">
            <div class="group-head">
              <span class="group-no display">01</span>
              <div class="group-titles">
                <span class="eyebrow">IDENTITY</span>
                <div class="section-title">个性化</div>
              </div>
              <span class="group-line" />
              <WoodButton
                variant="primary"
                size="sm"
                sheen
                :disabled="saving"
                @click="saveMeta"
              >
                <AppIcon name="check" :size="14" />
                保存
              </WoodButton>
            </div>

            <div class="setting">
              <div class="setting__label">
                <strong>显示名称</strong>
                <span>只影响启动器显示，不改内部版本 ID</span>
              </div>
              <div class="setting__control">
                <WoodInput v-model="form.display_name" :placeholder="info.id" />
              </div>
            </div>

            <div class="setting">
              <div class="setting__label">
                <strong>分类</strong>
                <span>用来在版本列表里区分用途</span>
              </div>
              <div class="setting__control">
                <WoodSelect v-model="form.category" :options="categoryOptions" />
              </div>
            </div>

            <div class="setting">
              <div class="setting__label">
                <strong>图标</strong>
                <span>版本列表与首页使用同一个图标</span>
              </div>
              <div class="setting__control setting__control--wide">
                <div class="icon-grid">
                  <button
                    v-for="preset in iconPresets"
                    :key="preset.value"
                    class="icon-chip"
                    :class="{ 'icon-chip--active': form.icon === preset.value }"
                    @click="form.icon = preset.value"
                  >
                    <span
                      class="icon-chip__tile"
                      :style="{ '--tile': preset.color }"
                    >
                      <AppIcon name="cube" :size="16" />
                    </span>
                    {{ preset.label }}
                  </button>
                </div>
              </div>
            </div>

            <div class="setting setting--last">
              <div class="setting__label">
                <strong>描述</strong>
                <span>写点这个版本是做什么的，方便以后辨认</span>
              </div>
              <div class="setting__control setting__control--wide">
                <textarea
                  v-model="form.description"
                  class="ins-textarea"
                  rows="3"
                  placeholder="例如：1.21.1 生存整合包，带优化 Mod"
                />
              </div>
            </div>
          </section>

          <section class="card card-pad group reveal" style="--i: 1">
            <div class="group-head">
              <span class="group-no display">02</span>
              <div class="group-titles">
                <span class="eyebrow">SHORTCUTS</span>
                <div class="section-title">快捷方式</div>
              </div>
              <span class="group-line" />
            </div>

            <div class="quick-grid">
              <WoodButton variant="soft" @click="openVersionFolder">
                <AppIcon name="folder" :size="15" />
                版本目录
              </WoodButton>
              <WoodButton variant="soft" @click="openGameFolder">
                <AppIcon name="folder" :size="15" />
                游戏目录
              </WoodButton>
              <WoodButton variant="soft" @click="goTab('mods')">
                <AppIcon name="grid" :size="15" />
                Mod 文件夹
              </WoodButton>
              <WoodButton variant="soft" @click="goTab('saves')">
                <AppIcon name="database" :size="15" />
                存档文件夹
              </WoodButton>
              <WoodButton variant="soft" @click="goTab('screenshots')">
                <AppIcon name="folder" :size="15" />
                截图文件夹
              </WoodButton>
            </div>
          </section>

          <section class="card card-pad group reveal" style="--i: 2">
            <div class="group-head">
              <span class="group-no display">03</span>
              <div class="group-titles">
                <span class="eyebrow">DETAILS</span>
                <div class="section-title">详细信息</div>
              </div>
              <span class="group-line" />
            </div>

            <div class="detail-grid">
              <div class="detail">
                <span class="detail__label">游戏版本</span>
                <span class="detail__value">{{ info.game_version }}</span>
              </div>
              <div class="detail">
                <span class="detail__label">加载器</span>
                <span class="detail__value">{{ loaderLabel }}</span>
              </div>
              <div class="detail">
                <span class="detail__label">类型</span>
                <span class="detail__value">{{ kindLabel(info.kind) }}</span>
              </div>
              <div class="detail">
                <span class="detail__label">所需 Java</span>
                <span class="detail__value">Java {{ info.java_major }}</span>
              </div>
              <div class="detail">
                <span class="detail__label">安装于</span>
                <span class="detail__value">{{ formatDate(new Date(info.installed_at * 1000).toISOString()) }}</span>
              </div>
              <div class="detail">
                <span class="detail__label">目录模式</span>
                <span class="detail__value">{{ store.config?.version_isolation ? "版本隔离" : "共用目录" }}</span>
              </div>
            </div>

            <div class="count-strip">
              <button class="count" @click="goTab('mods')">
                <span class="count__num display">{{ info.counts.mods }}</span>
                <span class="count__label">Mod</span>
              </button>
              <button class="count" @click="goTab('resourcepacks')">
                <span class="count__num display">{{ info.counts.resourcepacks }}</span>
                <span class="count__label">资源包</span>
              </button>
              <button class="count" @click="goTab('shaderpacks')">
                <span class="count__num display">{{ info.counts.shaderpacks }}</span>
                <span class="count__label">光影</span>
              </button>
              <button class="count" @click="goTab('saves')">
                <span class="count__num display">{{ info.counts.saves }}</span>
                <span class="count__label">存档</span>
              </button>
              <button class="count" @click="goTab('screenshots')">
                <span class="count__num display">{{ info.counts.screenshots }}</span>
                <span class="count__label">截图</span>
              </button>
            </div>
          </section>
        </template>

        <!-- 启动设置 -->
        <section v-else-if="tab === 'launch'" class="card card-pad group reveal">
          <div class="group-head">
            <span class="group-no display">02</span>
            <div class="group-titles">
              <span class="eyebrow">LAUNCH</span>
              <div class="section-title">启动设置</div>
            </div>
            <span class="group-line" />
            <WoodButton
              variant="primary"
              size="sm"
              sheen
              :disabled="saving"
              @click="saveMeta"
            >
              <AppIcon name="check" :size="14" />
              保存
            </WoodButton>
          </div>

          <p class="muted small">
            留空表示跟随全局设置；这里的值只对当前版本生效，下次启动时使用。
          </p>

          <div class="setting">
            <div class="setting__label">
              <strong>最大内存</strong>
              <span>
                {{ form.memory ? `${form.memory} MB` : `跟随全局 · ${store.config?.memory_mb ?? 4096} MB` }}
              </span>
            </div>
            <div class="setting__control">
              <WoodInput v-model="form.memory" type="number" placeholder="例如 6144" />
            </div>
          </div>

          <div class="setting">
            <div class="setting__label">
              <strong>Java 运行时</strong>
              <span>该版本独用的 Java 路径</span>
            </div>
            <div class="setting__control setting__control--wide">
              <WoodSelect v-model="form.javaPath" :options="javaOptions" />
            </div>
          </div>

          <div class="setting">
            <div class="setting__label">
              <strong>JVM 参数</strong>
              <span>追加在全局参数之后，空格分隔</span>
            </div>
            <div class="setting__control setting__control--wide">
              <WoodInput v-model="form.javaArgs" mono placeholder="例如 -XX:+UseZGC" />
            </div>
          </div>

          <div class="setting">
            <div class="setting__label">
              <strong>窗口尺寸</strong>
              <span>留空使用全局分辨率</span>
            </div>
            <div class="setting__control">
              <div class="row resolution">
                <WoodInput v-model="form.width" type="number" placeholder="宽" />
                <span class="muted resolution__x">×</span>
                <WoodInput v-model="form.height" type="number" placeholder="高" />
              </div>
            </div>
          </div>

          <div class="setting setting--last">
            <div class="setting__label">
              <strong>全屏</strong>
              <span>启动时是否直接进入全屏</span>
            </div>
            <div class="setting__control">
              <WoodSelect v-model="form.fullscreen" :options="fullscreenOptions" />
            </div>
          </div>
        </section>

        <!-- 内容管理 -->
        <section v-else-if="isContentTab" class="card card-pad group reveal">
          <div class="group-head">
            <span class="group-no display">03</span>
            <div class="group-titles">
              <span class="eyebrow">CONTENT</span>
              <div class="section-title">{{ contentLabel }}</div>
            </div>
            <span class="group-line" />
            <div class="row">
              <WoodButton
                v-if="canDownloadMore"
                variant="primary"
                size="sm"
                sheen
                @click="downloadMore"
              >
                <AppIcon name="download" :size="14" />
                下载新{{ contentLabel }}
              </WoodButton>
              <WoodButton variant="ghost" size="sm" @click="openContentFolder">
                <AppIcon name="folder" :size="14" />
                打开文件夹
              </WoodButton>
              <WoodButton variant="soft" size="sm" @click="importContent">
                <AppIcon name="plus" :size="14" />
                从文件导入
              </WoodButton>
              <IconButton
                icon="refresh"
                label="刷新列表"
                variant="soft"
                :size="32"
                @click="refreshFiles"
              />
            </div>
          </div>

          <p class="muted small">{{ contentHint }}</p>

          <div v-if="files.length" class="file-search">
            <WoodInput
              v-model="fileQuery"
              :placeholder="`搜索${contentLabel}文件名`"
            />
            <span class="muted small">{{ filteredFiles.length }} / {{ files.length }}</span>
          </div>

          <div v-if="filesLoading" class="file-state">
            <span class="skeleton" />
            <span class="muted small">正在读取…</span>
          </div>
          <div v-else-if="!files.length" class="file-empty">
            <AppIcon :name="contentIcon()" :size="28" />
            <div>
              <p class="file-empty__title">还没有{{ contentLabel }}</p>
              <p class="muted small">点击「下载新{{ contentLabel }}」去资源站挑选，或者从本地文件导入。</p>
            </div>
            <WoodButton variant="soft" size="sm" @click="importContent">
              <AppIcon name="plus" :size="14" />
              从文件导入
            </WoodButton>
          </div>
          <div v-else-if="!filteredFiles.length" class="file-state">
            <span class="muted small">没有匹配的文件</span>
          </div>
          <div v-else class="file-list">
            <div v-for="file in filteredFiles" :key="file.name" class="file-row">
              <span class="file-row__icon">
                <AppIcon :name="contentIcon()" :size="16" />
              </span>
              <div class="file-row__main">
                <span class="file-row__name">
                  {{ file.display_name }}
                  <span v-if="!file.enabled" class="tag file-row__off">已禁用</span>
                </span>
                <span class="file-row__meta mono">
                  {{ file.is_dir ? "文件夹" : formatBytes(file.size) }}
                  · {{ formatDate(new Date(file.modified * 1000).toISOString()) }}
                </span>
              </div>
              <IconButton
                v-if="tab === 'mods' || tab === 'resourcepacks' || tab === 'shaderpacks'"
                :icon="file.enabled ? 'check' : 'close'"
                :label="file.enabled ? '禁用' : '启用'"
                variant="ghost"
                :size="30"
                @click="toggleFile(file)"
              />
              <IconButton
                icon="trash"
                :label="confirmName === file.name ? '确认删除' : '删除'"
                variant="danger"
                :size="30"
                @click="requestDelete(file)"
              />
            </div>
          </div>
        </section>
      </div>
    </div>
  </div>
</template>

<style scoped>
.ins-head {
  align-items: center;
}

.ins-tile {
  width: 46px;
  height: 46px;
  border-radius: 12px;
  display: grid;
  place-items: center;
  color: rgba(255, 255, 255, 0.92);
  background: linear-gradient(160deg, color-mix(in srgb, var(--tile) 78%, #ffffff), var(--tile));
  box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.5), var(--shadow-1);
  flex-shrink: 0;
}

.ins-head__text {
  flex: 1;
  min-width: 0;
}

.ins-head__actions {
  flex-shrink: 0;
}

.ins-loading {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ins-loading .skeleton {
  width: 140px;
  height: 12px;
}

.ins-layout {
  display: grid;
  grid-template-columns: 190px minmax(0, 1fr);
  gap: 16px;
  align-items: start;
}

.ins-side {
  padding: 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  position: sticky;
  top: 0;
}

.ins-side__item {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 38px;
  padding: 0 11px;
  border-radius: 8px;
  color: var(--ink-soft);
  font-size: 13px;
  font-weight: 600;
  text-align: left;
  transition: background var(--dur-2) ease, color var(--dur-2) ease;
}

.ins-side__item:hover {
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--walnut-deep);
}

.ins-side__item--active {
  background: var(--panel);
  color: var(--walnut-deep);
  box-shadow: var(--shadow-1);
}

.ins-side__item--active .icon {
  color: var(--moss);
}

.ins-side__label {
  flex: 1;
}

.ins-side__no,
.ins-side__count {
  font-size: 10.5px;
  color: var(--muted);
  letter-spacing: 0.08em;
}

.ins-side__meta {
  margin-top: 14px;
  padding: 10px 11px;
  border-top: 1px dashed var(--line);
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.ins-side__key {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--muted);
  text-transform: uppercase;
}

.ins-side__path {
  font-size: 10.5px;
  color: var(--ink-soft);
  word-break: break-all;
  line-height: 1.5;
}

.ins-side__fav {
  align-self: flex-start;
}

.ins-main {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}

.group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.group-head {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 10px;
}

.group-no {
  font-size: 26px;
  line-height: 1;
  color: var(--line-strong);
}

.group-titles {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.group-line {
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, var(--line), transparent);
}

.setting {
  display: flex;
  align-items: center;
  gap: 18px;
  padding: 10px 0;
  border-bottom: 1px dashed var(--line);
}

.setting--last {
  border-bottom: none;
}

.setting__label {
  width: 200px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.setting__label strong {
  font-size: 13px;
  color: var(--walnut-deep);
}

.setting__label span {
  font-size: 11px;
  color: var(--muted);
  line-height: 1.45;
}

.setting__control {
  flex: 1;
  min-width: 0;
  display: flex;
  justify-content: flex-end;
}

.setting__control--wide {
  justify-content: stretch;
}

.setting__control--wide > * {
  width: 100%;
}

.resolution {
  width: 100%;
}

.resolution__x {
  flex-shrink: 0;
}

.ins-textarea {
  width: 100%;
  padding: 10px 13px;
  border-radius: 9px;
  border: 1px solid var(--line-strong);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  color: var(--ink);
  font-size: 13px;
  line-height: 1.5;
  resize: vertical;
  transition: border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
}

.ins-textarea:focus {
  outline: none;
  border-color: var(--moss);
  box-shadow: 0 0 0 3px rgba(94, 123, 73, 0.14);
}

.icon-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.icon-chip {
  display: flex;
  align-items: center;
  gap: 7px;
  height: 36px;
  padding: 0 10px 0 6px;
  border-radius: 9px;
  border: 1px solid var(--line);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  color: var(--ink-soft);
  font-size: 12px;
  transition: border-color var(--dur-2) ease, color var(--dur-2) ease;
}

.icon-chip:hover {
  border-color: var(--walnut-light);
}

.icon-chip--active {
  border-color: var(--moss);
  color: var(--walnut-deep);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
}

.icon-chip__tile {
  width: 24px;
  height: 24px;
  border-radius: 6px;
  display: grid;
  place-items: center;
  color: rgba(255, 255, 255, 0.9);
  background: linear-gradient(160deg, color-mix(in srgb, var(--tile) 78%, #ffffff), var(--tile));
}

.quick-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 8px;
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
  gap: 10px 16px;
  padding-bottom: 12px;
  border-bottom: 1px dashed var(--line);
}

.detail {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.detail__label {
  font-family: var(--font-mono);
  font-size: 9.5px;
  letter-spacing: 0.12em;
  color: var(--muted);
  text-transform: uppercase;
}

.detail__value {
  font-size: 13px;
  color: var(--ink);
}

.count-strip {
  display: grid;
  grid-template-columns: repeat(5, minmax(0, 1fr));
  gap: 6px;
  margin-top: 12px;
}

.count {
  display: flex;
  flex-direction: column;
  gap: 1px;
  padding: 8px 10px;
  border-radius: 9px;
  text-align: left;
  transition: background var(--dur-2) ease;
}

.count:hover {
  background: rgba(var(--accent-rgb), 0.06);
}

.count__num {
  font-size: 24px;
  line-height: 1.05;
  color: var(--walnut-deep);
}

.count__label {
  font-size: 10.5px;
  color: var(--muted);
}

.file-state {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 14px 2px;
}

.file-search {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 4px 0;
}

.file-search :deep(.input) {
  max-width: 320px;
}

.file-state .skeleton {
  width: 100px;
  height: 12px;
}

.file-empty {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 24px 10px;
  color: var(--walnut);
}

.file-empty__title {
  font-size: 14px;
  font-weight: 800;
  color: var(--walnut-deep);
  margin-bottom: 2px;
}

.file-empty > div {
  flex: 1;
}

.file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin-top: 4px;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 9px;
  border-radius: 8px;
  transition: background var(--dur-1) ease;
}

.file-row:hover {
  background: rgba(var(--accent-rgb), 0.05);
}

.file-row__icon {
  width: 30px;
  height: 30px;
  border-radius: 7px;
  display: grid;
  place-items: center;
  background: var(--moss-soft);
  color: var(--moss);
  flex-shrink: 0;
}

.file-row__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.file-row__name {
  font-size: 13px;
  color: var(--ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-row__off {
  margin-left: 6px;
  font-size: 9.5px;
  color: var(--muted);
  border-color: var(--line);
}

.file-row__meta {
  font-size: 10.5px;
  color: var(--muted);
}

@media (max-width: 1080px) {
  .ins-layout {
    grid-template-columns: 158px minmax(0, 1fr);
  }

  .setting__label {
    width: 160px;
  }

  .count-strip {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
</style>
