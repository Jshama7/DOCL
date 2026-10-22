<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { api, openExternal, openFolder, type CrashReport } from "../api";
import { store } from "../store";
import AppIcon from "./AppIcon.vue";
import WoodButton from "./WoodButton.vue";
import WoodModal from "./WoodModal.vue";

const report = ref<CrashReport | null>(null);
const loading = ref(false);
const error = ref("");

const sourceLabel = computed(() => {
  switch (report.value?.source_kind) {
    case "crash-report":
      return "崩溃报告";
    case "hs-err":
      return "JVM 崩溃日志";
    case "latest-log":
      return "latest.log";
    default:
      return "无日志";
  }
});

const sourceTime = computed(() => {
  const value = report.value?.modified ?? 0;
  if (!value) return "";
  return new Date(value * 1000).toLocaleString("zh-CN", { hour12: false });
});

const exitLabel = computed(() => {
  const code = store.crashExitCode;
  return code === null ? "最近一次运行" : `退出代码 ${code}`;
});

function close() {
  store.crashOpen = false;
}

async function load() {
  const versionId = store.crashVersionId;
  if (!versionId) {
    error.value = "没有指定要分析的版本";
    return;
  }
  loading.value = true;
  error.value = "";
  try {
    report.value = await api.crashAnalyze(versionId);
  } catch (err) {
    report.value = null;
    error.value = String(err);
  } finally {
    loading.value = false;
  }
}

function openLogDir() {
  const path = report.value?.source_path;
  if (!path) return;
  const dir = path.replace(/[\\/][^\\/]+$/, "");
  if (dir) void openFolder(dir);
}

watch(
  () => [store.crashOpen, store.crashVersionId] as const,
  ([open]) => {
    if (open) void load();
  }
);

onMounted(() => {
  if (store.crashOpen) void load();
});
</script>

<template>
  <WoodModal
    v-if="store.crashOpen"
    title="崩溃分析"
    :width="600"
    @close="close"
  >
    <div v-if="loading" class="crash-loading">
      <span class="skeleton" />
      正在读取日志并匹配已知问题…
    </div>

    <div v-else-if="error" class="crash-error">
      <AppIcon name="alert" :size="16" />
      {{ error }}
      <WoodButton variant="ghost" size="sm" @click="load">重试</WoodButton>
    </div>

    <template v-else-if="report">
      <div
        class="crash-headline"
        :class="{ 'crash-headline--ok': report.found && !report.matches.length }"
      >
        <span class="crash-headline__icon">
          <AppIcon :name="report.matches.length ? 'alert' : 'check'" :size="18" />
        </span>
        <div class="crash-headline__text">
          <strong>{{ report.headline }}</strong>
          <span class="muted small">
            {{ report.version_id }} · {{ exitLabel }}
            <template v-if="sourceTime"> · {{ sourceTime }}</template>
          </span>
        </div>
        <span class="tag mono">{{ sourceLabel }}</span>
      </div>

      <p v-if="!report.found" class="crash-note">
        这个版本还没有生成日志。先启动一次游戏，退出后会自动生成
        <span class="mono">logs/latest.log</span> 或崩溃报告。
      </p>

      <div v-else class="crash-matches">
        <article v-for="item in report.matches" :key="item.id" class="crash-match">
          <header class="crash-match__head">
            <strong>{{ item.title }}</strong>
            <span class="tag">{{ item.id }}</span>
          </header>
          <p class="crash-match__summary">{{ item.summary }}</p>
          <ul class="crash-advice">
            <li v-for="(tip, index) in item.advice" :key="index">{{ tip }}</li>
          </ul>
          <div v-if="item.evidence.length" class="crash-evidence mono">
            <div v-for="(line, index) in item.evidence" :key="index">{{ line }}</div>
          </div>
          <div v-if="item.links.length" class="crash-links">
            <button
              v-for="link in item.links"
              :key="link.url"
              @click="openExternal(link.url)"
            >
              {{ link.label }}
              <AppIcon name="external" :size="11" />
            </button>
          </div>
        </article>
      </div>

      <details v-if="report.excerpt.length" class="crash-excerpt">
        <summary>日志摘录（{{ report.source }}）</summary>
        <pre class="mono">{{ report.excerpt.join("\n") }}</pre>
      </details>
    </template>

    <template #footer>
      <WoodButton
        v-if="report?.found"
        variant="ghost"
        size="sm"
        @click="openLogDir"
      >
        <AppIcon name="folder" :size="13" />
        打开日志目录
      </WoodButton>
      <WoodButton variant="soft" size="sm" :disabled="loading" @click="load">
        <AppIcon name="refresh" :size="13" />
        重新分析
      </WoodButton>
      <WoodButton variant="primary" size="sm" @click="close">关闭</WoodButton>
    </template>
  </WoodModal>
</template>

<style scoped>
.crash-loading {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 18px 0;
  color: var(--muted);
  font-size: 12.5px;
}

.crash-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: 9px;
  border: 1px solid var(--line-strong);
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 12px;
}

.crash-headline {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--line-strong);
  background: var(--danger-soft);
}

.crash-headline--ok {
  background: var(--moss-soft);
  border-color: #d3dfc8;
}

.crash-headline__icon {
  display: grid;
  place-items: center;
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.55);
  color: var(--danger);
  flex-shrink: 0;
}

.crash-headline--ok .crash-headline__icon {
  color: var(--moss);
}

.crash-headline__text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.crash-headline__text strong {
  font-size: 13px;
  font-weight: 700;
  color: var(--walnut-deep);
  overflow-wrap: anywhere;
}

.crash-note {
  margin: 14px 0 0;
  padding: 12px 14px;
  border-radius: 10px;
  border: 1px dashed var(--line-strong);
  background: var(--panel-2);
  color: var(--muted);
  font-size: 12px;
  line-height: 1.6;
}

.crash-matches {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-top: 14px;
}

.crash-match {
  padding: 12px 14px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
}

.crash-match__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.crash-match__head strong {
  font-size: 13px;
  font-weight: 700;
  color: var(--walnut-deep);
}

.crash-match__summary {
  margin: 6px 0 8px;
  font-size: 12px;
  line-height: 1.6;
  color: var(--ink-soft);
}

.crash-advice {
  margin: 0;
  padding-left: 18px;
  display: flex;
  flex-direction: column;
  gap: 3px;
  font-size: 12px;
  line-height: 1.55;
  color: var(--ink-soft);
}

.crash-advice li::marker {
  color: var(--moss);
}

.crash-evidence {
  margin-top: 9px;
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px dashed var(--line-strong);
  background: var(--paper-2);
  font-size: 10.5px;
  line-height: 1.6;
  color: var(--muted);
  max-height: 120px;
  overflow: auto;
  user-select: text;
}

.crash-evidence div {
  white-space: pre-wrap;
  word-break: break-all;
}

.crash-links {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 8px;
}

.crash-links button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11.5px;
  color: var(--moss);
  text-decoration: underline;
  padding: 0;
}

.crash-excerpt {
  margin-top: 14px;
  border-top: 1px dashed var(--line);
}

.crash-excerpt summary {
  padding-top: 10px;
  font-size: 12px;
  font-weight: 600;
  color: var(--ink-soft);
  cursor: pointer;
}

.crash-excerpt pre {
  margin: 8px 0 0;
  padding: 10px 12px;
  max-height: 240px;
  overflow: auto;
  border-radius: 10px;
  border: 1px solid var(--line);
  background: var(--paper-2);
  font-size: 10.5px;
  line-height: 1.6;
  color: var(--muted);
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
}
</style>
