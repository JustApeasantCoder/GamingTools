import type { PixelRule } from '../../shared/types/profile'

export function getPixelRuleIssues(rule: PixelRule): string[] {
  const issues: string[] = []
  if (!rule.name.trim()) issues.push('Enter a rule name.')
  if (!/^#[0-9a-f]{6}$/i.test(rule.targetColor)) issues.push('Choose a valid target color.')
  if (rule.tolerance < 0 || rule.tolerance > 255) issues.push('Tolerance must be between 0 and 255.')
  if (rule.actionSteps.length === 0) issues.push('Add at least one action.')
  rule.actionSteps.forEach((step, index) => {
    if (!step.key.trim()) issues.push(`Action ${index + 1} needs a key or button.`)
    if (step.pressDuration.minMs > step.pressDuration.maxMs) issues.push(`Action ${index + 1} hold minimum is greater than its maximum.`)
    if (step.humanizedDelay.enabled && step.humanizedDelay.minMs > step.humanizedDelay.maxMs) issues.push(`Action ${index + 1} delay minimum is greater than its maximum.`)
  })
  return issues
}
