import { useMemo, useState } from 'react'
import { ChevronDown, ChevronRight, GripVertical, Pipette, Play, Plus, RefreshCw, Trash2 } from 'lucide-react'
import type { AppProfile, MacroStep, PixelCondition, PixelRule, PixelSampleRequest } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { RulePicker } from '../../shared/ui/RulePicker'
import { getStepTimingIssue } from '../macros/macroTiming'
import { getPixelRuleIssues } from './pixelRuleValidation'

interface PixelTriggerProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onSamplePixel: (request: PixelSampleRequest) => Promise<{ color: string; x: number; y: number }>
  onPickPixel: () => Promise<{ color: string; x: number; y: number }>
  onTestRule: (rule: PixelRule) => Promise<boolean>
  onTestActions: (rule: PixelRule) => Promise<void>
  selectedRuleId?: string
  selectedStepId?: string
  onSelectedRuleChange: (ruleId: string) => void
  onSelectedStepChange: (stepId: string) => void
}

type TestState = 'idle' | 'testing' | 'matching' | 'not-matching' | 'error'

export function PixelTrigger({ profile, onProfileChange, onSamplePixel, onPickPixel, onTestRule, onTestActions, selectedRuleId, selectedStepId, onSelectedRuleChange, onSelectedStepChange }: PixelTriggerProps) {
  const normalizedRules = useMemo(() => profile.pixelRules.map((item, index) => normalizeRule(item, index + 1)), [profile.pixelRules])
  const [isPicking, setIsPicking] = useState(false)
  const [advancedOpen, setAdvancedOpen] = useState(false)
  const [conditionsOpen, setConditionsOpen] = useState(false)
  const [testState, setTestState] = useState<TestState>('idle')
  const [isTestingActions, setIsTestingActions] = useState(false)
  const [draggedStepId, setDraggedStepId] = useState<string>()
  const rule = normalizedRules.find((item) => item.id === selectedRuleId) ?? normalizedRules[0] ?? createPixelRule()
  const issues = getPixelRuleIssues(rule)

  const updatePixelRules = (pixelRules: PixelRule[]) => onProfileChange({ ...profile, pixelRules })
  const updateRule = (nextRule: PixelRule) => updatePixelRules(normalizedRules.length > 0 ? normalizedRules.map((item) => item.id === nextRule.id ? nextRule : item) : [nextRule])
  const updateCondition = (key: 'secondaryCondition' | 'secondaryCondition2', condition: PixelCondition) => updateRule({ ...rule, [key]: condition })

  const addRule = () => {
    const newRule = createPixelRule(normalizedRules.length + 1)
    updatePixelRules([...normalizedRules, newRule])
    onSelectedRuleChange(newRule.id)
    onSelectedStepChange(newRule.actionSteps[0].id)
  }
  const deleteRule = () => {
    const nextRules = normalizedRules.filter((item) => item.id !== rule.id)
    updatePixelRules(nextRules)
    if (nextRules[0]) {
      onSelectedRuleChange(nextRules[0].id)
      if (nextRules[0].actionSteps[0]) onSelectedStepChange(nextRules[0].actionSteps[0].id)
    }
  }
  const addStep = () => {
    const step = {
      id: crypto.randomUUID(),
      key: String.fromCharCode(65 + Math.min(rule.actionSteps.length, 25)),
      pressDuration: { enabled: true, minMs: 50, maxMs: 90 },
      humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 },
    }
    updateRule({ ...rule, actionSteps: [...rule.actionSteps, step] })
    onSelectedStepChange(step.id)
  }
  const moveStep = (targetId: string) => {
    if (!draggedStepId || draggedStepId === targetId) return
    const steps = [...rule.actionSteps]
    const from = steps.findIndex((step) => step.id === draggedStepId)
    const to = steps.findIndex((step) => step.id === targetId)
    const [moved] = steps.splice(from, 1)
    steps.splice(to, 0, moved)
    updateRule({ ...rule, actionSteps: steps })
    setDraggedStepId(undefined)
  }
  const pickPrimary = async () => {
    setIsPicking(true)
    try {
      const result = await onPickPixel()
      updateRule({ ...rule, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
      setTestState('idle')
    } finally {
      setIsPicking(false)
    }
  }
  const resamplePrimary = async () => {
    const result = await onSamplePixel(rule.samplePoint)
    updateRule({ ...rule, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
    setTestState('idle')
  }
  const testRule = async () => {
    setTestState('testing')
    try {
      setTestState(await onTestRule(rule) ? 'matching' : 'not-matching')
    } catch {
      setTestState('error')
    }
  }
  const testActions = async () => {
    setIsTestingActions(true)
    try {
      await onTestActions(rule)
    } finally {
      setIsTestingActions(false)
    }
  }

  return (
    <div className="feature-surface pixel-trigger">
      <section className="pixel-primary">
        <section className="editor-picker-toolbar">
          <RulePicker
            ariaLabel="Pixel trigger rules"
            items={normalizedRules.map((item) => ({
              id: item.id,
              label: item.name,
              color: item.targetColor,
              disabled: !item.enabled,
              hasIssue: getPixelRuleIssues(item).length > 0,
            }))}
            selectedId={rule.id}
            onSelect={(ruleId) => {
              const nextRule = normalizedRules.find((item) => item.id === ruleId)
              if (!nextRule) return
              onSelectedRuleChange(nextRule.id)
              if (nextRule.actionSteps[0]) onSelectedStepChange(nextRule.actionSteps[0].id)
            }}
          />
          <div className="editor-picker-actions">
            <Button icon={Plus} variant="primary" onClick={addRule}>New rule</Button>
            <Button icon={Trash2} variant="danger" onClick={deleteRule} disabled={normalizedRules.length <= 1}>Delete</Button>
          </div>
        </section>

        <section className="editor-identity-grid pixel-identity-grid">
          <label>Rule name<input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} /></label>
          <label className="editor-enabled-control">
            Status
            <span className="editor-status-field">
              <span>{rule.enabled ? 'Included in automation' : 'Not included in automation'}</span>
              <span className="switch-row compact">
                <span className="sr-only">Enable {rule.name}</span>
                <input type="checkbox" checked={rule.enabled} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} />
              </span>
            </span>
          </label>
        </section>

        <section className="macro-summary pixel-summary">
          <div>
            <h2>{rule.name}</h2>
            <p>
              <span className="summary-color" style={{ background: rule.targetColor }} />
              <span>{rule.targetColor.toUpperCase()}</span>
              <span>Position {rule.samplePoint.x}, {rule.samplePoint.y}</span>
              <span>{rule.actionSteps.length} action{rule.actionSteps.length === 1 ? '' : 's'}</span>
            </p>
          </div>
          <div className="toolbar-group">
            <span className={`test-status ${testState}`}>{testLabel(testState)}</span>
            <Button icon={Play} onClick={testRule} disabled={testState === 'testing'}>{testState === 'testing' ? 'Checking...' : 'Test rule'}</Button>
            <Button icon={Plus} variant="primary" onClick={addStep}>Add action</Button>
          </div>
        </section>

        {issues.length > 0 ? <div className="notice notice-error"><strong>This rule needs attention:</strong> {issues[0]}</div> : null}

        <WorkflowSection number="1" title="Choose what to watch" description="Select the screen pixel and color this rule should monitor.">
          <div className="target-summary">
            <div className="color-preview" style={{ background: rule.targetColor }} />
            <div><strong>{rule.targetColor.toUpperCase()}</strong><span>Screen position {rule.samplePoint.x}, {rule.samplePoint.y}</span></div>
            <span className={`test-status ${testState}`}>{testLabel(testState)}</span>
          </div>
          <div className="section-actions">
            <Button variant="primary" icon={Pipette} onClick={pickPrimary} disabled={isPicking}>{isPicking ? 'Click a screen pixel...' : 'Choose target pixel'}</Button>
            <Button icon={RefreshCw} onClick={resamplePrimary}>Resample color here</Button>
          </div>
          <Disclosure title="Advanced position controls" open={advancedOpen} onToggle={() => setAdvancedOpen((open) => !open)}>
            <div className="coordinate-fields">
              <label>X coordinate<input type="number" value={rule.samplePoint.x} onChange={(event) => updateRule({ ...rule, samplePoint: { ...rule.samplePoint, x: Number(event.target.value) } })} /></label>
              <label>Y coordinate<input type="number" value={rule.samplePoint.y} onChange={(event) => updateRule({ ...rule, samplePoint: { ...rule.samplePoint, y: Number(event.target.value) } })} /></label>
            </div>
          </Disclosure>
        </WorkflowSection>

        <WorkflowSection number="2" title="Choose when it activates" description="Control how closely the screen color must match.">
          <div className="detection-grid">
            <label>Tolerance <small>Higher values accept more similar colors.</small><input type="number" min={0} max={255} value={rule.tolerance} onChange={(event) => updateRule({ ...rule, tolerance: Number(event.target.value) })} /></label>
            <label>Action mode <small>Run taps once, or hold actions while matching.</small><select value={rule.triggerMode} onChange={(event) => updateRule({ ...rule, triggerMode: event.target.value as PixelRule['triggerMode'] })}><option value="trigger">Run actions</option><option value="hold">Hold actions</option></select></label>
          </div>
          <div className="toggle-strip">
            <ToggleOption label="Check nearby pixels" help="Useful when the watched element moves slightly." checked={rule.adjacentPixels} onChange={(checked) => updateRule({ ...rule, adjacentPixels: checked })} />
            <ToggleOption label="Run when color is missing" help="Activates when the selected color is no longer visible." checked={rule.invertDetection} onChange={(checked) => updateRule({ ...rule, invertDetection: checked })} />
            <ToggleOption label="Repeat while color matches" help="Runs the action chain repeatedly until the color changes." checked={rule.continueWhileDetected} disabled={rule.triggerMode === 'hold'} onChange={(checked) => updateRule({ ...rule, continueWhileDetected: checked })} />
          </div>
          <Disclosure title="Additional color conditions" subtitle={rule.secondaryConditionEnabled ? 'Enabled' : 'Optional'} open={conditionsOpen} onToggle={() => setConditionsOpen((open) => !open)}>
            <div className="condition-enable-row">
              <span>Only activate when one or two other screen colors also match.</span>
              <label className="switch-row compact"><span>Enabled</span><input type="checkbox" checked={rule.secondaryConditionEnabled} onChange={(event) => updateRule({ ...rule, secondaryConditionEnabled: event.target.checked })} /></label>
            </div>
            {rule.secondaryConditionEnabled ? (
              <div className="condition-list">
                <ConditionEditor title="Condition 1" condition={rule.secondaryCondition} onChange={(condition) => updateCondition('secondaryCondition', condition)} onPick={onPickPixel} onSample={onSamplePixel} />
                <div className="condition-enable-row condition-logic-row">
                  <label className="switch-row compact"><span>Add condition 2</span><input type="checkbox" checked={rule.secondaryCondition2Enabled} onChange={(event) => updateRule({ ...rule, secondaryCondition2Enabled: event.target.checked })} /></label>
                  {rule.secondaryCondition2Enabled ? <label className="operator-select">Run when<select value={rule.secondaryConditionOperator} onChange={(event) => updateRule({ ...rule, secondaryConditionOperator: event.target.value as PixelRule['secondaryConditionOperator'] })}><option value="and">Both conditions match</option><option value="or">Either condition matches</option></select></label> : null}
                </div>
                {rule.secondaryCondition2Enabled ? <ConditionEditor title="Condition 2" condition={rule.secondaryCondition2} onChange={(condition) => updateCondition('secondaryCondition2', condition)} onPick={onPickPixel} onSample={onSamplePixel} /> : null}
              </div>
            ) : null}
          </Disclosure>
        </WorkflowSection>

        <WorkflowSection number="3" title="Choose what happens" description={`Action sequence: ${rule.actionSteps.map((step) => step.key).join(' → ') || 'No actions added'}`}>
          <div className="section-actions action-toolbar">
            <Button icon={Play} onClick={testActions} disabled={isTestingActions || issues.length > 0}>{isTestingActions ? 'Running test...' : 'Test actions'}</Button>
          </div>
          <div className="macro-table pixel-action-list" role="list" aria-label="Pixel action sequence">
            {rule.actionSteps.map((step, index) => {
              const issue = getStepTimingIssue(step)
              const wait = step.humanizedDelay.enabled ? `Wait ${step.humanizedDelay.minMs}-${step.humanizedDelay.maxMs} ms` : 'No wait'
              return (
                <div key={step.id} className={`${step.id === selectedStepId ? 'macro-row active' : 'macro-row'}${issue ? ' invalid' : ''}`} onClick={() => onSelectedStepChange(step.id)} onDragOver={(event) => event.preventDefault()} onDrop={() => moveStep(step.id)} role="listitem">
                  <button className="drag-handle" draggable onDragStart={(event) => { event.stopPropagation(); setDraggedStepId(step.id); event.dataTransfer.effectAllowed = 'move' }} onDragEnd={() => setDraggedStepId(undefined)} aria-label={`Drag action ${index + 1} to reorder`} title="Drag to reorder"><GripVertical size={17} /></button>
                  <span className="step-number">{index + 1}</span>
                  <kbd>{step.key}</kbd>
                  <span className="action-summary"><span>{issue?.message ?? `Hold ${step.pressDuration.minMs}-${step.pressDuration.maxMs} ms | ${wait}`}</span></span>
                  <ChevronRight className="row-edit-icon" size={18} aria-hidden />
                  <button className="row-delete-button" aria-label={`Delete action ${index + 1}`} onClick={(event) => {
                    event.stopPropagation()
                    const nextSteps = rule.actionSteps.filter((item) => item.id !== step.id)
                    updateRule({ ...rule, actionSteps: nextSteps })
                    if (selectedStepId === step.id && nextSteps[0]) onSelectedStepChange(nextSteps[0].id)
                  }}><Trash2 size={16} /></button>
                </div>
              )
            })}
            <button className="add-chain-end" onClick={addStep}><Plus size={16} /> Add action</button>
          </div>
        </WorkflowSection>
      </section>
    </div>
  )
}

function WorkflowSection({ number, title, description, children }: { number: string; title: string; description: string; children: React.ReactNode }) {
  return <section className="workflow-section"><header><span>{number}</span><div><h3>{title}</h3><p>{description}</p></div></header>{children}</section>
}

function Disclosure({ title, subtitle, open, onToggle, children }: { title: string; subtitle?: string; open: boolean; onToggle: () => void; children: React.ReactNode }) {
  return <section className="pixel-disclosure"><button onClick={onToggle}>{open ? <ChevronDown size={17} /> : <ChevronRight size={17} />}<strong>{title}</strong>{subtitle ? <span>{subtitle}</span> : null}</button>{open ? <div className="pixel-disclosure-content">{children}</div> : null}</section>
}

function ToggleOption({ label, help, checked, disabled, onChange }: { label: string; help: string; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  return <label className="toggle-option detailed"><span><strong>{label}</strong><small>{help}</small></span><input type="checkbox" checked={checked} disabled={disabled} onChange={(event) => onChange(event.target.checked)} /></label>
}

function ConditionEditor({ title, condition, onChange, onPick, onSample }: { title: string; condition: PixelCondition; onChange: (condition: PixelCondition) => void; onPick: () => Promise<{ color: string; x: number; y: number }>; onSample: (point: PixelSampleRequest) => Promise<{ color: string; x: number; y: number }> }) {
  const [picking, setPicking] = useState(false)
  const pick = async () => {
    setPicking(true)
    try {
      const result = await onPick()
      onChange({ ...condition, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
    } finally {
      setPicking(false)
    }
  }
  const sample = async () => {
    const result = await onSample(condition.samplePoint)
    onChange({ ...condition, targetColor: result.color, samplePoint: { x: result.x, y: result.y } })
  }
  return <section className="condition-editor">
    <div className="condition-preview"><div className="color-preview compact-preview" style={{ background: condition.targetColor }} /><strong>{title}</strong><span>{condition.targetColor.toUpperCase()} at {condition.samplePoint.x}, {condition.samplePoint.y}</span></div>
    <div className="condition-editor-grid">
      <div className="condition-coordinate-fields">
        <label>X<input type="number" value={condition.samplePoint.x} onChange={(event) => onChange({ ...condition, samplePoint: { ...condition.samplePoint, x: Number(event.target.value) } })} /></label>
        <label>Y<input type="number" value={condition.samplePoint.y} onChange={(event) => onChange({ ...condition, samplePoint: { ...condition.samplePoint, y: Number(event.target.value) } })} /></label>
        <label>Tolerance<input type="number" min={0} max={255} value={condition.tolerance} onChange={(event) => onChange({ ...condition, tolerance: Number(event.target.value) })} /></label>
      </div>
      <div className="condition-toggle-fields">
        <ToggleOption label="Nearby pixels" help="Allows slight movement." checked={condition.adjacentPixels} onChange={(adjacentPixels) => onChange({ ...condition, adjacentPixels })} />
        <ToggleOption label="Color is missing" help="Invert this condition." checked={condition.invertDetection} onChange={(invertDetection) => onChange({ ...condition, invertDetection })} />
      </div>
      <div className="condition-action-buttons">
        <Button icon={Pipette} onClick={pick} disabled={picking}>{picking ? 'Click a pixel...' : `Choose ${title.toLowerCase()} pixel`}</Button>
        <Button icon={RefreshCw} onClick={sample}>Resample color</Button>
      </div>
    </div>
  </section>
}

function testLabel(state: TestState) {
  if (state === 'matching') return 'Matching now'
  if (state === 'not-matching') return 'Not matching'
  if (state === 'testing') return 'Checking...'
  if (state === 'error') return 'Target unavailable'
  return 'Not tested'
}

function normalizeRule(rule: PixelRule, index: number): PixelRule {
  const normalized = {
    ...rule,
    invertDetection: rule.invertDetection ?? false,
    secondaryConditionEnabled: rule.secondaryConditionEnabled ?? false,
    secondaryCondition: rule.secondaryCondition ?? createCondition(),
    secondaryCondition2Enabled: rule.secondaryCondition2Enabled ?? false,
    secondaryCondition2: rule.secondaryCondition2 ?? createCondition(),
    secondaryConditionOperator: rule.secondaryConditionOperator ?? 'and',
  }
  if (normalized.actionSteps?.length) return normalized
  return { ...normalized, triggerMode: normalized.triggerMode ?? 'hold', continueWhileDetected: normalized.continueWhileDetected ?? true, actionSteps: [createStep(normalized.outputKey ?? 'Q')], name: normalized.name || `Color Watch ${index}` }
}

function createCondition(): PixelCondition {
  return { targetColor: '#ffffff', tolerance: 12, adjacentPixels: false, samplePoint: { x: 640, y: 360 }, invertDetection: false }
}

function createStep(key: string): MacroStep {
  return { id: crypto.randomUUID(), key, pressDuration: { enabled: true, minMs: 50, maxMs: 90 }, humanizedDelay: { enabled: true, minMs: 80, maxMs: 150 } }
}

function createPixelRule(index = 1): PixelRule {
  return { id: crypto.randomUUID(), name: `Color Watch ${index}`, enabled: true, targetColor: '#34d399', tolerance: 10, adjacentPixels: false, samplePoint: { x: 640, y: 360 }, invertDetection: false, secondaryConditionEnabled: false, secondaryCondition: createCondition(), secondaryCondition2Enabled: false, secondaryCondition2: createCondition(), secondaryConditionOperator: 'and', triggerMode: 'hold', continueWhileDetected: true, actionSteps: [createStep('Q')] }
}
