import type { AppProfile, MacroRule, MacroStep } from '../../shared/types/profile'

export interface TimingIssue {
  stepId: string
  message: string
}

export function getStepTimingIssue(step: MacroStep): TimingIssue | undefined {
  if (!isScrollAction(step.key) && step.pressDuration.enabled && step.pressDuration.minMs > step.pressDuration.maxMs) {
    return { stepId: step.id, message: 'Minimum hold must not exceed maximum hold.' }
  }
  if (step.humanizedDelay.enabled && step.humanizedDelay.minMs > step.humanizedDelay.maxMs) {
    return { stepId: step.id, message: 'Minimum wait must not exceed maximum wait.' }
  }
  return undefined
}

export function getProfileTimingIssues(profile: AppProfile) {
  return [
    ...profile.macroRules.flatMap((macro) => macro.steps.map(getStepTimingIssue)),
    ...profile.pixelRules.flatMap((rule) => rule.actionSteps.map(getStepTimingIssue)),
  ].filter((issue): issue is TimingIssue => Boolean(issue))
}

export function getMacroDurationRange(macro: MacroRule) {
  return macro.steps.reduce(
    (duration, step) => ({
      minMs: duration.minMs
        + (!isScrollAction(step.key) && step.pressDuration.enabled ? step.pressDuration.minMs : 0)
        + (step.humanizedDelay.enabled ? step.humanizedDelay.minMs : 0),
      maxMs: duration.maxMs
        + (!isScrollAction(step.key) && step.pressDuration.enabled ? step.pressDuration.maxMs : 0)
        + (step.humanizedDelay.enabled ? step.humanizedDelay.maxMs : 0),
    }),
    { minMs: 0, maxMs: 0 },
  )
}

function isScrollAction(key: string) {
  return key.trim().toUpperCase() === 'SCROLL UP' || key.trim().toUpperCase() === 'SCROLL DOWN'
}

export function formatDuration(milliseconds: number) {
  if (milliseconds < 1000) return `${milliseconds} ms`
  return `${(milliseconds / 1000).toFixed(milliseconds < 10_000 ? 1 : 0)} sec`
}
