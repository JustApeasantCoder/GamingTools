import type { ComponentType } from 'react'

interface TabItem {
  id: string
  label: string
  icon?: ComponentType<{ size?: number }>
}

interface TabsProps {
  tabs: TabItem[]
  activeId: string
  onChange: (id: string) => void
}

export function Tabs({ tabs, activeId, onChange }: TabsProps) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map((tab) => {
        const Icon = tab.icon
        return (
          <button
            key={tab.id}
            className={tab.id === activeId ? 'tab active' : 'tab'}
            role="tab"
            aria-selected={tab.id === activeId}
            onClick={() => onChange(tab.id)}
          >
            {Icon ? <Icon size={16} /> : null}
            {tab.label}
          </button>
        )
      })}
    </div>
  )
}
