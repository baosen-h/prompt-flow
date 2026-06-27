<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { ChevronRight, ExternalLink, Folder, FolderOpen, Languages, Moon, PlugZap, Plus, Save, Sun, Trash2, X } from '@lucide/vue'
import type { FlowHookStatus, FlowItem, PromptItem } from '@/types'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Textarea } from '@/components/ui/textarea'
import AppLogo from '@/components/AppLogo.vue'

const props = defineProps<{
  prompts: PromptItem[]
  flows: FlowItem[]
  hookStatus: FlowHookStatus | null
  hookMessage: string
  initialTab: 'prompts' | 'flows' | 'discover'
  theme: 'dark' | 'light'
  language: 'en' | 'zh'
}>()

const emit = defineEmits<{
  savePrompt: [prompt: PromptItem]
  deletePrompt: [id: string]
  saveFlow: [flow: FlowItem]
  deleteFlow: [id: string]
  installHook: [client: 'codex' | 'claude']
  toggleTheme: []
  toggleLanguage: []
  close: []
}>()

const labels = {
  en: {
    subtitle: 'Manage prompts and simple prompt sequences.',
    prompts: 'Prompts',
    flows: 'Flows',
    discover: 'Discover',
    newCategory: 'New category',
    addCategory: 'Add category',
    addPrompt: 'Add Prompt',
    title: 'Title',
    category: 'Category',
    prompt: 'Prompt',
    save: 'Save',
    delete: 'Delete',
    autoFlow: 'Auto Flow',
    autoFlowHelp: 'Enable once for accurate step-by-step flows.',
    enableCodex: 'Enable Codex',
    enableClaude: 'Enable Claude',
    codexEnabled: 'Codex Enabled',
    claudeEnabled: 'Claude Enabled',
    addFlow: 'Add Flow',
    flowName: 'Flow name',
    addPromptStep: 'Add prompt step by category',
    addCustomStep: 'Add custom step',
    customStepPlaceholder: 'Write a one-off flow step',
    add: 'Add',
    noSteps: 'No steps yet.',
    stepCount: 'steps',
    close: 'Close settings',
    theme: 'Toggle theme',
    language: 'Switch language',
  },
  zh: {
    subtitle: '管理提示词和提示词流程。',
    prompts: '提示词',
    flows: '流程',
    discover: '发现',
    newCategory: '新分类',
    addCategory: '添加分类',
    addPrompt: '添加提示词',
    title: '标题',
    category: '分类',
    prompt: '提示词',
    save: '保存',
    delete: '删除',
    autoFlow: '自动流程',
    autoFlowHelp: '启用一次即可准确执行多步流程。',
    enableCodex: '启用 Codex',
    enableClaude: '启用 Claude',
    codexEnabled: 'Codex 已启用',
    claudeEnabled: 'Claude 已启用',
    addFlow: '添加流程',
    flowName: '流程名',
    addPromptStep: '按分类添加提示词步骤',
    addCustomStep: '添加自定义步骤',
    customStepPlaceholder: '写一个临时流程步骤',
    add: '添加',
    noSteps: '还没有步骤。',
    stepCount: '步',
    close: '关闭设置',
    theme: '切换主题',
    language: '切换语言',
  },
}

const text = computed(() => labels[props.language])

const DEFAULT_CATEGORY = 'General'
const NEW_PROMPT_TITLE = 'new-prompt'
const NEW_FLOW_TITLE = 'new-flow'

const tab = ref<'prompts' | 'flows' | 'discover'>(props.initialTab)
const selectedPromptId = ref('')
const selectedFlowId = ref('')
const currentPromptCategory = ref(DEFAULT_CATEGORY)
const localCategories = ref<string[]>([DEFAULT_CATEGORY])
const openCategories = ref(new Set<string>([DEFAULT_CATEGORY]))
const newCategoryName = ref('')

const blankPrompt = (): PromptItem => ({
  id: crypto.randomUUID(),
  title: NEW_PROMPT_TITLE,
  category: DEFAULT_CATEGORY,
  content: '',
  updated_at: new Date().toISOString(),
})

const blankFlow = (): FlowItem => ({
  id: crypto.randomUUID(),
  title: NEW_FLOW_TITLE,
  steps: [],
  cursor: 0,
  updated_at: new Date().toISOString(),
})

const draftPrompt = ref<PromptItem>(blankPrompt())
const draftFlow = ref<FlowItem>(blankFlow())
const flowStepCategory = ref(DEFAULT_CATEGORY)
const flowStepPromptId = ref('')
const customFlowStep = ref('')

const selectedPrompt = computed(() => props.prompts.find((prompt) => prompt.id === selectedPromptId.value))
const selectedFlow = computed(() => props.flows.find((flow) => flow.id === selectedFlowId.value))
const promptCategories = computed(() => {
  // Keep user-created empty categories visible even before they contain prompts.
  const categories = [
    ...localCategories.value,
    ...props.prompts.map((prompt) => normalizeCategory(prompt.category)),
  ]
  return [...new Set(categories)].sort((a, b) => a.localeCompare(b))
})
const promptsByCategory = computed(() =>
  promptCategories.value.map((category) => ({
    category,
    prompts: props.prompts
      .filter((prompt) => normalizeCategory(prompt.category) === category)
      .sort((a, b) => a.title.localeCompare(b.title)),
  })),
)
const flowStepPrompts = computed(() =>
  props.prompts.filter((prompt) => normalizeCategory(prompt.category) === flowStepCategory.value),
)

function normalizeCategory(category: string) {
  return category.trim() || DEFAULT_CATEGORY
}

function findPromptByStep(step: string) {
  const normalizedStep = step.toLowerCase()
  // Flows store prompt ids when possible, but old/manual steps may still contain titles.
  return (
    props.prompts.find((prompt) => prompt.id === step) ??
    props.prompts.find((prompt) => prompt.title === step) ??
    props.prompts.find((prompt) => prompt.title.toLowerCase() === normalizedStep)
  )
}

function ensureLocalCategory(category: string) {
  if (localCategories.value.includes(category)) return
  localCategories.value = [...localCategories.value, category]
}

watch(selectedPrompt, (prompt) => {
  draftPrompt.value = prompt ? { ...prompt } : blankPrompt()
  if (prompt) {
    currentPromptCategory.value = normalizeCategory(prompt.category)
    openCategory(currentPromptCategory.value)
  }
})

watch(selectedFlow, (flow) => {
  draftFlow.value = flow ? { ...flow, steps: [...flow.steps] } : blankFlow()
})

watch(
  () => props.prompts,
  (prompts) => {
    if (!promptCategories.value.includes(flowStepCategory.value)) {
      flowStepCategory.value = promptCategories.value[0] || DEFAULT_CATEGORY
    }
    if (!flowStepPrompts.value.some((prompt) => prompt.id === flowStepPromptId.value)) {
      flowStepPromptId.value = flowStepPrompts.value[0]?.id || prompts[0]?.id || ''
    }
  },
  { immediate: true },
)

watch(flowStepCategory, () => {
  flowStepPromptId.value = flowStepPrompts.value[0]?.id || ''
})

function newPrompt() {
  selectedPromptId.value = ''
  draftPrompt.value = { ...blankPrompt(), category: normalizeCategory(currentPromptCategory.value) }
  openCategory(draftPrompt.value.category)
}

function addCategory() {
  const category = newCategoryName.value.trim()
  if (!category) return
  ensureLocalCategory(category)
  currentPromptCategory.value = category
  openCategory(category)
  selectedPromptId.value = ''
  draftPrompt.value = { ...draftPrompt.value, category }
  newCategoryName.value = ''
}

function newFlow() {
  selectedFlowId.value = ''
  draftFlow.value = blankFlow()
}

function savePrompt() {
  emit('savePrompt', { ...draftPrompt.value, updated_at: new Date().toISOString() })
  selectedPromptId.value = draftPrompt.value.id
}

function saveFlow() {
  emit('saveFlow', { ...draftFlow.value, steps: [...draftFlow.value.steps], updated_at: new Date().toISOString() })
  selectedFlowId.value = draftFlow.value.id
}

function deletePrompt() {
  emit('deletePrompt', draftPrompt.value.id)
  newPrompt()
}

function isCategoryOpen(category: string) {
  return openCategories.value.has(category)
}

function openCategory(category: string) {
  openCategories.value = new Set([...openCategories.value, category])
}

function toggleCategory(category: string) {
  const next = new Set(openCategories.value)
  if (!next.delete(category)) next.add(category)
  openCategories.value = next
  currentPromptCategory.value = category
}

function selectPrompt(prompt: PromptItem) {
  currentPromptCategory.value = normalizeCategory(prompt.category)
  selectedPromptId.value = prompt.id
}

function deleteFlow() {
  emit('deleteFlow', draftFlow.value.id)
  newFlow()
}

function addFlowStep() {
  if (!flowStepPromptId.value) return
  draftFlow.value.steps.push(flowStepPromptId.value)
}

function addCustomFlowStep() {
  const step = customFlowStep.value.trim()
  if (!step) return
  draftFlow.value.steps.push(step)
  customFlowStep.value = ''
}

function removeFlowStep(index: number) {
  draftFlow.value.steps.splice(index, 1)
  // A deleted step can leave an old cursor past the end; reset before saving the flow.
  if (draftFlow.value.cursor >= draftFlow.value.steps.length) {
    draftFlow.value.cursor = 0
  }
}

const markets = [
  ['PromptBase', 'https://promptbase.com/'],
  ['FlowGPT', 'https://flowgpt.com/'],
  ['Snack Prompt', 'https://snackprompt.com/'],
  ['PromptHero', 'https://prompthero.com/'],
  ['AIPRM', 'https://www.aiprm.com/'],
]

function stepLabel(step: string) {
  const prompt = findPromptByStep(step)
  return prompt ? prompt.title : `${step} (missing)`
}

function openMarket(url: string) {
  invoke('open_external_url', { url })
}
</script>

<template>
  <section class="settings-shell">
    <header class="settings-header drag-region">
      <div class="settings-title">
        <AppLogo :size="32" />
        <div>
          <h1>prompt-flow</h1>
          <p>{{ text.subtitle }}</p>
        </div>
      </div>
      <div class="settings-actions">
        <Button class="no-drag" variant="ghost" size="icon" :title="text.theme" @click="emit('toggleTheme')">
          <Sun v-if="theme === 'dark'" :size="17" />
          <Moon v-else :size="17" />
        </Button>
        <Button class="no-drag" variant="ghost" size="icon" :title="text.language" @click="emit('toggleLanguage')">
          <Languages :size="17" />
        </Button>
        <Button class="no-drag close-only" variant="ghost" size="icon" :title="text.close" @click="emit('close')">
          <X :size="18" />
        </Button>
      </div>
    </header>

    <nav class="settings-tabs">
      <button :class="{ active: tab === 'prompts' }" @click="tab = 'prompts'">{{ text.prompts }}</button>
      <button :class="{ active: tab === 'flows' }" @click="tab = 'flows'">{{ text.flows }}</button>
      <button :class="{ active: tab === 'discover' }" @click="tab = 'discover'">{{ text.discover }}</button>
    </nav>

    <main v-if="tab === 'prompts'" class="manager-grid">
      <aside class="manager-list">
        <div class="category-create">
          <Input v-model="newCategoryName" :placeholder="text.newCategory" @keyup.enter="addCategory" />
          <Button type="button" variant="secondary" size="icon" class="boxed-action" :title="text.addCategory" @click="addCategory">
            <Folder :size="15" />
          </Button>
        </div>
        <Button variant="secondary" class="w-full boxed-action" @click="newPrompt">
          <Plus :size="15" />
          {{ text.addPrompt }}
        </Button>
        <div v-for="group in promptsByCategory" :key="group.category" class="category-folder">
          <button
            type="button"
            class="category-row"
            :class="{ active: currentPromptCategory === group.category }"
            @click="toggleCategory(group.category)"
          >
            <ChevronRight :size="14" class="folder-chevron" :class="{ open: isCategoryOpen(group.category) }" />
            <FolderOpen v-if="isCategoryOpen(group.category)" :size="15" />
            <Folder v-else :size="15" />
            <strong>{{ group.category }}</strong>
            <span>{{ group.prompts.length }}</span>
          </button>
          <div v-if="isCategoryOpen(group.category)" class="category-prompts">
            <button
              v-for="prompt in group.prompts"
              :key="prompt.id"
              type="button"
              class="prompt-leaf"
              :class="{ active: selectedPromptId === prompt.id }"
              @click="selectPrompt(prompt)"
            >
              {{ prompt.title }}
            </button>
          </div>
        </div>
      </aside>

      <form class="editor-panel prompt-editor" @submit.prevent="savePrompt">
        <div class="prompt-meta-row">
          <label>
            {{ text.title }}
            <Input v-model="draftPrompt.title" required />
          </label>
          <label>
            {{ text.category }}
            <Input v-model="draftPrompt.category" placeholder="General" />
          </label>
        </div>
        <label class="prompt-content-field">
          {{ text.prompt }}
          <Textarea v-model="draftPrompt.content" class="settings-prompt-textarea" required />
        </label>
        <div class="editor-actions compact-actions">
          <Button type="submit" size="sm" class="settings-save-button">
            <Save :size="15" />
            {{ text.save }}
          </Button>
          <Button
            v-if="selectedPrompt"
            type="button"
            variant="destructive"
            size="sm"
            class="settings-save-button"
            @click="deletePrompt"
          >
            <Trash2 :size="15" />
            {{ text.delete }}
          </Button>
        </div>
      </form>
    </main>

    <main v-else-if="tab === 'flows'" class="manager-grid">
      <aside class="manager-list">
        <div class="hook-panel">
          <strong>{{ text.autoFlow }}</strong>
          <span v-if="hookMessage">{{ hookMessage }}</span>
          <span v-else>{{ text.autoFlowHelp }}</span>
          <Button
            type="button"
            size="sm"
            :variant="hookStatus?.codex_installed ? 'secondary' : 'default'"
            @click="emit('installHook', 'codex')"
          >
            <PlugZap :size="14" />
            {{ hookStatus?.codex_installed ? text.codexEnabled : text.enableCodex }}
          </Button>
          <Button
            type="button"
            size="sm"
            :variant="hookStatus?.claude_installed ? 'secondary' : 'default'"
            @click="emit('installHook', 'claude')"
          >
            <PlugZap :size="14" />
            {{ hookStatus?.claude_installed ? text.claudeEnabled : text.enableClaude }}
          </Button>
        </div>
        <Button variant="secondary" class="w-full boxed-action" @click="newFlow">
          <Plus :size="15" />
          {{ text.addFlow }}
        </Button>
        <button
          v-for="flow in flows"
          :key="flow.id"
          class="manager-item"
          :class="{ active: selectedFlowId === flow.id }"
          @click="selectedFlowId = flow.id"
        >
          <strong>{{ flow.title }}</strong>
          <span>{{ flow.steps.length }} {{ text.stepCount }}</span>
        </button>
      </aside>

      <form class="editor-panel flow-editor" @submit.prevent="saveFlow">
        <label>
          {{ text.flowName }}
          <Input v-model="draftFlow.title" required />
        </label>
        <label>
          {{ text.addPromptStep }}
          <div class="category-prompt-row">
            <select v-model="flowStepCategory" class="select-input">
              <option v-for="category in promptCategories" :key="category" :value="category">
                {{ category }}
              </option>
            </select>
            <select v-model="flowStepPromptId" class="select-input">
              <option v-for="prompt in flowStepPrompts" :key="prompt.id" :value="prompt.id">
                {{ prompt.title }}
              </option>
            </select>
            <Button type="button" variant="secondary" class="boxed-action" @click="addFlowStep">{{ text.add }}</Button>
          </div>
        </label>
        <label>
          {{ text.addCustomStep }}
          <div class="inline-row">
            <Input v-model="customFlowStep" :placeholder="text.customStepPlaceholder" />
            <Button type="button" variant="secondary" class="boxed-action" @click="addCustomFlowStep">{{ text.add }}</Button>
          </div>
        </label>
        <div class="step-list flow-step-list">
          <button v-for="(step, index) in draftFlow.steps" :key="`${step}-${index}`" type="button">
            <span>{{ index + 1 }}. {{ stepLabel(step) }}</span>
            <Trash2 :size="14" @click.stop="removeFlowStep(index)" />
          </button>
          <div v-if="!draftFlow.steps.length" class="empty-steps">{{ text.noSteps }}</div>
        </div>
        <div class="editor-actions compact-actions">
          <Button type="submit" size="sm" class="settings-save-button">
            <Save :size="15" />
            {{ text.save }}
          </Button>
          <Button
            v-if="selectedFlow"
            type="button"
            variant="destructive"
            size="sm"
            class="settings-save-button"
            @click="deleteFlow"
          >
            <Trash2 :size="15" />
            {{ text.delete }}
          </Button>
        </div>
      </form>
    </main>

    <main v-else class="discover-grid">
      <button v-for="[name, url] in markets" :key="name" type="button" @click="openMarket(url)">
        <span>{{ name }}</span>
        <ExternalLink :size="15" />
      </button>
    </main>
  </section>
</template>
