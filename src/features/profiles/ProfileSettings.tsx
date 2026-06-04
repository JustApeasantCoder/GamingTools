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

      <AppGuardSettings profile={profile} onProfileChange={onProfileChange} />
    </div>
  )
}
