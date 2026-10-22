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
  border-color: rgba(var(--glass-rgb), 0.58);
  background: rgba(var(--glass-rgb), 0.4);
  backdrop-filter: blur(10px) saturate(1.15);
}

.icon-btn--soft:hover:not(:disabled) {
  border-color: rgba(var(--accent-rgb), 0.3);
  background: rgba(var(--glass-rgb), 0.58);
}

.icon-btn--danger {
  color: var(--danger);
  background: rgba(190, 92, 70, 0.1);
  border-color: rgba(190, 92, 70, 0.22);
}

.icon-btn--danger:hover:not(:disabled) {
  background: rgba(190, 92, 70, 0.16);
  color: var(--danger);
}

.icon-btn--primary {
  color: var(--walnut-deep);
  background: linear-gradient(
    180deg,
    rgba(var(--accent-rgb), 0.14),
    rgba(var(--accent-rgb), 0.26)
  );
  border-color: rgba(var(--accent-rgb), 0.3);
  box-shadow: 0 1px 0 rgba(var(--glass-rgb), 0.65) inset,
    0 8px 18px rgba(70, 40, 18, 0.08);
}

.icon-btn--primary:hover:not(:disabled) {
  background: linear-gradient(
    180deg,
    rgba(var(--accent-rgb), 0.2),
    rgba(var(--accent-rgb), 0.34)
  );
  color: var(--walnut-deep);
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
