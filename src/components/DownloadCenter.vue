<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import IconButton from "./IconButton.vue";
import WoodButton from "./WoodButton.vue";
import WoodProgress from "./WoodProgress.vue";
import { cancelInstall, store } from "../store";
import { formatBytes, formatDuration, formatSpeed } from "../utils";

const hovered = ref(false);
let hoverTimer: number | undefined;
let hideTimer: number | undefined;

const snapshot = computed(() => store.progress ?? store.lastProgress);
const active = computed(() => !!store.installingId);
const failed = computed(() => !!store.lastJobError);
const finished = computed(() => !active.value && !failed.value && !!store.lastProgress);

const percent = computed(() => {
  const p = snapshot.value;
  if (!p) return finished.value ? 100 : 0;
  if (!active.value && !failed.value) return 100;
  if (p.bytes_total > 0) return Math.min(100, (p.bytes_done / p.bytes_total) * 100);
  if (p.files_total > 0) return Math.min(100, (p.files_done / p.files_total) * 100);
  return 0;
});

const speed = computed(() => {
  const p = snapshot.value;
  if (!p) return "—";
  return formatSpeed(p.bytes_done, p.elapsed_ms);
});

const eta = computed(() => {
  const p = snapshot.value;
  if (!p || p.bytes_total <= p.bytes_done) return active.value ? "计算中…" : "—";
  const elapsed = Math.max(1, p.elapsed_ms);
  const bytesPerMs = p.bytes_done / elapsed;
  if (bytesPerMs <= 0) return "计算中…";
  return formatDuration((p.bytes_total - p.bytes_done) / bytesPerMs);
});

const stageLabel = computed(() => {
  const p = snapshot.value;
  if (!p) return "";
  if (failed.value) return "安装失败";
  if (!active.value) return "安装完成";
  if (p.stage === "java") return "安装 Java 运行时";
  if (p.stage === "install") return "安装游戏文件";
  return p.stage;
});

const title = computed(
  () => store.installingId || store.lastJobTitle || "下载任务"
);

const ringStyle = computed(() => ({
  background: `conic-gradient(var(--moss) ${percent.value * 3.6}deg, rgba(var(--glass-rgb), 0.28) 0)`,
}));

function closePanel() {
  store.downloadPanel = active.value || !!store.lastProgress ? "ball" : "hidden";
}

function openPanel() {
  store.downloadPanel = "open";
  hovered.value = false;
}

function hideBall() {
  if (!active.value) store.downloadPanel = "hidden";
}

function showTip() {
  if (hoverTimer) window.clearTimeout(hoverTimer);
  hoverTimer = window.setTimeout(() => {
    hovered.value = true;
  }, 300);
}

function hideTip() {
  if (hoverTimer) window.clearTimeout(hoverTimer);
  hovered.value = false;
}

watch(
  () => store.downloadPanel,
  (mode) => {
    if (hideTimer) window.clearTimeout(hideTimer);
    if (mode === "ball" && !active.value) {
      hideTimer = window.setTimeout(() => {
        if (!active.value) store.downloadPanel = "hidden";
      }, 6000);
    }
  }
);

watch(
  () => store.installingId,
  (value, old) => {
    if (old && !value) {
      if (hideTimer) window.clearTimeout(hideTimer);
      hideTimer = window.setTimeout(() => {
        if (store.downloadPanel === "ball") store.downloadPanel = "hidden";
      }, 6000);
    }
  }
);

onBeforeUnmount(() => {
  if (hoverTimer) window.clearTimeout(hoverTimer);
  if (hideTimer) window.clearTimeout(hideTimer);
});
</script>

<template>
  <teleport to="body">
    <transition name="dl-fade">
      <div
        v-if="store.downloadPanel === 'open' && snapshot"
        class="dl-overlay"
        @mousedown.self="closePanel"
      >
        <section class="dl-panel">
          <header class="dl-panel__head">
            <div>
              <span class="eyebrow">DOCL / DOWNLOAD</span>
              <h3 class="dl-panel__title">{{ title }}</h3>
            </div>
            <div class="row">
              <span
                class="tag"
                :class="{ 'tag--grass': !failed, 'tag--danger': failed }"
              >
                {{ stageLabel }}
              </span>
              <IconButton icon="minimize" label="收起到右下角" :size="30" @click="closePanel" />
            </div>
          </header>

          <div class="dl-panel__progress">
            <div class="row-between">
              <strong class="dl-percent">{{ Math.round(percent) }}%</strong>
              <span class="muted small">{{ snapshot?.message }}</span>
            </div>
            <WoodProgress :value="percent" :height="9" />
          </div>

          <div class="dl-stats">
            <div class="dl-stat">
              <span>已下载</span>
              <strong>{{ formatBytes(snapshot?.bytes_done ?? 0) }}</strong>
            </div>
            <div class="dl-stat">
              <span>总大小</span>
              <strong>{{ formatBytes(snapshot?.bytes_total ?? 0) }}</strong>
            </div>
            <div class="dl-stat">
              <span>文件</span>
              <strong>{{ snapshot?.files_done ?? 0 }} / {{ snapshot?.files_total ?? 0 }}</strong>
            </div>
            <div class="dl-stat">
              <span>速度</span>
              <strong>{{ speed }}</strong>
            </div>
            <div class="dl-stat">
              <span>已用时</span>
              <strong>{{ formatDuration(snapshot?.elapsed_ms ?? 0) }}</strong>
            </div>
            <div class="dl-stat">
              <span>剩余</span>
              <strong>{{ eta }}</strong>
            </div>
          </div>

          <p v-if="failed" class="dl-error">{{ store.lastJobError }}</p>

          <footer class="dl-panel__foot">
            <WoodButton v-if="active" variant="danger" size="sm" @click="cancelInstall">
              取消安装
            </WoodButton>
            <WoodButton variant="ghost" size="sm" @click="closePanel">
              收起到悬浮球
            </WoodButton>
          </footer>
        </section>
      </div>
    </transition>

    <div v-if="store.downloadPanel === 'ball' && snapshot" class="dl-ball-wrap">
      <transition name="dl-tip">
        <div v-if="hovered" class="dl-tip">
          <div class="dl-tip__head">
            <strong>{{ title }}</strong>
            <span class="tag" :class="{ 'tag--grass': !failed, 'tag--danger': failed }">
              {{ stageLabel }}
            </span>
          </div>
          <div class="dl-tip__bar">
            <span :style="{ width: `${percent}%` }" />
          </div>
          <div class="dl-tip__rows">
            <span>{{ Math.round(percent) }}%</span>
            <span>{{ speed }}</span>
            <span>{{ formatBytes(snapshot?.bytes_done ?? 0) }} / {{ formatBytes(snapshot?.bytes_total ?? 0) }}</span>
            <span>剩余 {{ eta }}</span>
          </div>
          <p v-if="failed" class="dl-tip__err">{{ store.lastJobError }}</p>
          <button class="dl-tip__open" @click="openPanel">打开下载详情</button>
        </div>
      </transition>

      <button
        class="dl-ball"
        :class="{ 'dl-ball--done': finished && !failed, 'dl-ball--failed': failed }"
        :style="ringStyle"
        aria-label="下载进度"
        @mouseenter="showTip"
        @mouseleave="hideTip"
        @click="openPanel"
      >
        <span class="dl-ball__inner">
          <template v-if="failed">!</template>
          <template v-else-if="finished">✓</template>
          <template v-else>{{ Math.round(percent) }}</template>
        </span>
      </button>

      <button
        v-if="finished || failed"
        class="dl-ball__dismiss"
        title="关闭"
        @click="hideBall"
      >
        ×
      </button>
    </div>
  </teleport>
</template>

<style scoped>
.dl-overlay {
  position: fixed;
  inset: 0;
  z-index: 95;
  display: grid;
  place-items: center;
  background: rgba(40, 28, 16, 0.34);
  backdrop-filter: blur(4px);
  animation: dl-fade-in 180ms ease both;
}

.dl-panel {
  width: min(560px, 90vw);
  background: var(--panel);
  border: 1px solid var(--line-strong);
  border-radius: 16px;
  box-shadow: 0 30px 70px rgba(40, 28, 16, 0.32);
  padding: 20px 22px 16px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  position: relative;
  overflow: hidden;
  animation: dl-pop 280ms var(--ease-out-expo) both;
}

.dl-panel::before {
  content: "";
  position: absolute;
  inset: 0 0 auto;
  height: 4px;
  background: linear-gradient(90deg, var(--moss), var(--moss-bright), var(--amber));
}

.dl-panel__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 14px;
}

.dl-panel__title {
  font-size: 20px;
  font-weight: 800;
  color: var(--walnut-deep);
  margin-top: 2px;
}

.dl-panel__progress {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dl-percent {
  font-family: var(--font-display);
  font-size: 22px;
  color: var(--walnut-deep);
}

.dl-stats {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 10px;
}

.dl-stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 9px 11px;
  border: 1px solid var(--line);
  border-radius: 9px;
  background: var(--panel-2);
}

.dl-stat span {
  font-size: 10px;
  letter-spacing: 0.1em;
  color: var(--muted);
  text-transform: uppercase;
  font-family: var(--font-mono);
}

.dl-stat strong {
  font-size: 12.5px;
  color: var(--ink);
}

.dl-error {
  padding: 9px 11px;
  border-radius: 8px;
  background: var(--danger-soft);
  color: var(--danger);
  font-size: 12px;
}

.dl-panel__foot {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding-top: 4px;
}

/* 悬浮球 */
.dl-ball-wrap {
  position: fixed;
  right: 24px;
  bottom: calc(var(--statusbar) + 20px);
  z-index: 92;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 10px;
}

.dl-ball {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  padding: 4px;
  box-shadow: 0 10px 26px rgba(40, 28, 16, 0.28);
  transition: transform var(--dur-2) var(--ease-out-expo);
}

.dl-ball:hover {
  transform: translateY(-3px) scale(1.04);
}

.dl-ball__inner {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  display: grid;
  place-items: center;
  background: var(--panel);
  color: var(--walnut-deep);
  font-family: var(--font-display);
  font-size: 15px;
  font-weight: 700;
  border: 1px solid var(--line);
}

.dl-ball--done .dl-ball__inner {
  color: var(--moss);
}

.dl-ball--failed .dl-ball__inner {
  color: var(--danger);
}

.dl-ball__dismiss {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  background: var(--panel);
  border: 1px solid var(--line);
  color: var(--muted);
  font-size: 14px;
  line-height: 1;
  display: grid;
  place-items: center;
  margin-right: 2px;
}

.dl-ball__dismiss:hover {
  color: var(--danger);
  border-color: var(--danger);
}

.dl-tip {
  width: 260px;
  padding: 12px 13px;
  border-radius: 11px;
  background: #3c2b1c;
  color: #fbf4e6;
  box-shadow: 0 14px 34px rgba(40, 28, 16, 0.32);
  border: 1px solid rgba(255, 253, 246, 0.08);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dl-tip__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  font-size: 12.5px;
}

.dl-tip__bar {
  height: 5px;
  border-radius: 99px;
  background: rgba(255, 253, 246, 0.18);
  overflow: hidden;
}

.dl-tip__bar span {
  display: block;
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, #88a867, #c08a55);
  transition: width 240ms var(--ease-out-quart);
}

.dl-tip__rows {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 12px;
  font-size: 11px;
  color: #d8c8b0;
}

.dl-tip__err {
  font-size: 11px;
  color: #f0a58d;
}

.dl-tip__open {
  height: 28px;
  border-radius: 7px;
  background: rgba(255, 253, 246, 0.1);
  color: #fbf4e6;
  font-size: 11.5px;
  border: 1px solid rgba(255, 253, 246, 0.12);
}

.dl-tip__open:hover {
  background: rgba(255, 253, 246, 0.18);
}

@keyframes dl-fade-in {
  from {
    opacity: 0;
  }
}

@keyframes dl-pop {
  from {
    opacity: 0;
    transform: translateY(16px) scale(0.97);
  }
}

.dl-fade-enter-active,
.dl-fade-leave-active {
  transition: opacity 160ms ease;
}

.dl-fade-enter-from,
.dl-fade-leave-to {
  opacity: 0;
}

.dl-tip-enter-active,
.dl-tip-leave-active {
  transition: opacity 160ms ease, transform 160ms var(--ease-out-expo);
}

.dl-tip-enter-from,
.dl-tip-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
