<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import IconButton from "./IconButton.vue";
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
          <span class="eyebrow">DOCL / LOG</span>
          <strong class="drawer__title">运行日志</strong>
          <span class="muted small mono">{{ store.log.length }} LINES</span>
        </div>
        <div class="row">
          <IconButton icon="trash" label="清空日志" :size="28" @click="clear" />
          <IconButton
            icon="chevron-down"
            label="收起日志"
            :size="28"
            @click="store.logOpen = false"
          />
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
          <span class="drawer__prefix">{{ line.stream === "stderr" ? "!" : line.stream === "system" ? "›" : "·" }}</span>
          <span class="drawer__text">{{ line.line }}</span>
        </div>
      </div>
    </section>
  </transition>
</template>

<style scoped>
.drawer {
  position: fixed;
  left: 0;
  right: 0;
  bottom: var(--statusbar);
  height: 280px;
  background: var(--panel);
  border-top: 1px solid var(--line-strong);
  box-shadow: 0 -16px 40px rgba(70, 48, 26, 0.12);
  display: flex;
  flex-direction: column;
  z-index: 60;
}

.drawer__head {
  height: 44px;
  padding: 0 20px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  border-bottom: 1px solid var(--line);
  background: linear-gradient(180deg, #fffdf6, #faf2e3);
  flex-shrink: 0;
}

.drawer__title {
  font-size: 13px;
  color: var(--walnut-deep);
}

.drawer__btn {
  height: 27px;
  padding: 0 11px;
  border-radius: 7px;
  border: 1px solid var(--line);
  color: var(--ink-soft);
  font-size: 12px;
  transition: all var(--dur-1) ease;
}

.drawer__btn:hover {
  border-color: var(--walnut-light);
  color: var(--walnut-deep);
  background: var(--panel-2);
}

.drawer__body {
  flex: 1;
  overflow-y: auto;
  padding: 10px 20px 18px;
  font-size: 11.5px;
  line-height: 1.62;
  background: #fffdf7;
}

.drawer__empty {
  color: var(--muted);
  font-family: var(--font-ui);
  padding: 12px 0;
}

.drawer__line {
  display: flex;
  gap: 8px;
  padding: 1px 6px;
  border-radius: 4px;
  color: #4c4034;
  transition: background var(--dur-1) ease;
}

.drawer__line:hover {
  background: #f7eddc;
}

.drawer__prefix {
  color: var(--line-strong);
  flex-shrink: 0;
  width: 10px;
  text-align: center;
}

.drawer__text {
  white-space: pre-wrap;
  word-break: break-all;
}

.drawer__line--stderr .drawer__text,
.drawer__line--stderr .drawer__prefix {
  color: var(--danger);
}

.drawer__line--system .drawer__text,
.drawer__line--system .drawer__prefix {
  color: var(--moss);
  font-weight: 700;
}

.drawer-enter-active,
.drawer-leave-active {
  transition: transform 320ms var(--ease-out-expo), opacity 240ms ease;
}

.drawer-enter-from,
.drawer-leave-to {
  transform: translateY(100%);
  opacity: 0.4;
}
</style>
