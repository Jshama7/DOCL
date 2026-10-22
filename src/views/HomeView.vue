<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { openFolder } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import PlayerAvatar from "../components/PlayerAvatar.vue";
import {
  launchVersion,
  selectedAccount,
  selectedVersion,
  selectVersion,
  store,
} from "../store";
import { formatBytes, kindLabel } from "../utils";

const version = selectedVersion;
const account = selectedAccount;

const tiltX = ref(0);
const tiltY = ref(0);
const pickerOpen = ref(false);
const pickerRoot = ref<HTMLElement | null>(null);

const isInstalling = computed(
  () => !!store.installingId && store.installingId === version.value?.id
);

const serial = computed(() => {
  if (!version.value) return "—";
  const index = store.installed.findIndex((v) => v.id === version.value?.id);
  return String(index + 1).padStart(2, "0");
});

const versionLabel = computed(() => {
  const current = version.value;
  if (!current) return "";
  const name = store.metas[current.id]?.display_name;
  return name && name.trim() ? name : current.id;
});

const javaLabel = computed(() => {
  const v = version.value;
  if (!v) return "—";
  const java = store.javaList.find((j) => j.major === v.java_major);
  return java ? `Java ${java.major}` : `缺少 Java ${v.java_major}`;
});

const stageStyle = computed(() => ({
  transform: `rotateX(${tiltX.value}deg) rotateY(${tiltY.value}deg)`,
}));

function onHeroMove(event: PointerEvent) {
  if (!store.config?.animations) return;
  const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
  tiltY.value = ((event.clientX - rect.left) / rect.width - 0.5) * 10;
  tiltX.value = ((event.clientY - rect.top) / rect.height - 0.5) * -8;
}

function resetTilt() {
  tiltX.value = 0;
  tiltY.value = 0;
}

function openGameDir() {
  if (store.config?.game_dir) openFolder(store.config.game_dir);
}

function pickVersion(id: string) {
  selectVersion(id);
  pickerOpen.value = false;
}

function versionName(id: string) {
  const name = store.metas[id]?.display_name;
  return name && name.trim() ? name : id;
}

function onDocClick(event: MouseEvent) {
  if (pickerRoot.value && !pickerRoot.value.contains(event.target as Node)) {
    pickerOpen.value = false;
  }
}

onMounted(() => document.addEventListener("mousedown", onDocClick));
onBeforeUnmount(() => document.removeEventListener("mousedown", onDocClick));
</script>

<template>
  <div class="page home">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / LAUNCHPAD</span>
      </div>
      <span class="tag" :class="{ 'tag--grass': !!account }">
        {{ account ? (account.kind === "microsoft" ? "微软账号" : "离线账号") : "未登录" }}
      </span>
    </header>

    <section
      class="hero card reveal"
      @pointermove="onHeroMove"
      @pointerleave="resetTilt"
    >
      <div class="hero__content">
        <div class="hero__top">
          <span class="tag tag--grass">{{ version ? kindLabel(version.kind) : "未安装" }}</span>
          <span class="hero__serial mono">INSTANCE / {{ serial }}</span>
        </div>

        <h2 class="hero__title display" :class="{ 'hero__title--empty': !version }">
          {{ version ? versionLabel : "空白存档" }}
        </h2>

        <p class="hero__desc">
          {{
            version
              ? `已就绪，客户端 ${formatBytes(version.jar_size)}，需要 ${javaLabel}。`
              : "还没有安装任何版本。去下载页挑一个，安装完成后就能从这里进入世界。"
          }}
        </p>

        <div class="hero__meta">
          <span><i>账号</i>{{ account?.name ?? "未登录" }}</span>
          <span><i>内存</i>{{ store.config?.memory_mb ?? 0 }} MB</span>
          <span><i>版本</i>{{ store.installed.length }} 个</span>
          <span><i>Java</i>{{ store.javaList.length }} 个</span>
        </div>

        <div class="hero__actions">
          <WoodButton
            v-if="!version"
            variant="primary"
            size="lg"
            sheen
            class="hero-cta"
            @click="store.page = 'download'"
          >
            <AppIcon name="download" :size="16" />
            去下载版本
          </WoodButton>
          <template v-else>
            <WoodButton
              variant="primary"
              size="lg"
              sheen
              class="hero-cta"
              :disabled="isInstalling"
              @click="launchVersion()"
            >
              <AppIcon :name="store.gameRunning ? 'stop' : 'play'" :size="16" />
              {{ store.gameRunning ? "停止游戏" : "启动游戏" }}
            </WoodButton>

            <div ref="pickerRoot" class="vpick">
              <WoodButton
                variant="ghost"
                size="lg"
                class="vpick__trigger"
                @click="pickerOpen = !pickerOpen"
              >
                版本
                <AppIcon
                  name="chevron-down"
                  :size="14"
                  class="vpick__chev"
                  :class="{ 'vpick__chev--open': pickerOpen }"
                />
              </WoodButton>

              <Transition name="drop">
                <div v-if="pickerOpen" class="vpick__menu">
                  <button
                    v-for="item in store.installed"
                    :key="item.id"
                    class="vpick__item"
                    :class="{ 'vpick__item--active': item.id === version?.id }"
                    @click="pickVersion(item.id)"
                  >
                    <span class="vpick__name">{{ versionName(item.id) }}</span>
                    <span class="vpick__kind">{{ kindLabel(item.kind) }}</span>
                    <AppIcon
                      v-if="item.id === version?.id"
                      name="check"
                      :size="13"
                      class="vpick__check"
                    />
                  </button>
                  <button class="vpick__manage" @click="store.page = 'versions'">
                    <AppIcon name="layers" :size="14" />
                    管理版本
                  </button>
                </div>
              </Transition>
            </div>

            <IconButton icon="folder" label="打开游戏目录" :size="44" @click="openGameDir" />
          </template>
        </div>
      </div>

      <div class="hero__art" aria-hidden="true">
        <span class="hero__glow" />
        <div class="hero__player">
          <PlayerAvatar :account="account" :size="64" :radius="13" />
          <div class="hero__player-text">
            <span class="eyebrow">PLAYER</span>
            <strong>{{ account?.name ?? "未登录" }}</strong>
            <span>
              {{
                account
                  ? account.kind === "microsoft"
                    ? "微软账号"
                    : "离线账号"
                  : "到账号页添加"
              }}
            </span>
          </div>
        </div>
        <div class="hero__stage" :style="stageStyle">
          <div class="iso">
            <span class="iso__face iso__face--top" />
            <span class="iso__face iso__face--left" />
            <span class="iso__face iso__face--right" />
          </div>
          <span class="hero__shadow" />
        </div>
        <span class="hero__bit hero__bit--a" />
        <span class="hero__bit hero__bit--b" />
        <span class="hero__bit hero__bit--c" />
      </div>
    </section>
  </div>
</template>

<style scoped>
.home {
  overflow: visible;
}

/* 主视觉 */
.hero {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 320px;
  gap: 20px;
  padding: 28px 30px;
  overflow: hidden;
  background: linear-gradient(128deg, var(--hero-a) 0%, var(--hero-b) 58%, var(--hero-c) 100%);
  border-color: var(--line-strong);
  box-shadow: var(--shadow-2);
}

.hero::before {
  content: "";
  position: absolute;
  inset: 0;
  background-image: repeating-linear-gradient(
    90deg,
    rgba(var(--accent-rgb), 0.03) 0 1px,
    transparent 1px 46px
  );
  pointer-events: none;
}

.hero__content {
  position: relative;
  z-index: 2;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
}

.hero__top {
  display: flex;
  align-items: center;
  gap: 12px;
}

.hero__serial {
  font-size: 10px;
  letter-spacing: 0.2em;
  color: var(--muted);
}

.hero__title {
  font-size: clamp(46px, 6vw, 72px);
  line-height: 0.98;
  letter-spacing: -0.03em;
  margin: 2px 0 4px;
  word-break: break-word;
}

.hero__title--empty {
  font-family: var(--font-ui);
  font-size: clamp(30px, 4vw, 42px);
  font-weight: 800;
  letter-spacing: 0.02em;
  color: var(--walnut);
}

.hero__desc {
  color: var(--ink-soft);
  font-size: 13.5px;
  max-width: 520px;
}

.hero__meta {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 20px;
  padding: 12px 0 4px;
  border-top: 1px dashed var(--line-strong);
  color: var(--ink-soft);
  font-size: 12.5px;
}

.hero__meta span {
  display: inline-flex;
  align-items: baseline;
  gap: 8px;
}

.hero__meta i {
  font-style: normal;
  font-family: var(--font-mono);
  font-size: 10px;
  letter-spacing: 0.14em;
  color: var(--muted);
  text-transform: uppercase;
}

.hero__actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 6px;
}

.hero-cta {
  min-width: 172px;
}

/* 版本选择下拉 */
.vpick {
  position: relative;
}

.vpick__trigger {
  min-width: 108px;
  gap: 6px;
}

.vpick__chev {
  transition: transform var(--dur-2) var(--ease-out-expo);
}

.vpick__chev--open {
  transform: rotate(180deg);
}

.vpick__menu {
  position: absolute;
  left: 0;
  bottom: calc(100% + 8px);
  z-index: 30;
  min-width: 240px;
  max-height: 292px;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--line-strong);
  border-radius: 12px;
  background: rgba(var(--glass-rgb), 0.94);
  backdrop-filter: blur(var(--glass-blur-px, 16px)) saturate(1.15);
  box-shadow: var(--shadow-2);
}

.vpick__item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 9px;
  height: 36px;
  padding: 0 10px;
  border-radius: 8px;
  color: var(--ink);
  text-align: left;
  transition: background var(--dur-1) ease;
}

.vpick__item:hover {
  background: rgba(var(--accent-rgb), 0.07);
}

.vpick__item--active {
  background: var(--moss-soft);
  color: var(--walnut-deep);
}

.vpick__name {
  flex: 1;
  font-size: 13px;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.vpick__kind {
  font-size: 10.5px;
  color: var(--muted);
}

.vpick__check {
  color: var(--moss);
}

.vpick__manage {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 8px;
  height: 34px;
  margin-top: 4px;
  padding: 0 10px;
  border-top: 1px dashed var(--line);
  border-radius: 0 0 8px 8px;
  color: var(--muted);
  font-size: 12px;
  text-align: left;
  transition: color var(--dur-1) ease, background var(--dur-1) ease;
}

.vpick__manage:hover {
  color: var(--moss);
  background: rgba(var(--accent-rgb), 0.05);
}

/* 立方体插画 */
.hero__art {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 2px;
  min-height: 300px;
}

.hero__glow {
  position: absolute;
  width: 280px;
  height: 280px;
  border-radius: 50%;
  background: radial-gradient(circle, rgba(var(--ambient-a), 0.32), transparent 66%);
  filter: blur(18px);
}

.hero__player {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  max-width: 268px;
  padding: 10px 14px;
  border: 1px solid var(--line);
  border-radius: 14px;
  background: rgba(var(--glass-rgb), 0.78);
  box-shadow: var(--shadow-1);
}

.hero__player-text {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.hero__player-text strong {
  font-size: 16px;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.hero__player-text > span:last-child {
  font-size: 11.5px;
  color: var(--muted);
}

.hero__stage {
  position: relative;
  width: 240px;
  height: 210px;
  display: grid;
  place-items: center;
  perspective: 700px;
  transition: transform 600ms var(--ease-out-expo);
}

.iso {
  position: relative;
  width: 124px;
  height: 124px;
  transform-style: preserve-3d;
  transform: rotateX(-24deg) rotateY(38deg);
  animation: iso-float 7s var(--ease-in-out) infinite;
}

.iso__face {
  position: absolute;
  width: 124px;
  height: 124px;
  border: 1px solid rgba(70, 40, 18, 0.18);
}

.iso__face--top {
  background-color: #6f8f5a;
  background-image: repeating-linear-gradient(
      90deg,
      rgba(255, 255, 255, 0.1) 0 15px,
      transparent 15px 31px
    ),
    repeating-linear-gradient(0deg, rgba(40, 60, 30, 0.18) 0 15px, transparent 15px 31px);
  transform: rotateX(90deg) translateZ(62px);
  box-shadow: 0 0 24px rgba(94, 123, 73, 0.3) inset;
}

.iso__face--left {
  background: linear-gradient(180deg, #567243 0 25%, #5c3a20 25% 100%);
  background-image: radial-gradient(circle at 22% 66%, rgba(255, 255, 255, 0.1) 0 3px, transparent 3px),
    radial-gradient(circle at 72% 84%, rgba(0, 0, 0, 0.16) 0 4px, transparent 4px),
    linear-gradient(180deg, #567243 0 25%, #5c3a20 25% 100%);
  transform: rotateY(-90deg) translateZ(62px);
  filter: brightness(0.9);
}

.iso__face--right {
  background: radial-gradient(circle at 30% 58%, rgba(255, 255, 255, 0.12) 0 3px, transparent 3px),
    radial-gradient(circle at 74% 76%, rgba(0, 0, 0, 0.18) 0 5px, transparent 5px),
    linear-gradient(180deg, #7f9a63 0 25%, #7a5230 25% 100%);
  transform: translateZ(62px);
}

.hero__shadow {
  position: absolute;
  bottom: 18px;
  width: 210px;
  height: 26px;
  border-radius: 50%;
  background: radial-gradient(ellipse, rgba(70, 40, 18, 0.28), transparent 70%);
  filter: blur(6px);
  animation: shadow-breathe 7s var(--ease-in-out) infinite;
}

.hero__bit {
  position: absolute;
  width: 9px;
  height: 9px;
  border-radius: 2px;
  background: var(--amber);
  opacity: 0.75;
  animation: bit-float 6s var(--ease-in-out) infinite;
}

.hero__bit--a {
  left: 32px;
  top: 62px;
  animation-delay: -1.2s;
}

.hero__bit--b {
  right: 42px;
  top: 92px;
  width: 6px;
  height: 6px;
  background: var(--moss);
  animation-delay: -3.4s;
}

.hero__bit--c {
  right: 66px;
  bottom: 54px;
  width: 12px;
  height: 12px;
  background: var(--walnut-light);
  animation-delay: -2s;
}

@keyframes iso-float {
  0%,
  100% {
    transform: rotateX(-24deg) rotateY(38deg) translateY(0);
  }
  50% {
    transform: rotateX(-24deg) rotateY(38deg) translateY(-12px);
  }
}

@keyframes shadow-breathe {
  0%,
  100% {
    transform: scaleX(1);
    opacity: 1;
  }
  50% {
    transform: scaleX(0.84);
    opacity: 0.6;
  }
}

@keyframes bit-float {
  0%,
  100% {
    transform: translateY(0) rotate(0deg);
  }
  50% {
    transform: translateY(-14px) rotate(18deg);
  }
}

@media (max-width: 960px) {
  .hero {
    grid-template-columns: minmax(0, 1fr);
  }

  .hero__art {
    display: none;
  }
}
</style>
