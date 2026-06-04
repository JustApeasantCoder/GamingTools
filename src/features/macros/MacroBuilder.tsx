import { Plus, Shuffle, Trash2 } from 'lucide-react'
import type { AppProfile, MacroRule, MacroStep } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface MacroBuilderProps {
  profile: AppProfile
  selectedMacroId?: string
  selectedStepId?: string
  onSelectedMacroChange: (macroId: string) => void
  onSelectedStepChange: (stepId: string) => void
  onProfileChange: (profile: AppProfile) => void
}

export function MacroBuilder({
  profile,
  selectedMacroId,
  selectedStepId,
  onSelectedMacroChange,
  onSelectedStepChange,
  onProfileChange,
}: MacroBuilderProps) {
  const macro = profile.macroRules.find((item) => item.id === selectedMacroId) ?? profile.macroRules[0]

  const updateMacro = (nextMacro: MacroRule) => {
    onProfileChange({
      ...profile,
      macroRules: profile.macroRules.map((item) => (item.id === nextMacro.id ? nextMacro : item)),
    })
  }

  const addMacro = () => {
    const newStep: MacroStep = {
      id: crypto.randomUUID(),
      key: 'A',
      pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
      humanizedDelay: { ...profile.defaultHumanization },
    }
    const newMacro: MacroRule = {
      id: crypto.randomUUID(),
      name: `Macro ${profile.macroRules.length + 1}`,
      enabled: true,
      triggerKey: `F${Math.min(profile.macroRules.length + 6, 12)}`,
      steps: [newStep],
    }
    onProfileChange({
      ...profile,
      macroRules: [...profile.macroRules, newMacro],
    })
    onSelectedMacroChange(newMacro.id)
    onSelectedStepChange(newStep.id)
  }

  const deleteMacro = () => {
    if (profile.macroRules.length <= 1) return
    const nextMacros = profile.macroRules.filter((item) => item.id !== macro.id)
    onProfileChange({
      ...profile,
      macroRules: nextMacros,
    })
    if (nextMacros[0]) {
      onSelectedMacroChange(nextMacros[0].id)
      if (nextMacros[0].steps[0]) onSelectedStepChange(nextMacros[0].steps[0].id)
    }
  }

  const updateStep = (step: MacroStep) => {
    updateMacro({
      ...macro,
      steps: macro.steps.map((item) => (item.id === step.id ? step : item)),
    })
  }

  const addStep = () => {
    const nextKey = String.fromCharCode(65 + Math.min(macro.steps.length, 25))
    const step: MacroStep = {
      id: crypto.randomUUID(),
      key: nextKey,
      pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
      humanizedDelay: { ...profile.defaultHumanization },
    }
    updateMacro({ ...macro, steps: [...macro.steps, step] })
    onSelectedStepChange(step.id)
  }

  const removeStep = (stepId: string) => {
    const nextSteps = macro.steps.filter((step) => step.id !== stepId)
    updateMacro({ ...macro, steps: nextSteps })
    if (selectedStepId === stepId && nextSteps[0]) onSelectedStepChange(nextSteps[0].id)
  }

  const randomizeKeyDurations = () => {
    updateMacro({
      ...macro,
      steps: macro.steps.map((step) => ({
        ...step,
        pressDuration: {
          ...step.pressDuration,
          enabled: true,
          minMs: randomInt(45, 75),
          maxMs: randomInt(85, 130),
        },
      })),
    })
  }

  const randomizeDelayAfter = () => {
    updateMacro({
      ...macro,
      steps: macro.steps.map((step) => {
        const minMs = randomInt(90, 220)
        return {
          ...step,
          humanizedDelay: {
            ...step.humanizedDelay,
            enabled: true,
            minMs,
            maxMs: minMs + randomInt(80, 180),
          },
        }
      }),
    })
  }

  return (
    <div className="feature-surface">
      <section className="settings-strip">
        <label>
          Macro
          <select
            value={macro.id}
            onChange={(event) => {
              const nextMacro = profile.macroRules.find((item) => item.id === event.target.value)
              if (!nextMacro) return
              onSelectedMacroChange(nextMacro.id)
              if (nextMacro.steps[0]) onSelectedStepChange(nextMacro.steps[0].id)
            }}
          >
            {profile.macroRules.map((item) => (
              <option key={item.id} value={item.id}>{item.name}</option>
            ))}
          </select>
        </label>
        <label>
          Trigger key
          <KeyCaptureButton
            value={macro.triggerKey}
            label="Listen"
            onChange={(triggerKey) => updateMacro({ ...macro, triggerKey })}
          />
        </label>
        <label>
          Rule name
          <input value={macro.name} onChange={(event) => updateMacro({ ...macro, name: event.target.value })} />
        </label>
      </section>

      <section className="chain-header macro-chain-heading">
        <div>
          <h2>Action chain</h2>
          <p>{macro.triggerKey} = {macro.steps.map((step) => step.key).join(' -> ')}</p>
        </div>
      </section>

      <div className="macro-toolbar" aria-label="Macro actions">
        <div className="toolbar-group">
          <Button icon={Plus} variant="primary" onClick={addMacro}>New macro</Button>
          <Button icon={Trash2} variant="danger" onClick={deleteMacro} disabled={profile.macroRules.length <= 1}>Delete macro</Button>
        </div>
        <div className="toolbar-group">
          <Button icon={Shuffle} onClick={randomizeKeyDurations}>Randomize press</Button>
          <Button icon={Shuffle} onClick={randomizeDelayAfter}>Randomize delay</Button>
          <Button icon={Plus} variant="primary" onClick={addStep}>Add step</Button>
        </div>
      </div>

      <div className="macro-table" role="table" aria-label="Macro action chain">
        <div className="macro-row macro-head" role="row">
          <span>#</span>
          <span>Key / Action</span>
          <span>Press Min</span>
          <span>Press Max</span>
          <span>Delay After</span>
          <span />
        </div>
        {macro.steps.map((step, index) => (
          <div
            key={step.id}
            className={step.id === selectedStepId ? 'macro-row active' : 'macro-row'}
            onClick={() => onSelectedStepChange(step.id)}
            role="row"
          >
            <span className="step-number">{index + 1}</span>
            <span className="key-action">
              <kbd>{step.key}</kbd>
              <KeyCaptureButton value={step.key} label="Listen" onChange={(key) => updateStep({ ...step, key })} />
            </span>
            <span><input type="number" value={step.pressDuration.minMs} onChange={(event) => updateStep({ ...step, pressDuration: { ...step.pressDuration, minMs: Number(event.target.value) } })} /> ms</span>
            <span><input type="number" value={step.pressDuration.maxMs} onChange={(event) => updateStep({ ...step, pressDuration: { ...step.pressDuration, maxMs: Number(event.target.value) } })} /> ms</span>
            <span className="delay-cell">
              <input type="checkbox" checked={step.humanizedDelay.enabled} onChange={(event) => updateStep({ ...step, humanizedDelay: { ...step.humanizedDelay, enabled: event.target.checked } })} />
              Min <input type="number" value={step.humanizedDelay.minMs} onChange={(event) => updateStep({ ...step, humanizedDelay: { ...step.humanizedDelay, minMs: Number(event.target.value) } })} />
              Max <input type="number" value={step.humanizedDelay.maxMs} onChange={(event) => updateStep({ ...step, humanizedDelay: { ...step.humanizedDelay, maxMs: Number(event.target.value) } })} />
            </span>
            <span>
              <button
                className="row-delete-button"
                aria-label={`Delete step ${index + 1}`}
                title={`Delete step ${index + 1}`}
                onClick={(event) => { event.stopPropagation(); removeStep(step.id) }}
              >
                <Trash2 size={16} />
              </button>
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}

function randomInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min
}
