<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{ value: number; indeterminate?: boolean; height?: number }>(),
  { indeterminate: false, height: 6 }
);
const width = computed(() => (props.indeterminate ? 38 : Math.min(100, Math.max(0, props.value))));
</script>

<template>
  <div class="bar" :style="{ height: `${height}px` }">
    <div
      class="bar__fill"
      :class="{ 'bar__fill--indeterminate': indeterminate }"
      :style="{ width: `${width}%` }"
    />
  </div>
</template>

<style scoped>
.bar {
  width: 100%;
  border-radius: 99px;
  background: #e8ddc9;
  overflow: hidden;
}

.bar__fill {
  height: 100%;
  border-radius: 99px;
  background: linear-gradient(90deg, var(--grass), #86a56e);
  transition: width 0.2s ease;
}

.bar__fill--indeterminate {
  animation: slide 1.2s ease-in-out infinite;
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
