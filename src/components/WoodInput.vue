<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    type?: string;
    disabled?: boolean;
    mono?: boolean;
  }>(),
  { placeholder: "", type: "text", disabled: false, mono: false }
);
defineEmits<{ (e: "update:modelValue", value: string): void }>();
</script>

<template>
  <input
    class="input"
    :class="{ mono }"
    :type="type"
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>

<style scoped>
.input {
  width: 100%;
  height: 36px;
  border-radius: 8px;
  border: 1px solid var(--line-strong);
  background: var(--panel);
  padding: 0 12px;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
  color: var(--ink);
}

.input::placeholder {
  color: #b6a894;
}

.input:hover:not(:disabled) {
  border-color: var(--wood-light);
}

.input:focus {
  outline: none;
  border-color: var(--grass);
  box-shadow: 0 0 0 3px rgba(111, 143, 90, 0.14);
}

.input:disabled {
  opacity: 0.55;
}

.input.mono {
  font-family: "Cascadia Mono", "Consolas", monospace;
  font-size: 12.5px;
}
</style>
