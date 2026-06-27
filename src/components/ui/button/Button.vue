<script setup lang="ts">
import { computed } from 'vue'
import { cn } from '@/lib/utils'

const props = withDefaults(
  defineProps<{
    variant?: 'default' | 'secondary' | 'ghost' | 'destructive'
    size?: 'sm' | 'md' | 'icon'
    class?: string
  }>(),
  {
    variant: 'default',
    size: 'md',
  },
)

const classes = computed(() =>
  cn(
    'inline-flex items-center justify-center gap-2 rounded-md text-sm font-medium transition-colors',
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[hsl(var(--ring))]',
    'disabled:pointer-events-none disabled:opacity-50',
    props.variant === 'default' &&
      'bg-[hsl(var(--primary))] text-[hsl(var(--primary-foreground))] hover:bg-[hsl(var(--primary))]/90',
    props.variant === 'secondary' &&
      'bg-[hsl(var(--secondary))] text-[hsl(var(--secondary-foreground))] hover:bg-[hsl(var(--secondary))]/85',
    props.variant === 'ghost' &&
      'text-[hsl(var(--foreground))] hover:bg-[hsl(var(--accent))]',
    props.variant === 'destructive' &&
      'bg-[hsl(var(--destructive))] text-[hsl(var(--destructive-foreground))] hover:bg-[hsl(var(--destructive))]/90',
    props.size === 'sm' && 'h-8 px-3',
    props.size === 'md' && 'h-9 px-4',
    props.size === 'icon' && 'h-8 w-8',
    props.class,
  ),
)
</script>

<template>
  <button :class="classes">
    <slot />
  </button>
</template>
