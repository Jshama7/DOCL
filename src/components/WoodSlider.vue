<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
  }>(),
  { min: 0, max: 100, step: 1 }
);
const emit = defineEmits<{ (e: "update:modelValue", value: number): void }>();

const percent = computed(() => {
  const span = props.max - props.min || 1;
  return Math.min(100, Math.max(0, ((props.modelValue - props.min) / span) * 100));
});

function onInput(event: Event) {
  emit("update:modelValue", Number((event.target as HTMLInputElement).value));
}
</script>

<template>
  <input
    class="slider"
    type="range"
    :min="min"
    :max="max"
    :step="step"
    :value="modelValue"
    :style="{
      background: `linear-gradient(90deg, var(--wood) 0 ${percent}%, var(--line) ${percent}% 100%)`,
    }"
    @input="onInput"
  />
</template>

<style scoped>
.slider {
  -webkit-appearance: none;
  appearance: none;
  width: 100%;
  height: 6px;
  border-radius: 3px;
  outline: none;
  cursor: pointer;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fffdf8;
  border: 3px solid var(--wood);
  box-shadow: 0 1px 4px rgba(80, 58, 34, 0.25);
  transition: transform 0.12s ease;
}

.slider::-webkit-slider-thumb:hover {
  transform: scale(1.15);
}

.slider::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #fffdf8;
  border: 3px solid var(--wood);
}
</style>
