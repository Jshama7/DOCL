<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { store } from "../store";

const list = ref<HTMLElement | null>(null);

watch(
  () => store.log.length,
  async () => {
    await nextTick();
    if (list.value) list.value.scrollTop = list.value.scrollHeight;
  }
);

watch(
  () => store.logOpen,
  async (open) => {
    if (!open) return;
    await nextTick();
    if (list.value) list.value.scrollTop = list.value.scrollHeight;
  }
);

function clear() {
  store.log = [];
}
</script>

<template>
  <transition name="drawer">
    <section v-if="store.logOpen" class="drawer">
      <header class="drawer__head">
        <div class="row">
          <span class="statusbar__dot" :class="{ 'statusbar__dot--run': store.gameRunning }" />
          <strong>运行日志</strong>
          <span class="muted small">{{ store.log.length }} 行</span>
        </div>
        <div class="row">
          <button class="drawer__btn" @click="clear">清空</button>
          <button class="drawer__btn" @click="store.logOpen = false">收起</button>
        </div>
      </header>
      <div ref="list" class="drawer__body mono">
        <div v-if="!store.log.length" class="drawer__empty">
          暂无日志。启动游戏或安装版本后，这里会显示实时输出。
        </div>
        <div
          v-for="(line, index) in store.log"
          :key="index"
          class="drawer__line"
          :class="`drawer__line--${line.stream}`"
        >
          {{ line.line }}
        </div>
      </div>
    </section>
  </transition>
</template>

<style scoped>
.drawer {
  position: fixed;
  left: 176px;
  right: 0;
  bottom: var(--statusbar);
  height: 260px;
  background: #fffdf8;
  border-top: 1px solid var(--line-strong);
  box-shadow: 0 -8px 24px rgba(80, 58, 34, 0.1);
  display: flex;
  flex-direction: column;
  z-index: 60;
}

.drawer__head {
  height: 40px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--line);
  flex-shrink: 0;
}

.drawer__btn {
  height: 26px;
  padding: 0 10px;
  border-radius: 6px;
  border: 1px solid var(--line);
  color: var(--ink-soft);
  font-size: 12px;
  transition: all 0.13s ease;
}

.drawer__btn:hover {
  border-color: var(--wood-light);
  color: var(--wood-dark);
}

.drawer__body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 16px 16px;
  font-size: 12px;
  line-height: 1.55;
}

.drawer__empty {
  color: var(--muted);
  font-family: "Microsoft YaHei UI", sans-serif;
}

.drawer__line {
  white-space: pre-wrap;
  word-break: break-all;
  color: #4f4436;
}

.drawer__line--stderr {
  color: #a8543c;
}

.drawer__line--system {
  color: var(--grass-dark);
  font-weight: 600;
}

.drawer-enter-active,
.drawer-leave-active {
  transition: transform 0.18s ease, opacity 0.18s ease;
}

.drawer-enter-from,
.drawer-leave-to {
  transform: translateY(100%);
  opacity: 0.4;
}
</style>
