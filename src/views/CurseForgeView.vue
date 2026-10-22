<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  api,
  type CfFileView,
  type CfProject,
  type InstanceContext,
} from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodSelect from "../components/WoodSelect.vue";
import { installCfFile, notify, store } from "../store";
import { formatBytes, formatDate } from "../utils";

const props = defineProps<{ kind: string }>();

const CLASS_IDS: Record<string, number> = {
  mods: 6,
  resourcepacks: 12,
  modpacks: 4471,
  shaders: 6552,
  datapacks: 6945,
};

const CLASS_LABELS: Record<string, string> = {
  mods: "Mod",
  resourcepacks: "资源包",
  shaders: "光影包",
  modpacks: "整合包",
  datapacks: "数据包",
};

const LOADER_LABELS: Record<string, string> = {
  vanilla: "原版",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};

const instanceId = ref<string>("");
const context = ref<InstanceContext | null>(null);
const query = ref("");
const items = ref<CfProject[]>([]);
const total = ref(0);
const index = ref(0);
const loading = ref(false);
const error = ref("");
const active = ref<CfProject | null>(null);
const files = ref<CfFileView[]>([]);
const filesLoading = ref(false);
const filesError = ref("");

const classId = computed(() => CLASS_IDS[props.kind] ?? 6);
const installable = computed(() => props.kind !== "modpacks");
const instanceOptions = computed(() =>
  store.installed.map((v) => ({ value: v.id, label: v.id }))
);
const gameVersion = computed(() => context.value?.game_version ?? null);
const loader = computed(() =>
  context.value && context.value.loader !== "vanilla" ? context.value.loader : null
);

onMounted(async () => {
  instanceId.value =
    store.config?.selected_version &&
    store.installed.some((v) => v.id === store.config?.selected_version)
      ? store.config.selected_version
      : store.installed[0]?.id ?? "";
  await loadContext();
  await search(true);
});

watch(instanceId, async () => {
  await loadContext();
  active.value = null;
  await search(true);
});

watch(
  () => props.kind,
  async () => {
    active.value = null;
    items.value = [];
    await search(true);
  }
);

async function loadContext() {
  context.value = null;
  if (!instanceId.value) return;
  try {
    context.value = await api.instanceContext(instanceId.value);
  } catch (err) {
    context.value = null;
    error.value = String(err);
  }
}

async function search(reset = false) {
  if (!instanceId.value) {
    items.value = [];
    total.value = 0;
    return;
  }
  if (loading.value) return;
  if (reset) {
    index.value = 0;
    items.value = [];
  }
  loading.value = true;
  error.value = "";
  try {
    const result = await api.cfSearch(
      classId.value,
      query.value,
      gameVersion.value,
      loader.value,
      index.value
    );
    items.value = reset ? result.items : [...items.value, ...result.items];
    total.value = result.total;
  } catch (err) {
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  if (loading.value || items.value.length >= total.value) return;
  index.value += 20;
  await search(false);
}

function openProject(project: CfProject) {
  active.value = project;
  files.value = [];
  filesError.value = "";
  loadFiles(project);
}

async function loadFiles(project: CfProject) {
  filesLoading.value = true;
  try {
    files.value = await api.cfFiles(
      project.id,
      gameVersion.value,
      props.kind === "mods" ? loader.value : null
    );
  } catch (err) {
    filesError.value = String(err);
  } finally {
    filesLoading.value = false;
  }
}

function install(project: CfProject, file: CfFileView) {
  if (!file.download_url || !instanceId.value) return;
  installCfFile(
    instanceId.value,
    props.kind,
    file.download_url,
    file.file_name,
    `${project.name} ${file.name}`
  );
}

function releaseTag(type: number) {
  if (type === 1) return "正式版";
  if (type === 2) return "测试版";
  if (type === 3) return "Alpha";
  return "";
}

function compactNumber(value: number) {
  if (value >= 100_000_000) return `${(value / 100_000_000).toFixed(1)} 亿`;
  if (value >= 10_000) return `${(value / 10_000).toFixed(1)} 万`;
  return String(value);
}

function shortVersions(list: string[]) {
  return list.slice(0, 2).join(" / ");
}

function refresh() {
  notify("正在刷新 CurseForge 列表");
  search(true);
}
</script>

<template>
  <div class="cf">
    <section class="card card-pad cf-toolbar">
      <div class="cf-toolbar__instance">
        <div class="cf-toolbar__label">目标实例</div>
        <WoodSelect
          v-if="instanceOptions.length"
          v-model="instanceId"
          :options="instanceOptions"
        />
        <p v-else class="muted small">
          还没有已安装的版本，先到原版游戏里安装一个实例。
        </p>
        <div v-if="context" class="cf-toolbar__meta">
          <span class="tag">{{ context.game_version }}</span>
          <span class="tag">{{ LOADER_LABELS[context.loader] ?? context.loader }}</span>
          <span class="muted small">下载内容会写入该实例目录</span>
        </div>
      </div>

      <div class="cf-toolbar__search">
        <WoodInput
          v-model="query"
          :placeholder="`搜索 ${CLASS_LABELS[kind] ?? '资源'} 名称`"
          @keyup.enter="search(true)"
        />
        <WoodButton variant="primary" sheen :disabled="loading" @click="search(true)">
          <AppIcon name="grid" :size="15" />
          搜索
        </WoodButton>
        <IconButton
          icon="refresh"
          label="刷新列表"
          variant="soft"
          :size="38"
          @click="refresh"
        />
      </div>
    </section>

    <template v-if="active">
      <div class="cf-detail-head">
        <IconButton
          icon="chevron-left"
          label="返回列表"
          :size="34"
          @click="active = null"
        />
        <img v-if="active.logo_url" class="cf-detail__logo" :src="active.logo_url" alt="" />
        <div class="cf-detail__text">
          <h2 class="cf-detail__title">{{ active.name }}</h2>
          <p class="muted small">
            {{ active.author || "未知作者" }} · {{ compactNumber(active.download_count) }} 次下载
          </p>
          <p class="muted small cf-detail__summary">{{ active.summary }}</p>
        </div>
      </div>

      <section class="card card-pad cf-files">
        <div class="row-between">
          <div>
            <div class="section-title">可用文件</div>
            <p class="muted small">
              {{ context ? `按 ${context.game_version} 筛选` : "按当前实例筛选" }}
              <template v-if="kind === 'mods' && loader">
                · {{ LOADER_LABELS[loader] }}
              </template>
            </p>
          </div>
        </div>

        <div v-if="filesLoading" class="cf-files__state">
          <span class="skeleton" />
          <span class="muted small">正在获取文件列表…</span>
        </div>
        <div v-else-if="filesError" class="cf-files__state cf-files__state--error">
          {{ filesError }}
        </div>
        <div v-else-if="!files.length" class="cf-files__state muted small">
          没有找到适配当前实例的文件。
        </div>
        <div v-else class="cf-file-list">
          <div v-for="file in files" :key="file.id" class="cf-file">
            <div class="cf-file__main">
              <span class="cf-file__name">{{ file.name }}</span>
              <span class="cf-file__meta mono">
                {{ shortVersions(file.game_versions) }}
                <template v-if="file.game_versions.length > 2">…</template>
              </span>
            </div>
            <span v-if="releaseTag(file.release_type)" class="tag">
              {{ releaseTag(file.release_type) }}
            </span>
            <span class="cf-file__size mono">{{ formatBytes(file.size) }}</span>
            <span class="cf-file__date mono">{{ formatDate(file.file_date) }}</span>
            <IconButton
              v-if="file.download_url && installable"
              icon="download"
              :label="`下载 ${file.name}`"
              variant="soft"
              :size="30"
              :disabled="!!store.installingId"
              @click="install(active, file)"
            />
            <span v-else-if="!installable" class="muted small cf-file__hint">
              整合包安装将在下一步支持
            </span>
            <span v-else class="muted small cf-file__hint">无下载地址</span>
          </div>
        </div>
      </section>
    </template>

    <template v-else>
      <div v-if="error" class="card card-pad cf-state cf-state--error">
        <span>{{ error }}</span>
        <WoodButton variant="ghost" size="sm" @click="search(true)">重试</WoodButton>
      </div>

      <div v-else-if="loading && !items.length" class="cf-grid">
        <div v-for="n in 8" :key="n" class="card cf-card cf-card--skeleton">
          <span class="skeleton cf-skeleton__logo" />
          <div class="cf-card__body">
            <span class="skeleton cf-skeleton__title" />
            <span class="skeleton cf-skeleton__line" />
            <span class="skeleton cf-skeleton__line cf-skeleton__line--short" />
          </div>
        </div>
      </div>

      <div v-else-if="!items.length" class="card card-pad cf-state">
        <AppIcon name="package" :size="30" />
        <div>
          <p class="cf-state__title">没有匹配的{{ CLASS_LABELS[kind] ?? "资源" }}</p>
          <p class="muted small">换个关键词，或者切换实例的游戏版本再试。</p>
        </div>
      </div>

      <div v-else class="cf-grid">
        <button
          v-for="(project, i) in items"
          :key="project.id"
          class="card cf-card"
          :style="{ '--i': i }"
          @click="openProject(project)"
        >
          <img
            v-if="project.logo_url"
            class="cf-card__logo"
            :src="project.logo_url"
            alt=""
            loading="lazy"
          />
          <span v-else class="cf-card__logo cf-card__logo--empty">
            {{ project.name.slice(0, 1) }}
          </span>
          <div class="cf-card__body">
            <strong class="cf-card__name">{{ project.name }}</strong>
            <span class="cf-card__meta mono">
              {{ project.author || "未知作者" }} · {{ compactNumber(project.download_count) }} 下载
            </span>
            <p class="cf-card__summary">{{ project.summary }}</p>
          </div>
        </button>
      </div>

      <div v-if="items.length && items.length < total" class="load-more">
        <WoodButton variant="ghost" size="sm" :disabled="loading" @click="loadMore">
          <AppIcon name="chevron-down" :size="14" />
          加载更多（还有 {{ total - items.length }} 个）
        </WoodButton>
      </div>

      <p v-if="items.length" class="muted small footnote">
        数据来自 CurseForge，安装时会把对应文件写入实例的
        {{ kind }} 目录；加载器与版本已按当前实例自动筛选。
      </p>
    </template>
  </div>
</template>

<style scoped>
.cf {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.cf-toolbar {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 22px;
  flex-wrap: wrap;
}

.cf-toolbar__instance {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 220px;
}

.cf-toolbar__label {
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.16em;
  color: var(--muted);
  text-transform: uppercase;
}

.cf-toolbar__meta {
  display: flex;
  align-items: center;
  gap: 7px;
  flex-wrap: wrap;
}

.cf-toolbar__search {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  justify-content: flex-end;
  min-width: 320px;
}

.cf-toolbar__search :deep(.input) {
  max-width: 320px;
  flex: 1;
}

.cf-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
  gap: 12px;
}

.cf-card {
  display: flex;
  gap: 13px;
  padding: 14px;
  text-align: left;
  color: var(--ink);
  animation: rise-in 460ms var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 16ms);
  transition: border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
}

.cf-card:hover {
  border-color: rgba(var(--accent-rgb), 0.34);
  box-shadow: var(--shadow-2);
}

.cf-card__logo {
  width: 46px;
  height: 46px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--paper-2);
}

.cf-card__logo--empty {
  display: grid;
  place-items: center;
  font-family: var(--font-display);
  font-size: 20px;
  color: var(--moss);
  background: var(--moss-soft);
}

.cf-card__body {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.cf-card__name {
  font-size: 14.5px;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cf-card__meta {
  font-size: 10.5px;
  color: var(--muted);
  letter-spacing: 0.03em;
}

.cf-card__summary {
  font-size: 12px;
  color: var(--ink-soft);
  line-height: 1.5;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.cf-card--skeleton {
  pointer-events: none;
}

.cf-skeleton__logo {
  width: 46px;
  height: 46px;
  border-radius: 10px;
}

.cf-skeleton__title {
  width: 110px;
  height: 14px;
}

.cf-skeleton__line {
  width: 100%;
  height: 10px;
}

.cf-skeleton__line--short {
  width: 62%;
}

.cf-state {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 34px 26px;
  color: var(--walnut);
}

.cf-state--error {
  color: var(--danger);
}

.cf-state__title {
  font-size: 15px;
  font-weight: 800;
  color: var(--walnut-deep);
  margin-bottom: 3px;
}

.cf-detail-head {
  display: flex;
  align-items: flex-start;
  gap: 13px;
}

.cf-detail__logo {
  width: 54px;
  height: 54px;
  border-radius: 12px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--paper-2);
}

.cf-detail__text {
  min-width: 0;
}

.cf-detail__title {
  font-family: var(--font-display);
  font-size: 30px;
  line-height: 1.1;
  color: var(--walnut-deep);
}

.cf-detail__summary {
  margin-top: 4px;
  max-width: 720px;
}

.cf-files {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.cf-files__state {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 12px 4px;
  color: var(--muted);
  font-size: 12px;
}

.cf-files__state .skeleton {
  width: 90px;
  height: 12px;
}

.cf-files__state--error {
  color: var(--danger);
}

.cf-file-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.cf-file {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 11px;
  border-radius: 8px;
  transition: background var(--dur-1) ease;
}

.cf-file:hover {
  background: rgba(var(--accent-rgb), 0.06);
}

.cf-file__main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.cf-file__name {
  font-size: 13px;
  color: var(--ink);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cf-file__meta {
  font-size: 10.5px;
  color: var(--muted);
}

.cf-file__size,
.cf-file__date {
  font-size: 10.5px;
  color: var(--muted);
  flex-shrink: 0;
}

.cf-file__size {
  width: 72px;
  text-align: right;
}

.cf-file__date {
  width: 78px;
  text-align: right;
}

.cf-file__hint {
  font-size: 11px;
}

.footnote {
  padding-left: 2px;
}

@media (max-width: 1080px) {
  .cf-toolbar__search {
    min-width: 100%;
  }

  .cf-file__date {
    display: none;
  }
}
</style>
