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

const preset = store.communityContext;

type SourceKey = "curseforge" | "modrinth";

interface Bucket {
  items: CfProject[];
  total: number;
  next: number;
  done: boolean;
}

const CLASS_LABELS: Record<string, string> = {
  mods: "Mod",
  resourcepacks: "资源包",
  shaders: "光影包",
  modpacks: "整合包",
  datapacks: "数据包",
};

const CLASS_IDS: Record<string, number> = {
  mods: 6,
  resourcepacks: 12,
  shaders: 6552,
  datapacks: 6945,
  modpacks: 4471,
};

const LOADER_LABELS: Record<string, string> = {
  vanilla: "原版",
  fabric: "Fabric",
  quilt: "Quilt",
  forge: "Forge",
  neoforge: "NeoForge",
};

const sourceOptions = [
  { value: "all", label: "全部来源" },
  { value: "curseforge", label: "CurseForge" },
  { value: "modrinth", label: "Modrinth" },
];

function emptyBuckets(): Record<SourceKey, Bucket> {
  return {
    curseforge: { items: [], total: 0, next: 0, done: false },
    modrinth: { items: [], total: 0, next: 0, done: false },
  };
}

const instanceId = ref<string>("");
const context = ref<InstanceContext | null>(null);
const query = ref("");
const source = ref<"all" | SourceKey>("all");
const versionMode = ref("all");
const sortField = ref("2");
const versions = ref<string[]>([]);
const buckets = ref<Record<SourceKey, Bucket>>(emptyBuckets());
const loading = ref(false);
const error = ref("");
const active = ref<CfProject | null>(null);
const files = ref<CfFileView[]>([]);
const filesLoading = ref(false);
const filesError = ref("");

const label = computed(() => CLASS_LABELS[props.kind] ?? "资源");
const installable = computed(() => props.kind !== "modpacks");
const instanceOptions = computed(() =>
  store.installed.map((v) => ({ value: v.id, label: v.id }))
);
const versionOptions = computed(() => [
  {
    value: "instance",
    label: context.value ? `跟随实例 · ${context.value.game_version}` : "跟随实例",
  },
  { value: "all", label: "全部版本" },
  ...versions.value.map((version) => ({ value: version, label: version })),
]);
const sortOptions = [
  { value: "2", label: "按热度" },
  { value: "3", label: "最近更新" },
  { value: "6", label: "总下载量" },
];
const queryGameVersion = computed(() => {
  if (versionMode.value === "all") return null;
  if (versionMode.value === "instance") return context.value?.game_version ?? null;
  return versionMode.value;
});
const loader = computed(() => {
  if (context.value && context.value.loader !== "vanilla") {
    return context.value.loader;
  }
  if (!instanceId.value && preset?.loader && preset.loader !== "vanilla") {
    return preset.loader;
  }
  return null;
});
const activeSources = computed<SourceKey[]>(() =>
  source.value === "all" ? ["curseforge", "modrinth"] : [source.value]
);
const items = computed(() => {
  const list = activeSources.value.flatMap((key) => buckets.value[key].items);
  const sorted = [...list].sort((a, b) =>
    sortField.value === "3"
      ? b.updated.localeCompare(a.updated)
      : b.download_count - a.download_count
  );
  return sorted;
});
const total = computed(() =>
  activeSources.value.reduce((sum, key) => sum + buckets.value[key].total, 0)
);
const hasMore = computed(() =>
  activeSources.value.some((key) => !buckets.value[key].done)
);

onMounted(async () => {
  const fromPreset =
    preset?.instanceId &&
    store.installed.some((v) => v.id === preset.instanceId)
      ? preset.instanceId
      : "";
  instanceId.value =
    fromPreset ||
    (store.config?.selected_version &&
    store.installed.some((v) => v.id === store.config?.selected_version)
      ? store.config.selected_version
      : store.installed[0]?.id ?? "");
  if (instanceId.value) {
    versionMode.value = "instance";
  } else if (preset?.gameVersion) {
    versionMode.value = preset.gameVersion;
  } else {
    versionMode.value = "all";
  }
  loadVersions();
  await loadContext();
  await search(true);
  store.communityContext = null;
});

watch(instanceId, async () => {
  versionMode.value = instanceId.value ? "instance" : "all";
  active.value = null;
  await loadContext();
  await search(true);
});

watch(
  () => props.kind,
  async () => {
    active.value = null;
    await search(true);
  }
);

async function loadVersions() {
  try {
    versions.value = await api.cfGameVersions();
  } catch {
    versions.value = [];
  }
}

async function loadContext() {
  context.value = null;
  if (!instanceId.value) return;
  try {
    context.value = await api.instanceContext(instanceId.value);
  } catch {
    context.value = null;
  }
}

async function search(reset = true) {
  if (loading.value) return;
  if (reset) buckets.value = emptyBuckets();
  const targets = activeSources.value.filter(
    (key) => reset || !buckets.value[key].done
  );
  if (!targets.length) return;
  loading.value = true;
  error.value = "";
  const results = await Promise.allSettled(
    targets.map((key) => fetchBucket(key, reset ? 0 : buckets.value[key].next))
  );
  const failed = results.find((result) => result.status === "rejected");
  if (failed && failed.status === "rejected") {
    error.value = String(failed.reason);
  }
  loading.value = false;
}

async function fetchBucket(key: SourceKey, offset: number) {
  const version = queryGameVersion.value;
  const result =
    key === "curseforge"
      ? await api.cfSearch(
          CLASS_IDS[props.kind] ?? 6,
          query.value,
          version,
          loader.value,
          offset,
          Number(sortField.value)
        )
      : await api.modrinthSearch(
          props.kind,
          query.value,
          version,
          loader.value,
          offset,
          Number(sortField.value)
        );
  const bucket = buckets.value[key];
  const items = [...bucket.items, ...result.items];
  buckets.value = {
    ...buckets.value,
    [key]: {
      items,
      total: result.total,
      next: offset + 20,
      done: result.items.length === 0 || items.length >= result.total,
    },
  };
}

function resetSearch() {
  query.value = "";
  source.value = "all";
  versionMode.value = instanceId.value ? "instance" : "all";
  sortField.value = "2";
  search(true);
}

async function loadMore() {
  if (loading.value) return;
  const targets = activeSources.value.filter(
    (key) => !buckets.value[key].done
  );
  if (!targets.length) return;
  loading.value = true;
  await Promise.allSettled(
    targets.map((key) => fetchBucket(key, buckets.value[key].next))
  );
  loading.value = false;
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
    files.value =
      project.source === "modrinth"
        ? await api.modrinthFiles(
            project.id,
            queryGameVersion.value,
            props.kind === "mods" ? loader.value : null
          )
        : await api.cfFiles(
            project.id,
            queryGameVersion.value,
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

function sourceLabel(value: string) {
  return value === "modrinth" ? "Modrinth" : "CurseForge";
}

function compactNumber(value: number) {
  if (value >= 100_000_000) return `${(value / 100_000_000).toFixed(1)} 亿`;
  if (value >= 10_000) return `${(value / 10_000).toFixed(1)} 万`;
  return String(value);
}

function updatedLabel(iso: string) {
  if (!iso) return "";
  const time = new Date(iso).getTime();
  if (Number.isNaN(time)) return "";
  const diff = Date.now() - time;
  const hour = 3_600_000;
  const day = 24 * hour;
  if (diff < hour) return "刚刚更新";
  if (diff < day) return `${Math.floor(diff / hour)} 小时前`;
  if (diff < 30 * day) return `${Math.floor(diff / day)} 天前`;
  return formatDate(iso);
}

function shortVersions(list: string[]) {
  return list.slice(0, 2).join(" / ");
}

function refresh() {
  notify("正在刷新社区资源");
  loadVersions();
  search(true);
}
</script>

<template>
  <div class="cf">
    <section class="card card-pad cf-search">
      <div class="cf-search__head">
        <div class="cf-search__title">搜索{{ label }}</div>
        <span v-if="context" class="tag">
          {{ LOADER_LABELS[context.loader] ?? context.loader }} · {{ context.game_version }}
        </span>
      </div>

      <div class="cf-fields">
        <label class="cf-field">
          <span class="cf-field__label">来源</span>
          <WoodSelect v-model="source" :options="sourceOptions" />
        </label>

        <label class="cf-field cf-field--wide">
          <span class="cf-field__label">名称</span>
          <WoodInput
            v-model="query"
            :placeholder="`输入${label}名称，留空浏览全部`"
            @keyup.enter="search(true)"
          />
        </label>

        <label class="cf-field">
          <span class="cf-field__label">版本</span>
          <WoodSelect v-model="versionMode" :options="versionOptions" />
        </label>

        <label class="cf-field">
          <span class="cf-field__label">排序</span>
          <WoodSelect v-model="sortField" :options="sortOptions" />
        </label>

        <label class="cf-field">
          <span class="cf-field__label">目标实例</span>
          <WoodSelect
            v-if="instanceOptions.length"
            v-model="instanceId"
            :options="instanceOptions"
          />
          <span v-else class="cf-field__fallback muted small">暂未安装版本</span>
        </label>
      </div>

      <div class="cf-actions">
        <WoodButton variant="primary" sheen :disabled="loading" @click="search(true)">
          <AppIcon name="grid" :size="15" />
          搜索
        </WoodButton>
        <WoodButton variant="ghost" @click="resetSearch">重置条件</WoodButton>
        <IconButton
          icon="refresh"
          label="刷新列表"
          variant="soft"
          :size="38"
          @click="refresh"
        />
        <span v-if="!instanceOptions.length" class="muted small cf-actions__hint">
          还没安装版本也可以浏览；下载文件前需要先在「原版游戏」里安装一个实例。
        </span>
      </div>
    </section>

    <template v-if="active">
      <div class="cf-detail-head">
        <IconButton
          icon="chevron-left"
          label="返回搜索结果"
          :size="34"
          @click="active = null"
        />
        <img v-if="active.logo_url" class="cf-detail__logo" :src="active.logo_url" alt="" />
        <div class="cf-detail__text">
          <div class="cf-detail__title-row">
            <h2 class="cf-detail__title">{{ active.name }}</h2>
            <span class="tag">{{ sourceLabel(active.source) }}</span>
            <span
              v-for="category in active.categories.slice(0, 3)"
              :key="category"
              class="chip"
            >
              {{ category }}
            </span>
          </div>
          <p class="muted small">
            {{ active.author || "未知作者" }} · {{ compactNumber(active.download_count) }} 次下载
            <template v-if="active.updated"> · {{ updatedLabel(active.updated) }}</template>
          </p>
          <p class="muted small cf-detail__summary">{{ active.summary }}</p>
        </div>
      </div>

      <section class="card card-pad cf-files">
        <div class="row-between">
          <div>
            <div class="section-title">可用文件</div>
            <p class="muted small">
              {{ queryGameVersion ? `按 ${queryGameVersion} 筛选` : "全部版本" }}
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
          没有找到适配当前筛选的文件。
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
              v-if="file.download_url && installable && instanceId"
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
            <span v-else-if="!instanceId" class="muted small cf-file__hint">
              先安装一个实例
            </span>
            <span v-else class="muted small cf-file__hint">无下载地址</span>
          </div>
        </div>
      </section>
    </template>

    <template v-else>
      <div v-if="error && !items.length" class="card card-pad cf-state cf-state--error">
        <span>{{ error }}</span>
        <WoodButton variant="ghost" size="sm" @click="search(true)">重试</WoodButton>
      </div>

      <div v-else-if="loading && !items.length" class="card cf-list">
        <div v-for="n in 8" :key="n" class="cf-row cf-row--skeleton">
          <span class="skeleton cf-skeleton__logo" />
          <div class="cf-row__body">
            <span class="skeleton cf-skeleton__title" />
            <span class="skeleton cf-skeleton__line" />
            <span class="skeleton cf-skeleton__line cf-skeleton__line--short" />
          </div>
        </div>
      </div>

      <div v-else-if="!items.length" class="card card-pad cf-state">
        <AppIcon name="package" :size="30" />
        <div>
          <p class="cf-state__title">没有匹配的{{ label }}</p>
          <p class="muted small">换个关键词，或者把版本切换为「全部版本」再试。</p>
        </div>
      </div>

      <div v-else class="card cf-list">
        <button
          v-for="(project, i) in items"
          :key="`${project.source}-${project.id}`"
          class="cf-row"
          :style="{ '--i': i }"
          @click="openProject(project)"
        >
          <img
            v-if="project.logo_url"
            class="cf-row__logo"
            :src="project.logo_url"
            alt=""
            loading="lazy"
          />
          <span v-else class="cf-row__logo cf-row__logo--empty">
            {{ project.name.slice(0, 1) }}
          </span>
          <div class="cf-row__body">
            <div class="cf-row__title">
              <strong>{{ project.name }}</strong>
              <span
                v-for="category in project.categories.slice(0, 3)"
                :key="category"
                class="chip"
              >
                {{ category }}
              </span>
            </div>
            <p class="cf-row__summary">{{ project.summary }}</p>
            <div class="cf-row__meta">
              <span class="cf-row__meta-item">
                <AppIcon name="user" :size="12" />
                {{ project.author || "未知作者" }}
              </span>
              <span class="cf-row__meta-item">
                <AppIcon name="download" :size="12" />
                {{ compactNumber(project.download_count) }}
              </span>
              <span v-if="project.updated" class="cf-row__meta-item">
                <AppIcon name="refresh" :size="12" />
                {{ updatedLabel(project.updated) }}
              </span>
              <span class="cf-row__source">{{ sourceLabel(project.source) }}</span>
            </div>
          </div>
          <AppIcon name="chevron-left" :size="15" class="cf-row__chev" />
        </button>
      </div>

      <div v-if="items.length && hasMore" class="load-more">
        <WoodButton variant="ghost" size="sm" :disabled="loading" @click="loadMore">
          <AppIcon name="chevron-down" :size="14" />
          加载更多（还有 {{ Math.max(total - items.length, 0) }} 个）
        </WoodButton>
      </div>

      <p v-if="items.length" class="muted small footnote">
        共 {{ total }} 个结果；安装时会把文件写入实例的 {{ kind }} 目录。
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

/* 搜索面板 */
.cf-search {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.cf-search__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.cf-search__title {
  font-size: 15px;
  font-weight: 800;
  color: var(--walnut-deep);
}

.cf-fields {
  display: grid;
  grid-template-columns: 0.9fr 1.7fr 1fr 0.9fr 1fr;
  gap: 14px;
}

.cf-field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.cf-field__label {
  font-size: 11.5px;
  color: var(--muted);
  letter-spacing: 0.04em;
}

.cf-field__fallback {
  display: flex;
  align-items: center;
  height: 38px;
}

.cf-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.cf-actions__hint {
  margin-left: 4px;
}

/* 结果列表 */
.cf-list {
  padding: 6px;
}

.cf-row {
  position: relative;
  width: 100%;
  display: flex;
  align-items: center;
  gap: 13px;
  padding: 11px 12px;
  border-radius: 9px;
  text-align: left;
  color: var(--ink);
  transition: background var(--dur-2) ease;
  animation: rise-in 420ms var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 14ms);
}

.cf-row:hover {
  background: var(--hover);
}

.cf-row + .cf-row {
  border-top: 1px dashed var(--line);
}

.cf-row__logo {
  width: 50px;
  height: 50px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--paper-2);
}

.cf-row__logo--empty {
  display: grid;
  place-items: center;
  font-family: var(--font-display);
  font-size: 21px;
  color: var(--moss);
  background: var(--moss-soft);
}

.cf-row__body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.cf-row__title {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  flex-wrap: wrap;
}

.cf-row__title strong {
  font-size: 14.5px;
  color: var(--walnut-deep);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 46%;
}

.chip {
  font-size: 10px;
  line-height: 1;
  padding: 4px 6px;
  border-radius: 5px;
  color: var(--moss);
  background: var(--moss-soft);
  white-space: nowrap;
}

.cf-row__summary {
  font-size: 12px;
  color: var(--ink-soft);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cf-row__meta {
  display: flex;
  align-items: center;
  gap: 13px;
  flex-wrap: wrap;
  font-size: 10.5px;
  color: var(--muted);
}

.cf-row__meta-item {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.cf-row__source {
  color: var(--walnut-light);
  letter-spacing: 0.04em;
}

.cf-row__chev {
  transform: rotate(180deg);
  color: var(--line-strong);
  flex-shrink: 0;
  transition: color var(--dur-2) ease, transform var(--dur-2) var(--ease-out-expo);
}

.cf-row:hover .cf-row__chev {
  color: var(--moss);
  transform: rotate(180deg) translateX(-2px);
}

.cf-row--skeleton {
  pointer-events: none;
}

.cf-skeleton__logo {
  width: 50px;
  height: 50px;
  border-radius: 10px;
}

.cf-skeleton__title {
  width: 130px;
  height: 14px;
}

.cf-skeleton__line {
  width: 78%;
  height: 10px;
}

.cf-skeleton__line--short {
  width: 46%;
}

/* 空态 / 错误 */
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

/* 详情 */
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

.cf-detail__title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
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

@media (max-width: 1220px) {
  .cf-fields {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  }

  .cf-field--wide {
    grid-column: span 2;
  }

  .cf-row__title strong {
    max-width: 100%;
  }

  .cf-file__date {
    display: none;
  }
}
</style>
