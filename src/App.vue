<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow, PhysicalSize } from '@tauri-apps/api/window'
import PromptPicker from '@/components/PromptPicker.vue'
import SettingsPage from '@/components/SettingsPage.vue'
import type { FlowHookInstallResult, FlowHookStatus, FlowItem, FlowRunLaunch, PromptItem, PromptStore } from '@/types'

type SettingsTab = 'prompts' | 'flows' | 'discover'
type View = 'picker' | 'settings'
type Theme = 'dark' | 'light'
type Language = 'en' | 'zh'

const prompts = ref<PromptItem[]>([])
const flows = ref<FlowItem[]>([])
const view = ref<View>('picker')
const settingsInitialTab = ref<SettingsTab>('prompts')
const pickerRef = ref<InstanceType<typeof PromptPicker> | null>(null)
const appWindow = getCurrentWindow()
const flowRunning = ref(false)
const hookStatus = ref<FlowHookStatus | null>(null)
const hookMessage = ref('')
const pickerMessage = ref('')
const theme = ref<Theme>('dark')
const language = ref<Language>('en')
const pickerSize = new PhysicalSize(360, 350)
const settingsSize = new PhysicalSize(800, 700)
let unlistenPickerOpened: UnlistenFn | null = null
let unlistenSettingsOpened: UnlistenFn | null = null

async function sizeAndCenter(size: PhysicalSize) {
  await appWindow.setSize(size)
  await appWindow.center()
}

async function loadStore() {
  const store = await invoke<PromptStore>('load_store')
  prompts.value = store.prompts
  flows.value = store.flows
}

async function loadHookStatus() {
  hookStatus.value = await invoke<FlowHookStatus>('flow_hook_status')
}

async function saveStore() {
  await invoke('save_store', {
    store: {
      prompts: prompts.value,
      flows: flows.value,
    },
  })
}

async function insertText(text: string) {
  await invoke('insert_text', { text, submit: false })
}

async function insertAndSubmitText(text: string) {
  await invoke('insert_text', { text, submit: true })
}

async function insertPrompt(prompt: PromptItem) {
  pickerMessage.value = ''
  try {
    await insertText(prompt.content)
  } catch (error) {
    pickerMessage.value = String(error)
  }
}

async function insertFlow(flow: FlowItem) {
  if (!flow.steps.length || flowRunning.value) return
  pickerMessage.value = ''

  // Flows need Stop hooks because the app cannot know when Codex or Claude has finished answering.
  if (!hasAutoFlowHook()) {
    hookMessage.value = 'Enable Auto Flow first. Install both Codex and Claude Stop hooks from Flow settings.'
    openSettings('flows')
    return
  }

  const steps = resolveFlowSteps(flow)
  if (!steps.length) return

  flowRunning.value = true
  try {
    // Later steps are sent by the Stop hook after each model answer finishes.
    const launch = await invoke<FlowRunLaunch>('start_flow_run', {
      flow: {
        title: flow.title,
        steps,
      },
    })
    await insertAndSubmitText(launch.first_prompt)
  } catch (error) {
    pickerMessage.value = String(error)
  } finally {
    flowRunning.value = false
  }
}

async function installHook(client: 'codex' | 'claude') {
  const result = await invoke<FlowHookInstallResult>('install_flow_hook', { client })
  await loadHookStatus()
  hookMessage.value = result.next_step
}

function hasAutoFlowHook() {
  return Boolean(hookStatus.value?.codex_installed && hookStatus.value?.claude_installed)
}

function resolvePromptStep(step: string) {
  const normalized = step.toLowerCase()
  return (
    prompts.value.find((entry) => entry.id === step) ??
    prompts.value.find((entry) => entry.title === step) ??
    prompts.value.find((entry) => entry.title.toLowerCase() === normalized)
  )
}

function resolveFlowSteps(flow: FlowItem) {
  return flow.steps
    .map((step) => resolvePromptStep(step)?.content ?? step.trim())
    .filter((step) => step.length > 0)
}

function upsertPrompt(prompt: PromptItem) {
  const promptIndex = prompts.value.findIndex((entry) => entry.id === prompt.id)
  if (promptIndex === -1) {
    prompts.value.unshift(prompt)
    return
  }
  prompts.value[promptIndex] = prompt
}

function upsertFlow(flow: FlowItem) {
  const flowIndex = flows.value.findIndex((entry) => entry.id === flow.id)
  if (flowIndex === -1) {
    flows.value.unshift(flow)
    return
  }
  flows.value[flowIndex] = flow
}

function removePromptFromFlows(prompt: PromptItem) {
  flows.value = flows.value.map((flow) => ({
    ...flow,
    steps: flow.steps.filter((step) => step !== prompt.id && step !== prompt.title),
    cursor: 0,
  }))
}

async function savePrompt(prompt: PromptItem) {
  const category = prompt.category.trim() || 'General'
  const title = prompt.title.trim()
  if (!title || !prompt.content.trim()) return
  const cleaned = { ...prompt, title, category }
  upsertPrompt(cleaned)
  await saveStore()
}

async function deletePrompt(id: string) {
  const prompt = prompts.value.find((entry) => entry.id === id)
  prompts.value = prompts.value.filter((entry) => entry.id !== id)
  if (prompt) removePromptFromFlows(prompt)
  await saveStore()
}

async function saveFlow(flow: FlowItem) {
  const title = flow.title.trim()
  if (!title) return
  const cleaned = {
    ...flow,
    title,
    steps: flow.steps
      .map((step) => resolvePromptStep(step)?.id ?? step.trim())
      .filter(Boolean),
  }
  upsertFlow(cleaned)
  await saveStore()
}

async function deleteFlow(id: string) {
  flows.value = flows.value.filter((entry) => entry.id !== id)
  await saveStore()
}

async function openSettings(initialTab: SettingsTab = 'prompts') {
  settingsInitialTab.value = initialTab
  view.value = 'settings'
  loadHookStatus()
  // Center after Vue swaps views so the native window uses the final rendered size.
  await nextTick()
  await sizeAndCenter(settingsSize)
}

async function closeSettings() {
  view.value = 'picker'
  // Recenter on every return; Windows can keep the previous settings-window position.
  await nextTick()
  await sizeAndCenter(pickerSize)
  nextTick(() => pickerRef.value?.focus())
}

function toggleTheme() {
  theme.value = theme.value === 'dark' ? 'light' : 'dark'
}

function toggleLanguage() {
  language.value = language.value === 'en' ? 'zh' : 'en'
}

onMounted(async () => {
  await sizeAndCenter(pickerSize)
  await loadStore()
  await loadHookStatus()
  unlistenPickerOpened = await listen('picker-opened', async () => {
    view.value = 'picker'
    await nextTick()
    await sizeAndCenter(pickerSize)
    nextTick(() => pickerRef.value?.focus())
  })
  unlistenSettingsOpened = await listen('settings-opened', async () => {
    settingsInitialTab.value = 'prompts'
    view.value = 'settings'
    loadHookStatus()
    await nextTick()
    await sizeAndCenter(settingsSize)
  })
  await nextTick()
  pickerRef.value?.focus()
})

onUnmounted(() => {
  unlistenPickerOpened?.()
  unlistenSettingsOpened?.()
})
</script>

<template>
  <div :class="['app-root', `theme-${theme}`]">
    <PromptPicker
      v-if="view === 'picker'"
      ref="pickerRef"
      :prompts="prompts"
      :flows="flows"
      :message="pickerMessage"
      @insert-prompt="insertPrompt"
      @insert-flow="insertFlow"
    />
    <SettingsPage
      v-else
      :prompts="prompts"
      :flows="flows"
      :hook-status="hookStatus"
      :hook-message="hookMessage"
      :initial-tab="settingsInitialTab"
      :theme="theme"
      :language="language"
      @save-prompt="savePrompt"
      @delete-prompt="deletePrompt"
      @save-flow="saveFlow"
      @delete-flow="deleteFlow"
      @install-hook="installHook"
      @toggle-theme="toggleTheme"
      @toggle-language="toggleLanguage"
      @close="closeSettings"
    />
  </div>
</template>
