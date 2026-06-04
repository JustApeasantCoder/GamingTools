import { BellRing, Keyboard, Save } from 'lucide-react'
import type { AppProfile } from '../../shared/types/profile'
import type { ProfileStore } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'
import { ProfileTransfer } from '../profile-transfer/ProfileTransfer'

interface SettingsPanelProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onSaveProfile: () => void
  onImported: (store: ProfileStore) => void
}

export function SettingsPanel({ profile, onProfileChange, onSaveProfile, onImported }: SettingsPanelProps) {
  const updateRuntimeSettings = (runtimeSettings: AppProfile['runtimeSettings']) => {
    onProfileChange({ ...profile, runtimeSettings })
  }

  return (
    <div className="feature-surface">
      <section className="chain-header">
        <div>
          <h2>Settings</h2>
          <p>Automation preferences and profile transfer tools.</p>
        </div>
        <Button icon={Save} variant="primary" onClick={onSaveProfile}>Save settings</Button>
      </section>

      <section className="tool-card settings-card">
        <div className="settings-card-heading">
          <Keyboard size={18} />
          <div>
            <strong>Global start and stop shortcut</strong>
            <span>Start or stop automation while Gaming Toolkit is minimized or another app is active.</span>
          </div>
        </div>
        <label>
          Shortcut
          <KeyCaptureButton
            value={profile.runtimeSettings.toggleHotkey}
            label="Change"
            onChange={(toggleHotkey) => updateRuntimeSettings({ ...profile.runtimeSettings, toggleHotkey })}
          />
        </label>
      </section>

      <section className="tool-card settings-card">
        <div className="settings-card-heading">
          <BellRing size={18} />
          <div>
            <strong>Start and stop sound</strong>
            <span>Play a subtle system sound when automation starts or stops.</span>
          </div>
        </div>
        <label className="switch-row">
          <span>Play sound</span>
          <input
            type="checkbox"
            checked={profile.runtimeSettings.soundEnabled}
            onChange={(event) => updateRuntimeSettings({ ...profile.runtimeSettings, soundEnabled: event.target.checked })}
          />
        </label>
      </section>
      <ProfileTransfer profile={profile} onImported={onImported} />
    </div>
  )
}
