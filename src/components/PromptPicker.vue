<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { Search } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import type { FlowItem, Mode, PromptItem, RankedItem } from '@/types'

const props = defineProps<{
  prompts: PromptItem[]
  flows: FlowItem[]
  message: string
}>()

const emit = defineEmits<{
  insertPrompt: [prompt: PromptItem]
  insertFlow: [flow: FlowItem]
}>()

const mode = ref<Mode>('prompts')
const query = ref('')
const selected = ref(0)
const searchRef = ref<HTMLInputElement | null>(null)
const itemRefs = ref<HTMLElement[]>([])

const EMPTY_QUERY_SCORE = 1
const EXACT_MATCH_SCORE = 1000
const PREFIX_MATCH_SCORE = 800
const SUBSTRING_MATCH_SCORE = 500
const FUZZY_MATCH_BASE_SCORE = 80
const FUZZY_MATCH_MIN_SCORE = 8
const TITLE_SCORE_WEIGHT = 2
const CONTENT_PREVIEW_LENGTH = 160
const MAX_VISIBLE_RESULTS = 8

function scoreText(text: string, queryText: string) {
  if (!queryText) return EMPTY_QUERY_SCORE
  const source = text.toLowerCase()
  const needle = queryText.toLowerCase()
  if (source === needle) return EXACT_MATCH_SCORE
  if (source.startsWith(needle)) return PREFIX_MATCH_SCORE
  if (source.includes(needle)) return SUBSTRING_MATCH_SCORE

  // Fuzzy match keeps short partial searches useful without pulling in a search dependency.
  let cursor = 0
  let score = 0
  for (const char of needle) {
    const found = source.indexOf(char, cursor)
    if (found === -1) return 0
    score += Math.max(FUZZY_MATCH_MIN_SCORE, FUZZY_MATCH_BASE_SCORE - found)
    cursor = found + 1
  }
  return score
}

function sortedVisibleItems(items: RankedItem[]) {
  return items
    .filter((item) => item.score > 0)
    .sort((a, b) => b.score - a.score)
    .slice(0, MAX_VISIBLE_RESULTS)
}

const promptItems = computed<RankedItem[]>(() =>
  sortedVisibleItems(
    props.prompts.map((prompt) => {
      const score =
        scoreText(prompt.title, query.value) * TITLE_SCORE_WEIGHT +
        scoreText(prompt.category, query.value) +
        scoreText(prompt.content.slice(0, CONTENT_PREVIEW_LENGTH), query.value)
      return {
        id: prompt.id,
        title: prompt.title,
        body: prompt.content,
        score,
      }
    }),
  ),
)

const flowItems = computed<RankedItem[]>(() =>
  sortedVisibleItems(
    props.flows.map((flow) => {
      const stepNames = flow.steps.map((step) => stepLabelForFlow(step)).join(' -> ')
      return {
        id: flow.id,
        title: flow.title,
        body: stepNames,
        score: scoreText(flow.title, query.value) * TITLE_SCORE_WEIGHT + scoreText(stepNames, query.value),
      }
    }),
  ),
)

function stepLabelForFlow(step: string) {
  const normalized = step.toLowerCase()
  const prompt =
    props.prompts.find((entry) => entry.id === step) ??
    props.prompts.find((entry) => entry.title === step) ??
    props.prompts.find((entry) => entry.title.toLowerCase() === normalized)
  return prompt?.title ?? step
}

const items = computed(() => (mode.value === 'prompts' ? promptItems.value : flowItems.value))
const modeLabel = computed(() => (mode.value === 'prompts' ? 'prompt' : 'flow'))

watch([mode, query], () => {
  selected.value = 0
  itemRefs.value = []
  nextTick(scrollSelectedIntoView)
})

function focus() {
  nextTick(() => searchRef.value?.focus())
}

function switchMode() {
  mode.value = mode.value === 'prompts' ? 'flows' : 'prompts'
}

function move(delta: number) {
  if (!items.value.length) return
  // Wrap selection so keyboard use stays fast and never dead-ends at the list edges.
  selected.value = (selected.value + delta + items.value.length) % items.value.length
  nextTick(scrollSelectedIntoView)
}

function setItemRef(element: unknown, index: number) {
  if (element instanceof HTMLElement) {
    itemRefs.value[index] = element
  }
}

function scrollSelectedIntoView() {
  itemRefs.value[selected.value]?.scrollIntoView({
    block: 'nearest',
  })
}

function submit() {
  const item = items.value[selected.value]
  if (!item) return
  if (mode.value === 'prompts') {
    const prompt = props.prompts.find((entry) => entry.id === item.id)
    if (prompt) emit('insertPrompt', prompt)
    return
  }
  const flow = props.flows.find((entry) => entry.id === item.id)
  if (flow) emit('insertFlow', flow)
}

const keyActions: Record<string, () => void> = {
  Tab: switchMode,
  ArrowDown: () => move(1),
  ArrowUp: () => move(-1),
  Enter: submit,
  Escape: () => invoke('minimize_window'),
}

function onKeydown(event: KeyboardEvent) {
  const action = keyActions[event.key]
  if (!action) return
  event.preventDefault()
  action()
}

defineExpose({ focus })
</script>

<template>
  <section class="picker-shell" @keydown="onKeydown">
    <div class="picker-grip drag-region" />

    <div class="result-list no-drag">
      <div v-if="message" class="picker-message">{{ message }}</div>

      <button
        v-for="(item, index) in items"
        :key="item.id"
        :ref="(element) => setItemRef(element, index)"
        class="result-item"
        :class="{ selected: selected === index }"
        @mouseenter="selected = index"
        @click="submit"
      >
        <strong>{{ item.title }}</strong>
        <em>{{ item.body }}</em>
      </button>

      <div v-if="!items.length" class="empty-state">
        No {{ modeLabel }} found.
      </div>
    </div>

    <div class="picker-tail">
      <div class="mode-tabs no-drag" aria-label="Picker mode">
        <button :class="{ active: mode === 'prompts' }" @click="mode = 'prompts'">Prompt</button>
        <button :class="{ active: mode === 'flows' }" @click="mode = 'flows'">Flow</button>
      </div>

      <label class="search-row no-drag">
        <Search :size="16" />
        <input
          ref="searchRef"
          v-model="query"
          autocomplete="off"
          spellcheck="false"
          :placeholder="`Search ${modeLabel}`"
        />
      </label>
    </div>
  </section>
</template>
