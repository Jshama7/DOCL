<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ value: number; indeterminate?: boolean; height?: number }>(),
  { indeterminate: false, height: 7 }
);
const width = computed(() => (props.indeterminate ? 38 : Math.min(100, Math.max(0, props.value))));
</script>

<template>
  <div class="bar" :style="{ height: `${height}px` }">
    <div
      class="bar__fill"
      :class="{ 'bar__fill--indeterminate': indeterminate }"
      :style="{ width: `${width}%` }"
    >
      <span class="bar__shine" />
    </div>
  </div>
</template>

<style scoped>
.bar {
  width: 100%;
  border-radius: 99px;
  background: #e7dac2;
  overflow: hidden;
  box-shadow: 0 1px 2px rgba(80, 58, 34, 0.12) inset;
}

.bar__fill {
  position: relative;
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, var(--walnut), var(--moss) 65%, var(--moss-bright));
  transition: width 260ms var(--ease-out-quart);
  overflow: hidden;
}

.bar__shine {
  position: absolute;
  inset: 0;
  background: linear-gradient(
    90deg,
    transparent,
    rgba(255, 253, 246, 0.42),
    transparent
  );
  transform: translateX(-100%);
  animation: shine 1.8s var(--ease-in-out) infinite;
}

@keyframes shine {
  100% {
    transform: translateX(100%);
  }
}

.bar__fill--indeterminate {
  animation: slide 1.4s var(--ease-in-out) infinite;
}

@keyframes slide {
  0% {
    margin-left: -40%;
  }
  100% {
    margin-left: 100%;
  }
}
</style>
