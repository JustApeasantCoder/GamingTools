import { Activity, X } from 'lucide-react'
import type { MacroRule, MacroStep } from '../../shared/types/profile'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'
import { getStepTimingIssue } from './macroTiming'

interface MacroInspectorProps {
  macro?: MacroRule
  step?: MacroStep
  open: boolean
  onClose: () => void
  onStepChange: (step: MacroStep) => void
}

export function MacroInspector({ macro, step, open, onClose, onStepChange }: MacroInspectorProps) {
  const stepNumber = step && macro ? macro.steps.findIndex((item) => item.id === step.id) + 1 : 0
  const timingIssue = step ? getStepTimingIssue(step) : undefined

  return (
    <aside className={open ? 'inspector inspector-open' : 'inspector'}>
      <div className="inspector-heading">
        <div>
          <span>Action Settings</span>
          <strong>{step && macro ? `${macro.name} / Action ${stepNumber}` : 'Select an action'}</strong>
        </div>
        <button className="inspector-close" onClick={onClose} aria-label="Close action settings">
          <X size={17} />
        </button>
      </div>
      {step ? (
        <div className="inspector-form">
          <label>
            Key or mouse button
            <KeyCaptureButton value={step.key} label="Change" onChange={(key) => onStepChange({ ...step, key })} />
          </label>

          <fieldset className="timing-group">
            <legend>Hold duration</legend>
            <div className="two-col">
              <label>Minimum (ms)<input type="number" min="0" value={step.pressDuration.minMs} onChange={(event) => onStepChange({ ...step, pressDuration: { ...step.pressDuration, minMs: Number(event.target.value) } })} /></label>
              <label>Maximum (ms)<input type="number" min="0" value={step.pressDuration.maxMs} onChange={(event) => onStepChange({ ...step, pressDuration: { ...step.pressDuration, maxMs: Number(event.target.value) } })} /></label>
            </div>
          </fieldset>

          <fieldset className="timing-group">
            <legend className="switch-row">
              <span>Wait after action</span>
              <input type="checkbox" checked={step.humanizedDelay.enabled} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, enabled: event.target.checked } })} />
            </legend>
            <div className="two-col">
              <label>Minimum (ms)<input type="number" min="0" disabled={!step.humanizedDelay.enabled} value={step.humanizedDelay.minMs} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, minMs: Number(event.target.value) } })} /></label>
              <label>Maximum (ms)<input type="number" min="0" disabled={!step.humanizedDelay.enabled} value={step.humanizedDelay.maxMs} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, maxMs: Number(event.target.value) } })} /></label>
            </div>
          </fieldset>

          {timingIssue
            ? <div className="notice notice-error"><Activity size={16} /> {timingIssue.message}</div>
            : <div className="notice"><Activity size={16} /> The wait begins after this action is released.</div>}
        </div>
      ) : <div className="empty-panel">Select an action to edit its timing.</div>}
    </aside>
  )
}
