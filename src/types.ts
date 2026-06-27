export type Mode = 'prompts' | 'flows'

export interface PromptItem {
  id: string
  title: string
  category: string
  content: string
  updated_at: string
}

export interface FlowItem {
  id: string
  title: string
  steps: string[]
  cursor: number
  updated_at: string
}

export interface PromptStore {
  prompts: PromptItem[]
  flows: FlowItem[]
}

export interface FlowHookStatus {
  codex_installed: boolean
  claude_installed: boolean
  codex_stale: boolean
  claude_stale: boolean
  codex_config_path: string
  claude_config_path: string
  script_path: string
  state_dir: string
}

export interface FlowHookInstallResult {
  client: 'codex' | 'claude'
  installed: boolean
  config_path: string
  script_path: string
  backup_path?: string | null
  next_step: string
}

export interface FlowRunLaunch {
  run_id: string
  first_prompt: string
}

export interface RankedItem {
  id: string
  title: string
  body: string
  score: number
}
