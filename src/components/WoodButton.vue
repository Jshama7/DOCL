<script setup lang="ts">
import { ref } from "vue";

withDefaults(
  defineProps<{
    variant?: "primary" | "ghost" | "soft" | "danger";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    block?: boolean;
    sheen?: boolean;
  }>(),
  { variant: "soft", size: "md", disabled: false, block: false, sheen: false }
);
const emit = defineEmits<{ (e: "click", ev: MouseEvent): void }>();
const root = ref<HTMLButtonElement | null>(null);
const ripple = ref<HTMLSpanElement | null>(null);

function onPointerDown(event: PointerEvent) {
  const button = root.value;
  const wave = ripple.value;
  if (!button || !wave || button.disabled) return;
  const rect = button.getBoundingClientRect();
  const size = Math.max(rect.width, rect.height) * 2;
  wave.style.width = `${size}px`;
  wave.style.height = `${size}px`;
  wave.style.left = `${event.clientX - rect.left - size / 2}px`;
  wave.style.top = `${event.clientY - rect.top - size / 2}px`;
  wave.classList.remove("is-active");
  void wave.offsetWidth;
  wave.classList.add("is-active");
}

function onClick(event: MouseEvent) {
  emit("click", event);
}
</script>

<template>
  <button
    ref="root"
    class="btn"
    :class="[
      `btn--${variant}`,
      `btn--${size}`,
      { 'btn--block': block, 'btn--sheen': sheen },
    ]"
    :disabled="disabled"
    @pointerdown="onPointerDown"
    @click="onClick"
  >
    <span ref="ripple" class="btn__ripple" />
    <span class="btn__label"><slot /></span>
  </button>
</template>

<style scoped>
.btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  border-radius: 4px;
  border: 1px solid transparent;
  font-weight: 700;
  line-height: 1;
  letter-spacing: 0.01em;
  overflow: hidden;
  isolation: isolate;
  transition: background var(--dur-2) ease, border-color var(--dur-2) ease,
    transform var(--dur-1) ease, color var(--dur-2) ease;
}

.btn__label {
  position: relative;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}

.btn:active:not(:disabled) {
  transform: translateY(1px);
}

.btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn__ripple {
  position: absolute;
  border-radius: 50%;
  pointer-events: none;
  transform: scale(0);
  opacity: 0;
  background: radial-gradient(circle, rgba(var(--glass-rgb), 0.5), transparent 62%);
  z-index: 1;
}

.btn__ripple.is-active {
  animation: btn-ripple 600ms var(--ease-out-expo) forwards;
}

@keyframes btn-ripple {
  0% {
    transform: scale(0);
    opacity: 1;
  }
  100% {
    transform: scale(1);
    opacity: 0;
  }
}

.btn--sm {
  height: 30px;
  padding: 0 12px;
  font-size: 12.5px;
  border-radius: 4px;
}

.btn--md {
  height: 38px;
  padding: 0 18px;
  font-size: 13.5px;
}

.btn--lg {
  height: 52px;
  padding: 0 30px;
  font-size: 16px;
  border-radius: 6px;
}

.btn--block {
  width: 100%;
}

.btn--primary {
  color: var(--btn-fg);
  background: var(--btn-b);
  border-color: var(--btn-border);
}

.btn--primary:hover:not(:disabled) {
  background: var(--btn-a);
  border-color: var(--btn-border);
}

.btn--ghost {
  background: transparent;
  border-color: var(--line);
  color: var(--ink-soft);
}

.btn--ghost:hover:not(:disabled) {
  border-color: var(--line-strong);
  color: var(--walnut-deep);
  background: rgba(var(--accent-rgb), 0.05);
}

.btn--soft {
  background: var(--panel-2);
  border-color: var(--line);
  color: var(--walnut);
}

.btn--soft:hover:not(:disabled) {
  border-color: var(--line-strong);
  color: var(--walnut-deep);
  background: var(--hover);
}

.btn--danger {
  background: rgba(190, 92, 70, 0.1);
  border-color: rgba(190, 92, 70, 0.24);
  color: var(--danger);
}

.btn--danger:hover:not(:disabled) {
  background: rgba(190, 92, 70, 0.16);
  border-color: rgba(190, 92, 70, 0.36);
}
</style>
