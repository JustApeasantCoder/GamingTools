import { useEffect, useMemo, useRef, useState } from 'react'
import { ChevronDown, ChevronRight, Plus, RotateCcw, Trash2 } from 'lucide-react'
import type { AppProfile, ToggleHoldRule } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'
import { getToggleHoldRuleIssues } from './toggleHoldValidation'

interface ToggleHoldProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
}

export function ToggleHold({ profile, onProfileChange }: ToggleHoldProps) {
  const rules = useMemo(() => profile.toggleHoldRules ?? [], [profile.toggleHoldRules])
  const [expandedRuleIds, setExpandedRuleIds] = useState<Set<string>>(new Set())
  const [deletedRule, setDeletedRule] = useState<{ rule: ToggleHoldRule; index: number }>()
  const undoTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)

  useEffect(() => () => {
    if (undoTimer.current) clearTimeout(undoTimer.current)
  }, [])

  useEffect(() => {
    const hasEnabledInvalidRules = rules.some((rule) => rule.enabled && getToggleHoldRuleIssues(rule, profile.runtimeSettings.toggleHotkey).length > 0)
    if (!hasEnabledInvalidRules) return
    onProfileChange({
      ...profile,
      toggleHoldRules: rules.map((rule) => getToggleHoldRuleIssues(rule, profile.runtimeSettings.toggleHotkey).length > 0 ? { ...rule, enabled: false } : rule),
    })
  }, [onProfileChange, profile, rules])

  const updateRule = (nextRule: ToggleHoldRule) => {
    const issues = getToggleHoldRuleIssues(nextRule, profile.runtimeSettings.toggleHotkey)
    const validatedRule = issues.length > 0 ? { ...nextRule, enabled: false } : nextRule
    onProfileChange({
      ...profile,
      toggleHoldRules: rules.map((rule) => (rule.id === validatedRule.id ? validatedRule : rule)),
    })
  }

  const addRule = () => {
    const nextRule: ToggleHoldRule = {
      id: crypto.randomUUID(),
      name: `Toggle Hold ${rules.length + 1}`,
      enabled: true,
      triggerKey: 'F8',
      holdKey: 'RIGHT CLICK',
    }
    onProfileChange({ ...profile, toggleHoldRules: [...rules, nextRule] })
    setExpandedRuleIds((ids) => new Set(ids).add(nextRule.id))
  }

  const removeRule = (ruleId: string) => {
    const index = rules.findIndex((rule) => rule.id === ruleId)
    const rule = rules[index]
    if (!rule) return
    onProfileChange({ ...profile, toggleHoldRules: rules.filter((item) => item.id !== ruleId) })
    setDeletedRule({ rule, index })
    if (undoTimer.current) clearTimeout(undoTimer.current)
    undoTimer.current = setTimeout(() => setDeletedRule(undefined), 6000)
  }

  const undoRemove = () => {
    if (!deletedRule) return
    const nextRules = [...rules]
    nextRules.splice(Math.min(deletedRule.index, nextRules.length), 0, deletedRule.rule)
    onProfileChange({ ...profile, toggleHoldRules: nextRules })
    setExpandedRuleIds((ids) => new Set(ids).add(deletedRule.rule.id))
    setDeletedRule(undefined)
    if (undoTimer.current) clearTimeout(undoTimer.current)
  }

  const toggleExpanded = (ruleId: string) => {
    setExpandedRuleIds((ids) => {
      const nextIds = new Set(ids)
      if (nextIds.has(ruleId)) nextIds.delete(ruleId)
      else nextIds.add(ruleId)
      return nextIds
    })
  }

  return (
    <div className="feature-surface">
      <section className="chain-header">
        <div>
          <h2>Toggle Hold</h2>
          <p>Press a shortcut once to hold a key or mouse button, then press it again to release.</p>
        </div>
        <Button icon={Plus} onClick={addRule}>Add hold rule</Button>
      </section>

      <div className="notice toggle-hold-help">
        Choose a shortcut and what it should hold. Use the same input for both to make it a button toggler.
      </div>

      <div className="toggle-rule-list">
        {rules.length === 0 ? <div className="empty-panel">No toggle hold rules yet.</div> : null}

        {rules.map((rule) => {
          const issues = getToggleHoldRuleIssues(rule, profile.runtimeSettings.toggleHotkey)
          const expanded = issues.length > 0 || expandedRuleIds.has(rule.id)
          return (
            <section className={issues.length > 0 ? 'tool-card toggle-rule-card has-issues' : 'tool-card toggle-rule-card'} key={rule.id}>
              <div className="toggle-rule-header">
                <button className="toggle-rule-expand" onClick={() => toggleExpanded(rule.id)} aria-expanded={expanded} aria-label={`${expanded ? 'Collapse' : 'Edit'} ${rule.name || 'unnamed rule'}`}>
                  {expanded ? <ChevronDown size={17} /> : <ChevronRight size={17} />}
                  <span className="toggle-rule-identity">
                    <strong>{rule.name.trim() || 'Unnamed hold rule'}</strong>
                    <span>Press {rule.triggerKey.trim() || 'Not selected'} to hold or release {rule.holdKey.trim() || 'Not selected'}</span>
                  </span>
                </button>
                <span className={issues.length > 0 ? 'rule-status issue' : rule.enabled ? 'rule-status enabled' : 'rule-status'}>
                  {issues.length > 0 ? 'Needs attention' : rule.enabled ? 'Enabled' : 'Disabled'}
                </span>
                <label className="switch-row compact" title={issues.length > 0 ? 'Fix this rule before enabling it.' : undefined}>
                  <span className="sr-only">Enable {rule.name}</span>
                  <input type="checkbox" checked={rule.enabled} disabled={issues.length > 0} onChange={(event) => updateRule({ ...rule, enabled: event.target.checked })} />
                </label>
                <button className="row-delete-button" aria-label={`Delete ${rule.name}`} title={`Delete ${rule.name}`} onClick={() => removeRule(rule.id)}>
                  <Trash2 size={16} />
                </button>
              </div>

              {expanded ? (
                <div className="toggle-rule-editor">
                  <label className={issues.some((issue) => issue.field === 'name') ? 'field-error' : undefined}>
                    Rule name
                    <input value={rule.name} onChange={(event) => updateRule({ ...rule, name: event.target.value })} />
                  </label>
                  <div className="toggle-rule-sentence">
                    <span>When I press</span>
                    <div className={issues.some((issue) => issue.field === 'triggerKey') ? 'field-error' : undefined}>
                      <KeyCaptureButton value={rule.triggerKey} onChange={(triggerKey) => updateRule({ ...rule, triggerKey })} />
                    </div>
                    <span>hold or release</span>
                    <div className={issues.some((issue) => issue.field === 'holdKey') ? 'field-error' : undefined}>
                      <KeyCaptureButton value={rule.holdKey} onChange={(holdKey) => updateRule({ ...rule, holdKey })} />
                    </div>
                    <Button variant="ghost" onClick={() => updateRule({ ...rule, triggerKey: rule.holdKey, holdKey: rule.triggerKey })} disabled={!rule.triggerKey.trim() && !rule.holdKey.trim()}>
                      Swap
                    </Button>
                  </div>
                  {issues.length > 0 ? (
                    <div className="toggle-rule-errors" role="alert">
                      {issues.map((issue) => <span key={`${issue.field}-${issue.message}`}>{issue.message}</span>)}
                      <span>This rule was turned off until it is fixed.</span>
                    </div>
                  ) : null}
                </div>
              ) : null}
            </section>
          )
        })}
      </div>

      {deletedRule ? (
        <div className="undo-toast" role="status">
          <span>Deleted {deletedRule.rule.name || 'hold rule'}.</span>
          <Button variant="ghost" icon={RotateCcw} onClick={undoRemove}>Undo</Button>
        </div>
      ) : null}
    </div>
  )
}
