import { useEffect, useState } from 'react'
import { Keyboard } from 'lucide-react'
import { Button } from './Button'

interface KeyCaptureButtonProps {
  value: string
  onChange: (value: string) => void
  label?: string
}

export function KeyCaptureButton({ value, onChange, label = 'Set key' }: KeyCaptureButtonProps) {
  const [listening, setListening] = useState(false)

  useEffect(() => {
    if (!listening) return

    const finish = (nextValue: string) => {
      onChange(nextValue)
      setListening(false)
    }

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault()
      finish(normalizeKey(event))
    }

    const onMouseDown = (event: MouseEvent) => {
      event.preventDefault()
      finish(normalizeMouseButton(event.button))
    }

    const onContextMenu = (event: MouseEvent) => {
      event.preventDefault()
    }

    window.addEventListener('keydown', onKeyDown, true)
    window.addEventListener('mousedown', onMouseDown, true)
    window.addEventListener('contextmenu', onContextMenu, true)

    return () => {
      window.removeEventListener('keydown', onKeyDown, true)
      window.removeEventListener('mousedown', onMouseDown, true)
      window.removeEventListener('contextmenu', onContextMenu, true)
    }
  }, [listening, onChange])

  return (
    <Button icon={Keyboard} onClick={() => setListening(true)}>
      {listening ? 'Listening...' : `${label}: ${value}`}
    </Button>
  )
}

function normalizeKey(event: KeyboardEvent) {
  if (event.key.length === 1) return event.key.toUpperCase()

  const specialKeys: Record<string, string> = {
    ' ': 'SPACE',
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
  return `MOUSE ${button}`
}
