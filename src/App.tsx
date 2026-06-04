import { useEffect, useMemo, useRef, useState } from 'react'
import {
  Activity,
  Gamepad2,
  Keyboard,
  MousePointer2,
  Play,
  Settings,
  Square,
  ToggleRight,
} from 'lucide-react'
import './App.css'
import { callBackend } from './shared/api/client'
import type { AppProfile, MacroStep, PixelSampleRequest, ProfileStore } from './shared/types/profile'
import { MacroBuilder } from './features/macros/MacroBuilder'
import { PixelTrigger } from './features/pixel-trigger/PixelTrigger'
import { ToggleHold } from './features/toggle-hold/ToggleHold'
import { ProfileRail } from './features/profiles/ProfileRail'
import { Button } from './shared/ui/Button'
import { KeyCaptureButton } from './shared/ui/KeyCaptureButton'
import { SettingsPanel } from './features/settings/SettingsPanel'

type FeatureTab = 'macros' | 'pixels' | 'toggleHold' | 'settings'

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
      runtimeSettings: { toggleHotkey: 'F4', soundEnabled: true },
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
          triggerKey: 'RIGHT CLICK',
          holdKey: 'RIGHT CLICK',
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
  const didLoadProfiles = useRef(false)
  const [logLines, setLogLines] = useState<string[]>([
    `${currentLogTime()} Ready. Global input monitoring enabled.`,
    `${currentLogTime()} Profiles are stored in the app config folder.`,
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
  }, [])

  useEffect(() => {
    let cancelled = false
    let unlisten: (() => void) | undefined
    import('@tauri-apps/api/event')
      .then(({ listen }) => listen<{ kind: string; message: string }>('runtime-event', (event) => {
        addLog(event.payload.message)
        if (event.payload.message.startsWith('Runtime started')) setIsRunning(true)
        if (event.payload.message === 'Runtime stopped') setIsRunning(false)
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
    const nextStore = {
      ...store,
      profiles: store.profiles.map((item) => (item.id === profile.id ? profile : item)),
    }
    setStore(nextStore)
    await callBackend('save_profile', { profile }).catch(() => undefined)
  }

  const updateActiveProfile = async (profile: AppProfile) => {
    await persistProfile(profile)
  }

  const handleSaveActiveProfile = async () => {
    if (!activeProfile) return
    await persistProfile(activeProfile)
    addLog(`Saved profile: ${activeProfile.name}`)
  }

  const handleDeleteActiveProfile = async () => {
    if (!activeProfile || store.profiles.length <= 1) return
    const fallbackStore = () => {
      const remainingProfiles = store.profiles.filter((profile) => profile.id !== activeProfile.id)
      return {
        activeProfileId: remainingProfiles[0]?.id ?? store.activeProfileId,
        profiles: remainingProfiles.length > 0 ? remainingProfiles : store.profiles,
      }
    }
    const backendStore = await callBackend<ProfileStore>('delete_profile', { profileId: activeProfile.id })
      .catch(() => undefined)
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
    const duplicateProfile = duplicateProfileWithNewIds(activeProfile, store.profiles)
    setStore((current) => ({
      activeProfileId: duplicateProfile.id,
      profiles: [...current.profiles, duplicateProfile],
    }))
    await callBackend('save_profile', { profile: duplicateProfile }).catch(() => undefined)
    await callBackend('set_active_profile', { profileId: duplicateProfile.id }).catch(() => undefined)
    const firstMacro = duplicateProfile.macroRules[0]
    if (firstMacro) setSelectedMacroId(firstMacro.id)
    if (firstMacro?.steps[0]) setSelectedMacroStepId(firstMacro.steps[0].id)
    addLog(`Duplicated profile: ${activeProfile.name}`)
  }

  const handleStart = async () => {
    if (!activeProfile) return
    await callBackend('start_runtime', { profileId: activeProfile.id })
    setIsRunning(true)
  }

  const handleStop = async () => {
    await callBackend('stop_runtime')
    setIsRunning(false)
  }

  const handleProfileChange = async (profileId: string) => {
    const nextProfile = store.profiles.find((profile) => profile.id === profileId)
    const nextStore = { ...store, activeProfileId: profileId }
    setStore(nextStore)
    if (nextProfile?.macroRules[0]) {
      setSelectedMacroId(nextProfile.macroRules[0].id)
      if (nextProfile.macroRules[0].steps[0]) setSelectedMacroStepId(nextProfile.macroRules[0].steps[0].id)
    }
    await callBackend('set_active_profile', { profileId }).catch(() => undefined)
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

  const handleAddProfile = async () => {
    const id = crypto.randomUUID()
    const newProfile: AppProfile = {
      id,
      name: `Profile ${store.profiles.length + 1}`,
      defaultHumanization: { enabled: true, minMs: 90, maxMs: 180 },
      runtimeSettings: { toggleHotkey: 'F4', soundEnabled: true },
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
    }
    setStore((current) => ({ activeProfileId: id, profiles: [...current.profiles, newProfile] }))
    await callBackend('save_profile', { profile: newProfile }).catch(() => undefined)
    await callBackend('set_active_profile', { profileId: id }).catch(() => undefined)
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
        />

        <div className="sidebar-footer">
          <button className={activeTab === 'settings' ? 'nav-item active' : 'nav-item'} onClick={() => setActiveTab('settings')}>
            <Settings size={18} /> Settings
          </button>
          <div className="connection"><span /> Backend connected</div>
        </div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div className="status-block">
            <span className={isRunning ? 'status-dot running' : 'status-dot'} />
            <div>
              <strong>{isRunning ? 'Running' : 'Ready'}</strong>
              <span>{isRunning ? `${activeProfile.name} is active` : 'No macros running'}</span>
            </div>
          </div>
          <div className="runtime-actions">
            <Button variant="primary" icon={Play} onClick={handleStart} disabled={isRunning}>Start</Button>
            <Button variant="danger" icon={Square} onClick={handleStop} disabled={!isRunning}>Stop</Button>
          </div>
        </header>

        <div className={activeTab === 'macros' ? 'content-grid' : 'content-grid content-grid-full'}>
          <section className="main-panel">
            {activeTab === 'macros' ? (
              <MacroBuilder
                profile={activeProfile}
                selectedMacroId={selectedMacro?.id}
                selectedStepId={selectedStep?.id}
                onSelectedMacroChange={setSelectedMacroId}
                onSelectedStepChange={setSelectedMacroStepId}
                onProfileChange={updateActiveProfile}
              />
            ) : activeTab === 'pixels' ? (
              <PixelTrigger
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onSamplePixel={handleSamplePixel}
                onPickPixel={handlePickPixel}
              />
            ) : activeTab === 'toggleHold' ? (
              <ToggleHold
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
              />
            ) : (
              <SettingsPanel
                profile={activeProfile}
                onProfileChange={updateActiveProfile}
                onSaveProfile={handleSaveActiveProfile}
              />
            )}
          </section>

          {activeTab === 'macros' ? <aside className="inspector">
            <div className="inspector-heading">
              <span>Step Settings</span>
              <strong>{selectedStep && selectedMacro ? `${selectedMacro.name} / Step ${selectedMacro.steps.findIndex((item) => item.id === selectedStep.id) + 1}` : 'No step'}</strong>
            </div>
            {selectedStep ? (
              <div className="inspector-form">
                <label>
                  Key
                  <KeyCaptureButton value={selectedStep.key} label="Listen" onChange={(key) => updateSelectedStep({ ...selectedStep, key })} />
                </label>
                <div className="two-col">
                  <label>Press min ms<input type="number" value={selectedStep.pressDuration.minMs} onChange={(event) => updateSelectedStep({ ...selectedStep, pressDuration: { ...selectedStep.pressDuration, minMs: Number(event.target.value) } })} /></label>
                  <label>Press max ms<input type="number" value={selectedStep.pressDuration.maxMs} onChange={(event) => updateSelectedStep({ ...selectedStep, pressDuration: { ...selectedStep.pressDuration, maxMs: Number(event.target.value) } })} /></label>
                </div>
                <label className="switch-row">
                  <span>Humanized timing</span>
                  <input type="checkbox" checked={selectedStep.humanizedDelay.enabled} onChange={(event) => updateSelectedStep({ ...selectedStep, humanizedDelay: { ...selectedStep.humanizedDelay, enabled: event.target.checked } })} />
                </label>
                <div className="two-col">
                  <label>Min ms<input type="number" value={selectedStep.humanizedDelay.minMs} onChange={(event) => updateSelectedStep({ ...selectedStep, humanizedDelay: { ...selectedStep.humanizedDelay, minMs: Number(event.target.value) } })} /></label>
                  <label>Max ms<input type="number" value={selectedStep.humanizedDelay.maxMs} onChange={(event) => updateSelectedStep({ ...selectedStep, humanizedDelay: { ...selectedStep.humanizedDelay, maxMs: Number(event.target.value) } })} /></label>
                </div>
                <div className="notice"><Activity size={16} /> Delay applies after key release before the next step.</div>
              </div>
            ) : null}
          </aside> : null}
        </div>

        <footer className="bottom-grid">
          <section className="log-panel">
            <div className="panel-title">Execution Log</div>
            {logLines.slice(0, 5).map((line, index) => (
              <code key={`${line}-${index}`}>{line}</code>
            ))}
          </section>
          <section className="stats-panel">
            <div className="panel-title">Statistics</div>
            <dl>
              <dt>Macro rules</dt><dd>{activeProfile.macroRules.length}</dd>
              <dt>Pixel rules</dt><dd>{activeProfile.pixelRules.length}</dd>
              <dt>Runtime</dt><dd>{isRunning ? 'Active' : 'Idle'}</dd>
            </dl>
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
  }
}

function currentLogTime() {
  return new Date().toLocaleTimeString([], { hour12: false })
}
