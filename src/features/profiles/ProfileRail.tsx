import { Copy, Plus, Save, Trash2 } from 'lucide-react'
import type { AppProfile } from '../../shared/types/profile'

interface ProfileRailProps {
  profiles: AppProfile[]
  activeProfileId: string
  onSelect: (profileId: string) => void
  onAdd: () => void
  onSave: () => void
  onDuplicate: () => void
  onDelete: () => void
  canDelete: boolean
}

export function ProfileRail({
  profiles,
  activeProfileId,
  onSelect,
  onAdd,
  onSave,
  onDuplicate,
  onDelete,
  canDelete,
}: ProfileRailProps) {
  return (
    <section className="profiles-rail">
      <div className="rail-heading">
        <span>Profiles</span>
        <button aria-label="Add profile" title="Add profile" onClick={onAdd}><Plus size={16} /></button>
      </div>
      <div className="profile-tools" aria-label="Active profile actions">
        <button className="profile-tool-button primary" aria-label="Save active profile" title="Save active profile" onClick={onSave}>
          <Save size={16} />
        </button>
        <button className="profile-tool-button" aria-label="Duplicate active profile" title="Duplicate active profile" onClick={onDuplicate}>
          <Copy size={16} />
        </button>
        <button className="profile-tool-button danger" aria-label="Delete active profile" title="Delete active profile" onClick={onDelete} disabled={!canDelete}>
          <Trash2 size={16} />
        </button>
      </div>
      <div className="profile-list">
        {profiles.map((profile) => (
          <button
            key={profile.id}
            className={profile.id === activeProfileId ? 'profile-item active' : 'profile-item'}
            onClick={() => onSelect(profile.id)}
          >
            <span className="profile-light" />
            {profile.name}
            <span className="profile-dot" />
          </button>
        ))}
      </div>
    </section>
  )
}
