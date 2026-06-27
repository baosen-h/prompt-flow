<script setup lang="ts">
defineProps<{
  size?: number
}>()
</script>

<template>
  <svg
    class="app-logo"
    :width="size ?? 24"
    :height="size ?? 24"
    viewBox="0 0 1024 1024"
    aria-hidden="true"
  >
    <rect x="120" y="120" width="784" height="784" rx="176" fill="#ffffff" />
    <rect
      x="120"
      y="120"
      width="784"
      height="784"
      rx="176"
      fill="none"
      stroke="#15131a"
      stroke-width="34"
    />
    <path
      d="M304 368L448 512L304 656"
      fill="none"
      stroke="#815cff"
      stroke-width="64"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <path
      d="M556 674H724M606 736H782"
      fill="none"
      stroke="#5ed8ce"
      stroke-width="36"
      stroke-linecap="round"
    />
    <text
      x="540"
      y="590"
      fill="#15131a"
      font-family="Inter, Segoe UI, Arial, sans-serif"
      font-size="292"
      font-weight="800"
      letter-spacing="0"
    >P</text>
  </svg>
</template>
