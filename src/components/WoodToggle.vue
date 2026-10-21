<script setup lang="ts">
defineProps<{ modelValue: boolean; disabled?: boolean }>();
defineEmits<{ (e: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <button
    class="toggle"
    :class="{ 'toggle--on': modelValue }"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    @click="$emit('update:modelValue', !modelValue)"
  >
    <span class="toggle__knob" />
  </button>
</template>

<style scoped>
.toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: #ded2bd;
  border: 1px solid #d0c0a6;
  padding: 2px;
  display: flex;
  align-items: center;
  transition: background 0.18s ease, border-color 0.18s ease;
  flex-shrink: 0;
}

.toggle__knob {
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: #fffdf7;
  box-shadow: 0 1px 3px rgba(80, 58, 34, 0.28);
  transition: transform 0.18s cubic-bezier(0.3, 0.9, 0.4, 1.2);
}

.toggle--on {
  background: var(--grass);
  border-color: var(--grass-dark);
}

.toggle--on .toggle__knob {
  transform: translateX(18px);
}

.toggle:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}
</style>
