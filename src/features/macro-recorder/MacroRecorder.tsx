import { Circle, Square } from 'lucide-react'
import { useState } from 'react'
import { callBackend } from '../../shared/api/client'
import type { MacroStep } from '../../shared/types/profile'
import { Button } from '../../shared/ui/Button'

interface MacroRecorderProps {
  onRecorded: (steps: MacroStep[]) => void
  willReplaceExisting?: boolean
}

export function MacroRecorder({ onRecorded, willReplaceExisting = false }: MacroRecorderProps) {
  const [recording, setRecording] = useState(false)
  const [message, setMessage] = useState('Captures supported keyboard and mouse-button presses with measured timing.')

  const start = async () => {
    if (willReplaceExisting && !window.confirm('Recording will replace the current action list. Continue?')) return
    try {
      await callBackend('start_macro_recording')
      setRecording(true)
      setMessage('Recording input now. Return here and stop when finished.')
    } catch (error) {
      setMessage(String(error))
    }
  }

  const stop = async () => {
    try {
      const steps = await callBackend<MacroStep[]>('stop_macro_recording')
      setRecording(false)
      if (steps.length > 0) {
        onRecorded(steps)
        setMessage(`Captured ${steps.length} action${steps.length === 1 ? '' : 's'} into the selected macro.`)
      } else {
        setMessage('No supported input was captured.')
      }
    } catch (error) {
      setRecording(false)
      setMessage(String(error))
    }
  }

  return (
    <div className={recording ? 'recorder-strip recording' : 'recorder-strip'}>
      <span>{message}</span>
      {recording
        ? <Button icon={Square} variant="danger" onClick={stop}>Stop recording</Button>
        : <Button icon={Circle} onClick={start}>Record macro</Button>}
    </div>
  )
}
