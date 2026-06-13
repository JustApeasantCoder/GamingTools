import type { ToggleHoldRule } from '../../shared/types/profile'

export interface ToggleHoldIssue {
  field: 'name' | 'triggerKey' | 'holdKey' | 'releaseKey' | 'rule'
  message: string
}

export function getToggleHoldRuleIssues(rule: ToggleHoldRule, automationToggleHotkey?: string): ToggleHoldIssue[] {
  const issues: ToggleHoldIssue[] = []
  const triggerKey = rule.triggerKey.trim()
  const holdKey = rule.holdKey.trim()
  const releaseKey = rule.releaseKey?.trim() ?? ''

  if (!rule.name.trim()) issues.push({ field: 'name', message: 'Enter a rule name.' })
  if (!triggerKey) issues.push({ field: 'triggerKey', message: 'Choose a shortcut.' })
  if (!holdKey) issues.push({ field: 'holdKey', message: 'Choose a key or mouse button to hold.' })
  if (rule.releaseMode === 'specific' && !releaseKey) {
    issues.push({ field: 'releaseKey', message: 'Choose an input that releases the hold.' })
  }
  if (rule.releaseMode === 'specific' && releaseKey && releaseKey.toLowerCase() === triggerKey.toLowerCase()) {
    issues.push({ field: 'releaseKey', message: 'Choose a release input other than the shortcut.' })
  }
  if (rule.releaseMode === 'specific' && releaseKey && releaseKey.toLowerCase() === holdKey.toLowerCase()) {
    issues.push({ field: 'releaseKey', message: 'Choose a release input other than the held input.' })
  }
  if (automationToggleHotkey && triggerKey.toLowerCase() === automationToggleHotkey.trim().toLowerCase()) {
    issues.push({ field: 'triggerKey', message: `Choose a shortcut other than the automation shortcut (${automationToggleHotkey}).` })
  }
  if (automationToggleHotkey && holdKey.toLowerCase() === automationToggleHotkey.trim().toLowerCase()) {
    issues.push({ field: 'holdKey', message: `Choose a held input other than the automation shortcut (${automationToggleHotkey}).` })
  }
  if (automationToggleHotkey && rule.releaseMode === 'specific' && releaseKey.toLowerCase() === automationToggleHotkey.trim().toLowerCase()) {
    issues.push({ field: 'releaseKey', message: `Choose a release input other than the automation shortcut (${automationToggleHotkey}).` })
  }

  return issues
}
