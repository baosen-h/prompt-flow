<script setup lang="ts">
import { cn } from '@/lib/utils'

defineProps<{
  class?: string
}>()

const model = defineModel<string>()
</script>

<template>
  <input
    v-model="model"
    :class="
      cn(
        'h-9 w-full rounded-md border border-[hsl(var(--input))] bg-[hsl(var(--background))] px-3 py-1 text-sm',
        'text-[hsl(var(--foreground))] placeholder:text-[hsl(var(--muted-foreground))]',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--ring))]',
        $props.class,
      )
    "
  />
</template>
