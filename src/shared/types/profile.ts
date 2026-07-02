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

export interface TabletScannerRule {
  id: string
  name: string
  triggerKey: string
  targetExecutable: string
  columns: number
  rows: number
  grid: {
    x: number
    y: number
    width: number
    height: number
  }
  scanDelayMs: number
  craft: TabletCraftSettings
  valueRules: TabletValueRuleConfig[]
}

export interface ScreenPoint {
  x: number
  y: number
}

export interface TabletCraftSettings {
  transmutation: ScreenPoint
  augmentation: ScreenPoint
  regal: ScreenPoint
  exalted: ScreenPoint
  alchemy: ScreenPoint
  tabSwitchDelayMs: number
  craftDelayMs: number
}

export interface TabletValueRuleConfig {
  id: string
  label: string
  tabletMatch: string
  textMatch: string
  affixType: 'prefix' | 'suffix' | 'unknown'
  tier: 'S' | 'A' | 'B'
  score: number
  highRollAt?: number
}

export interface InventorySlotSnapshot {
  slot: string
  color: string
}

export interface TabletValueMod {
  text: string
  affixType: 'prefix' | 'suffix' | 'unknown'
  tier: 'S' | 'A' | 'B' | 'C'
  score: number
}

export interface TabletScanItem {
  slot: string
  column: number
  row: number
  name?: string
  tabletType: string
  rarity: string
  usesRemaining?: number
  valueTier: 'S' | 'A' | 'B' | 'C' | 'Low'
  valueScore: number
  prefixes: TabletValueMod[]
  suffixes: TabletValueMod[]
  unknownMods: string[]
  reasons: string[]
  rawText: string
}

export interface TabletScanReport {
  scannedSlots: number
  tablets: TabletScanItem[]
  skippedSlots: string[]
}

export interface TabletCraftAction {
  slot: string
  currency: 'transmutation' | 'augmentation' | 'regal' | 'exalted' | 'alchemy'
  reason: string
}

export interface TabletCraftReport {
  initialScan: TabletScanReport
  finalScan: TabletScanReport
  actions: TabletCraftAction[]
}

export interface TabletScanEvent {
  ruleId: string
  report: TabletScanReport
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
  tabletScannerRules: TabletScannerRule[]
}

export interface ProfileStore {
  activeProfileId: string
  profiles: AppProfile[]
}

export interface PixelSampleRequest {
  x: number
  y: number
}
