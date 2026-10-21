<script setup lang="ts">
import { computed, ref } from "vue";
import { openFolder } from "../api";
import AppIcon from "../components/AppIcon.vue";
import IconButton from "../components/IconButton.vue";
import WoodButton from "../components/WoodButton.vue";
import WoodProgress from "../components/WoodProgress.vue";
import PlayerAvatar from "../components/PlayerAvatar.vue";
import {
  launchVersion,
  selectedAccount,
  selectedVersion,
  selectVersion,
  store,
} from "../store";
import { formatBytes, formatSpeed, kindLabel } from "../utils";

const version = selectedVersion;
const account = selectedAccount;

const tiltX = ref(0);
const tiltY = ref(0);

const isInstalling = computed(
  () => !!store.installingId && store.installingId === version.value?.id
);

const progress = computed(() => {
  if (!store.progress || store.progress.stage === "java") return null;
  if (!store.installingId) return null;
  return store.progress;
});

const percent = computed(() => {
  const p = progress.value;
  if (!p) return 0;
  if (p.bytes_total > 0) return Math.min(100, (p.bytes_done / p.bytes_total) * 100);
  if (p.files_total > 0) return Math.min(100, (p.files_done / p.files_total) * 100);
  return 0;
});

const speed = computed(() => {
  const p = progress.value;
  if (!p) return "";
  return `${formatSpeed(p.bytes_done, p.elapsed_ms)} · ${p.files_done}/${p.files_total} 个文件`;
});

const serial = computed(() => {
  if (!version.value) return "—";
  const index = store.installed.findIndex((v) => v.id === version.value?.id);
  return String(index + 1).padStart(2, "0");
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
</script>

<template>
  <div class="page home">
    <header class="page-head">
      <div>
        <span class="eyebrow">DOCL / LAUNCHPAD</span>
        <h1 class="page-title">今天想玩点什么</h1>
        <p class="page-sub">版本、账号与运行状态，一眼看完</p>
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
          {{ version?.id ?? "空白存档" }}
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
          <span><i>目录</i>{{ store.config?.version_isolation ? "版本隔离" : "共用目录" }}</span>
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

    <section v-if="progress" class="card card-pad progress-card reveal">
      <div class="row-between">
        <div class="row">
          <span class="tag tag--grass">安装中</span>
          <strong>{{ store.installingId }}</strong>
        </div>
        <span class="muted small">{{ speed }}</span>
      </div>
      <WoodProgress :value="percent" />
      <p class="muted small">{{ progress.message }}</p>
    </section>

    <section class="stat-strip card reveal" style="--i: 1">
      <div class="stat">
        <span class="stat__num display">{{ String(store.installed.length).padStart(2, "0") }}</span>
        <span class="stat__label">已安装版本</span>
      </div>
      <div class="stat">
        <span class="stat__num display">{{ String(store.javaList.length).padStart(2, "0") }}</span>
        <span class="stat__label">可用 Java</span>
      </div>
      <div class="stat">
        <span class="stat__num display">{{ String(store.accounts.length).padStart(2, "0") }}</span>
        <span class="stat__label">账号</span>
      </div>
      <div class="stat">
        <span class="stat__num display">{{ store.config?.memory_mb ?? 0 }}</span>
        <span class="stat__label">内存上限 MB</span>
      </div>
    </section>

    <section class="shelf card reveal" style="--i: 2">
      <div class="shelf__head">
        <div>
          <span class="eyebrow">LIBRARY</span>
          <div class="section-title">版本书架</div>
        </div>
        <WoodButton variant="ghost" size="sm" @click="store.page = 'versions'">
          管理版本
        </WoodButton>
      </div>

      <div v-if="store.installed.length" class="chips">
        <button
          v-for="(item, index) in store.installed"
          :key="item.id"
          class="chip"
          :class="{ 'chip--active': item.id === version?.id }"
          :style="{ '--i': index }"
          @click="selectVersion(item.id)"
        >
          <span class="chip__idx mono">{{ String(index + 1).padStart(2, "0") }}</span>
          <span class="chip__name">{{ item.id }}</span>
          <span class="chip__kind">{{ kindLabel(item.kind) }}</span>
        </button>
      </div>
      <div v-else class="shelf__empty">
        <p class="muted small">书架还是空的，去下载页放一本进来。</p>
        <WoodButton variant="soft" size="sm" @click="store.page = 'download'">
          <AppIcon name="download" :size="14" />
          去下载
        </WoodButton>
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

/* 进度 */
.progress-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 数据条 */
.stat-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  padding: 18px 8px;
}

.stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 2px 20px;
  border-right: 1px solid var(--line);
}

.stat:last-child {
  border-right: none;
}

.stat__num {
  font-size: 34px;
  line-height: 1.05;
  color: var(--walnut-deep);
}

.stat__label {
  font-size: 11.5px;
  color: var(--muted);
  letter-spacing: 0.04em;
}

/* 书架 */
.shelf {
  padding: 20px 22px 22px;
}

.shelf__head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  margin-bottom: 14px;
}

.chips {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
  gap: 10px;
}

.chip {
  display: flex;
  align-items: center;
  gap: 10px;
  height: 46px;
  padding: 0 14px;
  border: 1px solid var(--line);
  border-radius: 10px;
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  text-align: left;
  transition: transform var(--dur-2) var(--ease-out-expo),
    border-color var(--dur-2) ease, box-shadow var(--dur-2) ease;
  animation: rise-in var(--dur-4) var(--ease-out-expo) both;
  animation-delay: calc(var(--i, 0) * 45ms);
}

.chip:hover {
  transform: translateY(-2px);
  border-color: var(--walnut-light);
  box-shadow: var(--shadow-1);
}

.chip__idx {
  font-size: 10px;
  color: var(--muted);
  letter-spacing: 0.08em;
}

.chip__name {
  flex: 1;
  font-weight: 700;
  color: var(--walnut-deep);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.chip__kind {
  font-size: 10.5px;
  color: var(--muted);
}

.chip--active {
  border-color: var(--moss);
  background: linear-gradient(180deg, var(--panel), var(--moss-soft));
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.08);
}

.chip--active .chip__idx,
.chip--active .chip__kind {
  color: var(--moss);
}

.shelf__empty {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 0 2px;
}

@media (max-width: 960px) {
  .hero {
    grid-template-columns: minmax(0, 1fr);
  }

  .hero__art {
    display: none;
  }

  .stat-strip {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px 0;
  }

  .stat:nth-child(2) {
    border-right: none;
  }
}
</style>
