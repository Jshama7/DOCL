<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";
import AppIcon from "./AppIcon.vue";
import { store } from "../store";

const props = withDefaults(
  defineProps<{
    icon: string;
    label: string;
    variant?: "ghost" | "soft" | "danger" | "primary";
    size?: number;
    disabled?: boolean;
    active?: boolean;
  }>(),
  { variant: "ghost", size: 34, disabled: false, active: false }
);
const emit = defineEmits<{ (e: "click", ev: MouseEvent): void }>();

const button = ref<HTMLButtonElement | null>(null);
const visible = ref(false);
const position = ref({ left: 0, top: 0 });
let timer: number | undefined;

function show() {
  if (props.disabled) return;
  if (timer) window.clearTimeout(timer);
  timer = window.setTimeout(() => {
    const element = button.value;
    if (!element) return;
    const rect = element.getBoundingClientRect();
    const width = Math.min(240, Math.max(60, props.label.length * 13 + 26));
    let left = rect.left + rect.width / 2 - width / 2;
    left = Math.max(8, Math.min(window.innerWidth - width - 8, left));
    const top = rect.top > 60 ? rect.top - 34 : rect.bottom + 8;
    position.value = { left, top };
    visible.value = true;
  }, 320);
}

function hide() {
  if (timer) window.clearTimeout(timer);
  visible.value = false;
}

onBeforeUnmount(() => {
  if (timer) window.clearTimeout(timer);
});
</script>

<template>
  <span class="icon-btn-wrap">
    <button
      ref="button"
      class="icon-btn"
      :class="[`icon-btn--${variant}`, { 'icon-btn--active': active }]"
      :style="{ width: `${size}px`, height: `${size}px` }"
      :disabled="disabled"
      :aria-label="label"
      @mouseenter="show"
      @mouseleave="hide"
      @focus="show"
      @blur="hide"
      @click="$emit('click', $event)"
    >
      <AppIcon :name="icon" :size="Math.round(size * 0.53)" />
    </button>

    <Teleport to="body">
      <Transition name="tip">
        <div
          v-if="visible"
          class="icon-tip"
          :class="{ 'icon-tip--instant': store.config && !store.config.animations }"
          :style="{ left: `${position.left}px`, top: `${position.top}px` }"
          role="tooltip"
        >
          {{ label }}
        </div>
      </Transition>
    </Teleport>
  </span>
</template>

<style scoped>
.icon-btn-wrap {
  display: inline-flex;
}

.icon-btn {
  display: grid;
  place-items: center;
  border-radius: 9px;
  border: 1px solid transparent;
  color: var(--ink-soft);
  background: transparent;
  transition: background var(--dur-2) ease, color var(--dur-2) ease,
    border-color var(--dur-2) ease;
}

.icon-btn:hover:not(:disabled) {
  background: rgba(111, 66, 36, 0.07);
  color: var(--walnut-deep);
}

.icon-btn:active:not(:disabled) {
  background: rgba(111, 66, 36, 0.12);
}

.icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.icon-btn--soft {
  border-color: var(--line);
  background: var(--panel-2);
}

.icon-btn--soft:hover:not(:disabled) {
  border-color: var(--walnut-light);
}

.icon-btn--danger {
  color: var(--danger);
  background: var(--danger-soft);
  border-color: #e7cfc5;
}

.icon-btn--danger:hover:not(:disabled) {
  background: #eed6cd;
  color: #8d3f29;
}

.icon-btn--primary {
  color: var(--btn-fg);
  background: linear-gradient(180deg, var(--btn-a) 0%, var(--btn-b) 60%, var(--btn-c) 100%);
  border-color: var(--btn-border);
  box-shadow: 0 2px 8px rgba(70, 40, 18, 0.2);
}

.icon-btn--primary:hover:not(:disabled) {
  filter: brightness(1.07);
  color: var(--btn-fg);
}

.icon-btn--active {
  color: var(--moss);
  background: var(--moss-soft);
  border-color: var(--moss);
}

.icon-tip {
  position: fixed;
  z-index: 120;
  padding: 5px 9px;
  border-radius: 6px;
  background: #3c2b1c;
  color: #fbf4e6;
  font-size: 11.5px;
  line-height: 1.4;
  white-space: nowrap;
  pointer-events: none;
  box-shadow: 0 6px 18px rgba(40, 28, 16, 0.28);
  border: 1px solid rgba(255, 253, 246, 0.08);
}

.icon-tip--instant {
  transition: none !important;
}

.tip-enter-active,
.tip-leave-active {
  transition: opacity 130ms ease, transform 130ms var(--ease-out-expo);
}

.tip-enter-from,
.tip-leave-to {
  opacity: 0;
  transform: translateY(3px);
}
</style>
