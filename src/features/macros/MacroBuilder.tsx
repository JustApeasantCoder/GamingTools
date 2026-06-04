import { ChevronRight, GripVertical, Plus, Shuffle, Trash2 } from 'lucide-react'
import type { AppProfile, MacroRule, MacroStep } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'
import { RulePicker } from '../../shared/ui/RulePicker'
import { MacroRecorder } from '../macro-recorder/MacroRecorder'
import { formatDuration, getMacroDurationRange, getStepTimingIssue } from './macroTiming'
import { useState } from 'react'

interface MacroBuilderProps {
  profile: AppProfile
  saveStatus: 'saved' | 'saving' | 'error'
  selectedMacroId?: string
  selectedStepId?: string
  onSelectedMacroChange: (macroId: string) => void
  onSelectedStepChange: (stepId: string) => void
  onProfileChange: (profile: AppProfile) => void
}

export function MacroBuilder({
  profile,
  saveStatus,
  selectedMacroId,
  selectedStepId,
  onSelectedMacroChange,
  onSelectedStepChange,
  onProfileChange,
}: MacroBuilderProps) {
  const [draggedStepId, setDraggedStepId] = useState<string>()
  const macro = profile.macroRules.find((item) => item.id === selectedMacroId) ?? profile.macroRules[0]
  const duration = getMacroDurationRange(macro)

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
    if (!window.confirm(`Delete "${macro.name}"? This cannot be undone.`)) return
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

  const useRecordedSteps = (steps: MacroStep[]) => {
    updateMacro({ ...macro, steps })
    if (steps[0]) onSelectedStepChange(steps[0].id)
  }

  const randomizeTiming = () => {
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
        humanizedDelay: {
          ...step.humanizedDelay,
          enabled: true,
          minMs: randomInt(90, 220),
          maxMs: randomInt(260, 400),
        },
      })),
    })
  }

  const moveStep = (targetStepId: string) => {
    if (!draggedStepId || draggedStepId === targetStepId) return
    const fromIndex = macro.steps.findIndex((step) => step.id === draggedStepId)
    const toIndex = macro.steps.findIndex((step) => step.id === targetStepId)
    if (fromIndex < 0 || toIndex < 0) return
    const steps = [...macro.steps]
    const [moved] = steps.splice(fromIndex, 1)
    steps.splice(toIndex, 0, moved)
    updateMacro({ ...macro, steps })
    setDraggedStepId(undefined)
  }

  const timingLabel = (step: MacroStep) => {
    const wait = step.humanizedDelay.enabled
      ? `Wait ${step.humanizedDelay.minMs}-${step.humanizedDelay.maxMs} ms`
      : 'No wait'
    return `Hold ${step.pressDuration.minMs}-${step.pressDuration.maxMs} ms | ${wait}`
  }

  return (
    <div className="feature-surface macro-builder">
      <section className="editor-picker-toolbar">
        <RulePicker
          ariaLabel="Macros"
          items={profile.macroRules.map((item) => ({ id: item.id, label: item.name, disabled: !item.enabled }))}
          selectedId={macro.id}
          onSelect={(macroId) => {
            const nextMacro = profile.macroRules.find((item) => item.id === macroId)
            if (!nextMacro) return
            onSelectedMacroChange(nextMacro.id)
            if (nextMacro.steps[0]) onSelectedStepChange(nextMacro.steps[0].id)
          }}
        />
        <div className="editor-picker-actions">
          <Button icon={Plus} variant="primary" onClick={addMacro}>New macro</Button>
          <Button icon={Trash2} variant="danger" onClick={deleteMacro} disabled={profile.macroRules.length <= 1}>Delete</Button>
        </div>
      </section>

      <section className="editor-identity-grid">
        <label>
          Shortcut key
          <KeyCaptureButton
            value={macro.triggerKey}
            label="Change"
            onChange={(triggerKey) => updateMacro({ ...macro, triggerKey })}
          />
        </label>
        <label>
          <span className="field-label-with-status">
            Rule name
            <span className={`save-indicator ${saveStatus}`}>{saveStatus === 'saving' ? 'Saving...' : saveStatus === 'error' ? 'Not saved' : 'Saved'}</span>
          </span>
          <input value={macro.name} onChange={(event) => updateMacro({ ...macro, name: event.target.value })} />
        </label>
        <label className="editor-enabled-control">
          Status
          <span className="editor-status-field">
            <span>{macro.enabled ? 'Included in automation' : 'Not included in automation'}</span>
            <span className="switch-row compact">
              <span className="sr-only">Enable {macro.name}</span>
              <input type="checkbox" checked={macro.enabled} onChange={(event) => updateMacro({ ...macro, enabled: event.target.checked })} />
            </span>
          </span>
        </label>
      </section>

      <section className="macro-summary">
        <div>
          <h2>{macro.name}</h2>
          <p><kbd>{macro.triggerKey}</kbd><span>{macro.steps.length} action{macro.steps.length === 1 ? '' : 's'}</span><span>Approx. {formatDuration(duration.minMs)} to {formatDuration(duration.maxMs)}</span></p>
        </div>
        <div className="toolbar-group">
          <Button icon={Shuffle} onClick={randomizeTiming}>Randomize timing</Button>
          <Button icon={Plus} variant="primary" onClick={addStep}>Add action</Button>
        </div>
      </section>

      <MacroRecorder onRecorded={useRecordedSteps} willReplaceExisting={macro.steps.length > 0} />

      <div className="macro-table" role="list" aria-label="Macro action sequence">
        {macro.steps.map((step, index) => {
          const issue = getStepTimingIssue(step)
          return (
            <div
              key={step.id}
              className={`${step.id === selectedStepId ? 'macro-row active' : 'macro-row'}${issue ? ' invalid' : ''}`}
              onClick={() => onSelectedStepChange(step.id)}
              onDragOver={(event) => event.preventDefault()}
              onDrop={() => moveStep(step.id)}
              role="listitem"
            >
              <button
                className="drag-handle"
                draggable
                onDragStart={(event) => {
                  event.stopPropagation()
                  setDraggedStepId(step.id)
                  event.dataTransfer.effectAllowed = 'move'
                }}
                onDragEnd={() => setDraggedStepId(undefined)}
                aria-label={`Drag action ${index + 1} to reorder`}
                title="Drag to reorder"
              >
                <GripVertical size={17} />
              </button>
              <span className="step-number">{index + 1}</span>
              <kbd>{step.key}</kbd>
              <span className="action-summary">
                <span>{issue?.message ?? timingLabel(step)}</span>
              </span>
              <ChevronRight className="row-edit-icon" size={18} aria-hidden />
              <button
                className="row-delete-button"
                aria-label={`Delete action ${index + 1}`}
                title={`Delete action ${index + 1}`}
                onClick={(event) => { event.stopPropagation(); removeStep(step.id) }}
              >
                <Trash2 size={16} />
              </button>
            </div>
          )
        })}
        <button className="add-chain-end" onClick={addStep}><Plus size={16} /> Add action</button>
      </div>
    </div>
  )
}

function randomInt(min: number, max: number) {
  return Math.floor(Math.random() * (max - min + 1)) + min
}
