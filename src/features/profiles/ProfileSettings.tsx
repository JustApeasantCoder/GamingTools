import { Pencil } from 'lucide-react'
import { useState } from 'react'
import type { AppProfile } from '../../shared/types/profile'
import { AppGuardSettings } from '../app-guard/AppGuardSettings'

interface ProfileSettingsProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
}

export function ProfileSettings({ profile, onProfileChange }: ProfileSettingsProps) {
  return (
    <div className="feature-surface">
      <section className="chain-header">
        <div>
          <h2>Profile Settings</h2>
          <p>Controls that apply only to {profile.name}.</p>
        </div>
      </section>

      <ProfileNameEditor
        key={`${profile.id}:${profile.name}`}
        profile={profile}
        onProfileChange={onProfileChange}
      />

      <AppGuardSettings profile={profile} onProfileChange={onProfileChange} />
    </div>
  )
}

function ProfileNameEditor({ profile, onProfileChange }: ProfileSettingsProps) {
  const [nameDraft, setNameDraft] = useState(profile.name)
  const [nameError, setNameError] = useState<string>()

  const commitName = (value: string) => {
    const name = value.trim()
    if (!name) {
      setNameDraft(profile.name)
      setNameError('Profile name cannot be empty.')
      return
    }
    setNameDraft(name)
    setNameError(undefined)
    if (name !== profile.name) onProfileChange({ ...profile, name })
  }

  return (
    <section className="tool-card profile-name-card">
      <div className="settings-card-heading">
        <Pencil size={18} />
        <div>
          <strong>Profile name</strong>
          <span>This name appears in the profile list and runtime status.</span>
        </div>
      </div>
      <label>
        Name
        <input
          value={nameDraft}
          maxLength={80}
          aria-invalid={Boolean(nameError)}
          onChange={(event) => {
            setNameDraft(event.target.value)
            if (nameError) setNameError(undefined)
          }}
          onBlur={(event) => commitName(event.currentTarget.value)}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault()
              event.currentTarget.blur()
            } else if (event.key === 'Escape') {
              event.preventDefault()
              event.currentTarget.value = profile.name
              setNameDraft(profile.name)
              setNameError(undefined)
              event.currentTarget.blur()
            }
          }}
        />
        <span className={nameError ? 'field-message error' : 'field-message'}>
          {nameError ?? 'Press Enter or click elsewhere to save.'}
        </span>
      </label>
    </section>
  )
}
