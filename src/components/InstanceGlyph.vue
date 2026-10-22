<script setup lang="ts">
withDefaults(defineProps<{ preset: string; size?: number }>(), { size: 20 });
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    class="mc-glyph"
    aria-hidden="true"
  >
    <!-- 草方块 -->
    <template v-if="preset === 'grass'">
      <rect x="3" y="4" width="18" height="5" fill="#7da365" />
      <rect x="3" y="4" width="18" height="2" fill="#93b977" />
      <rect x="3" y="9" width="18" height="11" fill="#8a5f3f" />
      <rect x="3" y="9" width="18" height="1.5" fill="#6f4a30" />
      <rect x="6" y="12" width="2" height="2" fill="#79513a" />
      <rect x="11" y="15" width="2" height="2" fill="#6d4730" />
      <rect x="15" y="11" width="2" height="1" fill="#a07050" />
      <rect x="17" y="16" width="2" height="2" fill="#a07050" />
      <rect x="8" y="18" width="1" height="1" fill="#a07050" />
    </template>

    <template v-else-if="preset === 'diamond'">
      <path d="M4 9h16l-8 12z" fill="#4f958f" />
      <path d="M4 9l3-4h10l3 4z" fill="#7fc9c0" />
      <path d="M7 5l2 4H4z" fill="#a9ded8" />
      <path d="M4 9h5l3 12z" fill="#5aa8a1" />
      <path d="M15 9h5l-8 12z" fill="#3d7a75" />
      <path d="M9 9h6l-3 12z" fill="#6fbab3" />
    </template>

    <template v-else-if="preset === 'gold'">
      <path d="M6 10h12l3 5H3z" fill="#c99a3c" />
      <path d="M7 10l1-4h8l1 4z" fill="#e8c266" />
      <path d="M7 10h10l1.2 1.6H5.8z" fill="#f2dc9a" />
      <path d="M5 15h14l-.8 1.4H5.8z" fill="#9a7026" />
    </template>

    <template v-else-if="preset === 'redstone'">
      <rect x="3.5" y="3.5" width="17" height="17" fill="#7d6f66" />
      <rect x="3.5" y="3.5" width="17" height="3" fill="#92837a" />
      <rect x="6" y="8" width="3" height="3" fill="#c0392b" />
      <rect x="12" y="9" width="2" height="2" fill="#e05a45" />
      <rect x="15" y="7" width="2" height="2" fill="#b23225" />
      <rect x="8" y="13" width="2" height="2" fill="#e05a45" />
      <rect x="13" y="14" width="3" height="3" fill="#c0392b" />
      <rect x="17" y="13" width="2" height="2" fill="#8f2a20" />
      <rect x="5" y="16" width="2" height="2" fill="#b23225" />
    </template>

    <template v-else-if="preset === 'nether'">
      <rect x="3.5" y="3.5" width="17" height="17" fill="#8a4a3e" />
      <rect x="3.5" y="3.5" width="17" height="3" fill="#a35c4c" />
      <rect x="6" y="8" width="3" height="2" fill="#6d382f" />
      <rect x="12" y="6" width="2" height="3" fill="#6d382f" />
      <rect x="15" y="11" width="3" height="2" fill="#5b2f28" />
      <rect x="7" y="13" width="2" height="3" fill="#5b2f28" />
      <rect x="12" y="15" width="4" height="2" fill="#6d382f" />
      <rect x="18" y="8" width="1" height="2" fill="#a35c4c" />
    </template>

    <template v-else-if="preset === 'end'">
      <rect x="3.5" y="3.5" width="17" height="17" fill="#cfc894" />
      <rect x="3.5" y="3.5" width="17" height="3" fill="#e4dfb6" />
      <rect x="8" y="8" width="8" height="8" fill="#3a2a4e" />
      <rect x="9.5" y="9.5" width="5" height="5" fill="#7c5aa3" />
      <rect x="10.5" y="10.5" width="3" height="3" fill="#c6b2e2" />
      <rect x="6" y="6" width="1" height="1" fill="#b9b27f" />
      <rect x="16" y="15" width="2" height="2" fill="#b9b27f" />
      <rect x="14" y="6" width="1" height="1" fill="#b9b27f" />
    </template>

    <template v-else>
      <path d="M12 2.8 20.4 7.5v9L12 21.2 3.6 16.5v-9z" fill="#8b9d6f" />
      <path d="M12 2.8 20.4 7.5 12 12.2 3.6 7.5z" fill="#a9bd8c" />
      <path d="M12 12.2v9l8.4-4.7v-9z" fill="#71835a" />
    </template>
  </svg>
</template>

<style scoped>
.mc-glyph {
  display: block;
}
</style>
