import { BellRing, Keyboard, Save } from 'lucide-react'
import type { AppProfile } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'
import { KeyCaptureButton } from '../../shared/ui/KeyCaptureButton'

interface SettingsPanelProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
  onSaveProfile: () => void
}

export function SettingsPanel({ profile, onProfileChange, onSaveProfile }: SettingsPanelProps) {
  const updateRuntimeSettings = (runtimeSettings: AppProfile['runtimeSettings']) => {
    onProfileChange({ ...profile, runtimeSettings })
  }

  return (
    <div className="feature-surface">
      <section className="chain-header">
        <div>
          <h2>Settings</h2>
          <p>Runtime controls for {profile.name}.</p>
        </div>
        <Button icon={Save} variant="primary" onClick={onSaveProfile}>Save settings</Button>
      </section>

      <section className="tool-card settings-card">
        <div className="settings-card-heading">
          <Keyboard size={18} />
          <div>
            <strong>Global runtime hotkey</strong>
            <span>Works while Gaming Toolkit is minimized or another app has focus.</span>
          </div>
        </div>
        <label>
          Toggle Start / Stop
          <KeyCaptureButton
            value={profile.runtimeSettings.toggleHotkey}
            label="Listen"
            onChange={(toggleHotkey) => updateRuntimeSettings({ ...profile.runtimeSettings, toggleHotkey })}
          />
        </label>
      </section>

      <section className="tool-card settings-card">
        <div className="settings-card-heading">
          <BellRing size={18} />
          <div>
            <strong>Toggle sound</strong>
            <span>Play a subtle system cue when the runtime starts or stops.</span>
          </div>
        </div>
        <label className="switch-row">
          <span>Sound enabled</span>
          <input
            type="checkbox"
            checked={profile.runtimeSettings.soundEnabled}
            onChange={(event) => updateRuntimeSettings({ ...profile.runtimeSettings, soundEnabled: event.target.checked })}
          />
        </label>
      </section>
    </div>
  )
}
