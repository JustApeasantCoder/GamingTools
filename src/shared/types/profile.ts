export interface HumanizationSettings {
  enabled: boolean
  minMs: number
  maxMs: number
}

export interface MacroStep {
  id: string
  key: string
  pressDuration: HumanizationSettings
  humanizedDelay: HumanizationSettings
}

export interface MacroRule {
  id: string
  name: string
  enabled: boolean
  triggerKey: string
  steps: MacroStep[]
}

export interface PixelPoint {
  x: number
  y: number
}

export interface PixelCondition {
  targetColor: string
  tolerance: number
  adjacentPixels: boolean
  samplePoint: PixelPoint
  invertDetection: boolean
}

export interface PixelRule {
  id: string
  name: string
  enabled: boolean
  targetColor: string
  tolerance: number
  adjacentPixels: boolean
  samplePoint: PixelPoint
  invertDetection: boolean
  secondaryConditionEnabled: boolean
  secondaryCondition: PixelCondition
  secondaryCondition2Enabled: boolean
  secondaryCondition2: PixelCondition
  secondaryConditionOperator: 'and' | 'or'
  triggerMode: 'trigger' | 'hold'
  continueWhileDetected: boolean
  actionSteps: MacroStep[]
  outputKey?: string
}

export interface ToggleHoldRule {
  id: string
  name: string
  enabled: boolean
  triggerKey: string
  holdKey: string
  releaseMode: 'off' | 'anyOther' | 'specific'
  releaseKey: string
}

export interface InventoryStashRule {
  id: string
  name: string
  enabled: boolean
  triggerKey: string
  captureBaselineKey: string
  detectionMode: 'emptyColor' | 'snapshot'
  columns: number
  rows: number
  grid: {
    x: number
    y: number
    width: number
    height: number
  }
  emptyColor: string
  ignoreWaystone: boolean
  waystoneColor: string
  tolerance: number
  ignoredSlots: string[]
  waystoneSlots: string[]
  snapshotColors: InventorySlotSnapshot[]
  humanization: HumanizationSettings
}

export interface InventorySlotSnapshot {
  slot: string
  color: string
}

export interface AppProfile {
  id: string
  name: string
  defaultHumanization: HumanizationSettings
  runtimeSettings: {
    toggleHotkey: string
    soundEnabled: boolean
    foregroundGuard: {
      enabled: boolean
      executable: string
      onFocusLost: 'pause' | 'stop'
    }
  }
  macroRules: MacroRule[]
  pixelRules: PixelRule[]
  toggleHoldRules: ToggleHoldRule[]
  inventoryStashRules: InventoryStashRule[]
}

export interface ProfileStore {
  activeProfileId: string
  profiles: AppProfile[]
}

export interface PixelSampleRequest {
  x: number
  y: number
}
