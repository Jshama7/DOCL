<script setup lang="ts">
import autoImg from "../assets/icons/auto.png";
import grassImg from "../assets/icons/grass.png";
import diamondImg from "../assets/icons/diamond.png";
import goldImg from "../assets/icons/gold.png";
import redstoneImg from "../assets/icons/redstone.png";
import netherImg from "../assets/icons/nether.png";
import endImg from "../assets/icons/end.png";

withDefaults(defineProps<{ preset: string; size?: number }>(), { size: 20 });

/// 原版游戏贴图（Mojang 16x16 资源），按预设名映射
const IMAGES: Record<string, string> = {
  auto: autoImg,
  grass: grassImg,
  diamond: diamondImg,
  gold: goldImg,
  redstone: redstoneImg,
  nether: netherImg,
  end: endImg,
};
</script>

<template>
  <img
    v-if="IMAGES[preset]"
    :src="IMAGES[preset]"
    :width="size"
    :height="size"
    class="mc-glyph"
    alt=""
  />
  <svg
    v-else
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    class="mc-glyph"
    aria-hidden="true"
  >
    <path d="M12 2.8 20.4 7.5v9L12 21.2 3.6 16.5v-9z" fill="#8b9d6f" />
    <path d="M12 2.8 20.4 7.5 12 12.2 3.6 7.5z" fill="#a9bd8c" />
    <path d="M12 12.2v9l8.4-4.7v-9z" fill="#71835a" />
  </svg>
</template>

<style scoped>
.mc-glyph {
  display: block;
  border-radius: 3px;
  image-rendering: pixelated;
}
</style>
