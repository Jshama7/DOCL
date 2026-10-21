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
  height: 38px;
  border-radius: 9px;
  border: 1px solid var(--line-strong);
  background: linear-gradient(180deg, var(--panel), var(--panel-2));
  padding: 0 13px;
  transition: border-color var(--dur-2) ease, box-shadow var(--dur-2) ease,
    background var(--dur-2) ease;
  color: var(--ink);
  box-shadow: 0 1px 2px rgba(80, 58, 34, 0.05) inset;
}

.input::placeholder {
  color: #b7a68f;
}

.input:hover:not(:disabled) {
  border-color: var(--walnut-light);
}

.input:focus {
  outline: none;
  border-color: var(--moss);
  background: var(--panel);
  box-shadow: 0 0 0 3px rgba(94, 123, 73, 0.14);
}

.input:disabled {
  opacity: 0.55;
}

.input.mono {
  font-family: var(--font-mono);
  font-size: 12.5px;
}
</style>
