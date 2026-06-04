import { Focus, ScanSearch } from 'lucide-react'
import type { AppProfile } from '../../shared/types/profile'
import { callBackend } from '../../shared/api/client'
import { Button } from '../../shared/ui/Button'
import { useState } from 'react'

interface AppGuardSettingsProps {
  profile: AppProfile
  onProfileChange: (profile: AppProfile) => void
}

export function AppGuardSettings({ profile, onProfileChange }: AppGuardSettingsProps) {
  const [message, setMessage] = useState<string>()
  const [capturing, setCapturing] = useState(false)
  const guard = profile.runtimeSettings.foregroundGuard
  const updateGuard = (foregroundGuard: typeof guard) => {
    onProfileChange({ ...profile, runtimeSettings: { ...profile.runtimeSettings, foregroundGuard } })
  }

  const captureForegroundApp = async () => {
    setCapturing(true)
    setMessage('Switch to the target application now. Capturing in 3 seconds...')
    try {
      await new Promise((resolve) => window.setTimeout(resolve, 3_000))
      const app = await callBackend<{ executable: string; path: string }>('get_foreground_app')
      updateGuard({ ...guard, executable: app.executable })
      setMessage(app.path)
    } catch (error) {
      setMessage(String(error))
    } finally {
      setCapturing(false)
    }
  }

  return (
    <section className="tool-card app-guard-card">
      <div className="settings-card-heading">
        <Focus size={18} />
        <div>
          <strong>Foreground app guard</strong>
          <span>Only allow this profile to act while its target application is foregrounded.</span>
        </div>
      </div>
      <label className="switch-row">
        <span>Enable guard</span>
        <input type="checkbox" checked={guard.enabled} onChange={(event) => updateGuard({ ...guard, enabled: event.target.checked })} />
      </label>
      <label>
        Target executable
        <input value={guard.executable} placeholder="Game.exe" onChange={(event) => updateGuard({ ...guard, executable: event.target.value })} />
      </label>
      <Button icon={ScanSearch} onClick={captureForegroundApp} disabled={capturing}>
        {capturing ? 'Capturing in 3 seconds...' : 'Capture target app'}
      </Button>
      <label>
        When target loses focus
        <select value={guard.onFocusLost} onChange={(event) => updateGuard({ ...guard, onFocusLost: event.target.value as 'pause' | 'stop' })}>
          <option value="pause">Pause and release held actions</option>
          <option value="stop">Stop automation</option>
        </select>
      </label>
      {message ? <div className="notice">{message}</div> : null}
    </section>
  )
}
