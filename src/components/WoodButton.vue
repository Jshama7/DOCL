<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "ghost" | "soft" | "danger";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    block?: boolean;
  }>(),
  { variant: "soft", size: "md", disabled: false, block: false }
);
defineEmits<{ (e: "click", ev: MouseEvent): void }>();
</script>

<template>
  <button
    class="btn"
    :class="[`btn--${variant}`, `btn--${size}`, { 'btn--block': block }]"
    :disabled="disabled"
    @click="$emit('click', $event)"
  >
    <slot />
  </button>
</template>

<style scoped>
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border-radius: 8px;
  border: 1px solid transparent;
  font-weight: 600;
  letter-spacing: 0.01em;
  transition: background 0.15s ease, border-color 0.15s ease, transform 0.12s ease,
    box-shadow 0.15s ease;
  white-space: nowrap;
}

.btn:active:not(:disabled) {
  transform: translateY(1px);
}

.btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.btn--sm {
  height: 30px;
  padding: 0 12px;
  font-size: 12.5px;
}

.btn--md {
  height: 36px;
  padding: 0 16px;
  font-size: 13.5px;
}

.btn--lg {
  height: 46px;
  padding: 0 26px;
  font-size: 15.5px;
  border-radius: 10px;
}

.btn--block {
  width: 100%;
}

.btn--primary {
  color: #fffdf6;
  background: linear-gradient(180deg, #9a6a3d, #84562f);
  border-color: #744a27;
  box-shadow: 0 3px 0 rgba(74, 48, 24, 0.18);
}

.btn--primary:hover:not(:disabled) {
  background: linear-gradient(180deg, #a57345, #8b5c33);
  box-shadow: 0 5px 12px rgba(90, 60, 32, 0.22);
}

.btn--ghost {
  background: transparent;
  border-color: var(--line-strong);
  color: var(--ink-soft);
}

.btn--ghost:hover:not(:disabled) {
  border-color: var(--wood-light);
  color: var(--wood-dark);
  background: rgba(139, 94, 52, 0.05);
}

.btn--soft {
  background: var(--panel-2);
  border-color: var(--line);
  color: var(--wood-dark);
}

.btn--soft:hover:not(:disabled) {
  border-color: var(--wood-light);
  background: #f6ecda;
}

.btn--danger {
  background: var(--danger-soft);
  border-color: #e7cfc5;
  color: var(--danger);
}

.btn--danger:hover:not(:disabled) {
  background: #efd7ce;
}
</style>
