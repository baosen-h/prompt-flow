<script setup lang="ts">
import { cn } from '@/lib/utils'

defineProps<{
  class?: string
}>()

const model = defineModel<string>()
</script>

<template>
  <textarea
    v-model="model"
    :class="
      cn(
        'min-h-28 w-full resize-none rounded-md border border-[hsl(var(--input))] bg-[hsl(var(--background))] px-3 py-2 text-sm',
        'text-[hsl(var(--foreground))] placeholder:text-[hsl(var(--muted-foreground))]',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--ring))]',
        $props.class,
      )
    "
  />
</template>
