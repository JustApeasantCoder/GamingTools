import { useMemo, useState } from 'react'
import { Pipette, Plus, Trash2 } from 'lucide-react'
import type { AppProfile, MacroStep, PixelRule, PixelSampleRequest } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface PixelTriggerProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onSamplePixel: (request: PixelSampleRequest) => Promise<{ color: string; x: number; y: number }>
  onPickPixel: () => Promise<{ color: string; x: number; y: number }>
}

export function PixelTrigger({ profile, onProfileChange, onSamplePixel, onPickPixel }: PixelTriggerProps) {
  const normalizedRules = useMemo(
    () => profile.pixelRules.map((item, index) => normalizeRule(item, index + 1)),
    [profile.pixelRules],
  )
  const [selectedRuleId, setSelectedRuleId] = useState(normalizedRules[0]?.id)
  const [isPicking, setIsPicking] = useState(false)
  const [isPickingSecondary, setIsPickingSecondary] = useState(false)
  const [isPickingSecondary2, setIsPickingSecondary2] = useState(false)
  const rule = normalizedRules.find((item) => item.id === selectedRuleId) ?? normalizedRules[0] ?? createPixelRule()

  const updatePixelRules = (pixelRules: PixelRule[]) => {
    onProfileChange({
      ...profile,
      pixelRules,
    })
  }

  const updateRule = (nextRule: PixelRule) => {
    updatePixelRules(
      normalizedRules.length > 0
        ? normalizedRules.map((item) => (item.id === nextRule.id ? nextRule : item))
        : [nextRule],
    )
  }

  const addRule = () => {
    const newRule = createPixelRule(normalizedRules.length + 1)
    updatePixelRules([...normalizedRules, newRule])
    setSelectedRuleId(newRule.id)
  }

  const deleteRule = () => {
    const nextRules = normalizedRules.filter((item) => item.id !== rule.id)
    updatePixelRules(nextRules)
    setSelectedRuleId(nextRules[0]?.id)
  }

  const updateStep = (step: MacroStep) => {
    updateRule({
      ...rule,
      actionSteps: rule.actionSteps.map((item) => (item.id === step.id ? step : item)),
    })
  }

  const addStep = () => {
    const nextKey = String.fromCharCode(65 + Math.min(rule.actionSteps.length, 25))
    const nextStep: MacroStep = {
      id: crypto.randomUUID(),
      key: nextKey,
      pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
      humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 },
    }
    updateRule({ ...rule, actionSteps: [...rule.actionSteps, nextStep] })
  }

  const removeStep = (stepId: string) => {
    updateRule({ ...rule, actionSteps: rule.actionSteps.filter((step) => step.id !== stepId) })
  }

  const refreshCurrentPixel = async () => {
    const result = await onSamplePixel(rule.samplePoint)
    updateRule({ ...rule, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
  }

  const pickTargetPixel = async () => {
    setIsPicking(true)
    try {
      const result = await onPickPixel()
      updateRule({ ...rule, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
    } finally {
      setIsPicking(false)
    }
  }

  const pickSecondaryPixel = async () => {
    setIsPickingSecondary(true)
    try {
      const result = await onPickPixel()
      updateRule({
        ...rule,
        secondaryCondition: {
          ...rule.secondaryCondition,
          targetColor: result.color,
          samplePoint: { x: result.x, y: result.y },
        },
      })
    } finally {
      setIsPickingSecondary(false)
    }
  }

  const refreshSecondaryPixel = async () => {
    const result = await onSamplePixel(rule.secondaryCondition.samplePoint)
    updateRule({
      ...rule,
      secondaryCondition: {
        ...rule.secondaryCondition,
        targetColor: result.color,
        samplePoint: { x: result.x, y: result.y },
      },
    })
  }

  const pickSecondaryPixel2 = async () => {
    setIsPickingSecondary2(true)
    try {
      const result = await onPickPixel()
      updateRule({
        ...rule,
        secondaryCondition2: {
          ...rule.secondaryCondition2,
          targetColor: result.color,
          samplePoint: { x: result.x, y: result.y },
        },
      })
    } finally {
      setIsPickingSecondary2(false)
    }
  }

  const refreshSecondaryPixel2 = async () => {
    const result = await onSamplePixel(rule.secondaryCondition2.samplePoint)
    updateRule({
      ...rule,
      secondaryCondition2: {
        ...rule.secondaryCondition2,
        targetColor: result.color,
        samplePoint: { x: result.x, y: result.y },
      },
    })
  }

  return (
    <div className="feature-surface">
      <section className="pixel-primary">
        <div className="chain-header">
          <div>
            <h2>Pixel Trigger</h2>
            <p>Detect a target color and run or hold an action chain.</p>
          </div>
          <div className="chain-actions">
            <Button icon={Trash2} variant="danger" onClick={deleteRule} disabled={normalizedRules.length <= 1}>Delete rule</Button>
            <Button icon={Plus} onClick={addRule}>Add rule</Button>
          </div>
        </div>

        <div className="pixel-rule-list">
          {normalizedRules.map((item, index) => (
            <button
              key={item.id}
              className={item.id === rule.id ? 'pixel-rule-item active' : 'pixel-rule-item'}
              onClick={() => setSelectedRuleId(item.id)}
            >
              <span>{index + 1}</span>
              <strong>{item.name}</strong>
              <i style={{ background: item.targetColor }} />
            </button>
          ))}
        </div>

        <div className="tool-card pixel-card">
          <div className="color-preview" style={{ background: rule.targetColor }} />
          <div>
            <strong>{rule.name}</strong>
            <span>{rule.targetColor.toUpperCase()} at {rule.samplePoint.x}, {rule.samplePoint.y}</span>
          </div>
          <label className="switch-row compact">
            <span>Enabled</span>
            <input type="checkbox" checked={rule.enabled} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} />
          </label>
        </div>

        <section className="settings-strip pixel-settings">
          <label>
            Rule name
            <input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} />
          </label>
          <label>
            Tolerance
            <input type="number" min={0} max={255} value={rule.tolerance} onChange={(event) => updateRule({ ...rule, tolerance: Number(event.target.value) })} />
          </label>
          <label>
            Mode
            <select value={rule.triggerMode} onChange={(event) => updateRule({ ...rule, triggerMode: event.target.value as PixelRule['triggerMode'] })}>
              <option value="trigger">Trigger</option>
              <option value="hold">Hold</option>
            </select>
          </label>
        </section>

        <section className="pixel-target-controls">
          <div className="coordinate-fields">
            <label>X<input type="number" value={rule.samplePoint.x} onChange={(event) => updateRule({ ...rule, samplePoint: { ...rule.samplePoint, x: Number(event.target.value) } })} /></label>
            <label>Y<input type="number" value={rule.samplePoint.y} onChange={(event) => updateRule({ ...rule, samplePoint: { ...rule.samplePoint, y: Number(event.target.value) } })} /></label>
          </div>
          <div className="toggle-strip">
          <label className="toggle-option">
            <span>Adjacent pixels</span>
            <input type="checkbox" checked={rule.adjacentPixels} onChange={(event) => updateRule({ ...rule, adjacentPixels: event.target.checked })} />
          </label>
          <label className="toggle-option">
            <span>Trigger when color is not detected</span>
            <input type="checkbox" checked={rule.invertDetection} onChange={(event) => updateRule({ ...rule, invertDetection: event.target.checked })} />
          </label>
          <label className="toggle-option">
            <span>Repeat while detected</span>
            <input type="checkbox" checked={rule.continueWhileDetected} disabled={rule.triggerMode === 'hold'} onChange={(event) => updateRule({ ...rule, continueWhileDetected: event.target.checked })} />
          </label>
          </div>
          <div className="target-actions">
          <Button icon={Pipette} onClick={pickTargetPixel} disabled={isPicking}>{isPicking ? 'Click a pixel...' : 'Pick target pixel'}</Button>
          <Button onClick={refreshCurrentPixel}>Refresh current</Button>
          </div>
        </section>

        <section className="tool-card condition-card">
          <div className="condition-card-heading">
            <div>
              <strong>Target B condition group</strong>
              <span>Require one or two additional pixel conditions before this rule can trigger.</span>
            </div>
            <label className="switch-row compact">
              <span>Enabled</span>
              <input
                type="checkbox"
                checked={rule.secondaryConditionEnabled}
                onChange={(event) => updateRule({ ...rule, secondaryConditionEnabled: event.target.checked })}
              />
            </label>
          </div>
          {rule.secondaryConditionEnabled ? (
            <>
              <div className="condition-subheader">
                <strong>Target B1</strong>
                <label className="switch-row compact">
                  <span>Use Target B2</span>
                  <input
                    type="checkbox"
                    checked={rule.secondaryCondition2Enabled}
                    onChange={(event) => updateRule({ ...rule, secondaryCondition2Enabled: event.target.checked })}
                  />
                </label>
                {rule.secondaryCondition2Enabled ? (
                  <label className="operator-select">
                    Match B1 and B2 using
                    <select
                      value={rule.secondaryConditionOperator}
                      onChange={(event) => updateRule({ ...rule, secondaryConditionOperator: event.target.value as PixelRule['secondaryConditionOperator'] })}
                    >
                      <option value="and">AND - both targets</option>
                      <option value="or">OR - either target</option>
                    </select>
                  </label>
                ) : null}
              </div>
              <div className="condition-preview">
                <div className="color-preview compact-preview" style={{ background: rule.secondaryCondition.targetColor }} />
                <span>
                  {rule.secondaryCondition.invertDetection ? 'Color is not' : 'Color is'} {rule.secondaryCondition.targetColor.toUpperCase()}
                  {' '}at {rule.secondaryCondition.samplePoint.x}, {rule.secondaryCondition.samplePoint.y}
                </span>
              </div>
              <div className="secondary-condition-grid">
                <label>X<input type="number" value={rule.secondaryCondition.samplePoint.x} onChange={(event) => updateRule({ ...rule, secondaryCondition: { ...rule.secondaryCondition, samplePoint: { ...rule.secondaryCondition.samplePoint, x: Number(event.target.value) } } })} /></label>
                <label>Y<input type="number" value={rule.secondaryCondition.samplePoint.y} onChange={(event) => updateRule({ ...rule, secondaryCondition: { ...rule.secondaryCondition, samplePoint: { ...rule.secondaryCondition.samplePoint, y: Number(event.target.value) } } })} /></label>
                <label>Tolerance<input type="number" min={0} max={255} value={rule.secondaryCondition.tolerance} onChange={(event) => updateRule({ ...rule, secondaryCondition: { ...rule.secondaryCondition, tolerance: Number(event.target.value) } })} /></label>
                <label className="toggle-option"><span>Adjacent pixels</span><input type="checkbox" checked={rule.secondaryCondition.adjacentPixels} onChange={(event) => updateRule({ ...rule, secondaryCondition: { ...rule.secondaryCondition, adjacentPixels: event.target.checked } })} /></label>
                <label className="toggle-option"><span>Color is not detected</span><input type="checkbox" checked={rule.secondaryCondition.invertDetection} onChange={(event) => updateRule({ ...rule, secondaryCondition: { ...rule.secondaryCondition, invertDetection: event.target.checked } })} /></label>
                <Button icon={Pipette} onClick={pickSecondaryPixel} disabled={isPickingSecondary}>{isPickingSecondary ? 'Click a pixel...' : 'Pick Target B1'}</Button>
                <Button onClick={refreshSecondaryPixel}>Refresh Target B1</Button>
              </div>
              {rule.secondaryCondition2Enabled ? (
                <div className="condition-secondary">
                  <div className="condition-subheader">
                    <strong>Target B2</strong>
                    <span>{rule.secondaryConditionOperator === 'and' ? 'Both B targets must match.' : 'Either B target may match.'}</span>
                  </div>
                  <div className="condition-preview">
                    <div className="color-preview compact-preview" style={{ background: rule.secondaryCondition2.targetColor }} />
                    <span>
                      {rule.secondaryCondition2.invertDetection ? 'Color is not' : 'Color is'} {rule.secondaryCondition2.targetColor.toUpperCase()}
                      {' '}at {rule.secondaryCondition2.samplePoint.x}, {rule.secondaryCondition2.samplePoint.y}
                    </span>
                  </div>
                  <div className="secondary-condition-grid">
                    <label>X<input type="number" value={rule.secondaryCondition2.samplePoint.x} onChange={(event) => updateRule({ ...rule, secondaryCondition2: { ...rule.secondaryCondition2, samplePoint: { ...rule.secondaryCondition2.samplePoint, x: Number(event.target.value) } } })} /></label>
                    <label>Y<input type="number" value={rule.secondaryCondition2.samplePoint.y} onChange={(event) => updateRule({ ...rule, secondaryCondition2: { ...rule.secondaryCondition2, samplePoint: { ...rule.secondaryCondition2.samplePoint, y: Number(event.target.value) } } })} /></label>
                    <label>Tolerance<input type="number" min={0} max={255} value={rule.secondaryCondition2.tolerance} onChange={(event) => updateRule({ ...rule, secondaryCondition2: { ...rule.secondaryCondition2, tolerance: Number(event.target.value) } })} /></label>
                    <label className="toggle-option"><span>Adjacent pixels</span><input type="checkbox" checked={rule.secondaryCondition2.adjacentPixels} onChange={(event) => updateRule({ ...rule, secondaryCondition2: { ...rule.secondaryCondition2, adjacentPixels: event.target.checked } })} /></label>
                    <label className="toggle-option"><span>Color is not detected</span><input type="checkbox" checked={rule.secondaryCondition2.invertDetection} onChange={(event) => updateRule({ ...rule, secondaryCondition2: { ...rule.secondaryCondition2, invertDetection: event.target.checked } })} /></label>
                    <Button icon={Pipette} onClick={pickSecondaryPixel2} disabled={isPickingSecondary2}>{isPickingSecondary2 ? 'Click a pixel...' : 'Pick Target B2'}</Button>
                    <Button onClick={refreshSecondaryPixel2}>Refresh Target B2</Button>
                  </div>
                </div>
              ) : null}
            </>
          ) : null}
        </section>

        <section className="chain-header">
          <div>
            <h2>Output chain</h2>
            <p>{rule.targetColor.toUpperCase()} = {rule.actionSteps.map((step) => step.key).join(' -> ') || 'No actions'}</p>
          </div>
          <Button icon={Plus} onClick={addStep}>Add output</Button>
        </section>

        <div className="macro-table" role="table" aria-label="Pixel output action chain">
          <div className="macro-row macro-head" role="row">
            <span>#</span>
            <span>Key / Action</span>
            <span>Press Min</span>
            <span>Press Max</span>
            <span>Delay After</span>
            <span />
          </div>
          {rule.actionSteps.map((step, index) => (
            <div key={step.id} className="macro-row" role="row">
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
                  aria-label={`Delete output ${index + 1}`}
                  title={`Delete output ${index + 1}`}
                  onClick={() => removeStep(step.id)}
                >
                  <Trash2 size={16} />
                </button>
              </span>
            </div>
          ))}
        </div>
      </section>
    </div>
  )
}

function normalizeRule(rule: PixelRule, index: number): PixelRule {
  const normalized = {
    ...rule,
    invertDetection: rule.invertDetection ?? false,
    secondaryConditionEnabled: rule.secondaryConditionEnabled ?? false,
    secondaryCondition: rule.secondaryCondition ?? {
      targetColor: '#ffffff',
      tolerance: 12,
      adjacentPixels: false,
      samplePoint: { x: 640, y: 360 },
      invertDetection: false,
    },
    secondaryCondition2Enabled: rule.secondaryCondition2Enabled ?? false,
    secondaryCondition2: rule.secondaryCondition2 ?? {
      targetColor: '#ffffff',
      tolerance: 12,
      adjacentPixels: false,
      samplePoint: { x: 640, y: 360 },
      invertDetection: false,
    },
    secondaryConditionOperator: rule.secondaryConditionOperator ?? 'and',
  }
  if (normalized.actionSteps?.length) return normalized

  return {
    ...normalized,
    triggerMode: normalized.triggerMode ?? 'hold',
    continueWhileDetected: normalized.continueWhileDetected ?? true,
    actionSteps: [
      {
        id: crypto.randomUUID(),
        key: normalized.outputKey ?? 'Q',
        pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
        humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 },
      },
    ],
    name: normalized.name || `Color Watch ${index}`,
  }
}

function createPixelRule(index = 1): PixelRule {
  return {
    id: crypto.randomUUID(),
    name: `Color Watch ${index}`,
    enabled: true,
    targetColor: '#34d399',
    tolerance: 10,
    adjacentPixels: false,
    samplePoint: { x: 640, y: 360 },
    invertDetection: false,
    secondaryConditionEnabled: false,
    secondaryCondition: {
      targetColor: '#ffffff',
      tolerance: 10,
      adjacentPixels: false,
      samplePoint: { x: 640, y: 360 },
      invertDetection: false,
    },
    secondaryCondition2Enabled: false,
    secondaryCondition2: {
      targetColor: '#ffffff',
      tolerance: 10,
      adjacentPixels: false,
      samplePoint: { x: 640, y: 360 },
      invertDetection: false,
    },
    secondaryConditionOperator: 'and',
    triggerMode: 'hold',
    continueWhileDetected: true,
    actionSteps: [
      {
        id: crypto.randomUUID(),
        key: 'Q',
        pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
        humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 },
      },
    ],
  }
}
