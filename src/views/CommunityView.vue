<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import {
  api,
  type CfFileView,
  type CfProject,
  type InstanceContext,
  type PackPreview,
} from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodInput from "../components/WoodInput.vue";
import WoodSelect from "../components/WoodSelect.vue";
import WoodToggle from "../components/WoodToggle.vue";
import {
  importModpack,
  installCfFile,
  notify,
  openInstance,
  store,
} from "../store";
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

const showImport = ref(false);
const packPath = ref("");
const packPreview = ref<PackPreview | null>(null);
const inspecting = ref(false);
const importing = ref(false);
const importForm = reactive({
  name: "",
  id: "",
  loaderVersion: "",
  parentVersion: "",
  optional: false,
});

const label = computed(() => CLASS_LABELS[props.kind] ?? "资源");
const installable = computed(() => props.kind !== "modpacks");
const parentOptions = computed(() =>
  store.installed.map((item) => ({ value: item.id, label: item.id }))
);
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

async function pickPack() {
  if (store.installingId) {
    notify("已有下载任务正在进行", "err");
    return;
  }
  const selection = await openDialog({
    multiple: false,
    directory: false,
    title: "选择整合包压缩包",
    filters: [{ name: "整合包", extensions: ["zip", "mrpack"] }],
  });
  const path = Array.isArray(selection) ? selection[0] : selection;
  if (!path) return;
  inspecting.value = true;
  try {
    const preview = await api.modpackInspect(path);
    packPath.value = path;
    packPreview.value = preview;
    importForm.name = preview.name;
    importForm.id = preview.suggested_id;
    importForm.loaderVersion = preview.loader_version ?? "";
    const installedIds = store.installed.map((item) => item.id);
    importForm.parentVersion =
      preview.detected_parent && installedIds.includes(preview.detected_parent)
        ? preview.detected_parent
        : installedIds[0] ?? "";
    importForm.optional = false;
    showImport.value = true;
  } catch (err) {
    notify(`读取整合包失败：${String(err)}`, "err");
  } finally {
    inspecting.value = false;
  }
}

async function confirmImport() {
  if (!packPreview.value || importing.value) return;
  if (packPreview.value.format === "generic" && !importForm.parentVersion) {
    notify("请选择一个已安装的父版本", "err");
    return;
  }
  importing.value = true;
  const result = await importModpack(
    packPath.value,
    {
      instance_name: importForm.name.trim() || null,
      instance_id: importForm.id.trim() || null,
      loader_version: importForm.loaderVersion.trim() || null,
      parent_version: importForm.parentVersion || null,
      install_optional: importForm.optional,
    },
    packPreview.value.name
  );
  importing.value = false;
  if (result) {
    showImport.value = false;
    packPreview.value = null;
    openInstance(result.instance_id);
  }
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
        <WoodButton
          v-if="kind === 'modpacks'"
          variant="soft"
          :disabled="inspecting || !!store.installingId"
          @click="pickPack"
        >
          <AppIcon name="package" :size="15" />
          {{ inspecting ? "读取中…" : "导入整合包" }}
        </WoodButton>
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
    </template>

    <Teleport to="body">
      <Transition name="pack-modal">
        <div v-if="showImport && packPreview" class="pack-modal">
          <div class="pack-modal__mask" @click="!importing && (showImport = false)" />
          <div class="pack-modal__card">
            <header class="pack-modal__head">
              <div>
                <span class="eyebrow">DOCL / MODPACK</span>
                <h2 class="pack-modal__title">{{ packPreview.name }}</h2>
                <p class="muted small">
                  {{
                    packPreview.format === "curseforge"
                      ? "CurseForge 整合包"
                      : packPreview.format === "mrpack"
                        ? "Modrinth 整合包"
                        : "通用压缩包"
                  }}
                  <template v-if="packPreview.version"> · {{ packPreview.version }}</template>
                  <template v-if="packPreview.game_version"> · {{ packPreview.game_version }}</template>
                </p>
              </div>
              <IconButton
                icon="close"
                label="关闭"
                :size="32"
                :disabled="importing"
                @click="showImport = false"
              />
            </header>

            <div class="pack-modal__body">
              <div class="pack-summary">
                <span class="tag">{{ packPreview.file_count }} 个文件</span>
                <span v-if="packPreview.download_bytes" class="tag">
                  下载 {{ formatBytes(packPreview.download_bytes) }}
                </span>
                <span v-if="packPreview.override_bytes" class="tag">
                  覆盖文件 {{ formatBytes(packPreview.override_bytes) }}
                </span>
                <span class="tag">
                  {{
                    packPreview.loader === "vanilla"
                      ? "原版"
                      : packPreview.loader.charAt(0).toUpperCase() + packPreview.loader.slice(1)
                  }}
                  <template v-if="packPreview.loader_version">
                    {{ packPreview.loader_version }}
                  </template>
                </span>
              </div>

              <label class="pack-field">
                <span class="pack-field__label">实例名称</span>
                <WoodInput v-model="importForm.name" placeholder="显示在版本列表里的名字" />
              </label>

              <label class="pack-field">
                <span class="pack-field__label">实例 ID</span>
                <WoodInput v-model="importForm.id" mono placeholder="用于版本目录名" />
              </label>

              <label v-if="packPreview.loader !== 'vanilla'" class="pack-field">
                <span class="pack-field__label">加载器版本</span>
                <WoodInput v-model="importForm.loaderVersion" mono placeholder="留空自动选择" />
              </label>

              <label v-if="packPreview.format === 'generic'" class="pack-field">
                <span class="pack-field__label">父版本</span>
                <WoodSelect
                  v-if="parentOptions.length"
                  v-model="importForm.parentVersion"
                  :options="parentOptions"
                />
                <span v-else class="muted small pack-field__empty">
                  还没有已安装版本，请先去原版游戏安装一个。
                </span>
              </label>

              <div v-if="packPreview.format !== 'generic'" class="pack-toggle">
                <div>
                  <strong>安装可选文件</strong>
                  <span>清单中标记为可选的 Mod，体积可能较大</span>
                </div>
                <WoodToggle v-model="importForm.optional" />
              </div>

              <ul v-if="packPreview.warnings.length" class="pack-warnings">
                <li v-for="warning in packPreview.warnings" :key="warning">{{ warning }}</li>
              </ul>
            </div>

            <footer class="pack-modal__foot">
              <span class="muted small">
                导入会自动安装基础版本与加载器，并写入独立的实例目录。
              </span>
              <div class="row">
                <WoodButton variant="ghost" :disabled="importing" @click="showImport = false">
                  取消
                </WoodButton>
                <WoodButton
                  variant="primary"
                  sheen
                  :disabled="importing || (packPreview.format === 'generic' && !importForm.parentVersion)"
                  @click="confirmImport"
                >
                  <AppIcon name="download" :size="15" />
                  {{ importing ? "导入中…" : "开始导入" }}
                </WoodButton>
              </div>
            </footer>
          </div>
        </div>
      </Transition>
    </Teleport>
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

/* 导入整合包弹层 */
.pack-modal {
  position: fixed;
  inset: 0;
  z-index: 140;
  display: grid;
  place-items: center;
}

.pack-modal__mask {
  position: absolute;
  inset: 0;
  background: rgba(28, 20, 12, 0.38);
  backdrop-filter: blur(3px);
}

.pack-modal__card {
  position: relative;
  width: min(560px, calc(100vw - 56px));
  max-height: calc(100vh - 72px);
  overflow-y: auto;
  padding: 20px 22px;
  border-radius: 16px;
  border: 1px solid var(--line-strong);
  background: rgba(var(--glass-rgb), 0.97);
  box-shadow: var(--shadow-2);
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.pack-modal__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
}

.pack-modal__title {
  font-family: var(--font-display);
  font-size: 28px;
  line-height: 1.1;
  color: var(--walnut-deep);
  margin: 2px 0;
}

.pack-modal__body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.pack-summary {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.pack-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.pack-field__label {
  font-size: 11.5px;
  color: var(--muted);
  letter-spacing: 0.04em;
}

.pack-field__empty {
  display: flex;
  align-items: center;
  height: 38px;
}

.pack-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0;
  border-top: 1px dashed var(--line);
  border-bottom: 1px dashed var(--line);
}

.pack-toggle strong {
  display: block;
  font-size: 13px;
  color: var(--walnut-deep);
}

.pack-toggle span {
  font-size: 11px;
  color: var(--muted);
}

.pack-warnings {
  margin: 0;
  padding: 8px 12px 8px 26px;
  border-radius: 8px;
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--ink-soft);
  font-size: 11.5px;
  line-height: 1.6;
}

.pack-modal__foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding-top: 10px;
  border-top: 1px dashed var(--line);
}

.pack-modal__foot > .muted {
  max-width: 260px;
  line-height: 1.5;
}

.pack-modal-enter-active,
.pack-modal-leave-active {
  transition: opacity 180ms ease;
}

.pack-modal-enter-active .pack-modal__card,
.pack-modal-leave-active .pack-modal__card {
  transition: transform 220ms var(--ease-out-expo);
}

.pack-modal-enter-from,
.pack-modal-leave-to {
  opacity: 0;
}

.pack-modal-enter-from .pack-modal__card,
.pack-modal-leave-to .pack-modal__card {
  transform: translateY(10px) scale(0.98);
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
