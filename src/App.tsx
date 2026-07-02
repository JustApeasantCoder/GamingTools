import { useEffect, useMemo, useRef, useState } from 'react'
import {
  ChevronDown,
  ChevronUp,
  Gamepad2,
  Keyboard,
  MousePointer2,
  Play,
  Settings,
  Square,
  Tablet,
  Warehouse,
  ToggleRight,
} from 'lucide-react'
import './App.css'
import { callBackend } from './shared/api/client'
import type { AppProfile, MacroStep, PixelSampleRequest, ProfileStore, TabletCraftReport, TabletScanReport } from './shared/types/profile'
import { MacroBuilder } from './features/macros/MacroBuilder'
import { MacroInspector } from './features/macros/MacroInspector'
import { getProfileTimingIssues } from './features/macros/macroTiming'
import { PixelTrigger } from './features/pixel-trigger/PixelTrigger'
import { PixelActionInspector } from './features/pixel-trigger/PixelActionInspector'
import { getPixelRuleIssues } from './features/pixel-trigger/pixelRuleValidation'
import { ToggleHold } from './features/toggle-hold/ToggleHold'
import { getToggleHoldRuleIssues } from './features/toggle-hold/toggleHoldValidation'
import { InventoryStash } from './features/inventory-stash/InventoryStash'
import { TabletScanner } from './features/tablet-scanner/TabletScanner'
import { ProfileRail } from './features/profiles/ProfileRail'
import { Button } from './shared/ui/Button'
import { SettingsPanel } from './features/settings/SettingsPanel'
import { ProfileSettings } from './features/profiles/ProfileSettings'

type FeatureTab = 'macros' | 'pixels' | 'toggleHold' | 'inventoryStash' | 'tabletScanner' | 'profile' | 'settings'
type RuntimeEventPayload = {
  kind: string
  message: string
  ruleId?: string
  snapshotColors?: AppProfile['inventoryStashRules'][number]['snapshotColors']
}

const defaultStore: ProfileStore = {
  activeProfileId: 'default',
  profiles: [
    {
      id: 'default',
      name: 'Default Profile',
      defaultHumanization: {
        enabled: true,
        minMs: 100,
        maxMs: 220,
      },
      runtimeSettings: { toggleHotkey: 'F4', soundEnabled: true, foregroundGuard: { enabled: false, executable: '', onFocusLost: 'pause' } },
      macroRules: [
        {
          id: 'macro-default',
          name: 'Farming Loop',
          enabled: true,
          triggerKey: 'F6',
          steps: [
            { id: 'step-a', key: 'A', pressDuration: { enabled: true, minMs: 50, maxMs: 90 }, humanizedDelay: { enabled: true, minMs: 100, maxMs: 200 } },
            { id: 'step-b', key: 'B', pressDuration: { enabled: true, minMs: 60, maxMs: 100 }, humanizedDelay: { enabled: true, minMs: 150, maxMs: 250 } },
            { id: 'step-c', key: 'C', pressDuration: { enabled: true, minMs: 70, maxMs: 110 }, humanizedDelay: { enabled: true, minMs: 200, maxMs: 300 } },
          ],
        },
      ],
      pixelRules: [
        {
          id: 'pixel-default',
          name: 'Health Color Watch',
          enabled: true,
          targetColor: '#34d399',
          tolerance: 12,
          adjacentPixels: true,
          samplePoint: { x: 640, y: 360 },
          invertDetection: false,
          secondaryConditionEnabled: false,
          secondaryCondition: {
            targetColor: '#ffffff',
            tolerance: 12,
            adjacentPixels: false,
            samplePoint: { x: 640, y: 360 },
            invertDetection: false,
          },
          secondaryCondition2Enabled: false,
          secondaryCondition2: {
            targetColor: '#ffffff',
            tolerance: 12,
            adjacentPixels: false,
            samplePoint: { x: 640, y: 360 },
            invertDetection: false,
          },
          secondaryConditionOperator: 'and',
          triggerMode: 'hold',
          continueWhileDetected: true,
          actionSteps: [
            { id: 'pixel-step-q', key: 'Q', pressDuration: { enabled: true, minMs: 50, maxMs: 90 }, humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 } },
          ],
        },
      ],
      toggleHoldRules: [
        {
          id: 'toggle-default',
          name: 'Right Click Hold',
          enabled: true,
          triggerKey: 'F8',
          holdKey: 'RIGHT CLICK',
          releaseMode: 'off',
          releaseKey: '',
        },
      ],
      inventoryStashRules: [
        {
          id: 'inventory-stash-default',
          name: 'Inventory to stash',
          enabled: false,
          triggerKey: 'F6',
          captureBaselineKey: 'F8',
          detectionMode: 'emptyColor',
          columns: 12,
          rows: 5,
          grid: { x: 34, y: 37, width: 844, height: 352 },
          emptyColor: '#0f1110',
          ignoreWaystone: false,
          waystoneColor: '#7a52c8',
          tolerance: 18,
          ignoredSlots: [],
          waystoneSlots: [],
          snapshotColors: [],
          humanization: { enabled: true, minMs: 120, maxMs: 240 },
        },
      ],
      tabletScannerRules: [
        {
          id: 'tablet-scanner-default',
          name: 'Tablet stash scanner',
          triggerKey: 'F9',
          targetExecutable: '',
          columns: 12,
          rows: 12,
          grid: { x: 18, y: 126, width: 632, height: 632 },
          scanDelayMs: 90,
          craft: {
            transmutation: { x: 0, y: 0 },
            augmentation: { x: 0, y: 0 },
            regal: { x: 0, y: 0 },
            exalted: { x: 0, y: 0 },
            alchemy: { x: 0, y: 0 },
            tabSwitchDelayMs: 120,
            craftDelayMs: 90,
          },
          valueRules: [],
        },
      ],
    },
  ],
}

function App() {
  const [store, setStore] = useState<ProfileStore>(defaultStore)
  const [activeTab, setActiveTab] = useState<FeatureTab>('macros')
  const [isRunning, setIsRunning] = useState(false)
  const [selectedMacroId, setSelectedMacroId] = useState<string>('macro-default')
  const [selectedMacroStepId, setSelectedMacroStepId] = useState<string>('step-b')
  const [inspectorOpen, setInspectorOpen] = useState(false)
  const [pixelInspectorOpen, setPixelInspectorOpen] = useState(false)
  const [selectedPixelRuleId, setSelectedPixelRuleId] = useState<string>('pixel-default')
  const [selectedPixelStepId, setSelectedPixelStepId] = useState<string>('pixel-step-q')
  const [activityOpen, setActivityOpen] = useState(false)
  const [overviewOpen, setOverviewOpen] = useState(false)
  const [saveStatus, setSaveStatus] = useState<'saved' | 'saving' | 'error'>('saved')
  const didLoadProfiles = useRef(false)
  const saveQueue = useRef<Promise<void>>(Promise.resolve())
  const saveRevision = useRef(0)
  const [persistenceError, setPersistenceError] = useState<string>()
  const [logLines, setLogLines] = useState<string[]>([
    `${currentLogTime()} Ready. Global hotkey monitoring is active.`,
    `${currentLogTime()} Profile changes are saved automatically.`,
  ])
  const addLog = (message: string) => setLogLines((lines) => [`${currentLogTime()} ${message}`, ...lines].slice(0, 100))

  const activeProfile = useMemo(
    () => store.profiles.find((profile) => profile.id === store.activeProfileId) ?? store.profiles[0],
    [store],
  )

  const selectedMacro = activeProfile?.macroRules.find((macro) => macro.id === selectedMacroId)
    ?? activeProfile?.macroRules[0]

  const selectedStep = selectedMacro?.steps.find((step) => step.id === selectedMacroStepId)
    ?? selectedMacro?.steps[0]
  const selectedPixelRule = activeProfile?.pixelRules.find((rule) => rule.id === selectedPixelRuleId)
    ?? activeProfile?.pixelRules[0]
  const selectedPixelStep = selectedPixelRule?.actionSteps.find((step) => step.id === selectedPixelStepId)
    ?? selectedPixelRule?.actionSteps[0]
  const timingIssues = activeProfile ? getProfileTimingIssues(activeProfile) : []
  const pixelIssues = activeProfile?.pixelRules.filter((rule) => rule.enabled).flatMap(getPixelRuleIssues) ?? []
  const toggleHoldRuleIssues = activeProfile?.toggleHoldRules.map((rule) => ({
    rule,
    issues: getToggleHoldRuleIssues(rule, activeProfile.runtimeSettings.toggleHotkey),
  })) ?? []
  const enabledToggleHoldIssues = toggleHoldRuleIssues.filter(({ rule }) => rule.enabled).flatMap(({ issues }) => issues)
  const invalidToggleHoldRuleCount = toggleHoldRuleIssues.filter(({ issues }) => issues.length > 0).length
  const enabledRuleCount = activeProfile
    ? activeProfile.macroRules.filter((rule) => rule.enabled).length + activeProfile.pixelRules.filter((rule) => rule.enabled).length + activeProfile.toggleHoldRules.filter((rule) => rule.enabled).length + (activeProfile.inventoryStashRules ?? []).filter((rule) => rule.enabled).length
    : 0
  const startIssues = [...timingIssues.map((issue) => issue.message), ...pixelIssues, ...enabledToggleHoldIssues.map((issue) => issue.message)]

  useEffect(() => {
    if (didLoadProfiles.current) return
    didLoadProfiles.current = true

    callBackend<ProfileStore>('get_profiles')
      .then((loadedStore) => {
        setStore(loadedStore)
        const loadedProfile = loadedStore.profiles.find((profile) => profile.id === loadedStore.activeProfileId)
          ?? loadedStore.profiles[0]
        const firstMacroId = loadedProfile?.macroRules[0]?.id
        const firstStepId = loadedProfile?.macroRules[0]?.steps[0]?.id
        if (firstMacroId) setSelectedMacroId(firstMacroId)
        if (firstStepId) setSelectedMacroStepId(firstStepId)
      })
      .catch(() => {
        addLog('Using local preview profile until Tauri backend is available.')
      })
    callBackend<boolean>('is_runtime_running').then(setIsRunning).catch(() => undefined)
  }, [])

  useEffect(() => {
    let cancelled = false
    let unlisten: (() => void) | undefined
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen<RuntimeEventPayload>('runtime-event', (event) => {
        addLog(event.payload.message)
        if (event.payload.message.startsWith('Automation started')) setIsRunning(true)
        if (event.payload.message.startsWith('Automation stopped')) setIsRunning(false)
        if (event.payload.kind === 'inventorySnapshot' && event.payload.ruleId && event.payload.snapshotColors) {
          let updatedProfile: AppProfile | undefined
          setStore((current) => {
            const profiles = current.profiles.map((profile) => {
              const hasRule = profile.inventoryStashRules.some((rule) => rule.id === event.payload.ruleId)
              if (!hasRule) return profile
              updatedProfile = {
                ...profile,
                inventoryStashRules: profile.inventoryStashRules.map((rule) => rule.id === event.payload.ruleId
                  ? { ...rule, detectionMode: 'snapshot', snapshotColors: event.payload.snapshotColors ?? [] }
                  : rule),
              }
              return updatedProfile
            })
            return { ...current, profiles }
          })
          if (updatedProfile) {
            const revision = ++saveRevision.current
            setSaveStatus('saving')
            const operation = saveQueue.current.then(async () => {
              try {
                await callBackend('save_profile', { profile: updatedProfile })
                setPersistenceError(undefined)
                if (saveRevision.current === revision) setSaveStatus('saved')
              } catch (error) {
                const message = errorMessage(error)
                setPersistenceError(message)
                if (saveRevision.current === revision) setSaveStatus('error')
                addLog(`Could not save ${updatedProfile?.name ?? 'profile'}: ${message}`)
              }
            })
            saveQueue.current = operation
          }
        }
      }))
      .then((dispose) => {
        if (cancelled) {
          dispose()
        } else {
          unlisten = dispose
        }
      })
      .catch(() => undefined)
    return () => {
      cancelled = true
      unlisten?.()
    }
  }, [])

  const persistProfile = async (profile: AppProfile) => {
    const revision = ++saveRevision.current
    setSaveStatus('saving')
    const nextStore = {
      ...store,
      profiles: store.profiles.map((item) => (item.id === profile.id ? profile : item)),
    }
    setStore(nextStore)
    let saved = true
    const operation = saveQueue.current.then(async () => {
      try {
        await callBackend('save_profile', { profile })
        setPersistenceError(undefined)
        if (saveRevision.current === revision) setSaveStatus('saved')
      } catch (error) {
        saved = false
        const message = errorMessage(error)
        setPersistenceError(message)
        if (saveRevision.current === revision) setSaveStatus('error')
        addLog(`Could not save ${profile.name}: ${message}`)
      }
    })
    saveQueue.current = operation
    await operation
    return saved
  }

  const updateActiveProfile = async (profile: AppProfile) => {
    await persistProfile(profile)
  }

  const handleSaveActiveProfile = async () => {
    if (!activeProfile) return
    if (await persistProfile(activeProfile)) addLog(`Saved profile: ${activeProfile.name}`)
  }

  const handleDeleteActiveProfile = async () => {
    if (!activeProfile || store.profiles.length <= 1) return
    await saveQueue.current
    const fallbackStore = () => {
      const remainingProfiles = store.profiles.filter((profile) => profile.id !== activeProfile.id)
      return {
        activeProfileId: remainingProfiles[0]?.id ?? store.activeProfileId,
        profiles: remainingProfiles.length > 0 ? remainingProfiles : store.profiles,
      }
    }
    let backendStore: ProfileStore | undefined
    try {
      backendStore = await callBackend<ProfileStore>('delete_profile', { profileId: activeProfile.id })
    } catch (error) {
      addLog(`Could not delete ${activeProfile.name}: ${errorMessage(error)}`)
      return
    }
    const nextStore = backendStore?.profiles?.length ? backendStore : fallbackStore()
    setStore(nextStore)
    const firstStepId = nextStore.profiles.find((profile) => profile.id === nextStore.activeProfileId)?.macroRules[0]?.steps[0]?.id
    const firstMacroId = nextStore.profiles.find((profile) => profile.id === nextStore.activeProfileId)?.macroRules[0]?.id
    if (firstMacroId) setSelectedMacroId(firstMacroId)
    if (firstStepId) setSelectedMacroStepId(firstStepId)
    addLog(`Deleted profile: ${activeProfile.name}`)
  }

  const handleDuplicateActiveProfile = async () => {
    if (!activeProfile) return
    await saveQueue.current
    const duplicateProfile = duplicateProfileWithNewIds(activeProfile, store.profiles)
    try {
      await callBackend('save_profile', { profile: duplicateProfile })
      await callBackend('set_active_profile', { profileId: duplicateProfile.id })
    } catch (error) {
      addLog(`Could not duplicate ${activeProfile.name}: ${errorMessage(error)}`)
      return
    }
    setStore((current) => ({
      activeProfileId: duplicateProfile.id,
      profiles: [...current.profiles, duplicateProfile],
    }))
    const firstMacro = duplicateProfile.macroRules[0]
    if (firstMacro) setSelectedMacroId(firstMacro.id)
    if (firstMacro?.steps[0]) setSelectedMacroStepId(firstMacro.steps[0].id)
    addLog(`Duplicated profile: ${activeProfile.name}`)
  }

  const handleStart = async () => {
    if (!activeProfile) return
    if (!await persistProfile(activeProfile)) return
    try {
      await callBackend('start_runtime', { profileId: activeProfile.id })
      setIsRunning(true)
    } catch (error) {
      addLog(`Could not start automation: ${errorMessage(error)}`)
    }
  }

  const handleStop = async () => {
    try {
      await callBackend('stop_runtime')
      setIsRunning(false)
    } catch (error) {
      addLog(`Could not stop automation: ${errorMessage(error)}`)
    }
  }

  const handleProfileChange = async (profileId: string) => {
    await saveQueue.current
    const nextProfile = store.profiles.find((profile) => profile.id === profileId)
    try {
      await callBackend('set_active_profile', { profileId })
    } catch (error) {
      addLog(`Could not activate profile: ${errorMessage(error)}`)
      return
    }
    setStore({ ...store, activeProfileId: profileId })
    if (nextProfile?.macroRules[0]) {
      setSelectedMacroId(nextProfile.macroRules[0].id)
      if (nextProfile.macroRules[0].steps[0]) setSelectedMacroStepId(nextProfile.macroRules[0].steps[0].id)
    }
  }

  const handleSamplePixel = async (request: PixelSampleRequest) => {
    const result = await callBackend<{ color: string; x: number; y: number }>('sample_pixel', { request })
    addLog(`Sampled ${result.color} at ${result.x}, ${result.y}`)
    return result
  }

  const handlePickPixel = async () => {
    const result = await callBackend<{ color: string; x: number; y: number }>('pick_pixel')
    addLog(`Picked ${result.color} at ${result.x}, ${result.y}`)
    return result
  }

  const handleTestPixelRule = async (rule: AppProfile['pixelRules'][number]) => {
    const matches = await callBackend<boolean>('test_pixel_rule', { rule })
    addLog(`${rule.name} test: ${matches ? 'matching now' : 'not matching'}`)
    return matches
  }

  const handleTestPixelActions = async (rule: AppProfile['pixelRules'][number]) => {
    try {
      await callBackend('test_pixel_actions', { rule })
      addLog(`Tested actions for ${rule.name}`)
    } catch (error) {
      addLog(`Could not test ${rule.name}: ${errorMessage(error)}`)
      throw error
    }
  }

  const handleTestInventoryStashRule = async (rule: AppProfile['inventoryStashRules'][number]) => {
    const count = await callBackend<number>('test_inventory_stash_rule', { rule })
    addLog(`${rule.name} test: ${count} occupied slot${count === 1 ? '' : 's'} detected`)
    return count
  }

  const handleCaptureInventoryStashSnapshot = async (rule: AppProfile['inventoryStashRules'][number]) => {
    const snapshots = await callBackend<AppProfile['inventoryStashRules'][number]['snapshotColors']>('capture_inventory_stash_snapshot', { rule })
    addLog(`${rule.name} snapshot: ${snapshots.length} slot color${snapshots.length === 1 ? '' : 's'} captured`)
    return snapshots
  }

  const handleScanTabletStash = async (rule: AppProfile['tabletScannerRules'][number]) => {
    const report = await callBackend<TabletScanReport>('scan_tablet_stash', { rule })
    const valuableCount = report.tablets.filter((tablet) => tablet.valueTier !== 'Low').length
    addLog(`${rule.name} scan: ${report.tablets.length} tablet${report.tablets.length === 1 ? '' : 's'} found, ${valuableCount} worth checking`)
    return report
  }

  const handleScanAndCraftTablets = async (rule: AppProfile['tabletScannerRules'][number]) => {
    const report = await callBackend<TabletCraftReport>('scan_and_craft_tablets', { rule })
    addLog(`${rule.name} craft: ${report.actions.length} currency action${report.actions.length === 1 ? '' : 's'} completed`)
    return report
  }

  const handleHighlightTabletSlot = async (rule: AppProfile['tabletScannerRules'][number], slot: string) => {
    await callBackend('highlight_tablet_slot', { rule, slot })
    addLog(`${rule.name}: highlighted slot ${slot}`)
  }

  const handleMoveTabletToInventory = async (rule: AppProfile['tabletScannerRules'][number], slot: string) => {
    await callBackend('move_tablet_to_inventory', { rule, slot })
    addLog(`${rule.name}: moved slot ${slot} to inventory`)
  }

  const handleGetForegroundApp = async () => {
    return callBackend<{ executable: string; path: string }>('get_foreground_app')
  }

  const handleAddProfile = async () => {
    await saveQueue.current
    const id = crypto.randomUUID()
    const newProfile: AppProfile = {
      id,
      name: `Profile ${store.profiles.length + 1}`,
      defaultHumanization: { enabled: true, minMs: 90, maxMs: 180 },
      runtimeSettings: { toggleHotkey: 'F4', soundEnabled: true, foregroundGuard: { enabled: false, executable: '', onFocusLost: 'pause' } },
      macroRules: [
        {
          id: crypto.randomUUID(),
          name: 'New Macro',
          enabled: true,
          triggerKey: 'F7',
          steps: [{ id: crypto.randomUUID(), key: 'A', pressDuration: { enabled: true, minMs: 50, maxMs: 90 }, humanizedDelay: { enabled: true, minMs: 80, maxMs: 160 } }],
        },
      ],
      pixelRules: [],
      toggleHoldRules: [],
      inventoryStashRules: [],
      tabletScannerRules: [],
    }
    try {
      await callBackend('save_profile', { profile: newProfile })
      await callBackend('set_active_profile', { profileId: id })
    } catch (error) {
      addLog(`Could not add profile: ${errorMessage(error)}`)
      return
    }
    setStore((current) => ({ activeProfileId: id, profiles: [...current.profiles, newProfile] }))
  }

  const updateSelectedStep = (step: MacroStep) => {
    if (!activeProfile) return
    const macro = activeProfile.macroRules[0]
    const activeMacro = selectedMacro ?? macro
    updateActiveProfile({
      ...activeProfile,
      macroRules: [
        ...activeProfile.macroRules.map((item) => item.id === activeMacro.id
          ? { ...activeMacro, steps: activeMacro.steps.map((existingStep) => (existingStep.id === step.id ? step : existingStep)) }
          : item),
      ],
    })
  }

  const updateSelectedPixelStep = (step: MacroStep) => {
    if (!activeProfile || !selectedPixelRule) return
    updateActiveProfile({
      ...activeProfile,
      pixelRules: activeProfile.pixelRules.map((rule) => rule.id === selectedPixelRule.id
        ? { ...selectedPixelRule, actionSteps: selectedPixelRule.actionSteps.map((item) => item.id === step.id ? step : item) }
        : rule),
    })
  }

  if (!activeProfile) {
    return <main className="empty-state">No profile is available.</main>
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark"><Gamepad2 size={24} /></div>
          <div>
            <strong>Gaming Toolkit</strong>
            <span>v0.1.0</span>
          </div>
        </div>

        <nav className="nav-group" aria-label="Feature navigation">
          <span className="nav-label">Navigation</span>
          <button className={activeTab === 'macros' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('macros')}>
            <Keyboard size={18} /> Macro Builder
          </button>
          <button className={activeTab === 'pixels' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('pixels')}>
            <MousePointer2 size={18} /> Pixel Trigger
          </button>
          <button className={activeTab === 'toggleHold' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('toggleHold')}>
            <ToggleRight size={18} /> Toggle Hold
            {invalidToggleHoldRuleCount > 0 ? <span className="nav-count">{invalidToggleHoldRuleCount}</span> : null}
          </button>
          <button className={activeTab === 'inventoryStash' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('inventoryStash')}>
            <Warehouse size={18} /> Inventory Stash
          </button>
          <button className={activeTab === 'tabletScanner' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('tabletScanner')}>
            <Tablet size={18} /> Tablet Scanner
          </button>
        </nav>

        <ProfileRail
          profiles={store.profiles}
          activeProfileId={store.activeProfileId}
          onSelect={handleProfileChange}
          onAdd={handleAddProfile}
          onSave={handleSaveActiveProfile}
          onDuplicate={handleDuplicateActiveProfile}
          onDelete={handleDeleteActiveProfile}
          canDelete={store.profiles.length > 1}
          isConfiguring={activeTab === 'profile'}
          onConfigure={() => setActiveTab('profile')}
        />

        <div className="sidebar-footer">
          <button className={activeTab === 'settings' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('settings')}>
            <Settings size={18} /> Settings
          </button>
          <div className="connection"><span /> Foreground-only automation</div>
        </div>
      </aside>

      <section className={activityOpen || overviewOpen ? 'workspace activity-open' : 'workspace'}>
        <header className="topbar">
          <div className="status-block">
            <span className={isRunning ? 'status-dot running' : 'status-dot'} />
            <div>
              <strong>{isRunning ? 'Running' : 'Ready'}</strong>
              <span>{persistenceError ? `Changes not saved: ${persistenceError}` : isRunning ? `${activeProfile.name} is active` : invalidToggleHoldRuleCount > 0 ? `${invalidToggleHoldRuleCount} Toggle Hold ${invalidToggleHoldRuleCount === 1 ? 'rule needs' : 'rules need'} attention` : startIssues.length > 0 ? `${enabledRuleCount} enabled rules · ${startIssues.length} need attention` : `${enabledRuleCount} enabled rules · Ready to start`}</span>
            </div>
          </div>
          <div className="runtime-actions">
            <Button
              variant="primary"
              icon={Play}
              onClick={handleStart}
              disabled={isRunning || startIssues.length > 0}
              title={startIssues[0]}
            >
              Start automation
            </Button>
            <Button variant="danger" icon={Square} onClick={handleStop} disabled={!isRunning}>Stop automation</Button>
          </div>
        </header>

        <div className={activeTab === 'macros' || activeTab === 'pixels' ? 'content-grid' : 'content-grid content-grid-full'}>
          <section className="main-panel">
            {activeTab === 'macros' ? (
              <MacroBuilder
                profile={activeProfile}
                saveStatus={saveStatus}
                selectedMacroId={selectedMacro?.id}
                selectedStepId={selectedStep?.id}
                onSelectedMacroChange={setSelectedMacroId}
                onSelectedStepChange={(stepId) => {
                  setSelectedMacroStepId(stepId)
                  setInspectorOpen(true)
                }}
                onProfileChange={updateActiveProfile}
              />
            ) : activeTab === 'pixels' ? (
              <PixelTrigger
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onSamplePixel={handleSamplePixel}
                onPickPixel={handlePickPixel}
                onTestRule={handleTestPixelRule}
                onTestActions={handleTestPixelActions}
                selectedRuleId={selectedPixelRule?.id}
                selectedStepId={selectedPixelStep?.id}
                onSelectedRuleChange={setSelectedPixelRuleId}
                onSelectedStepChange={(stepId) => {
                  setSelectedPixelStepId(stepId)
                  setPixelInspectorOpen(true)
                }}
              />
            ) : activeTab === 'toggleHold' ? (
              <ToggleHold
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
              />
            ) : activeTab === 'inventoryStash' ? (
              <InventoryStash
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onPickPixel={handlePickPixel}
                onSamplePixel={handleSamplePixel}
                onTestRule={handleTestInventoryStashRule}
                onCaptureSnapshot={handleCaptureInventoryStashSnapshot}
              />
            ) : activeTab === 'tabletScanner' ? (
              <TabletScanner
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onScan={handleScanTabletStash}
                onScanAndCraft={handleScanAndCraftTablets}
                onHighlightSlot={handleHighlightTabletSlot}
                onMoveToInventory={handleMoveTabletToInventory}
                onGetForegroundApp={handleGetForegroundApp}
              />
            ) : activeTab === 'profile' ? (
              <ProfileSettings
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
              />
            ) : (
              <SettingsPanel
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onSaveProfile={handleSaveActiveProfile}
                onImported={(nextStore) => {
                  setStore(nextStore)
                  const imported = nextStore.profiles.find((profile) => profile.id === nextStore.activeProfileId)
                  if (imported?.macroRules[0]) {
                    setSelectedMacroId(imported.macroRules[0].id)
                    if (imported.macroRules[0].steps[0]) setSelectedMacroStepId(imported.macroRules[0].steps[0].id)
                  }
                }}
              />
            )}
          </section>

          {activeTab === 'macros' ? (
            <MacroInspector
              macro={selectedMacro}
              step={selectedStep}
              open={inspectorOpen}
              onClose={() => setInspectorOpen(false)}
              onStepChange={updateSelectedStep}
            />
          ) : activeTab === 'pixels' ? (
            <PixelActionInspector
              rule={selectedPixelRule}
              step={selectedPixelStep}
              open={pixelInspectorOpen}
              onClose={() => setPixelInspectorOpen(false)}
              onStepChange={updateSelectedPixelStep}
            />
          ) : null}
        </div>

        <footer className="bottom-grid">
          <section className="log-panel">
            <button className="log-toggle" onClick={() => setActivityOpen((open) => !open)} aria-expanded={activityOpen}>
              <span className="panel-title">Activity Log</span>
              <span className="latest-log">{logLines[0]}</span>
              {activityOpen ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
            </button>
            {activityOpen ? logLines.slice(0, 5).map((line, index) => (
              <code key={`${line}-${index}`}>{line}</code>
            )) : null}
          </section>
          <section className="stats-panel">
            <button className="log-toggle" onClick={() => setOverviewOpen((open) => !open)} aria-expanded={overviewOpen}>
              <span className="panel-title">Overview</span>
              <span className="latest-log">{enabledRuleCount} enabled rules · {isRunning ? 'Running' : 'Stopped'}</span>
              {overviewOpen ? <ChevronDown size={16} /> : <ChevronUp size={16} />}
            </button>
            {overviewOpen ? (
              <dl>
                <dt>Macros</dt><dd>{activeProfile.macroRules.length}</dd>
                <dt>Pixel triggers</dt><dd>{activeProfile.pixelRules.length}</dd>
                <dt>Toggle Hold</dt><dd>{activeProfile.toggleHoldRules.length}</dd>
                <dt>Inventory stash</dt><dd>{activeProfile.inventoryStashRules?.length ?? 0}</dd>
                <dt>Tablet scanner</dt><dd>{activeProfile.tabletScannerRules?.length ?? 0}</dd>
                <dt>Automation</dt><dd>{isRunning ? 'Running' : 'Stopped'}</dd>
              </dl>
            ) : null}
          </section>
        </footer>
      </section>
    </main>
  )
}

export default App

function duplicateProfileWithNewIds(profile: AppProfile, profiles: AppProfile[]): AppProfile {
  const baseName = `${profile.name} Copy`
  let name = baseName
  let suffix = 2
  while (profiles.some((item) => item.name === name)) {
    name = `${baseName} ${suffix}`
    suffix += 1
  }

  return {
    ...structuredClone(profile),
    id: crypto.randomUUID(),
    name,
    macroRules: profile.macroRules.map((macro) => ({
      ...macro,
      id: crypto.randomUUID(),
      steps: macro.steps.map((step) => ({ ...step, id: crypto.randomUUID() })),
    })),
    pixelRules: profile.pixelRules.map((rule) => ({
      ...rule,
      id: crypto.randomUUID(),
      actionSteps: rule.actionSteps.map((step) => ({ ...step, id: crypto.randomUUID() })),
    })),
    toggleHoldRules: profile.toggleHoldRules.map((rule) => ({ ...rule, id: crypto.randomUUID() })),
    inventoryStashRules: (profile.inventoryStashRules ?? []).map((rule) => ({ ...rule, id: crypto.randomUUID() })),
    tabletScannerRules: (profile.tabletScannerRules ?? []).map((rule) => ({ ...rule, id: crypto.randomUUID() })),
  }
}

function currentLogTime() {
  return new Date().toLocaleTimeString([], { hour12: false })
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error)
}
