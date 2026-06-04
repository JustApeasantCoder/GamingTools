import { clsx } from 'clsx'

export interface RulePickerItem {
  id: string
  label: string
  color?: string
  disabled?: boolean
  hasIssue?: boolean
}

interface RulePickerProps {
  ariaLabel: string
  items: RulePickerItem[]
  selectedId?: string
  onSelect: (id: string) => void
}

export function RulePicker({ ariaLabel, items, selectedId, onSelect }: RulePickerProps) {
  return (
    <div className="rule-picker" role="toolbar" aria-label={ariaLabel}>
      {items.map((item, index) => (
        <button
          key={item.id}
          className={clsx('rule-picker-item', item.id === selectedId && 'active', item.disabled && 'disabled')}
          onClick={() => onSelect(item.id)}
          aria-pressed={item.id === selectedId}
        >
          <span className="rule-picker-number">{index + 1}</span>
          <strong>{item.label}</strong>
          {item.color ? <i className="rule-picker-color" style={{ background: item.color }} /> : null}
          {item.hasIssue ? <b className="rule-picker-issue" title="Needs attention">!</b> : null}
        </button>
      ))}
    </div>
  )
}
