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
      background: `linear-gradient(90deg, var(--moss) 0 ${percent}%, #e2d4bb ${percent}% 100%)`,
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
  border-radius: 99px;
  outline: none;
  cursor: pointer;
  box-shadow: 0 1px 2px rgba(80, 58, 34, 0.12) inset;
}

.slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: linear-gradient(180deg, #fffdf6, #f5e9d4);
  border: 3px solid var(--walnut);
  box-shadow: 0 2px 6px rgba(70, 40, 18, 0.3);
  transition: transform var(--dur-1) var(--ease-out-expo),
    box-shadow var(--dur-2) ease;
}

.slider::-webkit-slider-thumb:hover {
  transform: scale(1.16);
  box-shadow: 0 3px 10px rgba(70, 40, 18, 0.34);
}

.slider::-moz-range-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: #fffdf6;
  border: 3px solid var(--walnut);
}
</style>
