<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { notify, openCrashDialog, store } from "../store";

type Filter = "all" | "stdout" | "stderr" | "system";

const filters: { value: Filter; label: string }[] = [
  { value: "all", label: "全部" },
  { value: "stdout", label: "输出" },
  { value: "stderr", label: "错误" },
  { value: "system", label: "系统" },
];

const filter = ref<Filter>("all");
const keyword = ref("");
const follow = ref(true);
const view = ref<HTMLElement | null>(null);

const counts = computed(() => {
  const result = { all: store.log.length, stdout: 0, stderr: 0, system: 0 };
  for (const item of store.log) {
    if (item.stream === "stderr") result.stderr += 1;
    else if (item.stream === "system") result.system += 1;
    else result.stdout += 1;
  }
  return result;
});

const lines = computed(() => {
  const word = keyword.value.trim().toLowerCase();
  return store.log
    .map((item, index) => ({ ...item, index: index + 1 }))
    .filter((item) => filter.value === "all" || item.stream === filter.value)
    .filter((item) => !word || item.line.toLowerCase().includes(word));
});

async function stick() {
  if (!follow.value) return;
  await nextTick();
  const element = view.value;
  if (element) element.scrollTop = element.scrollHeight;
}

function onScroll() {
  const element = view.value;
  if (!element) return;
  follow.value = element.scrollHeight - element.scrollTop - element.clientHeight < 40;
}

function close() {
  store.logPanel = false;
}

function clear() {
  store.log = [];
  notify("已清空运行日志");
}

async function copyAll() {
  if (!store.log.length) return;
  try {
    await navigator.clipboard.writeText(store.log.map((item) => item.line).join("\n"));
    notify(`已复制 ${store.log.length} 行日志`);
  } catch (err) {
    notify(`复制失败：${String(err)}`, "err");
  }
}

function onKey(event: KeyboardEvent) {
  if (event.key === "Escape" && store.logPanel) close();
}

watch(() => store.log.length, stick);
watch([filter, keyword], stick);
watch(
  () => store.logPanel,
  (open) => {
    if (open) void stick();
  }
);

onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));
</script>

<template>
  <teleport to="body">
    <transition name="log-fade">
      <div v-if="store.logPanel" class="log-overlay" @mousedown.self="close">
        <section class="log-panel" role="dialog" aria-label="运行日志">
          <header class="log-head">
            <span class="log-head__title">运行日志</span>
            <span class="log-head__count mono">{{ store.log.length }} 行</span>
            <span class="log-head__spacer" />
            <button class="log-tool" @click="openCrashDialog()">崩溃分析</button>
            <button class="log-icon" title="复制全部日志" aria-label="复制全部日志" @click="copyAll">
              <AppIcon name="copy" :size="15" />
            </button>
            <button class="log-icon" title="清空日志" aria-label="清空日志" @click="clear">
              <AppIcon name="trash" :size="15" />
            </button>
            <button class="log-icon" title="关闭" aria-label="关闭日志" @click="close">
              <AppIcon name="close" :size="15" />
            </button>
          </header>

          <div class="log-bar">
            <div class="log-tabs">
              <button
                v-for="item in filters"
                :key="item.value"
                class="log-tab"
                :class="{ 'log-tab--active': filter === item.value }"
                @click="filter = item.value"
              >
                {{ item.label }}
                <em>{{ counts[item.value] }}</em>
              </button>
            </div>

            <label class="log-search">
              <AppIcon name="search" :size="14" />
              <input
                v-model="keyword"
                type="text"
                placeholder="过滤关键字"
                spellcheck="false"
              />
            </label>

            <button
              class="log-follow"
              :class="{ 'log-follow--on': follow }"
              @click="follow = !follow"
            >
              <span class="log-follow__dot" />
              自动跟随
            </button>
          </div>

          <div ref="view" class="log-view mono" @scroll="onScroll">
            <div v-if="!lines.length" class="log-empty">
              <span class="log-empty__title">
                {{ store.log.length ? "没有匹配的日志行" : "还没有日志" }}
              </span>
              <span class="log-empty__hint">
                {{
                  store.log.length
                    ? "换个关键字或切回「全部」试试"
                    : "启动游戏或安装版本后，这里会实时显示进程输出"
                }}
              </span>
            </div>

            <div
              v-for="item in lines"
              :key="item.index"
              class="log-line"
              :class="`log-line--${item.stream}`"
            >
              <span class="log-line__no">{{ String(item.index).padStart(4, "0") }}</span>
              <span class="log-line__mark">{{
                item.stream === "stderr" ? "!" : item.stream === "system" ? "›" : "·"
              }}</span>
              <span class="log-line__text">{{ item.line }}</span>
            </div>
          </div>

          <footer class="log-foot">
            <span>{{ lines.length }} / {{ store.log.length }} 行</span>
            <span class="log-foot__state" :class="{ 'log-foot__state--live': store.gameRunning }">
              <span class="log-foot__dot" />
              {{ store.gameRunning ? "游戏运行中" : "进程未运行" }}
            </span>
          </footer>
        </section>
      </div>
    </transition>
  </teleport>
</template>

<style scoped>
.log-overlay {
  position: fixed;
  inset: 0;
  z-index: 96;
  display: grid;
  place-items: center;
  background: rgba(20, 16, 12, 0.45);
  backdrop-filter: blur(4px);
}

/* 面板材质跟随当前主题：浅色主题是纸张，深色主题自动变暗 */
.log-panel {
  width: min(900px, 92vw);
  height: min(620px, 80vh);
  display: flex;
  flex-direction: column;
  border: 1px solid var(--line-strong);
  border-radius: 4px;
  background: var(--panel);
  box-shadow: var(--shadow-3);
  overflow: hidden;
}

.log-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  height: 46px;
  padding: 0 12px 0 16px;
  border-bottom: 1px solid var(--line);
  background: var(--panel-2);
}

.log-head__title {
  font-size: 14px;
  font-weight: 700;
  color: var(--walnut-deep);
}

.log-head__count {
  font-size: 10.5px;
  color: var(--muted);
}

.log-head__spacer {
  flex: 1;
}

.log-tool {
  height: 27px;
  padding: 0 10px;
  border: 1px solid var(--line);
  border-radius: 3px;
  color: var(--ink-soft);
  font-size: 11.5px;
  transition: border-color var(--dur-1) ease, color var(--dur-1) ease;
}

.log-tool:hover {
  border-color: var(--line-strong);
  color: var(--walnut-deep);
}

.log-icon {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 3px;
  color: var(--muted);
  transition: background var(--dur-1) ease, color var(--dur-1) ease;
}

.log-icon:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--walnut-deep);
}

.log-bar {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-shrink: 0;
  height: 44px;
  padding: 0 16px;
  border-bottom: 1px solid var(--line);
  background: var(--panel);
}

.log-tabs {
  display: flex;
  align-items: stretch;
  gap: 14px;
  height: 100%;
}

.log-tab {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--muted);
  font-size: 12px;
  font-weight: 600;
  transition: color var(--dur-1) ease;
}

.log-tab:hover {
  color: var(--ink);
}

.log-tab em {
  font-style: normal;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--muted);
}

.log-tab--active {
  color: var(--walnut-deep);
}

.log-tab--active::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: -1px;
  height: 2px;
  background: var(--moss);
}

.log-tab--active em {
  color: var(--moss);
}

.log-search {
  display: flex;
  align-items: center;
  gap: 7px;
  flex: 1;
  min-width: 120px;
  height: 28px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 3px;
  background: var(--panel-2);
  color: var(--muted);
}

.log-search:focus-within {
  border-color: var(--line-strong);
}

.log-search input {
  flex: 1;
  min-width: 0;
  border: none;
  outline: none;
  background: none;
  font-size: 12px;
  color: var(--ink);
}

.log-search input::placeholder {
  color: var(--muted);
}

.log-follow {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 3px;
  color: var(--muted);
  font-size: 11px;
  font-weight: 600;
  transition: border-color var(--dur-1) ease, color var(--dur-1) ease;
}

.log-follow:hover {
  border-color: var(--line-strong);
  color: var(--ink);
}

.log-follow__dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--line-strong);
}

.log-follow--on {
  border-color: rgba(var(--accent-rgb), 0.4);
  color: var(--moss);
}

.log-follow--on .log-follow__dot {
  background: var(--moss);
}

.log-view {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 8px 0 10px;
  background: var(--paper-2);
  font-size: 11.8px;
  line-height: 1.8;
  color: var(--ink-soft);
  scrollbar-color: rgba(var(--accent-rgb), 0.2) transparent;
}

.log-view::-webkit-scrollbar-thumb {
  background: rgba(var(--accent-rgb), 0.18);
}

.log-view:hover::-webkit-scrollbar-thumb {
  background: rgba(var(--accent-rgb), 0.3);
}

.log-line {
  display: flex;
  gap: 8px;
  padding: 0 16px;
}

.log-line:hover {
  background: rgba(var(--accent-rgb), 0.06);
}

.log-line__no {
  flex-shrink: 0;
  width: 34px;
  text-align: right;
  color: var(--line-strong);
  user-select: none;
}

.log-line__mark {
  flex-shrink: 0;
  width: 8px;
  text-align: center;
  color: var(--line-strong);
  user-select: none;
}

.log-line__text {
  flex: 1;
  min-width: 0;
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
}

.log-line--stderr .log-line__text,
.log-line--stderr .log-line__mark {
  color: var(--danger);
}

.log-line--system .log-line__text,
.log-line--system .log-line__mark {
  color: var(--moss);
  font-weight: 700;
}

.log-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 72px 20px;
  text-align: center;
}

.log-empty__title {
  color: var(--ink-soft);
  font-size: 13px;
  font-weight: 700;
}

.log-empty__hint {
  color: var(--muted);
  font-size: 11.5px;
}

.log-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  height: 32px;
  padding: 0 16px;
  border-top: 1px solid var(--line);
  background: var(--panel);
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: 10.5px;
  letter-spacing: 0.04em;
}

.log-foot__state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.log-foot__dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--line-strong);
}

.log-foot__state--live .log-foot__dot {
  background: var(--moss);
  box-shadow: 0 0 6px rgba(var(--accent-rgb), 0.55);
}

.log-fade-enter-active,
.log-fade-leave-active {
  transition: opacity 170ms ease;
}

.log-fade-enter-from,
.log-fade-leave-to {
  opacity: 0;
}
</style>
