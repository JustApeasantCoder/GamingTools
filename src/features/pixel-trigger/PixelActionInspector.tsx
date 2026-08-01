import { Activity, X } from 'lucide-react'
import type { MacroStep, PixelRule } from '../../shared/types/profile'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'
import { getStepTimingIssue } from '../macros/macroTiming'

interface PixelActionInspectorProps {
  rule?: PixelRule
  step?: MacroStep
  open: boolean
  onClose: () => void
  onStepChange: (step: MacroStep) => void
}

export function PixelActionInspector({ rule, step, open, onClose, onStepChange }: PixelActionInspectorProps) {
  const stepNumber = step && rule ? rule.actionSteps.findIndex((item) => item.id === step.id) + 1 : 0
  const timingIssue = step ? getStepTimingIssue(step) : undefined
  const allowWheel = rule?.triggerMode === 'trigger'

  return (
    <aside className={open ? 'inspector inspector-open' : 'inspector'}>
      <div className="inspector-heading">
        <div>
          <span>Action Settings</span>
          <strong>{step && rule ? `${rule.name} / Action ${stepNumber}` : 'Select an action'}</strong>
        </div>
        <button className="inspector-close" onClick={onClose} aria-label="Close action settings"><X size={17} /></button>
      </div>
      {step ? (
        <div className="inspector-form">
          <label>{allowWheel ? 'Key, mouse button, or wheel' : 'Key or mouse button'}<KeyCaptureButton value={step.key} label="Change" allowWheel={allowWheel} onChange={(key) => onStepChange({ ...step, key })} /></label>
          {!isScrollAction(step.key) && (
            <fieldset className="timing-group">
              <legend>Hold duration</legend>
              <div className="two-col">
                <label>Minimum (ms)<input type="number" min="0" value={step.pressDuration.minMs} onChange={(event) => onStepChange({ ...step, pressDuration: { ...step.pressDuration, minMs: Number(event.target.value) } })} /></label>
                <label>Maximum (ms)<input type="number" min="0" value={step.pressDuration.maxMs} onChange={(event) => onStepChange({ ...step, pressDuration: { ...step.pressDuration, maxMs: Number(event.target.value) } })} /></label>
              </div>
            </fieldset>
          )}
          <fieldset className="timing-group">
            <legend className="switch-row"><span>Wait after action</span><input type="checkbox" checked={step.humanizedDelay.enabled} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, enabled: event.target.checked } })} /></legend>
            <div className="two-col">
              <label>Minimum (ms)<input type="number" min="0" disabled={!step.humanizedDelay.enabled} value={step.humanizedDelay.minMs} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, minMs: Number(event.target.value) } })} /></label>
              <label>Maximum (ms)<input type="number" min="0" disabled={!step.humanizedDelay.enabled} value={step.humanizedDelay.maxMs} onChange={(event) => onStepChange({ ...step, humanizedDelay: { ...step.humanizedDelay, maxMs: Number(event.target.value) } })} /></label>
            </div>
          </fieldset>
          {timingIssue ? <div className="notice notice-error"><Activity size={16} /> {timingIssue.message}</div> : <div className="notice"><Activity size={16} /> {isScrollAction(step.key) ? 'Each action emits one wheel step.' : 'The wait begins after this action is released.'}</div>}
        </div>
      ) : <div className="empty-panel">Select an action to edit its timing.</div>}
    </aside>
  )
}

function isScrollAction(key: string) {
  return key.trim().toUpperCase() === 'SCROLL UP' || key.trim().toUpperCase() === 'SCROLL DOWN'
}
