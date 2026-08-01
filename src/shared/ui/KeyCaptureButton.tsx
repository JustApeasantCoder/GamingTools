import { useEffect, useState } from 'react'
import { Keyboard } from 'lucide-react'
import { Button } from './Button'
import { callBackend } from '../api/client'

interface KeyCaptureButtonProps {
  value: string
  onChange: (value: string) => void
  label?: string
  className?: string
  allowWheel?: boolean
}

export function KeyCaptureButton({
  value,
  onChange,
  label = 'Change',
  className,
  allowWheel = false,
}: KeyCaptureButtonProps) {
  const [listening, setListening] = useState(false)
  const [captureError, setCaptureError] = useState(false)

  useEffect(() => {
    if (!listening) return
    let finished = false

    const finish = async (nextValue: string) => {
      if (finished) return
      finished = true
      setListening(false)
      const result = await callBackend<{ valid: boolean }>('validate_key_sequence', {
        sequence: [nextValue],
        allowScroll: allowWheel,
      })
        .catch(() => ({ valid: false }))
      if (result.valid) {
        onChange(nextValue)
        setCaptureError(false)
      } else {
        setCaptureError(true)
      }
    }

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault()
      void finish(normalizeKey(event))
    }

    const onMouseDown = (event: MouseEvent) => {
      event.preventDefault()
      void finish(normalizeMouseButton(event.button))
    }

    const onWheel = (event: WheelEvent) => {
      if (!allowWheel || event.deltaY === 0) return
      event.preventDefault()
      void finish(event.deltaY < 0 ? 'SCROLL UP' : 'SCROLL DOWN')
    }

    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault()
    }

    window.addEventListener('keydown', onKeyDown, true)
    window.addEventListener('mousedown', onMouseDown, true)
    window.addEventListener('wheel', onWheel, { capture: true, passive: false })
    window.addEventListener('contextmenu', onContextMenu, true)

    return () => {
      window.removeEventListener('keydown', onKeyDown, true)
      window.removeEventListener('mousedown', onMouseDown, true)
      window.removeEventListener('wheel', onWheel, true)
      window.removeEventListener('contextmenu', onContextMenu, true)
    }
  }, [allowWheel, listening, onChange])

  return (
    <Button className={`key-capture-button${className ? ` ${className}` : ''}`} icon={Keyboard} onClick={() => setListening(true)}>
      {listening ? (
        <span className="key-capture-value">
          {allowWheel ? 'Press a key, mouse button, or scroll...' : 'Press a key or mouse button...'}
        </span>
      ) : captureError ? (
        <span className="key-capture-value key-capture-error">Input not supported</span>
      ) : (
        <span className="key-capture-copy">
          <span className={value.trim() ? 'key-capture-value' : 'key-capture-value key-capture-empty'}>{value.trim() || 'Not selected'}</span>
          <span className="key-capture-label">{label}</span>
        </span>
      )}
    </Button>
  )
}

function normalizeKey(event: KeyboardEvent) {
  if (event.code === 'ControlRight') return 'RIGHT CTRL'
  if (event.code === 'ControlLeft') return 'LEFT CTRL'

  const specialKeys: Record<string, string> = {
    ' ': 'SPACE',
    Spacebar: 'SPACE',
    Escape: 'ESC',
    Control: 'CTRL',
    Alt: 'ALT',
    Shift: 'SHIFT',
    Enter: 'ENTER',
    Tab: 'TAB',
  }

  return specialKeys[event.key] ?? event.key.toUpperCase()
}

function normalizeMouseButton(button: number) {
  if (button === 0) return 'LEFT CLICK'
  if (button === 1) return 'MIDDLE CLICK'
  if (button === 2) return 'RIGHT CLICK'
  if (button === 3) return 'MOUSE 4'
  if (button === 4) return 'MOUSE 5'
  return `MOUSE ${button}`
}
