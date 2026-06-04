import { Plus, Trash2 } from 'lucide-react'
import type { AppProfile, ToggleHoldRule } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface ToggleHoldProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
}

export function ToggleHold({ profile, onProfileChange }: ToggleHoldProps) {
  const rules = profile.toggleHoldRules ?? []

  const updateRule = (nextRule: ToggleHoldRule) => {
    onProfileChange({
      ...profile,
      toggleHoldRules: rules.map((rule) => (rule.id === nextRule.id ? nextRule : rule)),
    })
  }

  const addRule = () => {
    const nextRule: ToggleHoldRule = {
      id: crypto.randomUUID(),
      name: `Toggle Hold ${rules.length + 1}`,
      enabled: true,
      triggerKey: 'RIGHT CLICK',
      holdKey: 'RIGHT CLICK',
    }
    onProfileChange({
      ...profile,
      toggleHoldRules: [...rules, nextRule],
    })
  }

  const removeRule = (ruleId: string) => {
    onProfileChange({
      ...profile,
      toggleHoldRules: rules.filter((rule) => rule.id !== ruleId),
    })
  }

  return (
    <div className="feature-surface">
      <section className="chain-header">
        <div>
          <h2>Toggle Hold</h2>
          <p>Press once to hold an action down, press again to release it.</p>
        </div>
        <Button icon={Plus} onClick={addRule}>Add hold rule</Button>
      </section>

      <div className="toggle-rule-list">
        {rules.length === 0 ? (
          <div className="empty-panel">No toggle hold rules yet.</div>
        ) : null}

        {rules.map((rule) => (
          <section className="tool-card toggle-rule-card" key={rule.id}>
            <div className="toggle-rule-header">
              <label>
                Rule name
                <input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} />
              </label>
              <label className="switch-row compact">
                <span>Enabled</span>
                <input type="checkbox" checked={rule.enabled} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} />
              </label>
              <button className="icon-button" aria-label={`Delete ${rule.name}`} title={`Delete ${rule.name}`} onClick={() => removeRule(rule.id)}>
                <Trash2 size={17} />
              </button>
            </div>

            <div className="toggle-rule-grid">
              <label>
                Trigger
                <KeyCaptureButton value={rule.triggerKey} label="Listen" onChange={(triggerKey) => updateRule({ ...rule, triggerKey })} />
              </label>
              <label>
                Hold action
                <KeyCaptureButton value={rule.holdKey} label="Listen" onChange={(holdKey) => updateRule({ ...rule, holdKey })} />
              </label>
              <div className="notice">Example: set both fields to RIGHT CLICK to toggle right-click hold on and off.</div>
            </div>
          </section>
        ))}
      </div>
    </div>
  )
}
