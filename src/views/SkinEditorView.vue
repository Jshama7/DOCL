<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import { api, saveSkinDialog } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import SkinModel from "../components/SkinModel.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodModal from "../components/WoodModal.vue";
import type { AccountView } from "../api";
import { canvasFromSource, createDefaultSkin, seedFromString, SKIN_SIZE } from "../skin";
import { loadSelectedSkin, notify, refreshAccounts, store } from "../store";

const props = defineProps<{ account: AccountView }>();
const emit = defineEmits<{ (e: "close"): void }>();

const drawCanvas = ref<HTMLCanvasElement | null>(null);
const guideCanvas = ref<HTMLCanvasElement | null>(null);

const tool = ref<"brush" | "eraser" | "fill" | "picker">("brush");
const color = ref("#5e7b49");
const brushSize = ref(1);
const model = ref("classic");
const part = ref<"head" | "body" | "arms" | "legs">("head");
const showGuides = ref(true);
const zoom = ref(1);
const previewSkin = ref("");
const saving = ref(false);
const uploadOpen = ref(false);
const uploading = ref(false);
const canUndo = ref(false);
const canRedo = ref(false);

const palette = [
  "#2b2118", "#4a3220", "#7a5230", "#c08a55",
  "#e3b48a", "#f0c9a2", "#9c6a52", "#a8543c",
  "#5e7b49", "#88a867", "#47623a", "#3f4a58",
  "#243a5e", "#c9a26a", "#f4ebdd", "#ffffff",
];

const guideRects = computed<Record<string, [number, number, number, number][]>>(() => ({
  head: [[0, 0, 32, 16]],
  body: [[16, 16, 24, 16]],
  arms: [
    [32, 16, 16, 16],
    [32, 48, 16, 16],
  ],
  legs: [
    [0, 16, 16, 16],
    [16, 48, 16, 16],
  ],
}));

const canvasWidth = computed(() => 64 * 8 * zoom.value);
const isMicrosoft = computed(() => props.account.kind === "microsoft");

let ctx: CanvasRenderingContext2D | null = null;
let history: ImageData[] = [];
let cursor = -1;
let drawing = false;
let previewTimer: number | undefined;

function pushHistory() {
  if (!ctx) return;
  history = history.slice(0, cursor + 1);
  history.push(ctx.getImageData(0, 0, SKIN_SIZE, SKIN_SIZE));
  if (history.length > 40) history.shift();
  cursor = history.length - 1;
  canUndo.value = cursor > 0;
  canRedo.value = false;
}

function undo() {
  if (!ctx || cursor <= 0) return;
  cursor -= 1;
  ctx.putImageData(history[cursor], 0, 0);
  canUndo.value = cursor > 0;
  canRedo.value = true;
  schedulePreview();
}

function redo() {
  if (!ctx || cursor >= history.length - 1) return;
  cursor += 1;
  ctx.putImageData(history[cursor], 0, 0);
  canUndo.value = true;
  canRedo.value = cursor < history.length - 1;
  schedulePreview();
}

function schedulePreview() {
  if (previewTimer) window.clearTimeout(previewTimer);
  previewTimer = window.setTimeout(() => {
    const canvas = drawCanvas.value;
    if (canvas) previewSkin.value = canvas.toDataURL("image/png");
  }, 120);
}

function drawGuides() {
  const overlay = guideCanvas.value;
  if (!overlay) return;
  const g = overlay.getContext("2d");
  if (!g) return;
  g.clearRect(0, 0, SKIN_SIZE, SKIN_SIZE);
  if (!showGuides.value) return;
  g.save();
  g.setLineDash([2, 2]);
  g.lineWidth = 0.6;
  g.strokeStyle = "rgba(255, 200, 90, 0.9)";
  for (const [x, y, w, h] of guideRects.value[part.value]) {
    g.strokeRect(x + 0.3, y + 0.3, w - 0.6, h - 0.6);
  }
  g.restore();
}

function pointerPos(event: PointerEvent) {
  const canvas = drawCanvas.value;
  if (!canvas) return null;
  const rect = canvas.getBoundingClientRect();
  const x = Math.floor(((event.clientX - rect.left) / rect.width) * SKIN_SIZE);
  const y = Math.floor(((event.clientY - rect.top) / rect.height) * SKIN_SIZE);
  if (x < 0 || y < 0 || x >= SKIN_SIZE || y >= SKIN_SIZE) return null;
  return { x, y };
}

function floodFill(x: number, y: number, fillColor: string) {
  if (!ctx) return;
  const image = ctx.getImageData(0, 0, SKIN_SIZE, SKIN_SIZE);
  const data = image.data;
  const index = (px: number, py: number) => (py * SKIN_SIZE + px) * 4;
  const start = index(x, y);
  const target = [data[start], data[start + 1], data[start + 2], data[start + 3]];
  const hex = fillColor.replace("#", "");
  const fill = [
    parseInt(hex.slice(0, 2), 16),
    parseInt(hex.slice(2, 4), 16),
    parseInt(hex.slice(4, 6), 16),
    255,
  ];
  if (target.every((v, i) => v === fill[i])) return;
  const stack: [number, number][] = [[x, y]];
  while (stack.length) {
    const [px, py] = stack.pop()!;
    if (px < 0 || py < 0 || px >= SKIN_SIZE || py >= SKIN_SIZE) continue;
    const i = index(px, py);
    if (
      data[i] !== target[0] ||
      data[i + 1] !== target[1] ||
      data[i + 2] !== target[2] ||
      data[i + 3] !== target[3]
    ) {
      continue;
    }
    data[i] = fill[0];
    data[i + 1] = fill[1];
    data[i + 2] = fill[2];
    data[i + 3] = fill[3];
    stack.push([px + 1, py], [px - 1, py], [px, py + 1], [px, py - 1]);
  }
  ctx.putImageData(image, 0, 0);
}

function paintAt(x: number, y: number) {
  if (!ctx) return;
  if (tool.value === "picker") {
    const pixel = ctx.getImageData(x, y, 1, 1).data;
    const hex = `#${[pixel[0], pixel[1], pixel[2]]
      .map((v) => v.toString(16).padStart(2, "0"))
      .join("")}`;
    color.value = hex;
    tool.value = "brush";
    return;
  }
  if (tool.value === "fill") {
    floodFill(x, y, color.value);
    return;
  }
  const size = brushSize.value;
  const offset = Math.floor(size / 2);
  if (tool.value === "eraser") {
    ctx.clearRect(x - offset, y - offset, size, size);
  } else {
    ctx.fillStyle = color.value;
    ctx.fillRect(x - offset, y - offset, size, size);
  }
}

function onPointerDown(event: PointerEvent) {
  const pos = pointerPos(event);
  if (!pos) return;
  if (tool.value === "fill" || tool.value === "picker") {
    paintAt(pos.x, pos.y);
    pushHistory();
    schedulePreview();
    return;
  }
  drawing = true;
  (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  paintAt(pos.x, pos.y);
}

function onPointerMove(event: PointerEvent) {
  if (!drawing) return;
  const pos = pointerPos(event);
  if (!pos) return;
  paintAt(pos.x, pos.y);
}

function onPointerUp() {
  if (!drawing) return;
  drawing = false;
  pushHistory();
  schedulePreview();
}

async function loadSource() {
  const source = store.accountSkin?.data_url;
  let base: HTMLCanvasElement;
  if (source) {
    try {
      base = await canvasFromSource(source);
    } catch {
      base = createDefaultSkin(seedFromString(props.account.uuid));
    }
  } else {
    base = createDefaultSkin(seedFromString(props.account.uuid));
  }
  const canvas = drawCanvas.value;
  if (!canvas) return;
  ctx = canvas.getContext("2d");
  if (!ctx) return;
  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, SKIN_SIZE, SKIN_SIZE);
  ctx.drawImage(base, 0, 0);
  history = [];
  cursor = -1;
  pushHistory();
  previewSkin.value = base.toDataURL("image/png");
  model.value = store.accountSkin?.model ?? props.account.skin_model ?? "classic";
  drawGuides();
}

function currentPng() {
  return drawCanvas.value?.toDataURL("image/png") ?? "";
}

async function saveLocal() {
  if (saving.value) return;
  saving.value = true;
  try {
    await api.saveAccountSkin(props.account.id, currentPng(), model.value);
    await refreshAccounts();
    await loadSelectedSkin();
    notify("皮肤已保存到本地");
  } catch (err) {
    notify(`保存皮肤失败：${String(err)}`, "err");
  } finally {
    saving.value = false;
  }
}

async function exportPng() {
  const path = await saveSkinDialog(`${props.account.name}-skin.png`);
  if (!path) return;
  try {
    await api.saveAccountSkin(props.account.id, currentPng(), model.value);
    await api.exportAccountSkin(props.account.id, path);
    await refreshAccounts();
    await loadSelectedSkin();
    notify("皮肤已导出");
  } catch (err) {
    notify(`导出失败：${String(err)}`, "err");
  }
}

async function confirmUpload() {
  if (uploading.value) return;
  uploading.value = true;
  try {
    await api.saveAccountSkin(props.account.id, currentPng(), model.value);
    await api.uploadAccountSkin(props.account.id, currentPng(), model.value);
    await refreshAccounts();
    await loadSelectedSkin();
    uploadOpen.value = false;
    notify("皮肤已上传到 Mojang");
  } catch (err) {
    notify(`上传失败：${String(err)}`, "err");
  } finally {
    uploading.value = false;
  }
}

function setTool(value: typeof tool.value) {
  tool.value = value;
}

function selectPart(value: typeof part.value) {
  part.value = value;
  nextTick(drawGuides);
}

function resetSkin() {
  const base = createDefaultSkin(seedFromString(props.account.uuid));
  if (!ctx) return;
  ctx.clearRect(0, 0, SKIN_SIZE, SKIN_SIZE);
  ctx.drawImage(base, 0, 0);
  pushHistory();
  schedulePreview();
}

onMounted(async () => {
  await nextTick();
  await loadSource();
});

const tools = [
  { value: "brush", label: "画笔" },
  { value: "eraser", label: "橡皮" },
  { value: "fill", label: "填充" },
  { value: "picker", label: "取色" },
] as const;

const partOptions = [
  { value: "head", label: "头部" },
  { value: "body", label: "身体" },
  { value: "arms", label: "手臂" },
  { value: "legs", label: "腿部" },
] as const;
</script>

<template>
  <div class="page editor">
    <header class="editor-head">
      <div class="row">
        <IconButton icon="chevron-left" label="返回账号页" :size="34" @click="emit('close')" />
        <div>
          <span class="eyebrow">DOCL / SKIN EDITOR</span>
          <h1 class="page-title">{{ account.name }} 的皮肤</h1>
        </div>
      </div>
      <div class="row">
        <span class="tag" :class="{ 'tag--grass': isMicrosoft }">
          {{ isMicrosoft ? "微软账号" : "离线账号" }}
        </span>
        <WoodButton variant="soft" size="sm" @click="resetSkin">重置</WoodButton>
        <WoodButton variant="ghost" size="sm" @click="exportPng">导出 PNG</WoodButton>
        <WoodButton
          v-if="isMicrosoft"
          variant="danger"
          size="sm"
          @click="uploadOpen = true"
        >
          上传到 Mojang
        </WoodButton>
        <WoodButton variant="primary" size="sm" sheen :disabled="saving" @click="saveLocal">
          <AppIcon name="check" :size="14" />
          {{ saving ? "保存中…" : "保存到本地" }}
        </WoodButton>
      </div>
    </header>

    <div class="editor-layout">
      <section class="card editor-main">
        <div class="editor-toolbar">
          <div class="tool-group">
            <button
              v-for="item in tools"
              :key="item.value"
              class="tool"
              :class="{ 'tool--active': tool === item.value }"
              @click="setTool(item.value)"
            >
              {{ item.label }}
            </button>
          </div>

          <div class="tool-group">
            <span class="tool-label">笔刷</span>
            <button
              v-for="size in [1, 2, 4]"
              :key="size"
              class="tool tool--size"
              :class="{ 'tool--active': brushSize === size }"
              @click="brushSize = size"
            >
              {{ size }}
            </button>
          </div>

          <div class="tool-group">
            <IconButton
              icon="undo"
              label="撤销"
              :size="30"
              :disabled="!canUndo"
              @click="undo"
            />
            <IconButton
              icon="redo"
              label="重做"
              :size="30"
              :disabled="!canRedo"
              @click="redo"
            />
          </div>

          <div class="tool-group">
            <span class="tool-label">部位</span>
            <button
              v-for="item in partOptions"
              :key="item.value"
              class="tool"
              :class="{ 'tool--active': part === item.value }"
              @click="selectPart(item.value)"
            >
              {{ item.label }}
            </button>
          </div>

          <div class="tool-group">
            <button
              class="tool"
              :class="{ 'tool--active': showGuides }"
              @click="showGuides = !showGuides; nextTick(drawGuides)"
            >
              网格
            </button>
            <button
              v-for="value in [1, 2, 4]"
              :key="value"
              class="tool tool--size"
              :class="{ 'tool--active': zoom === value }"
              @click="zoom = value"
            >
              {{ value }}×
            </button>
          </div>
        </div>

        <div class="color-row">
          <div class="color-current" :style="{ background: color }" />
          <div class="palette">
            <button
              v-for="item in palette"
              :key="item"
              class="swatch"
              :class="{ 'swatch--active': color.toLowerCase() === item }"
              :style="{ background: item }"
              @click="color = item"
            />
          </div>
          <input
            v-model="color"
            class="hex-input mono"
            maxlength="7"
            spellcheck="false"
          />
        </div>

        <div class="canvas-scroll">
          <div class="canvas-stack" :style="{ width: `${canvasWidth}px`, height: `${canvasWidth}px` }">
            <canvas
              ref="drawCanvas"
              width="64"
              height="64"
              class="skin-canvas"
              @pointerdown="onPointerDown"
              @pointermove="onPointerMove"
              @pointerup="onPointerUp"
              @pointerleave="onPointerUp"
            />
            <canvas
              ref="guideCanvas"
              width="64"
              height="64"
              class="guide-canvas"
            />
          </div>
        </div>
      </section>

      <aside class="editor-side">
        <section class="card card-pad preview-card">
          <div class="row-between">
            <div>
              <span class="eyebrow">LIVE PREVIEW</span>
              <div class="section-title">实时 3D</div>
            </div>
            <div class="model-toggle">
              <button
                class="tool"
                :class="{ 'tool--active': model === 'classic' }"
                @click="model = 'classic'"
              >
                经典
              </button>
              <button
                class="tool"
                :class="{ 'tool--active': model === 'slim' }"
                @click="model = 'slim'"
              >
                纤细
              </button>
            </div>
          </div>
          <SkinModel
            :skin="previewSkin"
            :seed="account.uuid"
            :model="model"
            :height="360"
          />
          <p class="muted small">
            拖拽模型可以旋转。离线账号的皮肤只用于启动器预览与导出，不会改变游戏内显示。
          </p>
        </section>
      </aside>
    </div>

    <WoodModal
      v-if="uploadOpen"
      title="上传皮肤到 Mojang"
      :width="440"
      @close="uploadOpen = false"
    >
      <p class="warning">
        这会把该账号当前皮肤替换为本次编辑的皮肤，并影响所有使用该账号的客户端。
        Mojang 没有官方撤销功能，频繁上传可能触发频率限制。
      </p>
      <p class="muted small">确定要继续吗？</p>
      <template #footer>
        <WoodButton variant="ghost" size="sm" @click="uploadOpen = false">取消</WoodButton>
        <WoodButton
          variant="danger"
          size="sm"
          :disabled="uploading"
          @click="confirmUpload"
        >
          {{ uploading ? "上传中…" : "确认上传" }}
        </WoodButton>
      </template>
    </WoodModal>
  </div>
</template>

<style scoped>
.editor-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  flex-wrap: wrap;
}

.editor-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 380px;
  gap: 16px;
  align-items: start;
}

.editor-main {
  padding: 16px 18px 18px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.editor-toolbar {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  align-items: center;
}

.tool-group {
  display: flex;
  align-items: center;
  gap: 4px;
}

.tool-label {
  font-size: 10px;
  letter-spacing: 0.12em;
  color: var(--muted);
  text-transform: uppercase;
  margin-right: 2px;
}

.tool {
  height: 28px;
  min-width: 34px;
  padding: 0 9px;
  border-radius: 7px;
  border: 1px solid var(--line);
  background: var(--panel-2);
  color: var(--ink-soft);
  font-size: 11.5px;
  font-weight: 600;
  transition: all var(--dur-1) ease;
}

.tool:hover {
  border-color: var(--walnut-light);
  color: var(--walnut-deep);
}

.tool--active {
  border-color: var(--moss);
  background: var(--moss-soft);
  color: var(--moss);
}

.tool--size {
  min-width: 28px;
  padding: 0 6px;
}

.color-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.color-current {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--line-strong);
  box-shadow: 0 1px 3px rgba(80, 58, 34, 0.2);
  flex-shrink: 0;
}

.palette {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  flex: 1;
}

.swatch {
  width: 22px;
  height: 22px;
  border-radius: 5px;
  border: 1px solid rgba(0, 0, 0, 0.12);
  transition: transform var(--dur-1) ease;
}

.swatch:hover {
  transform: scale(1.12);
}

.swatch--active {
  box-shadow: 0 0 0 2px var(--moss);
}

.hex-input {
  width: 92px;
  height: 30px;
  border-radius: 7px;
  border: 1px solid var(--line-strong);
  background: var(--panel);
  padding: 0 8px;
  color: var(--ink);
  font-size: 12px;
}

.canvas-scroll {
  overflow: auto;
  max-height: 560px;
  border-radius: 12px;
  border: 1px solid var(--line);
  background: var(--paper-2);
  padding: 14px;
}

.canvas-stack {
  position: relative;
  margin: 0 auto;
}

.skin-canvas,
.guide-canvas {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  image-rendering: pixelated;
}

.skin-canvas {
  cursor: crosshair;
  border-radius: 4px;
  box-shadow: 0 8px 24px rgba(60, 40, 20, 0.18);
  background:
    linear-gradient(45deg, rgba(0, 0, 0, 0.04) 25%, transparent 25% 75%, rgba(0, 0, 0, 0.04) 75%),
    linear-gradient(45deg, rgba(0, 0, 0, 0.04) 25%, transparent 25% 75%, rgba(0, 0, 0, 0.04) 75%);
  background-size: 12px 12px;
  background-position: 0 0, 6px 6px;
}

.guide-canvas {
  pointer-events: none;
}

.editor-side {
  position: sticky;
  top: 0;
}

.preview-card {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.model-toggle {
  display: flex;
  gap: 4px;
}

.warning {
  padding: 11px 13px;
  border-radius: 9px;
  background: var(--danger-soft);
  border: 1px solid #e7cfc5;
  color: var(--danger);
  font-size: 12.5px;
  line-height: 1.6;
  margin-bottom: 12px;
}

@media (max-width: 1120px) {
  .editor-layout {
    grid-template-columns: minmax(0, 1fr);
  }

  .editor-side {
    position: static;
  }
}
</style>
