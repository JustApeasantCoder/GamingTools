import { Download, FileUp } from 'lucide-react'
import { useState } from 'react'
import { callBackend } from '../../shared/api/client'
import type { AppProfile, ProfileStore } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'

interface ProfileTransferProps {
  profile: AppProfile
  onImported: (store: ProfileStore) => void
}

export function ProfileTransfer({ profile, onImported }: ProfileTransferProps) {
  const [json, setJson] = useState('')
  const [message, setMessage] = useState('Export the active profile or import a profile JSON file.')

  const exportProfile = async () => {
    try {
      const exported = await callBackend<string>('export_profile', { profileId: profile.id })
      setJson(exported)
      const blob = new Blob([exported], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const link = document.createElement('a')
      link.href = url
      link.download = `${safeName(profile.name)}.gaming-profile.json`
      link.click()
      URL.revokeObjectURL(url)
      setMessage(`Exported ${profile.name}.`)
    } catch (error) {
      setMessage(String(error))
    }
  }

  const importProfile = async () => {
    try {
      const store = await callBackend<ProfileStore>('import_profile', { json })
      onImported(store)
      setMessage(`Imported and activated ${store.profiles.find((item) => item.id === store.activeProfileId)?.name ?? 'profile'}.`)
    } catch (error) {
      setMessage(String(error))
    }
  }

  const loadFile = async (file?: File) => {
    if (file) setJson(await file.text())
  }

  return (
    <section className="tool-card transfer-card">
      <div className="settings-card-heading">
        <div>
          <strong>Profile transfer</strong>
          <span>Move complete profiles between machines or keep a backup before experimenting.</span>
        </div>
      </div>
      <div className="transfer-actions">
        <Button icon={Download} variant="primary" onClick={exportProfile}>Export active profile</Button>
        <label className="file-picker">
          Load JSON file
          <input type="file" accept=".json,application/json" onChange={(event) => void loadFile(event.target.files?.[0])} />
        </label>
        <Button icon={FileUp} onClick={importProfile} disabled={!json.trim()}>Import as new profile</Button>
      </div>
      <label>
        Profile JSON
        <textarea value={json} onChange={(event) => setJson(event.target.value)} placeholder="Exported profile JSON appears here." />
      </label>
      <div className="notice">{message}</div>
    </section>
  )
}

function safeName(name: string) {
  return name.trim().replace(/[^a-z0-9_-]+/gi, '-').replace(/^-|-$/g, '') || 'profile'
}
